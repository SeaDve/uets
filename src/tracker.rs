use gtk::{glib, subclass::prelude::*};

use crate::{entity::Entity, entity_id::EntityId};

mod imp {
    use std::{
        cell::RefCell,
        collections::{HashMap, HashSet},
    };

    use super::*;

    #[derive(Default)]
    pub struct Tracker {
        pub(super) entities: RefCell<HashMap<EntityId, Entity>>,
        pub(super) inside_entities: RefCell<HashSet<EntityId>>,
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

    pub fn handle_entity(&self, id: &EntityId) {
        let imp = self.imp();

        imp.entities
            .borrow_mut()
            .entry(id.clone())
            .or_insert_with_key(Entity::new);

        if imp.inside_entities.borrow().contains(id) {
            imp.inside_entities.borrow_mut().remove(id);
        } else {
            imp.inside_entities.borrow_mut().insert(id.clone());
        }
    }
}

impl Default for Tracker {
    fn default() -> Self {
        Self::new()
    }
}
