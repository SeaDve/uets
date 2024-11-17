use chrono::TimeDelta;
use gtk::{glib, subclass::prelude::*};

use crate::{date_time::DateTime, db, entity_id::EntityId, timeline_item_kind::TimelineItemKind};

mod imp {
    use std::cell::OnceCell;

    use super::*;

    #[derive(Default)]
    pub struct TimelineItem {
        pub(super) dt: OnceCell<DateTime>,
        pub(super) kind: OnceCell<TimelineItemKind>,
        pub(super) entity_id: OnceCell<EntityId>,

        pub(super) n_inside: OnceCell<u32>,
        pub(super) pair: glib::WeakRef<super::TimelineItem>,
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
    pub fn new(dt: DateTime, kind: TimelineItemKind, entity_id: EntityId) -> Self {
        let this = glib::Object::new::<Self>();

        let imp = this.imp();
        imp.dt.set(dt).unwrap();
        imp.kind.set(kind).unwrap();
        imp.entity_id.set(entity_id).unwrap();

        this
    }

    pub fn from_db(dt: DateTime, raw: db::RawTimelineItem) -> Self {
        let kind = if raw.is_entry {
            TimelineItemKind::Entry
        } else {
            TimelineItemKind::Exit
        };
        Self::new(dt, kind, raw.entity_id)
    }

    pub fn to_db(&self) -> db::RawTimelineItem {
        db::RawTimelineItem {
            is_entry: self.kind().is_entry(),
            entity_id: self.entity_id().clone(),
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
        self.imp().n_inside.get().copied().unwrap_or(0)
    }

    pub fn set_n_inside(&self, n_inside: u32) {
        self.imp().n_inside.set(n_inside).unwrap();
    }

    pub fn pair(&self) -> Option<TimelineItem> {
        self.imp().pair.upgrade()
    }

    pub fn set_pair(&self, pair: &TimelineItem) {
        debug_assert_ne!(self, pair);
        debug_assert_ne!(self.kind(), pair.kind());

        self.imp().pair.set(Some(pair));
    }

    pub fn entry_to_exit_duration(&self) -> Option<TimeDelta> {
        match self.kind() {
            TimelineItemKind::Entry => {
                let exit_item = self.pair()?;
                Some(exit_item.dt().inner() - self.dt().inner())
            }
            TimelineItemKind::Exit => {
                let entry_item = self.pair().expect("exit item without entry pair");
                Some(self.dt().inner() - entry_item.dt().inner())
            }
        }
    }
}
