use std::fmt;

use gtk::{glib, subclass::prelude::*};

use crate::{entity_data::EntityData, entity_id::EntityId, entity_kind::EntityKind};

mod imp {
    use std::cell::{OnceCell, RefCell};

    use super::*;

    #[derive(Default)]
    pub struct Entity {
        pub(super) id: OnceCell<EntityId>,
        pub(super) data: RefCell<Option<EntityData>>,
        pub(super) entry_dts: RefCell<Vec<glib::DateTime>>,
        pub(super) exit_dts: RefCell<Vec<glib::DateTime>>,
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

    pub fn id(&self) -> &EntityId {
        let imp = self.imp();
        imp.id.get().unwrap()
    }

    pub fn is_inside(&self) -> bool {
        let imp = self.imp();
        imp.entry_dts.borrow().len() > imp.exit_dts.borrow().len()
    }

    pub fn kind(&self) -> EntityKind {
        let imp = self.imp();

        match imp.data.borrow().as_ref() {
            None => EntityKind::Generic,
            Some(data) => match data {
                EntityData::Inventory(_) => EntityKind::Inventory,
                EntityData::Refrigerator(_) => EntityKind::Refrigerator,
                EntityData::Attendance(_) => EntityKind::Attendance,
            },
        }
    }

    pub fn with_data(&self, cb: impl FnOnce(&mut EntityData)) {
        let imp = self.imp();

        if let Some(data) = imp.data.borrow_mut().as_mut() {
            cb(data);
        }
    }

    pub fn last_entry_dt(&self) -> Option<glib::DateTime> {
        let imp = self.imp();
        imp.entry_dts.borrow().last().cloned()
    }

    pub fn last_exit_dt(&self) -> Option<glib::DateTime> {
        let imp = self.imp();
        imp.exit_dts.borrow().last().cloned()
    }

    pub fn add_entry_dt(&self, dt: glib::DateTime) {
        let imp = self.imp();
        imp.entry_dts.borrow_mut().push(dt);
    }

    pub fn add_exit_dt(&self, dt: glib::DateTime) {
        let imp = self.imp();
        imp.exit_dts.borrow_mut().push(dt);
    }
}

impl fmt::Display for Entity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let imp = self.imp();

        f.debug_struct("Entity")
            .field("id", self.id())
            .field("is-inside", &self.is_inside())
            .field("n-entries", &imp.entry_dts.borrow().len())
            .field(
                "last-entry-dt",
                &self.last_entry_dt().map(|dt| dt.format_iso8601().unwrap()),
            )
            .field("n-exits", &imp.exit_dts.borrow().len())
            .field(
                "last-exit-dt",
                &self.last_exit_dt().map(|dt| dt.format_iso8601().unwrap()),
            )
            .finish()
    }
}
