use gtk::{
    glib::{self, clone, closure_local},
    prelude::*,
    subclass::prelude::*,
};

use crate::{
    stock::Stock,
    ui::{information_row::InformationRow, stock_graph::StockGraph},
};

mod imp {
    use std::{cell::RefCell, sync::OnceLock};

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
        pub(super) graph: TemplateChild<StockGraph>,

        pub(super) timeline_bindings: glib::BindingGroup,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for StockDetailsPane {
        const NAME: &'static str = "UetsStockDetailsPane";
        type Type = super::StockDetailsPane;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
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
        }

        fn dispose(&self) {
            self.dispose_template();
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();

            SIGNALS.get_or_init(|| vec![Signal::builder("close-request").build()])
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

            self.graph
                .set_timeline(stock.as_ref().map(|s| s.timeline().clone()));

            self.stock.replace(stock);
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

    pub fn connect_close_request<F>(&self, f: F) -> glib::SignalHandlerId
    where
        F: Fn(&Self) + 'static,
    {
        self.connect_closure("close-request", false, closure_local!(|obj: &Self| f(obj)))
    }
}
