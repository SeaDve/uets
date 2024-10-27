use gtk::{gio, glib, prelude::*, subclass::prelude::*};
use indexmap::{map::Entry, IndexMap};

use crate::{entity::Entity, entity_id::EntityId};

mod imp {
    use std::cell::RefCell;

    use super::*;

    #[derive(Default)]
    pub struct EntityList {
        pub(super) list: RefCell<IndexMap<EntityId, Entity>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EntityList {
        const NAME: &'static str = "UetsEntityList";
        type Type = super::EntityList;
        type Interfaces = (gio::ListModel,);
    }

    impl ObjectImpl for EntityList {}

    impl ListModelImpl for EntityList {
        fn item_type(&self) -> glib::Type {
            Entity::static_type()
        }

        fn n_items(&self) -> u32 {
            self.list.borrow().len() as u32
        }

        fn item(&self, position: u32) -> Option<glib::Object> {
            self.list
                .borrow()
                .get_index(position as usize)
                .map(|(_, v)| v.upcast_ref::<glib::Object>())
                .cloned()
        }
    }
}

glib::wrapper! {
    pub struct EntityList(ObjectSubclass<imp::EntityList>)
        @implements gio::ListModel;
}

impl EntityList {
    /// Must only be accessed by Timeline
    pub fn from_raw(value: IndexMap<EntityId, Entity>) -> Self {
        let this = glib::Object::new::<Self>();

        let imp = this.imp();
        imp.list.replace(value);

        this
    }

    pub fn len(&self) -> usize {
        self.imp().list.borrow().len()
    }

    pub fn get(&self, id: &EntityId) -> Option<Entity> {
        self.imp().list.borrow().get(id).cloned()
    }

    pub fn insert(&self, entity: Entity) {
        let imp = self.imp();

        let id = entity.id();
        let (index, removed, added) = match imp.list.borrow_mut().entry(id.clone()) {
            Entry::Occupied(entry) => (entry.index(), 1, 1),
            Entry::Vacant(entry) => {
                let index = entry.index();
                entry.insert(entity);
                (index, 0, 1)
            }
        };

        self.items_changed(index as u32, removed, added);
    }

    pub fn clear(&self) {
        let imp = self.imp();

        let prev_len = imp.list.borrow().len();

        if prev_len == 0 {
            return;
        }

        imp.list.borrow_mut().clear();
        self.items_changed(0, prev_len as u32, 0);
    }
}
