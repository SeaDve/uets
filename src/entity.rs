use gtk::{glib, subclass::prelude::*};

use crate::entity_id::EntityId;

mod imp {
    use std::cell::OnceCell;

    use super::*;

    #[derive(Default)]
    pub struct Entity {
        pub(super) id: OnceCell<EntityId>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Entity {
        const NAME: &'static str = "UetsEntity";
        type Type = super::Entity;
    }

    impl ObjectImpl for Entity {}
}

glib::wrapper! {
    pub struct Entity(ObjectSubclass<imp::Entity>);
}

impl Entity {
    pub fn new(id: &EntityId) -> Self {
        let obj = glib::Object::new::<Self>();

        let imp = obj.imp();
        imp.id.set(id.clone()).unwrap();

        obj
    }
}
