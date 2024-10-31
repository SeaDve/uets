use gtk::{glib, subclass::prelude::*};

use crate::{
    date_time::DateTime, db, entity_id::EntityId, stock_id::StockId,
    timeline_item_kind::TimelineItemKind,
};

mod imp {
    use std::cell::OnceCell;

    use super::*;

    #[derive(Default)]
    pub struct TimelineItem {
        pub(super) dt: OnceCell<DateTime>,
        pub(super) kind: OnceCell<TimelineItemKind>,
        /// Id of the entity associated with this item.
        pub(super) entity_id: OnceCell<EntityId>,
        /// Id of the stock associated with this item.
        pub(super) stock_id: OnceCell<Option<StockId>>,
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
    pub fn new(
        dt: DateTime,
        kind: TimelineItemKind,
        entity_id: EntityId,
        stock_id: Option<StockId>,
        n_inside: u32,
    ) -> Self {
        let this = glib::Object::new::<Self>();

        let imp = this.imp();
        imp.dt.set(dt).unwrap();
        imp.kind.set(kind).unwrap();
        imp.entity_id.set(entity_id).unwrap();
        imp.stock_id.set(stock_id).unwrap();
        imp.n_inside.set(n_inside).unwrap();

        this
    }

    pub fn from_db(
        dt: DateTime,
        raw: db::RawTimelineItem,
        stock_id: Option<StockId>,
        n_inside: u32,
    ) -> Self {
        Self::new(
            dt,
            TimelineItemKind::from_db(raw.kind),
            raw.entity_id,
            stock_id,
            n_inside,
        )
    }

    pub fn to_db(&self) -> db::RawTimelineItem {
        db::RawTimelineItem {
            kind: self.kind().to_db(),
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

    pub fn stock_id(&self) -> Option<&StockId> {
        self.imp().stock_id.get().unwrap().as_ref()
    }

    pub fn n_inside(&self) -> u32 {
        *self.imp().n_inside.get().unwrap()
    }
}
