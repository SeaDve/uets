use chrono::{DateTime, Utc};
use gtk::{glib, prelude::*, subclass::prelude::*};

use crate::{
    date_time_range::DateTimeRange, entity_data::EntityData, entity_id::EntityId, log::Log,
    stock_id::StockId,
};

mod imp {
    use std::{
        cell::{OnceCell, RefCell},
        marker::PhantomData,
    };

    use super::*;

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::Entity)]
    pub struct Entity {
        #[property(get, set = Self::set_data, explicit_notify)]
        pub(super) data: RefCell<EntityData>,
        #[property(get = Self::is_inside)]
        pub(super) is_inside: PhantomData<bool>,

        pub(super) id: OnceCell<EntityId>,

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
        fn set_data(&self, data: EntityData) {
            let obj = self.obj();

            if data == *self.data.borrow() {
                return;
            }

            self.data.replace(data);
            obj.notify_data();
        }

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
    pub fn new(id: EntityId, data: EntityData) -> Self {
        let this = glib::Object::builder::<Self>()
            .property("data", data)
            .build();

        let imp = this.imp();
        imp.id.set(id).unwrap();

        this
    }

    pub fn id(&self) -> &EntityId {
        self.imp().id.get().unwrap()
    }

    pub fn stock_id(&self) -> Option<StockId> {
        self.imp().data.borrow().stock_id().cloned()
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

    pub fn with_is_inside_log_mut(&self, f: impl FnOnce(&mut Log<bool>)) {
        let prev_is_inside = self.is_inside();

        f(&mut self.imp().is_inside_log.borrow_mut());

        if prev_is_inside != self.is_inside() {
            self.notify_is_inside();
        }
    }
}
