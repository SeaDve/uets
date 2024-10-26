use std::fmt;

use gtk::{glib, subclass::prelude::*};

use crate::{
    date_time::DateTime, date_time_pair::DateTimePair, db, entity_data::EntityData,
    entity_id::EntityId, entity_kind::EntityKind,
};

mod imp {
    use std::cell::{OnceCell, RefCell};

    use super::*;

    #[derive(Default)]
    pub struct Entity {
        pub(super) id: OnceCell<EntityId>,
        pub(super) data: RefCell<Option<EntityData>>,
        pub(super) dt_pairs: RefCell<Vec<DateTimePair>>,
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
        imp.dt_pairs
            .borrow_mut()
            .extend(raw.dt_pairs.into_iter().map(DateTimePair::from_db));

        this
    }

    pub fn to_db(&self) -> db::RawEntity {
        let imp = self.imp();

        db::RawEntity {
            dt_pairs: imp
                .dt_pairs
                .borrow()
                .iter()
                .map(|dt_pair| dt_pair.to_db())
                .collect(),
        }
    }

    pub fn id(&self) -> &EntityId {
        self.imp().id.get().unwrap()
    }

    pub fn dt_pairs(&self) -> Vec<DateTimePair> {
        self.imp().dt_pairs.borrow().clone()
    }

    pub fn last_dt_pair(&self) -> Option<DateTimePair> {
        self.imp().dt_pairs.borrow().last().cloned()
    }

    pub fn kind(&self) -> EntityKind {
        let imp = self.imp();

        match imp.data.borrow().as_ref() {
            None => EntityKind::Counter,
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

    pub fn is_inside(&self) -> bool {
        let imp = self.imp();

        imp.dt_pairs
            .borrow()
            .last()
            .is_some_and(|last_dt_pair| last_dt_pair.exit.is_none())
    }

    pub fn add_entry_dt(&self, dt: DateTime) {
        let imp = self.imp();

        if let Some(last_dt_pair) = imp.dt_pairs.borrow().last() {
            debug_assert!(last_dt_pair.exit.is_some(), "double entry");
        }

        imp.dt_pairs.borrow_mut().push(DateTimePair {
            entry: dt,
            exit: None,
        });
    }

    pub fn add_exit_dt(&self, dt: DateTime) {
        let imp = self.imp();

        let mut dt_pairs = imp.dt_pairs.borrow_mut();

        if let Some(pair) = dt_pairs.last_mut() {
            let prev_exit = pair.exit.replace(dt);
            debug_assert_eq!(prev_exit, None, "double exit");
        } else {
            unreachable!("exit without entry");
        }
    }
}

impl fmt::Display for Entity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let imp = self.imp();

        f.debug_struct("Entity")
            .field("id", self.id())
            .field("is-inside", &self.is_inside())
            .field("dt-pairs", &imp.dt_pairs.borrow())
            .finish()
    }
}
