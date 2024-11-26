use std::fmt;

use chrono::{DateTime, Utc};
use gtk::{glib, prelude::*, subclass::prelude::*};

use crate::{
    date_time_boxed::DateTimeBoxed, date_time_range::DateTimeRange, log::Log,
    stock_data::StockData, stock_id::StockId,
};

#[derive(Default)]
pub struct StockLogs {
    pub n_inside: Log<u32>,
    pub max_n_inside: Log<u32>,
    pub n_entries: Log<u32>,
    pub n_exits: Log<u32>,
    pub last_entry_dt: Log<DateTime<Utc>>,
    pub last_exit_dt: Log<DateTime<Utc>>,
}

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
        #[property(get = Self::max_n_inside)]
        pub(super) max_n_inside: PhantomData<u32>,
        #[property(get = Self::n_entries)]
        pub(super) n_entries: PhantomData<u32>,
        #[property(get = Self::n_exits)]
        pub(super) n_exits: PhantomData<u32>,
        #[property(get = Self::last_entry_dt)]
        pub(super) last_entry_dt: PhantomData<Option<DateTimeBoxed>>,
        #[property(get = Self::last_exit_dt)]
        pub(super) last_exit_dt: PhantomData<Option<DateTimeBoxed>>,

        pub(super) id: OnceCell<StockId>,
        pub(super) data: OnceCell<StockData>,

        pub(super) logs: RefCell<StockLogs>,
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
            self.logs.borrow().n_inside.latest().copied().unwrap_or(0)
        }

        fn max_n_inside(&self) -> u32 {
            self.logs
                .borrow()
                .max_n_inside
                .latest()
                .copied()
                .unwrap_or(0)
        }

        fn n_entries(&self) -> u32 {
            self.logs.borrow().n_entries.latest().copied().unwrap_or(0)
        }

        fn n_exits(&self) -> u32 {
            self.logs.borrow().n_exits.latest().copied().unwrap_or(0)
        }

        fn last_entry_dt(&self) -> Option<DateTimeBoxed> {
            self.logs
                .borrow()
                .last_entry_dt
                .latest()
                .copied()
                .map(DateTimeBoxed)
        }

        fn last_exit_dt(&self) -> Option<DateTimeBoxed> {
            self.logs
                .borrow()
                .last_exit_dt
                .latest()
                .copied()
                .map(DateTimeBoxed)
        }
    }
}

glib::wrapper! {
    pub struct Stock(ObjectSubclass<imp::Stock>);
}

impl Stock {
    pub fn new(id: StockId, data: StockData) -> Self {
        let this = glib::Object::new::<Self>();

        let imp = this.imp();
        imp.id.set(id).unwrap();
        imp.data.set(data).unwrap();

        this
    }

    pub fn id(&self) -> &StockId {
        self.imp().id.get().unwrap()
    }

    pub fn data(&self) -> &StockData {
        self.imp().data.get().unwrap()
    }

    pub fn n_inside_for_dt(&self, dt: DateTime<Utc>) -> u32 {
        self.imp()
            .logs
            .borrow()
            .n_inside
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
        self.imp().logs.borrow().n_inside.latest_dt()
    }

    pub fn max_n_inside_for_dt(&self, dt: DateTime<Utc>) -> u32 {
        self.imp()
            .logs
            .borrow()
            .max_n_inside
            .for_dt(dt)
            .copied()
            .unwrap_or(0)
    }

    pub fn max_n_inside_for_dt_range(&self, dt_range: &DateTimeRange) -> u32 {
        if let Some(end) = dt_range.end {
            self.max_n_inside_for_dt(end)
        } else {
            self.max_n_inside()
        }
    }

    pub fn n_entries_for_dt(&self, dt: DateTime<Utc>) -> u32 {
        self.imp()
            .logs
            .borrow()
            .n_entries
            .for_dt(dt)
            .copied()
            .unwrap_or(0)
    }

    pub fn n_entries_for_dt_range(&self, dt_range: &DateTimeRange) -> u32 {
        if let Some(end) = dt_range.end {
            self.n_entries_for_dt(end)
        } else {
            self.n_entries()
        }
    }

    pub fn n_exits_for_dt(&self, dt: DateTime<Utc>) -> u32 {
        self.imp()
            .logs
            .borrow()
            .n_exits
            .for_dt(dt)
            .copied()
            .unwrap_or(0)
    }

    pub fn n_exits_for_dt_range(&self, dt_range: &DateTimeRange) -> u32 {
        if let Some(end) = dt_range.end {
            self.n_exits_for_dt(end)
        } else {
            self.n_exits()
        }
    }

    pub fn last_entry_dt_for_dt(&self, dt: DateTime<Utc>) -> Option<DateTime<Utc>> {
        self.imp().logs.borrow().last_entry_dt.for_dt(dt).copied()
    }

    pub fn last_entry_dt_for_dt_range(&self, dt_range: &DateTimeRange) -> Option<DateTime<Utc>> {
        if let Some(end) = dt_range.end {
            self.last_entry_dt_for_dt(end)
        } else {
            self.last_entry_dt().map(|dt_boxed| dt_boxed.0)
        }
    }

    pub fn last_exit_dt_for_dt(&self, dt: DateTime<Utc>) -> Option<DateTime<Utc>> {
        self.imp().logs.borrow().last_exit_dt.for_dt(dt).copied()
    }

    pub fn last_exit_dt_for_dt_range(&self, dt_range: &DateTimeRange) -> Option<DateTime<Utc>> {
        if let Some(end) = dt_range.end {
            self.last_exit_dt_for_dt(end)
        } else {
            self.last_exit_dt().map(|dt_boxed| dt_boxed.0)
        }
    }

    pub fn with_logs_mut(&self, f: impl FnOnce(&mut StockLogs)) {
        let prev_n_inside = self.n_inside();
        let prev_max_n_inside = self.max_n_inside();
        let prev_n_entries = self.n_entries();
        let prev_n_exits = self.n_exits();
        let prev_last_entry_dt = self.last_entry_dt();
        let prev_last_exit_dt = self.last_exit_dt();

        f(&mut self.imp().logs.borrow_mut());

        if prev_n_inside != self.n_inside() {
            self.notify_n_inside();
        }

        if prev_max_n_inside != self.max_n_inside() {
            self.notify_max_n_inside();
        }

        if prev_n_entries != self.n_entries() {
            self.notify_n_entries();
        }

        if prev_n_exits != self.n_exits() {
            self.notify_n_exits();
        }

        if prev_last_entry_dt != self.last_entry_dt() {
            self.notify_last_entry_dt();
        }

        if prev_last_exit_dt != self.last_exit_dt() {
            self.notify_last_exit_dt();
        }
    }
}

impl fmt::Display for Stock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Stock").field("id", self.id()).finish()
    }
}
