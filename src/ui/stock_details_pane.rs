use gtk::{
    glib::{self, clone, closure_local},
    prelude::*,
    subclass::prelude::*,
};

use crate::{
    date_time_range::DateTimeRange,
    report::{self, ReportKind},
    report_table,
    stock::Stock,
    ui::{information_row::InformationRow, time_graph::TimeGraph, wormhole_window::WormholeWindow},
    Application,
};

mod imp {
    use std::{
        cell::{OnceCell, RefCell},
        sync::OnceLock,
    };

    use glib::subclass::Signal;

    use super::*;

    #[derive(Default, glib::Properties, gtk::CompositeTemplate)]
    #[properties(wrapper_type = super::StockDetailsPane)]
    #[template(resource = "/io/github/seadve/Uets/ui/stock_details_pane.ui")]
    pub struct StockDetailsPane {
        #[property(get, set = Self::set_stock, explicit_notify)]
        pub(super) stock: RefCell<Option<Stock>>,

        #[template_child]
        pub(super) vbox: TemplateChild<gtk::Box>, // Unused
        #[template_child]
        pub(super) close_image: TemplateChild<gtk::Image>,
        #[template_child]
        pub(super) id_row: TemplateChild<InformationRow>,
        #[template_child]
        pub(super) n_inside_row: TemplateChild<InformationRow>,
        #[template_child]
        pub(super) graph: TemplateChild<TimeGraph>,

        pub(super) dt_range: RefCell<DateTimeRange>,

        pub(super) stock_signals: OnceCell<glib::SignalGroup>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for StockDetailsPane {
        const NAME: &'static str = "UetsStockDetailsPane";
        type Type = super::StockDetailsPane;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action("stock-details-pane.show-timeline", None, |obj, _, _| {
                obj.emit_by_name::<()>("show-timeline-request", &[]);
            });
            klass.install_action("stock-details-pane.show-entities", None, |obj, _, _| {
                obj.emit_by_name::<()>("show-entities-request", &[]);
            });
            klass.install_action_async(
                "stock-details-pane.share-report",
                Some(&ReportKind::static_variant_type()),
                |obj, _, kind| async move {
                    let kind = kind.unwrap().get::<ReportKind>().unwrap();
                    obj.handle_share_report(kind).await;
                },
            );
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for StockDetailsPane {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            let gesture_click = gtk::GestureClick::new();
            gesture_click.connect_released(clone!(
                #[weak]
                obj,
                move |_, n_clicked, _, _| {
                    if n_clicked == 1 {
                        obj.emit_by_name::<()>("close-request", &[]);
                    }
                }
            ));
            self.close_image.add_controller(gesture_click);

            let stock_signals = glib::SignalGroup::new::<Stock>();
            stock_signals.connect_notify_local(
                Some("n-inside"),
                clone!(
                    #[weak]
                    obj,
                    move |_, _| {
                        obj.update_n_inside_row();
                    }
                ),
            );
            self.stock_signals.set(stock_signals).unwrap();

            Application::get().timeline().connect_items_changed(clone!(
                #[weak]
                obj,
                move |_, _, _, _| {
                    obj.update_graph_data();
                }
            ));
        }

        fn dispose(&self) {
            self.dispose_template();
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();

            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("show-timeline-request").build(),
                    Signal::builder("show-entities-request").build(),
                    Signal::builder("close-request").build(),
                ]
            })
        }
    }

    impl WidgetImpl for StockDetailsPane {}

    impl StockDetailsPane {
        fn set_stock(&self, stock: Option<Stock>) {
            let obj = self.obj();

            if stock == obj.stock() {
                return;
            }

            self.id_row.set_value(
                stock
                    .as_ref()
                    .map(|s| s.id().to_string())
                    .unwrap_or_default(),
            );

            self.stock_signals.get().unwrap().set_target(stock.as_ref());

            self.stock.replace(stock);
            obj.update_n_inside_row();
            obj.update_graph_data();
            obj.notify_stock();
        }
    }
}

glib::wrapper! {
    pub struct StockDetailsPane(ObjectSubclass<imp::StockDetailsPane>)
        @extends gtk::Widget;
}

impl StockDetailsPane {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn connect_show_timeline_request<F>(&self, f: F) -> glib::SignalHandlerId
    where
        F: Fn(&Self) + 'static,
    {
        self.connect_closure(
            "show-timeline-request",
            false,
            closure_local!(|obj: &Self| f(obj)),
        )
    }

    pub fn connect_show_entities_request<F>(&self, f: F) -> glib::SignalHandlerId
    where
        F: Fn(&Self) + 'static,
    {
        self.connect_closure(
            "show-entities-request",
            false,
            closure_local!(|obj: &Self| f(obj)),
        )
    }

    pub fn connect_close_request<F>(&self, f: F) -> glib::SignalHandlerId
    where
        F: Fn(&Self) + 'static,
    {
        self.connect_closure("close-request", false, closure_local!(|obj: &Self| f(obj)))
    }

    pub fn set_dt_range(&self, dt_range: DateTimeRange) {
        let imp = self.imp();
        imp.dt_range.replace(dt_range);
        self.update_n_inside_row();
        self.update_graph_data();
    }

    async fn handle_share_report(&self, kind: ReportKind) {
        let imp = self.imp();

        let stock = imp.stock.borrow().as_ref().unwrap().clone();
        let dt_range = *imp.dt_range.borrow();

        let app = Application::get();
        let timeline = app.timeline();

        let bytes_fut = async {
            report::builder(kind, "Stock Report")
                .prop("Stock Name", stock.id())
                .prop("Current Count", stock.n_inside_for_dt_range(&dt_range))
                .table(
                    report_table::builder("Timeline")
                        .column("Timestamp")
                        .column("Action")
                        .column("Entity ID")
                        .column("Count")
                        .rows(timeline.iter_stock(stock.id(), &dt_range).map(|item| {
                            report_table::row_builder()
                                .cell(item.dt())
                                .cell(item.kind().to_string())
                                .cell(item.entity_id().to_string())
                                .cell(stock.n_inside_for_dt(item.dt()))
                                .build()
                        }))
                        .graph("Count Over Time", 0, 3)
                        .build(),
                )
                .build()
                .await
        };

        if let Err(err) = WormholeWindow::send(
            bytes_fut,
            &report::file_name(&format!("Stock Report for “{}”", stock.id()), kind),
            self,
        )
        .await
        {
            tracing::error!("Failed to send report: {:?}", err);

            Application::get().add_message_toast("Failed to share report");
        }
    }

    fn update_n_inside_row(&self) {
        let imp = self.imp();

        let stock = imp.stock.borrow();
        let n_inside = stock
            .as_ref()
            .map(|s| s.n_inside_for_dt_range(&imp.dt_range.borrow()))
            .unwrap_or_default();
        imp.n_inside_row.set_value(n_inside.to_string());
    }

    fn update_graph_data(&self) {
        let imp = self.imp();

        let app = Application::get();
        let timeline = app.timeline();

        let data = self
            .stock()
            .map(|stock| {
                timeline
                    .iter_stock(stock.id(), &imp.dt_range.borrow())
                    .map(|item| (item.dt(), stock.n_inside_for_dt(item.dt())))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        imp.graph.set_data(data);
    }
}
