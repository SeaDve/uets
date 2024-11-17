use std::fmt;

use chrono::{DateTime, Utc};
use gtk::{glib, prelude::*, subclass::prelude::*};

use crate::{date_time_range::DateTimeRange, db, log::Log, stock_id::StockId};

mod imp {
    use std::{
        cell::{OnceCell, RefCell},
        marker::PhantomData,
    };

    use super::*;

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::Stock)]
    pub struct Stock {
        #[property(get = Self::n_inside)]
        pub(super) n_inside: PhantomData<u32>,

        pub(super) id: OnceCell<StockId>,

        pub(super) n_inside_log: RefCell<Log<u32>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Stock {
        const NAME: &'static str = "UetsStock";
        type Type = super::Stock;
    }

    #[glib::derived_properties]
    impl ObjectImpl for Stock {}

    impl Stock {
        fn n_inside(&self) -> u32 {
            self.n_inside_log.borrow().latest().copied().unwrap_or(0)
        }
    }
}

glib::wrapper! {
    pub struct Stock(ObjectSubclass<imp::Stock>);
}

impl Stock {
    pub fn new(id: StockId) -> Self {
        let this = glib::Object::new::<Self>();

        let imp = this.imp();
        imp.id.set(id).unwrap();

        this
    }

    pub fn from_db(id: StockId, _raw: db::RawStock) -> Self {
        let this = glib::Object::new::<Self>();

        let imp = this.imp();
        imp.id.set(id).unwrap();

        this
    }

    pub fn to_db(&self) -> db::RawStock {
        db::RawStock {}
    }

    pub fn id(&self) -> &StockId {
        self.imp().id.get().unwrap()
    }

    pub fn n_inside_for_dt(&self, dt: DateTime<Utc>) -> u32 {
        self.imp()
            .n_inside_log
            .borrow()
            .for_dt(dt)
            .copied()
            .unwrap_or(0)
    }

    pub fn n_inside_for_dt_range(&self, dt_range: &DateTimeRange) -> u32 {
        if let Some(end) = dt_range.end {
            self.n_inside_for_dt(end)
        } else {
            self.n_inside()
        }
    }

    pub fn last_action_dt(&self) -> Option<DateTime<Utc>> {
        self.imp().n_inside_log.borrow().latest_dt()
    }

    pub fn with_n_inside_log_mut(&self, f: impl FnOnce(&mut Log<u32>) -> bool) {
        if f(&mut self.imp().n_inside_log.borrow_mut()) {
            self.notify_n_inside();
        }
    }
}

impl fmt::Display for Stock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Stock").field("id", self.id()).finish()
    }
}
