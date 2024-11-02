use gtk::{
    glib::{self, clone, closure_local},
    prelude::*,
    subclass::prelude::*,
};

use crate::{
    fuzzy_filter::FuzzyFilter,
    search_query::SearchQueries,
    stock::Stock,
    stock_id::StockId,
    stock_list::StockList,
    ui::{search_entry::SearchEntry, stock_details_pane::StockDetailsPane, stock_row::StockRow},
};

mod imp {
    use std::{cell::OnceCell, sync::OnceLock};

    use glib::subclass::Signal;

    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/seadve/Uets/ui/stocks_view.ui")]
    pub struct StocksView {
        #[template_child]
        pub(super) flap: TemplateChild<adw::Flap>,
        #[template_child]
        pub(super) search_entry: TemplateChild<SearchEntry>,
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
        pub(super) sort_list_model: TemplateChild<gtk::SortListModel>,
        #[template_child]
        pub(super) filter_list_model: TemplateChild<gtk::FilterListModel>,
        #[template_child]
        pub(super) details_pane: TemplateChild<StockDetailsPane>,

        pub(super) fuzzy_filter: OnceCell<FuzzyFilter>,
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

            self.search_entry.connect_search_changed(clone!(
                #[weak]
                obj,
                move |entry| {
                    obj.handle_search_entry_search_changed(entry);
                }
            ));

            self.selection_model
                .bind_property("selected-item", &*self.flap, "reveal-flap")
                .transform_to(|_, stock: Option<Stock>| Some(stock.is_some()))
                .sync_create()
                .build();
            self.selection_model
                .bind_property("selected-item", &*self.details_pane, "stock")
                .sync_create()
                .build();
            self.selection_model.connect_items_changed(clone!(
                #[weak]
                obj,
                move |_, _, _, _| {
                    obj.update_stack();
                }
            ));

            self.details_pane.connect_show_timeline_request(clone!(
                #[weak]
                obj,
                move |details_pane| {
                    let stock = details_pane.stock().expect("stock must exist");
                    obj.emit_by_name::<()>("show-timeline-request", &[stock.id()]);
                }
            ));
            self.details_pane.connect_show_entities_request(clone!(
                #[weak]
                obj,
                move |details_pane| {
                    let stock = details_pane.stock().expect("stock must exist");
                    obj.emit_by_name::<()>("show-entities-request", &[stock.id()]);
                }
            ));
            self.details_pane.connect_close_request(clone!(
                #[weak]
                obj,
                move |_| {
                    obj.imp()
                        .selection_model
                        .set_selected(gtk::INVALID_LIST_POSITION);
                }
            ));

            let fuzzy_filter = FuzzyFilter::new(|o| {
                let stock = o.downcast_ref::<Stock>().unwrap();
                [Some(stock.id().to_string())]
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>()
                    .join(" ")
            });
            self.sort_list_model.set_sorter(Some(fuzzy_filter.sorter()));
            self.fuzzy_filter.set(fuzzy_filter).unwrap();

            obj.update_stack();
        }

        fn dispose(&self) {
            self.dispose_template();
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();

            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("show-timeline-request")
                        .param_types([StockId::static_type()])
                        .build(),
                    Signal::builder("show-entities-request")
                        .param_types([StockId::static_type()])
                        .build(),
                ]
            })
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

    pub fn connect_show_timeline_request<F>(&self, f: F) -> glib::SignalHandlerId
    where
        F: Fn(&Self, &StockId) + 'static,
    {
        self.connect_closure(
            "show-timeline-request",
            false,
            closure_local!(|obj: &Self, id: &StockId| f(obj, id)),
        )
    }

    pub fn connect_show_entities_request<F>(&self, f: F) -> glib::SignalHandlerId
    where
        F: Fn(&Self, &StockId) + 'static,
    {
        self.connect_closure(
            "show-entities-request",
            false,
            closure_local!(|obj: &Self, id: &StockId| f(obj, id)),
        )
    }

    pub fn bind_stock_list(&self, stock_list: &StockList) {
        let imp = self.imp();

        imp.filter_list_model.set_model(Some(stock_list));
    }

    pub fn show_stock(&self, stock_id: &StockId) {
        let imp = self.imp();

        // Clear search filter so we can find the entity
        imp.search_entry.set_queries(&SearchQueries::new());

        let position = imp
            .filter_list_model
            .iter::<glib::Object>()
            .position(|o| {
                let stock = o.unwrap().downcast::<Stock>().unwrap();
                stock.id() == stock_id
            })
            .expect("stock must exist") as u32;

        imp.selection_model.set_selected(position);

        imp.list_view
            .activate_action("list.scroll-to-item", Some(&position.to_variant()))
            .unwrap();
    }

    fn handle_search_entry_search_changed(&self, entry: &SearchEntry) {
        let imp = self.imp();

        let queries = entry.queries();

        if queries.is_empty() {
            imp.filter_list_model.set_filter(gtk::Filter::NONE);
            return;
        }

        let every_filter = gtk::EveryFilter::new();

        let fuzzy_filter = imp.fuzzy_filter.get().unwrap();
        fuzzy_filter.set_search(
            &queries
                .all_standalones()
                .into_iter()
                .collect::<Vec<_>>()
                .join(" "),
        );
        every_filter.append(fuzzy_filter.clone());

        imp.filter_list_model.set_filter(Some(&every_filter));
    }

    fn update_stack(&self) {
        let imp = self.imp();

        if imp.selection_model.n_items() == 0 {
            imp.stack.set_visible_child(&*imp.empty_page);
        } else {
            imp.stack.set_visible_child(&*imp.main_page);
        }
    }
}
