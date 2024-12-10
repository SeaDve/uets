use chrono::{DateTime, Utc};
use gtk::{glib, prelude::*, subclass::prelude::*};

use crate::{
    date_time_range::DateTimeRange, entity_data::EntityData, entity_id::EntityId, log::Log,
    stock_id::StockId, timeline_item_kind::TimelineItemKind,
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

        pub(super) action_log: RefCell<Log<TimelineItemKind>>,
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
            self.action_log
                .borrow()
                .latest()
                .is_some_and(|kind| kind.is_entry())
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
            .action_log
            .borrow()
            .for_dt(dt)
            .is_some_and(|kind| kind.is_entry())
    }

    pub fn is_inside_for_dt_range(&self, dt_range: &DateTimeRange) -> bool {
        if let Some(end) = dt_range.end {
            self.is_inside_for_dt(end)
        } else {
            self.is_inside()
        }
    }

    pub fn action_for_dt_range(
        &self,
        dt_range: &DateTimeRange,
    ) -> Option<(DateTime<Utc>, TimelineItemKind)> {
        let imp = self.imp();

        if let Some(end) = dt_range.end {
            imp.action_log
                .borrow()
                .for_dt_full(end)
                .map(|(dt, kind)| (dt, *kind))
        } else {
            imp.action_log
                .borrow()
                .latest_full()
                .map(|(dt, kind)| (dt, *kind))
        }
    }

    pub fn last_action_dt(&self) -> Option<DateTime<Utc>> {
        self.imp().action_log.borrow().latest_dt()
    }

    pub fn with_action_log_mut(&self, f: impl FnOnce(&mut Log<TimelineItemKind>)) {
        let prev_is_inside = self.is_inside();

        f(&mut self.imp().action_log.borrow_mut());

        if prev_is_inside != self.is_inside() {
            self.notify_is_inside();
        }
    }
}
