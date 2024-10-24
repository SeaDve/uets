use gtk::{gio, glib, prelude::*, subclass::prelude::*};
use indexmap::map::Entry;

use crate::{entity::Entity, entity_id::EntityId};

mod imp {
    use std::cell::RefCell;

    use indexmap::IndexMap;

    use super::*;

    #[derive(Default)]
    pub struct Tracker {
        pub(super) entities: RefCell<IndexMap<EntityId, Entity>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Tracker {
        const NAME: &'static str = "UetsTracker";
        type Type = super::Tracker;
        type Interfaces = (gio::ListModel,);
    }

    impl ObjectImpl for Tracker {}
    impl ListModelImpl for Tracker {
        fn item_type(&self) -> glib::Type {
            Entity::static_type()
        }

        fn n_items(&self) -> u32 {
            self.entities.borrow().len() as u32
        }

        fn item(&self, position: u32) -> Option<glib::Object> {
            self.entities
                .borrow()
                .get_index(position as usize)
                .map(|(_, v)| v.upcast_ref::<glib::Object>())
                .cloned()
        }
    }
}

glib::wrapper! {
    pub struct Tracker(ObjectSubclass<imp::Tracker>)
        @implements gio::ListModel;
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

        let (entity, index, removed, added) = match imp.entities.borrow_mut().entry(id.clone()) {
            Entry::Occupied(entry) => (entry.get().clone(), entry.index(), 1, 1),
            Entry::Vacant(entry) => {
                let index = entry.index();
                let entity = entry.insert(Entity::new(id));
                (entity.clone(), index, 0, 1)
            }
        };

        let now = glib::DateTime::now_utc().unwrap();
        if entity.is_inside() {
            entity.add_exit_dt(now);
        } else {
            entity.add_entry_dt(now);
        }

        self.items_changed(index as u32, removed, added);
    }
}

impl Default for Tracker {
    fn default() -> Self {
        Self::new()
    }
}
