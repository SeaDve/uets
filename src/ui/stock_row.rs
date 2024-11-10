use gtk::{
    glib::{self, clone},
    prelude::*,
    subclass::prelude::*,
};

use crate::{stock::Stock, stock_timeline::StockTimeline};

mod imp {
    use std::cell::{OnceCell, RefCell};

    use super::*;

    #[derive(Default, glib::Properties, gtk::CompositeTemplate)]
    #[properties(wrapper_type = super::StockRow)]
    #[template(resource = "/io/github/seadve/Uets/ui/stock_row.ui")]
    pub struct StockRow {
        #[property(get, set = Self::set_stock, explicit_notify)]
        pub(super) stock: RefCell<Option<Stock>>,

        #[template_child]
        pub(super) hbox: TemplateChild<gtk::Box>, // Unused
        #[template_child]
        pub(super) image: TemplateChild<gtk::Image>,
        #[template_child]
        pub(super) title_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) n_inside_label: TemplateChild<gtk::Label>,

        pub(super) timeline_signals: OnceCell<glib::SignalGroup>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for StockRow {
        const NAME: &'static str = "UetsStockRow";
        type Type = super::StockRow;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for StockRow {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            let timeline_signals = glib::SignalGroup::new::<StockTimeline>();
            timeline_signals.connect_notify_local(
                Some("n-inside"),
                clone!(
                    #[weak]
                    obj,
                    move |_, _| {
                        obj.update_n_inside_label();
                    }
                ),
            );
            self.timeline_signals.set(timeline_signals).unwrap();

            obj.update_n_inside_label();
        }

        fn dispose(&self) {
            self.dispose_template();
        }
    }

    impl WidgetImpl for StockRow {}

    impl StockRow {
        fn set_stock(&self, stock: Option<Stock>) {
            let obj = self.obj();

            if stock == obj.stock() {
                return;
            }

            if let Some(stock) = &stock {
                self.title_label.set_label(&stock.id().to_string());
            } else {
                self.title_label.set_label("");
            }

            self.timeline_signals
                .get()
                .unwrap()
                .set_target(stock.as_ref().map(|s| s.timeline()));

            self.stock.replace(stock);
            obj.update_n_inside_label();
            obj.notify_stock();
        }
    }
}

glib::wrapper! {
    pub struct StockRow(ObjectSubclass<imp::StockRow>)
        @extends gtk::Widget;
}

impl StockRow {
    pub fn new() -> Self {
        glib::Object::new()
    }

    fn update_n_inside_label(&self) {
        let imp = self.imp();

        if let Some(stock) = self.stock() {
            imp.n_inside_label
                .set_label(&stock.timeline().n_inside().to_string());
        } else {
            imp.n_inside_label.set_label("");
        }
    }
}
