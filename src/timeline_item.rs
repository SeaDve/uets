use chrono::TimeDelta;
use gtk::{glib, subclass::prelude::*};

use crate::{date_time::DateTime, db, entity_id::EntityId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineItemKind {
    Entry,
    Exit { inside_duration: TimeDelta },
}

impl TimelineItemKind {
    pub fn from_db(raw: &db::RawTimelineItemKind) -> Self {
        match raw {
            db::RawTimelineItemKind::Entry => Self::Entry,
            db::RawTimelineItemKind::Exit {
                inside_duration: raw_inside_duration,
            } => Self::Exit {
                inside_duration: TimeDelta::from_std(*raw_inside_duration).unwrap(),
            },
        }
    }

    pub fn to_db(self) -> db::RawTimelineItemKind {
        match self {
            Self::Entry => db::RawTimelineItemKind::Entry,
            Self::Exit { inside_duration } => db::RawTimelineItemKind::Exit {
                inside_duration: inside_duration.to_std().unwrap(),
            },
        }
    }
}

mod imp {
    use std::cell::OnceCell;

    use super::*;

    #[derive(Default)]
    pub struct TimelineItem {
        pub(super) dt: OnceCell<DateTime>,
        pub(super) kind: OnceCell<TimelineItemKind>,
        /// Id of the entity associated with this item.
        pub(super) entity_id: OnceCell<EntityId>,
        /// Number of entity inside at this dt point.
        pub(super) n_inside: OnceCell<u32>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TimelineItem {
        const NAME: &'static str = "UetsTimelineItem";
        type Type = super::TimelineItem;
    }

    impl ObjectImpl for TimelineItem {}
}

glib::wrapper! {
    pub struct TimelineItem(ObjectSubclass<imp::TimelineItem>);
}

impl TimelineItem {
    pub fn new(dt: DateTime, kind: TimelineItemKind, entity_id: EntityId, n_inside: u32) -> Self {
        let this = glib::Object::new::<Self>();

        let imp = this.imp();
        imp.dt.set(dt).unwrap();
        imp.kind.set(kind).unwrap();
        imp.entity_id.set(entity_id).unwrap();
        imp.n_inside.set(n_inside).unwrap();

        this
    }

    pub fn from_db(dt: DateTime, raw: &db::RawTimelineItem) -> Self {
        Self::new(
            dt,
            TimelineItemKind::from_db(&raw.kind),
            raw.entity_id.clone(),
            raw.n_inside,
        )
    }

    pub fn to_db(&self) -> db::RawTimelineItem {
        db::RawTimelineItem {
            kind: self.kind().to_db(),
            entity_id: self.entity_id().clone(),
            n_inside: self.n_inside(),
        }
    }

    pub fn dt(&self) -> DateTime {
        *self.imp().dt.get().unwrap()
    }

    pub fn kind(&self) -> TimelineItemKind {
        *self.imp().kind.get().unwrap()
    }

    pub fn entity_id(&self) -> &EntityId {
        self.imp().entity_id.get().unwrap()
    }

    pub fn n_inside(&self) -> u32 {
        *self.imp().n_inside.get().unwrap()
    }
}
