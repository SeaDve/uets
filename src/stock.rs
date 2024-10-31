use std::fmt;

use gtk::{glib, subclass::prelude::*};

use crate::{db, stock_id::StockId, stock_timeline::StockTimeline};

mod imp {
    use std::cell::OnceCell;

    use super::*;

    #[derive(Default)]
    pub struct Stock {
        pub(super) id: OnceCell<StockId>,
        pub(super) timeline: OnceCell<StockTimeline>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Stock {
        const NAME: &'static str = "UetsStock";
        type Type = super::Stock;
    }

    impl ObjectImpl for Stock {}
}

glib::wrapper! {
    pub struct Stock(ObjectSubclass<imp::Stock>);
}

impl Stock {
    pub fn new(id: StockId) -> Self {
        let this = glib::Object::new::<Self>();

        let imp = this.imp();
        imp.id.set(id).unwrap();
        imp.timeline.set(StockTimeline::new()).unwrap();

        this
    }

    pub fn from_db(id: StockId, _raw: db::RawStock, stock_timeline: StockTimeline) -> Self {
        let this = glib::Object::new::<Self>();

        let imp = this.imp();
        imp.id.set(id).unwrap();
        imp.timeline.set(stock_timeline).unwrap();

        this
    }

    pub fn to_db(&self) -> db::RawStock {
        db::RawStock {}
    }

    pub fn id(&self) -> &StockId {
        self.imp().id.get().unwrap()
    }

    pub fn timeline(&self) -> &StockTimeline {
        self.imp().timeline.get().unwrap()
    }
}

impl fmt::Display for Stock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Stock").field("id", self.id()).finish()
    }
}
