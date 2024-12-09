use std::collections::HashSet;

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

    pub fn iter(&self) -> impl Iterator<Item = Entity> + '_ {
        ListModelExtManual::iter(self).map(|item| item.unwrap())
    }

    pub fn insert(&self, entity: Entity) -> bool {
        let imp = self.imp();

        let (index, removed, added) = match imp.list.borrow_mut().entry(entity.id().clone()) {
            Entry::Occupied(entry) => (entry.index(), 1, 1),
            Entry::Vacant(entry) => {
                let index = entry.index();
                entry.insert(entity);
                (index, 0, 1)
            }
        };

        self.items_changed(index as u32, removed, added);

        removed == 0
    }

    pub fn insert_many(&self, entities: Vec<Entity>) -> u32 {
        let mut updated_indices = HashSet::new();
        let mut n_appended = 0;

        {
            let mut list = self.imp().list.borrow_mut();

            for entity in entities {
                let (index, prev_value) = list.insert_full(entity.id().clone(), entity);

                if prev_value.is_some() {
                    updated_indices.insert(index);
                } else {
                    n_appended += 1;
                }
            }
        }

        let index_of_first_append = self.n_items() - n_appended;

        // Emit about the appended items first, so GTK would know about
        // the new items and it won't error out because the n_items
        // does not match what GTK expect
        if n_appended != 0 {
            self.items_changed(index_of_first_append, 0, n_appended);
        }

        // This is emitted individually because each updated item
        // may be on different indices
        for index in updated_indices {
            // Only emit if the updated item is before the first appended item
            // because it is already handled by the emission above
            if (index as u32) < index_of_first_append {
                self.items_changed(index as u32, 1, 1);
            }
        }

        n_appended
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
