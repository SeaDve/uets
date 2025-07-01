use adw::{prelude::*, subclass::prelude::*};
use anyhow::Result;
use gtk::{
    gdk,
    glib::{self, clone},
};

use crate::{
    detector::Detector,
    entity_data::EntityDataField,
    entity_id::EntityId,
    list_model_enum,
    settings::{AccessMode, DetectorConfig},
    Application,
};

list_model_enum!(AccessMode);

mod imp {
    use std::cell::OnceCell;

    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/seadve/Uets/ui/test_window.ui")]
    pub struct TestWindow {
        #[template_child]
        pub(super) toast_overlay: TemplateChild<adw::ToastOverlay>,
        #[template_child]
        pub(super) access_mode_dropdown: TemplateChild<gtk::DropDown>,
        #[template_child]
        pub(super) value_entry: TemplateChild<gtk::Entry>,
        #[template_child]
        pub(super) enter_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) reset_button: TemplateChild<gtk::Button>,

        pub(super) detector: OnceCell<Detector>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TestWindow {
        const NAME: &'static str = "UetsTestWindow";
        type Type = super::TestWindow;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.add_binding_action(gdk::Key::W, gdk::ModifierType::CONTROL_MASK, "window.close");
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for TestWindow {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            self.access_mode_dropdown
                .set_expression(Some(&adw::EnumListItem::this_expression("name")));
            self.access_mode_dropdown
                .set_model(Some(&AccessMode::new_model()));
            self.access_mode_dropdown
                .set_selected(AccessMode::default().model_position());
            self.access_mode_dropdown
                .connect_selected_item_notify(clone!(
                    #[weak]
                    obj,
                    move |dropdown| {
                        let selected_item = dropdown
                            .selected_item()
                            .unwrap()
                            .downcast::<adw::EnumListItem>()
                            .unwrap();
                        obj.detector()
                            .set_access_mode(selected_item.value().try_into().unwrap());
                    }
                ));

            self.value_entry.connect_activate(clone!(
                #[weak]
                obj,
                move |_| {
                    obj.handle_enter();
                }
            ));
            self.enter_button.connect_clicked(clone!(
                #[weak]
                obj,
                move |_| {
                    obj.handle_enter();
                }
            ));
            self.reset_button.connect_clicked(|_button| {
                if let Err(err) = Application::get().timeline().reset() {
                    tracing::error!("Failed to reset timeline: {:?}", err);
                }
            });

            let detector = Detector::new(
                DetectorConfig {
                    name: "Test Detector".to_string(),
                    access_mode: AccessMode::default(),
                    camera_ip_addr: None,
                    rfid_reader_ip_addr: None,
                    remote_app_ip_addr: None,
                },
                Application::get().timeline(),
            );
            self.detector.set(detector).unwrap();
        }
    }

    impl WidgetImpl for TestWindow {}
    impl WindowImpl for TestWindow {}
    impl ApplicationWindowImpl for TestWindow {}
    impl AdwApplicationWindowImpl for TestWindow {}
}

glib::wrapper! {
    pub struct TestWindow(ObjectSubclass<imp::TestWindow>)
        @extends gtk::Widget, gtk::Window, adw::Window, adw::ApplicationWindow;
}

impl TestWindow {
    pub fn new(application: &Application) -> Self {
        glib::Object::builder()
            .property("application", application)
            .build()
    }

    fn detector(&self) -> &Detector {
        self.imp().detector.get().unwrap()
    }

    fn handle_enter(&self) {
        glib::spawn_future_local(clone!(
            #[weak(rename_to = obj)]
            self,
            async move {
                if let Err(err) = obj.handle_enter_inner().await {
                    tracing::error!("Failed to handle enter: {:?}", err);
                }
            }
        ));
    }

    async fn handle_enter_inner(&self) -> Result<()> {
        let imp = self.imp();

        let text = imp.value_entry.text();
        let (entity_id, entity_data) =
            if let Some((raw_entity_id, raw_entity_data)) = text.split_once(':') {
                (
                    EntityId::new(raw_entity_id),
                    serde_json::from_str::<Vec<EntityDataField>>(raw_entity_data)?,
                )
            } else {
                (EntityId::new(text.as_str()), vec![])
            };

        imp.value_entry.set_text("");

        if let Some(message) = Application::get()
            .simulate_detected(self.detector(), &entity_id, entity_data)
            .await
        {
            imp.toast_overlay.add_toast(adw::Toast::new(&message));
        }

        Ok(())
    }
}
