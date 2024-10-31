use gtk::{glib, subclass::prelude::*};

use crate::date_time::DateTime;

mod imp {
    use std::cell::OnceCell;

    use super::*;

    #[derive(Default)]
    pub struct StockTimelineItem {
        pub(super) dt: OnceCell<DateTime>,
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
    pub fn new(dt: DateTime, n_inside: u32) -> Self {
        let this = glib::Object::new::<Self>();

        let imp = this.imp();
        imp.dt.set(dt).unwrap();
        imp.n_inside.set(n_inside).unwrap();

        this
    }

    pub fn dt(&self) -> DateTime {
        *self.imp().dt.get().unwrap()
    }

    pub fn n_inside(&self) -> u32 {
        *self.imp().n_inside.get().unwrap()
    }
}
