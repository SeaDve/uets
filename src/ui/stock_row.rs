use gtk::{
    glib::{self, clone},
    prelude::*,
    subclass::prelude::*,
};

use crate::{
    date_time_range::DateTimeRange,
    limit_reached::{LabelExt, SettingsExt},
    stock::Stock,
    Application,
};

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
        pub(super) avatar: TemplateChild<adw::Avatar>,
        #[template_child]
        pub(super) title_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) n_inside_label: TemplateChild<gtk::Label>,

        pub(super) dt_range: RefCell<DateTimeRange>,

        pub(super) stock_signals: OnceCell<glib::SignalGroup>,
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

            let stock_signals = glib::SignalGroup::new::<Stock>();
            stock_signals.connect_notify_local(
                Some("n-inside"),
                clone!(
                    #[weak]
                    obj,
                    move |_, _| {
                        obj.update_n_inside_label();
                    }
                ),
            );
            self.stock_signals.set(stock_signals).unwrap();

            let app = Application::get();

            app.settings().connect_operation_mode_changed(clone!(
                #[weak]
                obj,
                move |_| {
                    obj.update_avatar_icon_name();
                }
            ));
            app.settings().connect_limit_reached_changed(clone!(
                #[weak]
                obj,
                move |_| {
                    obj.update_n_inside_label();
                }
            ));

            obj.update_n_inside_label();
            obj.update_avatar_icon_name();
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

                self.avatar.set_text(Some(&stock.id().to_string()));
            } else {
                self.title_label.set_label("");

                self.avatar.set_text(None);
            }

            self.stock_signals.get().unwrap().set_target(stock.as_ref());

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

    pub fn set_dt_range(&self, dt_range: DateTimeRange) {
        let imp = self.imp();
        imp.dt_range.replace(dt_range);
        self.update_n_inside_label();
    }

    fn update_n_inside_label(&self) {
        let imp = self.imp();

        if let Some(stock) = self.stock() {
            let n_inside = stock.n_inside_for_dt_range(&imp.dt_range.borrow());
            imp.n_inside_label
                .set_label_from_limit_reached(n_inside, Application::get().settings());
        } else {
            imp.n_inside_label.set_label("");
        }
    }

    fn update_avatar_icon_name(&self) {
        let imp = self.imp();

        imp.avatar.set_icon_name(
            Application::get()
                .settings()
                .operation_mode()
                .stocks_view_icon_name(),
        );
    }
}
