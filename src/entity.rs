use std::fmt;

use gtk::{glib, subclass::prelude::*};

use crate::{
    date_time::DateTime, db, entity_data::EntityData, entity_id::EntityId, entity_kind::EntityKind,
};

mod imp {
    use std::cell::{OnceCell, RefCell};

    use super::*;

    #[derive(Default)]
    pub struct Entity {
        pub(super) id: OnceCell<EntityId>,
        pub(super) data: RefCell<Option<EntityData>>,
        pub(super) entry_dts: RefCell<Vec<DateTime>>,
        pub(super) exit_dts: RefCell<Vec<DateTime>>,
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
        let this = glib::Object::new::<Self>();

        let imp = this.imp();
        imp.id.set(id.clone()).unwrap();

        this
    }

    pub fn from_db(id: &EntityId, raw: db::RawEntity) -> Self {
        let this = Self::new(id);

        let imp = this.imp();
        imp.entry_dts.replace(raw.entry_dts);
        imp.exit_dts.replace(raw.exit_dts);

        this
    }

    pub fn to_db(&self) -> db::RawEntity {
        let imp = self.imp();

        db::RawEntity {
            entry_dts: imp.entry_dts.borrow().clone(),
            exit_dts: imp.exit_dts.borrow().clone(),
        }
    }

    pub fn id(&self) -> &EntityId {
        self.imp().id.get().unwrap()
    }

    pub fn is_inside(&self) -> bool {
        let imp = self.imp();

        let n_entries = imp.entry_dts.borrow().len();
        let n_exits = imp.exit_dts.borrow().len();

        match (n_entries).abs_diff(n_exits) {
            0 => false,
            1 => true,
            2.. => unreachable!("diff must always be less than 1"),
        }
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

    pub fn entry_dts(&self) -> Vec<DateTime> {
        self.imp().entry_dts.borrow().clone()
    }

    pub fn exit_dts(&self) -> Vec<DateTime> {
        self.imp().exit_dts.borrow().clone()
    }

    pub fn last_entry_dt(&self) -> Option<DateTime> {
        self.imp().entry_dts.borrow().last().cloned()
    }

    pub fn last_exit_dt(&self) -> Option<DateTime> {
        self.imp().exit_dts.borrow().last().cloned()
    }

    pub fn add_entry_dt(&self, dt: DateTime) {
        self.imp().entry_dts.borrow_mut().push(dt);
    }

    pub fn add_exit_dt(&self, dt: DateTime) {
        self.imp().exit_dts.borrow_mut().push(dt);
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
                &self.last_entry_dt().map(|dt| dt.format_iso8601()),
            )
            .field("n-exits", &imp.exit_dts.borrow().len())
            .field(
                "last-exit-dt",
                &self.last_exit_dt().map(|dt| dt.format_iso8601()),
            )
            .finish()
    }
}
