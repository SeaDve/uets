use std::fmt;

use chrono::{DateTime, Utc};
use gtk::{glib, prelude::*, subclass::prelude::*};

use crate::{date_time_range::DateTimeRange, db, entity_id::EntityId, log::Log, stock_id::StockId};

mod imp {
    use std::{
        cell::{OnceCell, RefCell},
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

        pub(super) is_inside_log: RefCell<Log<bool>>,
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
            self.is_inside_log
                .borrow()
                .latest()
                .copied()
                .unwrap_or(false)
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

    pub fn is_inside_for_dt(&self, dt: DateTime<Utc>) -> bool {
        self.imp()
            .is_inside_log
            .borrow()
            .for_dt(dt)
            .copied()
            .unwrap_or(false)
    }

    pub fn is_inside_for_dt_range(&self, dt_range: &DateTimeRange) -> bool {
        if let Some(end) = dt_range.end {
            self.is_inside_for_dt(end)
        } else {
            self.is_inside()
        }
    }

    pub fn last_action_dt(&self) -> Option<DateTime<Utc>> {
        self.imp().is_inside_log.borrow().latest_dt()
    }

    pub fn with_is_inside_log_mut(&self, f: impl FnOnce(&mut Log<bool>) -> bool) {
        if f(&mut self.imp().is_inside_log.borrow_mut()) {
            self.notify_is_inside();
        }
    }
}

impl fmt::Display for Entity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Entity")
            .field("id", self.id())
            .field("stock-id", &self.stock_id())
            .field("is-inside", &self.is_inside())
            .finish()
    }
}
