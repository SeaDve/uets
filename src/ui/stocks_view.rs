use anyhow::Result;
use gtk::{
    glib::{self, clone, closure, closure_local},
    prelude::*,
    subclass::prelude::*,
};

use crate::{
    fuzzy_filter::FuzzyFilter,
    list_model_enum,
    report::{self, ReportKind},
    search_query::SearchQueries,
    stock::Stock,
    stock_id::StockId,
    stock_list::StockList,
    ui::{
        search_entry::SearchEntry, stock_details_pane::StockDetailsPane, stock_row::StockRow,
        wormhole_window::WormholeWindow,
    },
    utils::new_sorter,
    Application,
};

struct S;

impl S {
    const SORT: &str = "sort";
    const SORT_VALUES: &[&str] = &[
        Self::ID_ASC,
        Self::ID_DESC,
        Self::COUNT_ASC,
        Self::COUNT_DESC,
        Self::UPDATED_ASC,
        Self::UPDATED_DESC,
    ];

    const ID_ASC: &str = "id-asc";
    const ID_DESC: &str = "id-desc";
    const COUNT_ASC: &str = "count-asc";
    const COUNT_DESC: &str = "count-desc";
    const UPDATED_ASC: &str = "updated-asc";
    const UPDATED_DESC: &str = "updated-desc";
}

#[derive(Debug, Default, Clone, Copy, glib::Enum)]
#[enum_type(name = "UetsStockSort")]
enum StockSort {
    #[default]
    IdAsc,
    IdDesc,
    CountAsc,
    CountDesc,
    UpdatedAsc,
    UpdatedDesc,
}

list_model_enum!(StockSort);

impl StockSort {
    fn display(&self) -> &'static str {
        match self {
            StockSort::IdAsc => "A-Z",
            StockSort::IdDesc => "Z-A",
            StockSort::CountAsc => "Least Count",
            StockSort::CountDesc => "Most Count",
            StockSort::UpdatedAsc => "Least Recently Updated",
            StockSort::UpdatedDesc => "Recently Updated",
        }
    }
}

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
        pub(super) stock_sort_dropdown: TemplateChild<gtk::DropDown>,
        #[template_child]
        pub(super) n_results_label: TemplateChild<gtk::Label>,
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

        pub(super) stock_sort_dropdown_selected_item_id: OnceCell<glib::SignalHandlerId>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for StocksView {
        const NAME: &'static str = "UetsStocksView";
        type Type = super::StocksView;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            StockRow::ensure_type();

            klass.bind_template();

            klass.install_action_async("stocks-view.share-report", None, |obj, _, _| async move {
                obj.handle_share_report().await;
            });
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
                    obj.update_fallback_sorter();
                    obj.update_n_results_label();
                }
            ));

            self.stock_sort_dropdown
                .set_expression(Some(&gtk::ClosureExpression::new::<String>(
                    &[] as &[gtk::Expression],
                    closure!(|list_item: adw::EnumListItem| {
                        StockSort::try_from(list_item.value()).unwrap().display()
                    }),
                )));
            self.stock_sort_dropdown
                .set_model(Some(&StockSort::new_model()));
            let stock_sort_dropdown_selected_item_notify_id = self
                .stock_sort_dropdown
                .connect_selected_item_notify(clone!(
                    #[weak]
                    obj,
                    move |dropdown| {
                        obj.handle_stock_sort_dropdown_selected_item_notify(dropdown);
                    }
                ));
            self.stock_sort_dropdown_selected_item_id
                .set(stock_sort_dropdown_selected_item_notify_id)
                .unwrap();

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
                    obj.update_n_results_label();
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

            obj.update_fallback_sorter();
            obj.update_n_results_label();
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

        // Clear search filter so we can find the stock
        imp.search_entry.set_queries(&SearchQueries::new());

        let position = imp
            .selection_model
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

    async fn handle_share_report(&self) {
        let imp = self.imp();

        let stocks = imp
            .selection_model
            .iter::<glib::Object>()
            .map(|o| o.unwrap().downcast::<Stock>().unwrap())
            .collect::<Vec<_>>();

        let bytes_fut = report::builder(ReportKind::Pdf, "Stocks Report")
            .prop(
                "Total Stock Count",
                stocks.iter().map(|s| s.timeline().n_inside()).sum::<u32>(),
            )
            .prop("Search Query", imp.search_entry.queries())
            .table(
                "Stocks",
                ["ID", "Count"],
                stocks.iter().map(|stock| {
                    [
                        stock.id().to_string(),
                        stock.timeline().n_inside().to_string(),
                    ]
                }),
            )
            .build();

        if let Err(err) = WormholeWindow::send(
            bytes_fut,
            &report::file_name("Stocks Report", ReportKind::Pdf),
            self,
        )
        .await
        {
            tracing::error!("Failed to send report: {:?}", err);

            Application::get().add_message_toast("Failed to share report");
        }
    }

    fn handle_stock_sort_dropdown_selected_item_notify(&self, dropdown: &gtk::DropDown) {
        let imp = self.imp();

        let selected_item = dropdown
            .selected_item()
            .unwrap()
            .downcast::<adw::EnumListItem>()
            .unwrap();

        let mut queries = imp.search_entry.queries();

        match selected_item.value().try_into().unwrap() {
            StockSort::IdAsc => queries.replace_all_or_insert(S::SORT, S::SORT_VALUES, S::ID_ASC),
            StockSort::IdDesc => queries.replace_all_or_insert(S::SORT, S::SORT_VALUES, S::ID_DESC),
            StockSort::CountAsc => {
                queries.replace_all_or_insert(S::SORT, S::SORT_VALUES, S::COUNT_ASC)
            }
            StockSort::CountDesc => {
                queries.replace_all_or_insert(S::SORT, S::SORT_VALUES, S::COUNT_DESC)
            }
            StockSort::UpdatedAsc => {
                queries.replace_all_or_insert(S::SORT, S::SORT_VALUES, S::UPDATED_ASC)
            }
            StockSort::UpdatedDesc => {
                queries.replace_all_or_insert(S::SORT, S::SORT_VALUES, S::UPDATED_DESC)
            }
        }

        imp.search_entry.set_queries(&queries);
    }

    fn update_fallback_sorter(&self) {
        let imp = self.imp();

        let queries = imp.search_entry.queries();
        let stock_sort = match queries.find_last_match(S::SORT, S::SORT_VALUES) {
            Some(S::ID_ASC) => StockSort::IdAsc,
            Some(S::ID_DESC) => StockSort::IdDesc,
            Some(S::COUNT_ASC) => StockSort::CountAsc,
            Some(S::COUNT_DESC) => StockSort::CountDesc,
            Some(S::UPDATED_ASC) => StockSort::UpdatedAsc,
            Some(S::UPDATED_DESC) => StockSort::UpdatedDesc,
            _ => StockSort::default(),
        };

        let selected_item_notify_id = imp.stock_sort_dropdown_selected_item_id.get().unwrap();
        imp.stock_sort_dropdown
            .block_signal(selected_item_notify_id);
        imp.stock_sort_dropdown.set_selected(stock_sort.position());
        imp.stock_sort_dropdown
            .unblock_signal(selected_item_notify_id);

        let sorter = match stock_sort {
            StockSort::IdAsc | StockSort::IdDesc => {
                new_sorter(matches!(stock_sort, StockSort::IdDesc), |a: &Stock, b| {
                    a.id().cmp(b.id())
                })
            }
            StockSort::CountAsc | StockSort::CountDesc => new_sorter(
                matches!(stock_sort, StockSort::CountDesc),
                |a: &Stock, b| a.timeline().n_inside().cmp(&b.timeline().n_inside()),
            ),
            StockSort::UpdatedAsc | StockSort::UpdatedDesc => new_sorter(
                matches!(stock_sort, StockSort::UpdatedDesc),
                |a: &Stock, b| {
                    a.timeline()
                        .last()
                        .map(|i| i.dt())
                        .cmp(&b.timeline().last().map(|i| i.dt()))
                },
            ),
        };

        imp.fuzzy_filter
            .get()
            .unwrap()
            .sorter()
            .set_fallback_sorter(Some(sorter));
    }

    fn update_n_results_label(&self) {
        let imp = self.imp();

        let n_total = imp
            .selection_model
            .iter::<glib::Object>()
            .map(|o| {
                let stock = o.unwrap().downcast::<Stock>().unwrap();
                stock.timeline().n_inside()
            })
            .sum::<u32>();
        let text = if imp.search_entry.queries().is_empty() {
            format!("Total: {}", n_total)
        } else {
            format!("Results: {}", n_total)
        };

        imp.n_results_label.set_label(&text);
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
