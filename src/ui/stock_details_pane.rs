use gtk::{
    glib::{self, clone, closure_local},
    prelude::*,
    subclass::prelude::*,
};

use crate::{
    report::{self, ReportKind},
    stock::Stock,
    stock_timeline::StockTimeline,
    time_graph,
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

        pub(super) timeline_bindings: glib::BindingGroup,
        pub(super) timeline_signals: OnceCell<glib::SignalGroup>,
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

            self.timeline_bindings
                .bind("n-inside", &*self.n_inside_row, "value")
                .transform_to(|_, n_inside| {
                    let n_inside = n_inside.get::<u32>().unwrap();
                    Some(n_inside.to_string().into())
                })
                .flags(glib::BindingFlags::SYNC_CREATE)
                .build();

            let timeline_signals = glib::SignalGroup::new::<StockTimeline>();
            timeline_signals.connect_local(
                "items-changed",
                false,
                clone!(
                    #[weak]
                    obj,
                    #[upgrade_or_panic]
                    move |_| {
                        obj.update_graph_data();
                        None
                    }
                ),
            );
            self.timeline_signals.set(timeline_signals).unwrap();
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

            self.timeline_bindings
                .set_source(stock.as_ref().map(|s| s.timeline()));

            self.timeline_signals
                .get()
                .unwrap()
                .set_target(stock.as_ref().map(|s| s.timeline()));

            self.stock.replace(stock);
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

    async fn handle_share_report(&self, kind: ReportKind) {
        let imp = self.imp();

        let stock = imp.stock.borrow().as_ref().unwrap().clone();
        let stock_id = stock.id();
        let n_inside = stock.timeline().n_inside();
        let timeline_items = stock.timeline().iter().collect::<Vec<_>>();

        let bytes_fut = async {
            let time_graph_image = time_graph::draw_image(
                (800, 500),
                &timeline_items
                    .iter()
                    .map(|item| (item.dt().inner(), item.n_inside()))
                    .collect::<Vec<_>>(),
            )?;

            report::builder(kind, "Stock Report")
                .prop("Name", stock_id)
                .prop("Current Stock Count", n_inside)
                .image("Time Graph", time_graph_image)
                .table(
                    "Timeline",
                    ["Timestamp", "Stock Count"],
                    timeline_items.iter().map(|item| {
                        [
                            item.dt().inner().format("%Y-%m-%dT%H:%M:%S").to_string(),
                            item.n_inside().to_string(),
                        ]
                    }),
                )
                .build()
                .await
        };

        if let Err(err) = WormholeWindow::send(
            bytes_fut,
            &report::file_name(&format!("Stock Report for “{}”", stock_id), kind),
            self,
        )
        .await
        {
            tracing::error!("Failed to send report: {:?}", err);

            Application::get().add_message_toast("Failed to share report");
        }
    }

    fn update_graph_data(&self) {
        let imp = self.imp();

        let data = self
            .stock()
            .map(|stock| {
                stock
                    .timeline()
                    .iter()
                    .map(|item| (item.dt().inner(), item.n_inside()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        imp.graph.set_data(data);
    }
}
