use chrono::{DateTime, Utc};
use gtk::{glib, prelude::*, subclass::prelude::*};

use crate::{
    date_time, date_time_range::DateTimeRange, entity_data::EntityData, entity_id::EntityId,
    entity_kind::EntityKind, format, log::Log, stock_id::StockId,
    timeline_item_kind::TimelineItemKind,
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

    pub fn kind(&self) -> EntityKind {
        self.imp().data.borrow().kind()
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

    pub fn status_text(&self, for_dt_range: &DateTimeRange) -> String {
        self.status_markup(for_dt_range, false)
    }

    pub fn status_markup(
        &self,
        for_dt_range: &DateTimeRange,
        use_red_markup_on_entry_to_exit_duration: bool,
    ) -> String {
        match self.action_for_dt_range(for_dt_range) {
            Some((dt, TimelineItemKind::Entry)) => {
                let verb = match self.kind() {
                    EntityKind::Person => "Entered",
                    EntityKind::Vehicle => "Drove in",
                    EntityKind::Item => "Added",
                };
                let entry_to_exit_duration_prefix = match self.kind() {
                    EntityKind::Person => "stayed",
                    EntityKind::Vehicle => "parked",
                    EntityKind::Item => "kept",
                };

                let duration_start = if let Some(start) = for_dt_range.start {
                    start.max(dt)
                } else {
                    dt
                };
                let duration_end = if let Some(end) = for_dt_range.end {
                    end
                } else {
                    Utc::now()
                };
                let formatted_duration = format::duration(duration_end - duration_start);

                format!(
                    "{verb} {} and {entry_to_exit_duration_prefix} for {}",
                    date_time::format::fuzzy(dt),
                    if use_red_markup_on_entry_to_exit_duration {
                        format::red_markup(&formatted_duration)
                    } else {
                        formatted_duration
                    },
                )
            }
            Some((dt, TimelineItemKind::Exit)) => {
                let verb = match self.kind() {
                    EntityKind::Person => "Exited",
                    EntityKind::Vehicle => "Drove out",
                    EntityKind::Item => "Removed",
                };
                format!("{verb} {}", date_time::format::fuzzy(dt))
            }
            None => match self.kind() {
                EntityKind::Person => "Never entered".into(),
                EntityKind::Vehicle => "Never drove in".into(),
                EntityKind::Item => "Never added".into(),
            },
        }
    }
}
