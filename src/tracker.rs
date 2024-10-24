use gtk::{glib, subclass::prelude::*};

use crate::{entity::Entity, entity_id::EntityId};

mod imp {
    use std::{cell::RefCell, collections::HashMap};

    use super::*;

    #[derive(Default)]
    pub struct Tracker {
        pub(super) entities: RefCell<HashMap<EntityId, Entity>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Tracker {
        const NAME: &'static str = "UetsTracker";
        type Type = super::Tracker;
    }

    impl ObjectImpl for Tracker {}
}

glib::wrapper! {
    pub struct Tracker(ObjectSubclass<imp::Tracker>);
}

impl Tracker {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn inside_entities(&self) -> Vec<EntityId> {
        let imp = self.imp();

        imp.entities
            .borrow()
            .iter()
            .filter(|(_, entity)| entity.is_inside())
            .map(|(id, _)| id.clone())
            .collect()
    }

    pub fn handle_entity(&self, id: &EntityId) {
        let imp = self.imp();

        let mut entities = imp.entities.borrow_mut();
        let entity = entities.entry(id.clone()).or_insert_with_key(Entity::new);

        let now = glib::DateTime::now_utc().unwrap();

        if entity.is_inside() {
            entity.add_exit_dt(now);
        } else {
            entity.add_entry_dt(now);
        }
    }
}

impl Default for Tracker {
    fn default() -> Self {
        Self::new()
    }
}
