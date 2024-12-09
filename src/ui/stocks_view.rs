use anyhow::Result;
use gtk::{
    glib::{self, clone, closure, closure_local},
    prelude::*,
    subclass::prelude::*,
};

use crate::{
    date_time_range::DateTimeRange,
    fuzzy_filter::FuzzyFilter,
    limit_reached::LimitReachedSettingsExt,
    list_model_enum,
    report::{self, ReportKind},
    report_table,
    search_query::SearchQueries,
    search_query_ext::SearchQueriesDateTimeRangeExt,
    signal_handler_id_group::{SignalHandlerIdGroup, SignalHandlerIdGroupObjectExt},
    stock::Stock,
    stock_id::StockId,
    stock_list::StockList,
    ui::{
        date_time_range_button::DateTimeRangeButton, search_entry::SearchEntry,
        send_dialog::SendDialog, stock_details_pane::StockDetailsPane, stock_row::StockRow,
    },
    utils::{new_filter, new_sorter},
    Application,
};

struct S;

impl S {
    const FROM: &str = "from";
    const TO: &str = "to";

    const IS: &str = "is";

    const LIMIT_REACHED_VALUES: &[&str] = &[
        Self::LIMIT_REACHED,
        Self::LOWER_LIMIT_REACHED,
        Self::UPPER_LIMIT_REACHED,
    ];
    const LIMIT_REACHED: &str = "limit-reached";
    const LOWER_LIMIT_REACHED: &str = "lower-limit-reached";
    const UPPER_LIMIT_REACHED: &str = "upper-limit-reached";

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

#[derive(Debug, Clone, Copy, glib::Enum)]
#[enum_type(name = "UetsLimitReachedFilter")]
enum LimitReachedFilter {
    All,
    LimitReached,
    LowerLimitReached,
    UpperLimitReached,
}

list_model_enum!(LimitReachedFilter);

impl LimitReachedFilter {
    fn display(&self) -> &'static str {
        match self {
            Self::All => "All",
            Self::LimitReached => "Limit Reached",
            Self::LowerLimitReached => "Amount Depleted",
            Self::UpperLimitReached => "Capacity Exceeded",
        }
    }
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

#[allow(deprecated)]
mod imp {
    use std::{
        cell::{OnceCell, RefCell},
        sync::OnceLock,
    };

    use glib::{subclass::Signal, WeakRef};

    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/seadve/Uets/ui/stocks_view.ui")]
    pub struct StocksView {
        #[template_child]
        pub(super) flap: TemplateChild<adw::Flap>,
        #[template_child]
        pub(super) search_entry: TemplateChild<SearchEntry>,
        #[template_child]
        pub(super) limit_reached_dropdown: TemplateChild<gtk::DropDown>,
        #[template_child]
        pub(super) dt_range_button: TemplateChild<DateTimeRangeButton>,
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

        pub(super) dt_range: RefCell<DateTimeRange>,

        pub(super) fuzzy_filter: OnceCell<FuzzyFilter>,

        pub(super) limit_reached_dropdown_selected_item_id: OnceCell<glib::SignalHandlerId>,
        pub(super) dt_range_button_range_notify_id: OnceCell<glib::SignalHandlerId>,
        pub(super) stock_sort_dropdown_selected_item_id: OnceCell<glib::SignalHandlerId>,

        pub(super) settings_limit_reached_id: RefCell<Option<SignalHandlerIdGroup>>,

        pub(super) rows: RefCell<Vec<WeakRef<StockRow>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for StocksView {
        const NAME: &'static str = "UetsStocksView";
        type Type = super::StocksView;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action_async(
                "stocks-view.share-report",
                Some(&ReportKind::static_variant_type()),
                |obj, _, kind| async move {
                    let kind = kind.unwrap().get::<ReportKind>().unwrap();

                    if let Err(err) = SendDialog::send(
                        &report::file_name("Stocks Report", kind),
                        obj.create_report(kind),
                        Some(&obj),
                    )
                    .await
                    {
                        tracing::error!("Failed to send report: {:?}", err);

                        Application::get().add_message_toast("Failed to share report");
                    }
                },
            );
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

            self.limit_reached_dropdown.set_expression(Some(
                &gtk::ClosureExpression::new::<String>(
                    &[] as &[gtk::Expression],
                    closure!(|list_item: adw::EnumListItem| {
                        LimitReachedFilter::try_from(list_item.value())
                            .unwrap()
                            .display()
                    }),
                ),
            ));
            self.limit_reached_dropdown
                .set_model(Some(&LimitReachedFilter::new_model()));
            let limit_reached_dropdown_selected_item_notify_id = self
                .limit_reached_dropdown
                .connect_selected_item_notify(clone!(
                    #[weak]
                    obj,
                    move |dropdown| {
                        obj.handle_limit_reached_dropdown_selected_item_notify(dropdown);
                    }
                ));
            self.limit_reached_dropdown_selected_item_id
                .set(limit_reached_dropdown_selected_item_notify_id)
                .unwrap();

            let dt_range_button_range_notify_id =
                self.dt_range_button.connect_range_notify(clone!(
                    #[weak]
                    obj,
                    move |button| {
                        obj.handle_dt_range_button_range_notify(button);
                    }
                ));
            self.dt_range_button_range_notify_id
                .set(dt_range_button_range_notify_id)
                .unwrap();

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

            let factory = gtk::SignalListItemFactory::new();
            factory.connect_setup(clone!(
                #[weak]
                obj,
                move |_, list_item| {
                    let imp = obj.imp();

                    let list_item = list_item.downcast_ref::<gtk::ListItem>().unwrap();
                    let row = StockRow::new();
                    list_item
                        .property_expression("item")
                        .bind(&row, "stock", glib::Object::NONE);
                    list_item.set_child(Some(&row));

                    // Remove dead weak references
                    imp.rows.borrow_mut().retain(|i| i.upgrade().is_some());

                    debug_assert_eq!(imp.rows.borrow().iter().filter(|i| **i == row).count(), 0);
                    imp.rows.borrow_mut().push(row.downgrade());
                }
            ));
            factory.connect_teardown(clone!(
                #[weak]
                obj,
                move |_, list_item| {
                    let imp = obj.imp();

                    let list_item = list_item.downcast_ref::<gtk::ListItem>().unwrap();
                    if let Some(row) = list_item.child() {
                        let row = row.downcast_ref::<StockRow>().unwrap();

                        debug_assert_eq!(imp.rows.borrow().iter().filter(|i| *i == row).count(), 1);
                        imp.rows
                            .borrow_mut()
                            .retain(|i| i.upgrade().is_some_and(|i| &i != row));
                    }
                }
            ));
            self.list_view.set_factory(Some(&factory));

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
        imp.search_entry.set_queries(SearchQueries::new());

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

    pub async fn create_report(&self, kind: ReportKind) -> Result<Vec<u8>> {
        let imp = self.imp();

        let stocks = imp
            .selection_model
            .iter::<glib::Object>()
            .map(|o| o.unwrap().downcast::<Stock>().unwrap())
            .collect::<Vec<_>>();

        report::builder(kind, "Stocks Report")
            .prop(
                "Total Stock Count",
                stocks
                    .iter()
                    .map(|s| s.n_inside_for_dt_range(&imp.dt_range.borrow()))
                    .sum::<u32>(),
            )
            .prop("Search Query", imp.search_entry.queries())
            .table(
                report_table::builder("Stocks")
                    .column("ID")
                    .column("Count")
                    .rows(stocks.iter().map(|stock| {
                        report_table::row_builder()
                            .cell(stock.id().to_string())
                            .cell(stock.n_inside_for_dt_range(&imp.dt_range.borrow()))
                            .build()
                    }))
                    .build(),
            )
            .build()
            .await
    }

    fn set_dt_range(&self, dt_range: DateTimeRange) {
        let imp = self.imp();

        imp.dt_range.replace(dt_range);

        for row in imp.rows.borrow().iter().filter_map(|r| r.upgrade()) {
            row.set_dt_range(dt_range);
        }
        imp.details_pane.set_dt_range(dt_range);

        self.update_n_results_label();
    }

    fn bind_limit_reached_filter(&self, filter: &gtk::CustomFilter) {
        let imp = self.imp();

        let handler_id_group = Application::get()
            .settings()
            .connect_limit_reached_threshold_changed(clone!(
                #[weak]
                filter,
                move |_| {
                    filter.changed(gtk::FilterChange::Different);
                }
            ));
        imp.settings_limit_reached_id
            .replace(Some(handler_id_group));
    }

    fn handle_search_entry_search_changed(&self, entry: &SearchEntry) {
        let imp = self.imp();

        let queries = entry.queries();

        let limit_reached = match queries.find_last_with_values(S::IS, S::LIMIT_REACHED_VALUES) {
            Some(S::LIMIT_REACHED) => LimitReachedFilter::LimitReached,
            Some(S::LOWER_LIMIT_REACHED) => LimitReachedFilter::LowerLimitReached,
            Some(S::UPPER_LIMIT_REACHED) => LimitReachedFilter::UpperLimitReached,
            None => LimitReachedFilter::All,
            Some(_) => unreachable!(),
        };

        let selected_item_notify_id = imp.limit_reached_dropdown_selected_item_id.get().unwrap();
        imp.limit_reached_dropdown
            .block_signal(selected_item_notify_id);
        imp.limit_reached_dropdown
            .set_selected(limit_reached.model_position());
        imp.limit_reached_dropdown
            .unblock_signal(selected_item_notify_id);

        let dt_range = queries.dt_range(S::FROM, S::TO);

        let dt_range_button_range_notify_id = imp.dt_range_button_range_notify_id.get().unwrap();
        imp.dt_range_button
            .block_signal(dt_range_button_range_notify_id);
        imp.dt_range_button.set_range(dt_range);
        imp.dt_range_button
            .unblock_signal(dt_range_button_range_notify_id);

        self.set_dt_range(dt_range);

        if let Some(handler_id_group) = imp.settings_limit_reached_id.take() {
            Application::get()
                .settings()
                .disconnect_group(handler_id_group);
        }

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

        match limit_reached {
            LimitReachedFilter::All => {}
            LimitReachedFilter::LimitReached => {
                let filter = new_filter(move |stock: &Stock| {
                    Application::get()
                        .settings()
                        .compute_limit_reached(stock.n_inside_for_dt_range(&dt_range))
                        .is_some()
                });
                self.bind_limit_reached_filter(&filter);
                every_filter.append(filter);
            }
            LimitReachedFilter::LowerLimitReached => {
                let filter = new_filter(move |stock: &Stock| {
                    Application::get()
                        .settings()
                        .compute_limit_reached(stock.n_inside_for_dt_range(&dt_range))
                        .is_some_and(|l| l.is_lower())
                });
                self.bind_limit_reached_filter(&filter);
                every_filter.append(filter);
            }
            LimitReachedFilter::UpperLimitReached => {
                let filter = new_filter(move |stock: &Stock| {
                    Application::get()
                        .settings()
                        .compute_limit_reached(stock.n_inside_for_dt_range(&dt_range))
                        .is_some_and(|l| l.is_upper())
                });
                self.bind_limit_reached_filter(&filter);
                every_filter.append(filter);
            }
        };

        imp.filter_list_model.set_filter(Some(&every_filter));

        self.update_fallback_sorter();
        self.update_n_results_label();
    }

    fn handle_limit_reached_dropdown_selected_item_notify(&self, dropdown: &gtk::DropDown) {
        let imp = self.imp();

        let selected_item = dropdown
            .selected_item()
            .unwrap()
            .downcast::<adw::EnumListItem>()
            .unwrap();

        let mut queries = imp.search_entry.queries();

        match selected_item.value().try_into().unwrap() {
            LimitReachedFilter::All => {
                queries.remove_all(S::IS, S::LIMIT_REACHED_VALUES);
            }
            LimitReachedFilter::LimitReached => {
                queries.replace_all_or_insert(S::IS, S::LIMIT_REACHED_VALUES, S::LIMIT_REACHED);
            }
            LimitReachedFilter::LowerLimitReached => {
                queries.replace_all_or_insert(
                    S::IS,
                    S::LIMIT_REACHED_VALUES,
                    S::LOWER_LIMIT_REACHED,
                );
            }
            LimitReachedFilter::UpperLimitReached => {
                queries.replace_all_or_insert(
                    S::IS,
                    S::LIMIT_REACHED_VALUES,
                    S::UPPER_LIMIT_REACHED,
                );
            }
        }

        imp.search_entry.set_queries(queries);
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

        imp.search_entry.set_queries(queries);
    }

    fn handle_dt_range_button_range_notify(&self, button: &DateTimeRangeButton) {
        let imp = self.imp();

        let dt_range = button.range();

        let mut queries = imp.search_entry.queries();
        queries.set_dt_range(S::FROM, S::TO, dt_range);
        imp.search_entry.set_queries(queries);
    }

    fn update_fallback_sorter(&self) {
        let imp = self.imp();

        let queries = imp.search_entry.queries();
        let stock_sort = match queries.find_last_with_values(S::SORT, S::SORT_VALUES) {
            Some(S::ID_ASC) => StockSort::IdAsc,
            Some(S::ID_DESC) => StockSort::IdDesc,
            Some(S::COUNT_ASC) => StockSort::CountAsc,
            Some(S::COUNT_DESC) => StockSort::CountDesc,
            Some(S::UPDATED_ASC) => StockSort::UpdatedAsc,
            Some(S::UPDATED_DESC) => StockSort::UpdatedDesc,
            None => StockSort::default(),
            Some(_) => unreachable!(),
        };

        let selected_item_notify_id = imp.stock_sort_dropdown_selected_item_id.get().unwrap();
        imp.stock_sort_dropdown
            .block_signal(selected_item_notify_id);
        imp.stock_sort_dropdown
            .set_selected(stock_sort.model_position());
        imp.stock_sort_dropdown
            .unblock_signal(selected_item_notify_id);

        let dt_range = *imp.dt_range.borrow();
        let sorter = match stock_sort {
            StockSort::IdAsc | StockSort::IdDesc => {
                new_sorter(matches!(stock_sort, StockSort::IdDesc), |a: &Stock, b| {
                    a.id().cmp(b.id())
                })
            }
            StockSort::CountAsc | StockSort::CountDesc => new_sorter(
                matches!(stock_sort, StockSort::CountDesc),
                move |a: &Stock, b| {
                    a.n_inside_for_dt_range(&dt_range)
                        .cmp(&b.n_inside_for_dt_range(&dt_range))
                },
            ),
            StockSort::UpdatedAsc | StockSort::UpdatedDesc => new_sorter(
                matches!(stock_sort, StockSort::UpdatedDesc),
                |a: &Stock, b| a.last_action_dt().cmp(&b.last_action_dt()),
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
                stock.n_inside_for_dt_range(&imp.dt_range.borrow())
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
