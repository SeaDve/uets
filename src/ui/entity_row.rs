use gtk::{glib, prelude::*, subclass::prelude::*};

use crate::entity::Entity;

mod imp {
    use std::cell::RefCell;

    use super::*;

    #[derive(Default, glib::Properties, gtk::CompositeTemplate)]
    #[properties(wrapper_type = super::EntityRow)]
    #[template(resource = "/io/github/seadve/Uets/ui/entity_row.ui")]
    pub struct EntityRow {
        #[property(get, set = Self::set_entity, explicit_notify)]
        pub(super) entity: RefCell<Option<Entity>>,

        #[template_child]
        pub(super) hbox: TemplateChild<gtk::Box>, // Unused
        #[template_child]
        pub(super) image: TemplateChild<gtk::Image>,
        #[template_child]
        pub(super) title_label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EntityRow {
        const NAME: &'static str = "UetsEntityRow";
        type Type = super::EntityRow;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for EntityRow {
        fn dispose(&self) {
            self.dispose_template();
        }
    }

    impl WidgetImpl for EntityRow {}

    impl EntityRow {
        fn set_entity(&self, entity: Option<Entity>) {
            let obj = self.obj();

            if entity == obj.entity() {
                return;
            }

            if let Some(entity) = &entity {
                self.title_label.set_label(&entity.id().to_string())
            } else {
                self.title_label.set_label("");
            }

            self.entity.replace(entity);
            obj.notify_entity();
        }
    }
}

glib::wrapper! {
    pub struct EntityRow(ObjectSubclass<imp::EntityRow>)
        @extends gtk::Widget;
}

impl EntityRow {
    pub fn new() -> Self {
        glib::Object::new()
    }
}
