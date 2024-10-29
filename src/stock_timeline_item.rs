use gtk::{glib, subclass::prelude::*};

use crate::{
    date_time::DateTime, entity_id::EntityId, stock_id::StockId,
    timeline_item_kind::TimelineItemKind,
};

mod imp {
    use std::cell::OnceCell;

    use super::*;

    #[derive(Default)]
    pub struct StockTimelineItem {
        pub(super) dt: OnceCell<DateTime>,
        pub(super) kind: OnceCell<TimelineItemKind>,
        /// Id of the entity associated with this item.
        pub(super) entity_id: OnceCell<EntityId>,
        /// Id of the stock associated with this item.
        pub(super) stock_id: OnceCell<StockId>,
        /// Number of entity inside at this dt point.
        pub(super) n_inside: OnceCell<u32>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for StockTimelineItem {
        const NAME: &'static str = "UetsStockTimelineItem";
        type Type = super::StockTimelineItem;
    }

    impl ObjectImpl for StockTimelineItem {}
}

glib::wrapper! {
    pub struct StockTimelineItem(ObjectSubclass<imp::StockTimelineItem>);
}

impl StockTimelineItem {
    pub fn new(
        dt: DateTime,
        kind: TimelineItemKind,
        entity_id: EntityId,
        stock_id: StockId,
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

    pub fn dt(&self) -> DateTime {
        *self.imp().dt.get().unwrap()
    }

    pub fn kind(&self) -> TimelineItemKind {
        *self.imp().kind.get().unwrap()
    }

    pub fn entity_id(&self) -> &EntityId {
        self.imp().entity_id.get().unwrap()
    }

    pub fn stock_id(&self) -> &StockId {
        self.imp().stock_id.get().unwrap()
    }

    pub fn n_inside(&self) -> u32 {
        *self.imp().n_inside.get().unwrap()
    }
}
