use gtk::{
    glib::{self, clone},
    prelude::*,
    subclass::prelude::*,
};

use crate::{
    stock::Stock,
    stock_id::StockId,
    stock_list::StockList,
    ui::{stock_details_pane::StockDetailsPane, stock_row::StockRow},
};

mod imp {
    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/seadve/Uets/ui/stocks_view.ui")]
    pub struct StocksView {
        #[template_child]
        pub(super) flap: TemplateChild<adw::Flap>,
        #[template_child]
        pub(super) stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) empty_page: TemplateChild<adw::StatusPage>,
        #[template_child]
        pub(super) main_page: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub(super) list_view: TemplateChild<gtk::ListView>,
        #[template_child]
        pub(super) selection_model: TemplateChild<gtk::SingleSelection>,
        #[template_child]
        pub(super) details_pane: TemplateChild<StockDetailsPane>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for StocksView {
        const NAME: &'static str = "UetsStocksView";
        type Type = super::StocksView;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            StockRow::ensure_type();

            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for StocksView {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            self.selection_model
                .bind_property("selected-item", &*self.flap, "reveal-flap")
                .transform_to(|_, stock: Option<Stock>| Some(stock.is_some()))
                .sync_create()
                .build();
            self.selection_model
                .bind_property("selected-item", &*self.details_pane, "stock")
                .sync_create()
                .build();

            self.details_pane.connect_close_request(clone!(
                #[weak]
                obj,
                move |_| {
                    obj.imp()
                        .selection_model
                        .set_selected(gtk::INVALID_LIST_POSITION);
                }
            ));
        }

        fn dispose(&self) {
            self.dispose_template();
        }
    }

    impl WidgetImpl for StocksView {}
}

glib::wrapper! {
    pub struct StocksView(ObjectSubclass<imp::StocksView>)
        @extends gtk::Widget;
}

impl StocksView {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn bind_stock_list(&self, stock_list: &StockList) {
        let imp = self.imp();

        stock_list.connect_items_changed(clone!(
            #[weak(rename_to = obj)]
            self,
            move |_, _, _, _| {
                obj.update_stack();
            }
        ));

        imp.selection_model.set_model(Some(stock_list));

        self.update_stack();
    }

    pub fn show(&self, stock_id: &StockId) {
        let imp = self.imp();

        let position = self
            .stock_list()
            .get_index_of(stock_id)
            .expect("stock must exist") as u32;

        imp.selection_model.set_selected(position);
    }

    fn stock_list(&self) -> StockList {
        self.imp()
            .selection_model
            .model()
            .unwrap()
            .downcast()
            .unwrap()
    }

    fn update_stack(&self) {
        let imp = self.imp();

        if self.stock_list().is_empty() {
            imp.stack.set_visible_child(&*imp.empty_page);
        } else {
            imp.stack.set_visible_child(&*imp.main_page);
        }
    }
}
