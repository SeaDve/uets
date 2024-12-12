use gtk::{
    glib::{self, clone},
    prelude::*,
    subclass::prelude::*,
};

use crate::{
    entity_expiration::{EntityExpiration, EntityExpirationEntityExt},
    entity_list::EntityList,
};

mod imp {
    use std::cell::{Cell, OnceCell};

    use super::*;

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::EntityExpiredTracker)]
    pub struct EntityExpiredTracker {
        #[property(get)]
        pub(super) n_expired: Cell<u32>,

        pub(super) entity_list: OnceCell<EntityList>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EntityExpiredTracker {
        const NAME: &'static str = "UetsEntityExpiredTracker";
        type Type = super::EntityExpiredTracker;
    }

    #[glib::derived_properties]
    impl ObjectImpl for EntityExpiredTracker {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            obj.update_n_expired();
        }
    }
}

glib::wrapper! {
    pub struct EntityExpiredTracker(ObjectSubclass<imp::EntityExpiredTracker>);
}

impl EntityExpiredTracker {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn bind_entity_list(&self, entity_list: &EntityList) {
        let imp = self.imp();

        entity_list.connect_items_changed(clone!(
            #[weak(rename_to = obj)]
            self,
            move |_, _, _, _| {
                obj.update_n_expired();
            }
        ));

        imp.entity_list.set(entity_list.clone()).unwrap();

        self.update_n_expired();
    }

    fn update_n_expired(&self) {
        let imp = self.imp();

        let n_expired = imp
            .entity_list
            .get()
            .map(|entity_list| {
                entity_list
                    .iter()
                    .filter(|entity| {
                        entity
                            .expiration()
                            .is_some_and(|e| matches!(e, EntityExpiration::Expired))
                    })
                    .count()
            })
            .unwrap_or(0) as u32;

        if n_expired == self.n_expired() {
            return;
        }

        imp.n_expired.set(n_expired);
        self.notify_n_expired();
    }
}

impl Default for EntityExpiredTracker {
    fn default() -> Self {
        Self::new()
    }
}
