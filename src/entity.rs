use std::fmt;

use gtk::{glib, prelude::*, subclass::prelude::*};

use crate::{date_time::DateTime, db, entity_id::EntityId, stock_id::StockId};

#[derive(Debug, Clone, PartialEq, Eq)]
struct DateTimePair {
    entry: DateTime,
    exit: Option<DateTime>,
}

mod imp {
    use std::{
        cell::{OnceCell, RefCell},
        collections::HashMap,
        marker::PhantomData,
    };

    use super::*;

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::Entity)]
    pub struct Entity {
        #[property(get = Self::is_inside)]
        pub(super) is_inside: PhantomData<bool>,

        pub(super) id: OnceCell<EntityId>,
        pub(super) stock_id: OnceCell<Option<StockId>>,
        pub(super) dt_pairs: RefCell<Vec<DateTimePair>>,

        pub(super) inside_durations_on_exit: RefCell<HashMap<DateTime, chrono::TimeDelta>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Entity {
        const NAME: &'static str = "UetsEntity";
        type Type = super::Entity;
    }

    #[glib::derived_properties]
    impl ObjectImpl for Entity {}

    impl Entity {
        fn is_inside(&self) -> bool {
            self.dt_pairs
                .borrow()
                .last()
                .is_some_and(|last_dt_pair| last_dt_pair.exit.is_none())
        }
    }
}

glib::wrapper! {
    pub struct Entity(ObjectSubclass<imp::Entity>);
}

impl Entity {
    pub fn new(id: EntityId, stock_id: Option<StockId>) -> Self {
        let this = glib::Object::new::<Self>();

        let imp = this.imp();
        imp.id.set(id).unwrap();
        imp.stock_id.set(stock_id).unwrap();

        this
    }

    pub fn from_db(id: EntityId, raw: db::RawEntity) -> Self {
        let this = glib::Object::new::<Self>();

        let imp = this.imp();
        imp.id.set(id).unwrap();
        imp.stock_id.set(raw.stock_id).unwrap();

        this
    }

    pub fn to_db(&self) -> db::RawEntity {
        db::RawEntity {
            stock_id: self.stock_id().cloned(),
        }
    }

    pub fn id(&self) -> &EntityId {
        self.imp().id.get().unwrap()
    }

    pub fn stock_id(&self) -> Option<&StockId> {
        self.imp().stock_id.get().unwrap().as_ref()
    }

    pub fn inside_duration_on_exit(&self, exit_dt: DateTime) -> Option<chrono::Duration> {
        self.imp()
            .inside_durations_on_exit
            .borrow()
            .get(&exit_dt)
            .copied()
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

        self.notify_is_inside();
    }

    pub fn add_exit_dt(&self, exit_dt: DateTime) {
        let imp = self.imp();

        if let Some(pair) = imp.dt_pairs.borrow_mut().last_mut() {
            let prev_exit = pair.exit.replace(exit_dt);
            debug_assert_eq!(prev_exit, None, "double exit");

            imp.inside_durations_on_exit
                .borrow_mut()
                .insert(exit_dt, exit_dt.inner() - pair.entry.inner());
        } else {
            unreachable!("exit without entry");
        }

        self.notify_is_inside();
    }
}

impl fmt::Display for Entity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let imp = self.imp();

        f.debug_struct("Entity")
            .field("id", self.id())
            .field("stock-id", &self.stock_id())
            .field("is-inside", &self.is_inside())
            .field("dt-pairs", &imp.dt_pairs.borrow())
            .finish()
    }
}
