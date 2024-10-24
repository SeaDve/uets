use adw::{prelude::*, subclass::prelude::*};
use gtk::glib;

use crate::{application::Application, entity::Entity, entity_id::EntityId};

mod imp {
    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(file = "window.ui")]
    pub struct Window {
        #[template_child]
        pub(super) test_entry: TemplateChild<gtk::Entry>,
        #[template_child]
        pub(super) test_all_listbox: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub(super) test_inside_listbox: TemplateChild<gtk::ListBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Window {
        const NAME: &'static str = "UetsWindow";
        type Type = super::Window;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Window {
        fn constructed(&self) {
            self.parent_constructed();

            self.test_entry.connect_activate(move |entry| {
                let id = EntityId::new(entry.text());
                entry.set_text("");
                Application::get().detector().simulate_detected(&id);
            });

            self.test_all_listbox
                .bind_model(Some(Application::get().tracker()), |entity| {
                    let entity = entity.downcast_ref::<Entity>().unwrap();

                    let label = gtk::Label::builder()
                        .label(entity.to_string())
                        .wrap(true)
                        .build();
                    label.upcast()
                });

            let filter = gtk::CustomFilter::new(|entity| {
                let entity = entity.downcast_ref::<Entity>().unwrap();
                entity.is_inside()
            });
            let filter_list_model =
                gtk::FilterListModel::new(Some(Application::get().tracker().clone()), Some(filter));
            self.test_inside_listbox
                .bind_model(Some(&filter_list_model), |entity| {
                    let entity = entity.downcast_ref::<Entity>().unwrap();

                    let label = gtk::Label::builder()
                        .label(entity.to_string())
                        .wrap(true)
                        .build();
                    label.upcast()
                });
        }
    }

    impl WidgetImpl for Window {}
    impl WindowImpl for Window {}
    impl ApplicationWindowImpl for Window {}
    impl AdwApplicationWindowImpl for Window {}
}

glib::wrapper! {
    pub struct Window(ObjectSubclass<imp::Window>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow;
}

impl Window {
    pub fn new(application: &Application) -> Self {
        glib::Object::builder()
            .property("application", application)
            .build()
    }
}