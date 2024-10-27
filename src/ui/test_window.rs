use adw::{prelude::*, subclass::prelude::*};
use gtk::{gdk, glib, pango};

use crate::{entity::Entity, entity_id::EntityId, Application};

mod imp {
    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/seadve/Uets/ui/test_window.ui")]
    pub struct TestWindow {
        #[template_child]
        pub(super) entry: TemplateChild<gtk::Entry>,
        #[template_child]
        pub(super) clear_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) all_listbox: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub(super) inside_listbox: TemplateChild<gtk::ListBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TestWindow {
        const NAME: &'static str = "UetsTestWindow";
        type Type = super::TestWindow;
        type ParentType = adw::Window;

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

            self.entry.connect_activate(move |entry| {
                let id = EntityId::new(entry.text());
                entry.set_text("");
                Application::get().detector().simulate_detected(&id);
            });
            self.clear_button.connect_clicked(|_button| {
                if let Err(err) = Application::get().timeline().clear() {
                    eprintln!("Failed to reset timeline: {:?}", err);
                }
            });

            self.all_listbox.bind_model(
                Some(Application::get().timeline().entity_list()),
                |entity| {
                    let entity = entity.downcast_ref::<Entity>().unwrap();

                    let label = gtk::Label::builder()
                        .margin_start(3)
                        .margin_end(3)
                        .xalign(0.0)
                        .label(entity.to_string())
                        .wrap(true)
                        .wrap_mode(pango::WrapMode::WordChar)
                        .build();
                    label.upcast()
                },
            );

            let filter = gtk::CustomFilter::new(|entity| {
                let entity = entity.downcast_ref::<Entity>().unwrap();
                entity.is_inside()
            });
            let filter_list_model = gtk::FilterListModel::new(
                Some(Application::get().timeline().entity_list().clone()),
                Some(filter),
            );
            self.inside_listbox
                .bind_model(Some(&filter_list_model), |entity| {
                    let entity = entity.downcast_ref::<Entity>().unwrap();

                    let label = gtk::Label::builder()
                        .margin_start(3)
                        .margin_end(3)
                        .xalign(0.0)
                        .label(entity.id().to_string())
                        .wrap(true)
                        .wrap_mode(pango::WrapMode::WordChar)
                        .build();
                    label.upcast()
                });
        }
    }

    impl WidgetImpl for TestWindow {}
    impl WindowImpl for TestWindow {}
    impl AdwWindowImpl for TestWindow {}
}

glib::wrapper! {
    pub struct TestWindow(ObjectSubclass<imp::TestWindow>)
        @extends gtk::Widget, gtk::Window, adw::Window;
}

impl TestWindow {
    pub fn new() -> Self {
        glib::Object::new()
    }
}
