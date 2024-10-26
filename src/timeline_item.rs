use gtk::{glib, subclass::prelude::*};

use crate::{date_time::DateTime, entity::Entity};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineItemKind {
    Entry,
    Exit { inside_duration: glib::TimeSpan },
}

mod imp {
    use std::cell::OnceCell;

    use super::*;

    #[derive(Default)]
    pub struct TimelineItem {
        pub(super) kind: OnceCell<TimelineItemKind>,
        pub(super) dt: OnceCell<DateTime>,
        pub(super) entity: OnceCell<Entity>,
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
    pub fn new(kind: TimelineItemKind, dt: DateTime, entity: Entity) -> Self {
        let this = glib::Object::new::<Self>();

        let imp = this.imp();
        imp.kind.set(kind).unwrap();
        imp.dt.set(dt).unwrap();
        imp.entity.set(entity).unwrap();

        this
    }

    pub fn kind(&self) -> TimelineItemKind {
        *self.imp().kind.get().unwrap()
    }

    pub fn dt(&self) -> &DateTime {
        self.imp().dt.get().unwrap()
    }

    pub fn entity(&self) -> &Entity {
        self.imp().entity.get().unwrap()
    }
}
