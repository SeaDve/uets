use gtk::{gio, glib, prelude::*, subclass::prelude::*};
use indexmap::IndexMap;

use crate::{date_time::DateTime, stock_timeline_item::StockTimelineItem};

mod imp {
    use std::{cell::RefCell, marker::PhantomData};

    use super::*;

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::StockTimeline)]
    pub struct StockTimeline {
        #[property(get = Self::n_inside)]
        pub(super) n_inside: PhantomData<u32>,

        pub(super) list: RefCell<IndexMap<DateTime, StockTimelineItem>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for StockTimeline {
        const NAME: &'static str = "UetsStockTimeline";
        type Type = super::StockTimeline;
        type Interfaces = (gio::ListModel,);
    }

    #[glib::derived_properties]
    impl ObjectImpl for StockTimeline {}

    impl ListModelImpl for StockTimeline {
        fn item_type(&self) -> glib::Type {
            StockTimelineItem::static_type()
        }

        fn n_items(&self) -> u32 {
            self.list.borrow().len() as u32
        }

        fn item(&self, position: u32) -> Option<glib::Object> {
            self.list
                .borrow()
                .get_index(position as usize)
                .map(|(_, v)| v.upcast_ref::<glib::Object>().clone())
        }
    }

    impl StockTimeline {
        fn n_inside(&self) -> u32 {
            self.list
                .borrow()
                .last()
                .map_or(0, |(_, item)| item.n_inside())
        }
    }
}

glib::wrapper! {
    /// A timeline with sorted items by date-time.
    pub struct StockTimeline(ObjectSubclass<imp::StockTimeline>)
        @implements gio::ListModel;
}

impl StockTimeline {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn from_raw(raw: IndexMap<DateTime, StockTimelineItem>) -> Self {
        let this = glib::Object::new::<Self>();

        debug_assert!(raw.keys().is_sorted());

        let imp = this.imp();
        imp.list.replace(raw);

        this
    }

    pub fn is_empty(&self) -> bool {
        self.imp().list.borrow().is_empty()
    }

    pub fn first(&self) -> Option<StockTimelineItem> {
        self.imp()
            .list
            .borrow()
            .first()
            .map(|(_, item)| item.clone())
    }

    pub fn last(&self) -> Option<StockTimelineItem> {
        self.imp()
            .list
            .borrow()
            .last()
            .map(|(_, item)| item.clone())
    }

    pub fn iter(&self) -> impl Iterator<Item = StockTimelineItem> + '_ {
        ListModelExtManual::iter(self).map(|item| item.unwrap())
    }

    pub fn insert(&self, item: StockTimelineItem) {
        let imp = self.imp();

        let (index, prev_value) = imp.list.borrow_mut().insert_full(item.dt(), item);
        debug_assert_eq!(prev_value, None);

        self.notify_n_inside();
        self.items_changed(index as u32, 0, 1);

        debug_assert!(imp.list.borrow().keys().is_sorted());
    }

    pub fn reset(&self) {
        let imp = self.imp();

        let prev_len = imp.list.borrow().len();

        if prev_len == 0 {
            return;
        }

        imp.list.borrow_mut().clear();

        self.notify_n_inside();
        self.items_changed(0, prev_len as u32, 0);

        debug_assert!(imp.list.borrow().keys().is_sorted());
    }
}
