use gtk::{
    glib::{self, clone},
    prelude::*,
    subclass::prelude::*,
};

use crate::{
    application::Application,
    limit_reached::{LimitReached, LimitReachedSettingsExt},
    stock_list::StockList,
};

mod imp {
    use std::cell::{Cell, OnceCell};

    use super::*;

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::StockLimitReachedTracker)]
    pub struct StockLimitReachedTracker {
        #[property(get)]
        pub(super) n_lower_limit_reached: Cell<u32>,
        #[property(get)]
        pub(super) n_upper_limit_reached: Cell<u32>,

        pub(super) stock_list: OnceCell<StockList>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for StockLimitReachedTracker {
        const NAME: &'static str = "UetsStockLimitReachedTracker";
        type Type = super::StockLimitReachedTracker;
    }

    #[glib::derived_properties]
    impl ObjectImpl for StockLimitReachedTracker {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            let app = Application::get();
            let settings = app.settings();

            settings.connect_lower_limit_reached_threshold_changed(clone!(
                #[weak]
                obj,
                move |_| {
                    obj.update_n_lower_limit_reached();
                }
            ));
            settings.connect_upper_limit_reached_threshold_changed(clone!(
                #[weak]
                obj,
                move |_| {
                    obj.update_n_upper_limit_reached();
                }
            ));

            obj.update_n_lower_limit_reached();
            obj.update_n_upper_limit_reached();
        }
    }
}

glib::wrapper! {
    pub struct StockLimitReachedTracker(ObjectSubclass<imp::StockLimitReachedTracker>);
}

impl StockLimitReachedTracker {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn bind_stock_list(&self, stock_list: &StockList) {
        let imp = self.imp();

        stock_list.connect_items_changed(clone!(
            #[weak(rename_to = obj)]
            self,
            move |_, _, _, _| {
                obj.update_n_lower_limit_reached();
                obj.update_n_upper_limit_reached();
            }
        ));

        imp.stock_list.set(stock_list.clone()).unwrap();

        self.update_n_lower_limit_reached();
        self.update_n_upper_limit_reached();
    }

    fn update_n_lower_limit_reached(&self) {
        let imp = self.imp();

        let app = Application::get();
        let settings = app.settings();

        let n_lower_limit_reached = imp
            .stock_list
            .get()
            .map(|stock_list| {
                stock_list
                    .iter()
                    .filter(|stock| {
                        settings
                            .compute_limit_reached(stock.n_inside())
                            .is_some_and(|lr| matches!(lr, LimitReached::Lower))
                    })
                    .count()
            })
            .unwrap_or(0) as u32;

        if n_lower_limit_reached == self.n_lower_limit_reached() {
            return;
        }

        imp.n_lower_limit_reached.set(n_lower_limit_reached);
        self.notify_n_lower_limit_reached();
    }

    fn update_n_upper_limit_reached(&self) {
        let imp = self.imp();

        let app = Application::get();
        let settings = app.settings();

        let n_upper_limit_reached = imp
            .stock_list
            .get()
            .map(|stock_list| {
                stock_list
                    .iter()
                    .filter(|stock| {
                        settings
                            .compute_limit_reached(stock.n_inside())
                            .is_some_and(|lr| matches!(lr, LimitReached::Upper))
                    })
                    .count()
            })
            .unwrap_or(0) as u32;

        if n_upper_limit_reached == self.n_upper_limit_reached() {
            return;
        }

        imp.n_upper_limit_reached.set(n_upper_limit_reached);
        self.notify_n_upper_limit_reached();
    }
}

impl Default for StockLimitReachedTracker {
    fn default() -> Self {
        Self::new()
    }
}
