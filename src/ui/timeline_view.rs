use anyhow::Result;
use gtk::{
    glib::{self, clone, closure_local},
    prelude::*,
    subclass::prelude::*,
};

use crate::{
    date_time_range::DateTimeRange,
    entity_id::EntityId,
    fuzzy_filter::FuzzyFilter,
    list_model_enum,
    report::{self, ReportKind},
    report_table,
    search_query_ext::SearchQueriesDateTimeRangeExt,
    stock_id::StockId,
    timeline::Timeline,
    timeline_item::TimelineItem,
    ui::{
        date_time_range_button::DateTimeRangeButton, search_entry::SearchEntry,
        send_dialog::SendDialog, timeline_row::TimelineRow,
    },
    Application,
};

struct S;

impl S {
    const IS: &str = "is";

    const ENTRY: &str = "entry";
    const EXIT: &str = "exit";

    const FROM: &str = "from";
    const TO: &str = "to";

    const STOCK: &str = "stock";

    const ENTITY: &str = "entity";
}

#[derive(Debug, Clone, Copy, glib::Enum)]
#[enum_type(name = "UetsTimelineItemKindFilter")]
enum TimelineItemKindFilter {
    All,
    Entry,
    Exit,
}

list_model_enum!(TimelineItemKindFilter);

mod imp {
    use std::{
        cell::{Cell, OnceCell, RefCell},
        sync::OnceLock,
    };

    use glib::subclass::Signal;

    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/seadve/Uets/ui/timeline_view.ui")]
    pub struct TimelineView {
        #[template_child]
        pub(super) toolbar_view: TemplateChild<adw::ToolbarView>, // Unused
        #[template_child]
        pub(super) stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) search_entry: TemplateChild<SearchEntry>,
        #[template_child]
        pub(super) item_kind_dropdown: TemplateChild<gtk::DropDown>,
        #[template_child]
        pub(super) dt_range_button: TemplateChild<DateTimeRangeButton>,
        #[template_child]
        pub(super) n_results_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) no_data_page: TemplateChild<adw::StatusPage>,
        #[template_child]
        pub(super) main_page: TemplateChild<gtk::Overlay>,
        #[template_child]
        pub(super) scrolled_window: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub(super) list_view: TemplateChild<gtk::ListView>,
        #[template_child]
        pub(super) selection_model: TemplateChild<gtk::NoSelection>,
        #[template_child]
        pub(super) sort_list_model: TemplateChild<gtk::SortListModel>,
        #[template_child]
        pub(super) filter_list_model: TemplateChild<gtk::FilterListModel>,
        #[template_child]
        pub(super) scroll_to_bottom_revealer: TemplateChild<gtk::Revealer>,

        pub(super) dt_range: RefCell<DateTimeRange>,

        pub(super) is_sticky: Cell<bool>,
        pub(super) is_auto_scrolling: Cell<bool>,

        pub(super) item_kind_dropdown_selected_item_id: OnceCell<glib::SignalHandlerId>,
        pub(super) dt_range_button_range_notify_id: OnceCell<glib::SignalHandlerId>,

        pub(super) fuzzy_filter: OnceCell<FuzzyFilter>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TimelineView {
        const NAME: &'static str = "UetsTimelineView";
        type Type = super::TimelineView;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action_async(
                "timeline-view.share-report",
                Some(&ReportKind::static_variant_type()),
                |obj, _, kind| async move {
                    let kind = kind.unwrap().get::<ReportKind>().unwrap();
                    obj.handle_share_report(kind).await;
                },
            );
            klass.install_action("timeline-view.scroll-to-bottom", None, |obj, _, _| {
                obj.scroll_to_bottom();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for TimelineView {
        fn constructed(&self) {
            self.parent_constructed();

            self.list_view.remove_css_class("view");
            self.is_sticky.set(true);

            let obj = self.obj();

            let vadj = self.scrolled_window.vadjustment();
            debug_assert_eq!(vadj, self.list_view.vadjustment().unwrap());

            vadj.connect_value_changed(clone!(
                #[weak]
                obj,
                move |_| {
                    let imp = obj.imp();

                    let is_at_bottom = obj.is_at_bottom();
                    if imp.is_auto_scrolling.get() {
                        if is_at_bottom {
                            imp.is_auto_scrolling.set(false);
                            imp.is_sticky.set(true);
                        } else {
                            obj.scroll_to_bottom();
                        }
                    } else {
                        imp.is_sticky.set(is_at_bottom);
                    }

                    obj.update_scroll_to_bottom_revealer_reveal_child();
                }
            ));
            vadj.connect_upper_notify(clone!(
                #[weak]
                obj,
                move |_| {
                    let imp = obj.imp();

                    if imp.is_sticky.get() {
                        obj.scroll_to_bottom();
                    }

                    obj.update_scroll_to_bottom_revealer_reveal_child();
                }
            ));
            vadj.connect_page_size_notify(clone!(
                #[weak]
                obj,
                move |_| {
                    let imp = obj.imp();

                    if imp.is_sticky.get() {
                        obj.scroll_to_bottom();
                    }

                    obj.update_scroll_to_bottom_revealer_reveal_child();
                }
            ));

            self.search_entry.connect_search_changed(clone!(
                #[weak]
                obj,
                move |entry| {
                    obj.handle_search_entry_search_changed(entry);
                }
            ));

            self.item_kind_dropdown
                .set_expression(Some(&adw::EnumListItem::this_expression("name")));
            self.item_kind_dropdown
                .set_model(Some(&TimelineItemKindFilter::new_model()));
            let item_kind_dropdown_selected_item_notify_id =
                self.item_kind_dropdown.connect_selected_item_notify(clone!(
                    #[weak]
                    obj,
                    move |drop_down| {
                        obj.handle_item_kind_dropdown_selected_item_notify(drop_down);
                    }
                ));
            self.item_kind_dropdown_selected_item_id
                .set(item_kind_dropdown_selected_item_notify_id)
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

            self.selection_model.connect_items_changed(clone!(
                #[weak]
                obj,
                move |_, _, _, _| {
                    obj.update_stack();
                    obj.update_n_results_label();
                }
            ));

            self.scroll_to_bottom_revealer
                .connect_child_revealed_notify(clone!(
                    #[weak]
                    obj,
                    move |_| {
                        obj.update_scroll_to_bottom_revealer_can_target();
                    }
                ));

            let factory = gtk::SignalListItemFactory::new();
            factory.connect_setup(clone!(
                #[weak]
                obj,
                move |_, list_item| {
                    let list_item = list_item.downcast_ref::<gtk::ListItem>().unwrap();
                    list_item.set_selectable(false);
                    list_item.set_activatable(false);

                    let row = TimelineRow::new();
                    row.connect_show_entity_request(clone!(
                        #[weak]
                        obj,
                        move |_, id| {
                            obj.emit_by_name::<()>("show-entity-request", &[&id]);
                        }
                    ));
                    row.connect_show_stock_request(clone!(
                        #[weak]
                        obj,
                        move |_, id| {
                            obj.emit_by_name::<()>("show-stock-request", &[&id]);
                        }
                    ));

                    list_item
                        .property_expression("item")
                        .bind(&row, "item", glib::Object::NONE);

                    list_item.set_child(Some(&row));
                }
            ));
            self.list_view.set_factory(Some(&factory));

            let fuzzy_filter = FuzzyFilter::new(|o| {
                let item = o.downcast_ref::<TimelineItem>().unwrap();
                let entity = Application::get()
                    .timeline()
                    .entity_list()
                    .get(item.entity_id())
                    .expect("entity must be known");
                [
                    Some(item.entity_id().to_string()),
                    entity.stock_id().map(|s| s.to_string()),
                    Some(item.dt().format("%B %Y").to_string()),
                ]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(" ")
            });
            self.sort_list_model.set_sorter(Some(fuzzy_filter.sorter()));
            self.fuzzy_filter.set(fuzzy_filter).unwrap();

            obj.update_stack();
            obj.update_n_results_label();
        }

        fn dispose(&self) {
            self.dispose_template();
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();

            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("show-entity-request")
                        .param_types([EntityId::static_type()])
                        .build(),
                    Signal::builder("show-stock-request")
                        .param_types([StockId::static_type()])
                        .build(),
                ]
            })
        }
    }

    impl WidgetImpl for TimelineView {}
}

glib::wrapper! {
    pub struct TimelineView(ObjectSubclass<imp::TimelineView>)
        @extends gtk::Widget;
}

impl TimelineView {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn connect_show_entity_request<F>(&self, f: F) -> glib::SignalHandlerId
    where
        F: Fn(&Self, &EntityId) + 'static,
    {
        self.connect_closure(
            "show-entity-request",
            false,
            closure_local!(|obj: &Self, id: &EntityId| f(obj, id)),
        )
    }

    pub fn connect_show_stock_request<F>(&self, f: F) -> glib::SignalHandlerId
    where
        F: Fn(&Self, &StockId) + 'static,
    {
        self.connect_closure(
            "show-stock-request",
            false,
            closure_local!(|obj: &Self, id: &StockId| f(obj, id)),
        )
    }

    pub fn bind_timeline(&self, timeline: &Timeline) {
        let imp = self.imp();

        imp.filter_list_model.set_model(Some(timeline));
    }

    pub fn show_entity(&self, entity_id: &EntityId) {
        let imp = self.imp();

        let mut queries = imp.search_entry.queries();
        queries.remove_all_standalones();
        queries.remove_all_iden(S::STOCK);
        queries.replace_all_iden_or_insert(S::ENTITY, &entity_id.to_string());
        imp.search_entry.set_queries(queries);
    }

    pub fn show_stock(&self, stock_id: &StockId) {
        let imp = self.imp();

        let mut queries = imp.search_entry.queries();
        queries.remove_all_standalones();
        queries.remove_all_iden(S::ENTITY);
        queries.replace_all_iden_or_insert(S::STOCK, &stock_id.to_string());
        imp.search_entry.set_queries(queries);
    }

    fn set_dt_range(&self, dt_range: DateTimeRange) {
        let imp = self.imp();

        imp.dt_range.replace(dt_range);
    }

    fn scroll_to_bottom(&self) {
        let imp = self.imp();

        imp.is_auto_scrolling.set(true);
        imp.scrolled_window
            .emit_scroll_child(gtk::ScrollType::End, false);

        self.update_scroll_to_bottom_revealer_reveal_child();
    }

    fn is_at_bottom(&self) -> bool {
        let imp = self.imp();
        let vadj = imp.scrolled_window.vadjustment();
        vadj.value() + vadj.page_size() == vadj.upper()
    }

    async fn handle_share_report(&self, kind: ReportKind) {
        let imp = self.imp();

        let items = imp
            .selection_model
            .iter::<glib::Object>()
            .map(|o| o.unwrap().downcast::<TimelineItem>().unwrap())
            .collect::<Vec<_>>();
        let dt_range = *imp.dt_range.borrow();

        let app = Application::get();
        let timeline = app.timeline();

        let bytes_fut = async {
            report::builder(kind, "Timeline Report")
                .prop(
                    "Current Inside Count",
                    timeline.n_inside_for_dt_range(&dt_range),
                )
                .prop(
                    "Current Max Inside Count",
                    timeline.max_n_inside_for_dt_range(&dt_range),
                )
                .prop("Total Entries", timeline.n_entries_for_dt_range(&dt_range))
                .prop("Total Exits", timeline.n_exits_for_dt_range(&dt_range))
                .prop("Search Query", imp.search_entry.queries())
                .table(
                    report_table::builder("Timeline")
                        .column("Timestamp")
                        .column("Action")
                        .column("Entity ID")
                        .column("Inside Count")
                        .column("Max Inside Count")
                        .column("Entry Count")
                        .column("Exit Count")
                        .rows(items.iter().map(|item| {
                            report_table::row_builder()
                                .cell(item.dt())
                                .cell(item.kind().to_string())
                                .cell(item.entity_id().to_string())
                                .cell(timeline.n_inside_for_dt(item.dt()))
                                .cell(timeline.max_n_inside_for_dt(item.dt()))
                                .cell(timeline.n_entries_for_dt(item.dt()))
                                .cell(timeline.n_exits_for_dt(item.dt()))
                                .build()
                        }))
                        .graph("Inside Count Over Time", 0, 3)
                        .graph("Max Inside Count Over Time", 0, 4)
                        .graph("Entry Count Over Time", 0, 5)
                        .graph("Exit Count Over Time", 0, 6)
                        .build(),
                )
                .build()
                .await
        };

        if let Err(err) = SendDialog::send(
            &report::file_name("Timeline Report", kind),
            bytes_fut,
            Some(self),
        )
        .await
        {
            tracing::error!("Failed to send report: {:?}", err);

            Application::get().add_message_toast("Failed to share report");
        }
    }

    fn handle_search_entry_search_changed(&self, entry: &SearchEntry) {
        let imp = self.imp();

        let queries = entry.queries();

        let item_kind = match queries.find_last_with_values(S::IS, &[S::ENTRY, S::EXIT]) {
            Some(S::ENTRY) => TimelineItemKindFilter::Entry,
            Some(S::EXIT) => TimelineItemKindFilter::Exit,
            _ => TimelineItemKindFilter::All,
        };

        let selected_item_notify_id = imp.item_kind_dropdown_selected_item_id.get().unwrap();
        imp.item_kind_dropdown.block_signal(selected_item_notify_id);
        imp.item_kind_dropdown.set_selected(item_kind.position());
        imp.item_kind_dropdown
            .unblock_signal(selected_item_notify_id);

        let dt_range = queries.dt_range(S::FROM, S::TO);

        let dt_range_button_range_notify_id = imp.dt_range_button_range_notify_id.get().unwrap();
        imp.dt_range_button
            .block_signal(dt_range_button_range_notify_id);
        imp.dt_range_button.set_range(dt_range);
        imp.dt_range_button
            .unblock_signal(dt_range_button_range_notify_id);

        self.set_dt_range(dt_range);

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

        match item_kind {
            TimelineItemKindFilter::All => {}
            TimelineItemKindFilter::Entry => {
                every_filter.append(gtk::CustomFilter::new(|o| {
                    let entity = o.downcast_ref::<TimelineItem>().unwrap();
                    entity.kind().is_entry()
                }));
            }
            TimelineItemKindFilter::Exit => {
                every_filter.append(gtk::CustomFilter::new(|o| {
                    let entity = o.downcast_ref::<TimelineItem>().unwrap();
                    entity.kind().is_exit()
                }));
            }
        }

        if !dt_range.is_all_time() {
            every_filter.append(gtk::CustomFilter::new(move |o| {
                let entity = o.downcast_ref::<TimelineItem>().unwrap();
                dt_range.contains(entity.dt())
            }));
        }

        let any_stock_filter = gtk::AnyFilter::new();
        for stock_id in queries.all_values(S::STOCK).into_iter().map(StockId::new) {
            any_stock_filter.append(gtk::CustomFilter::new(move |o| {
                let item = o.downcast_ref::<TimelineItem>().unwrap();
                let entity = Application::get()
                    .timeline()
                    .entity_list()
                    .get(item.entity_id())
                    .expect("entity must be known");
                entity.stock_id().is_some_and(|s_id| s_id == &stock_id)
            }));
        }

        let any_entity_filter = gtk::AnyFilter::new();
        for entity_id in queries.all_values(S::ENTITY).into_iter().map(EntityId::new) {
            any_entity_filter.append(gtk::CustomFilter::new(move |o| {
                let item = o.downcast_ref::<TimelineItem>().unwrap();
                item.entity_id() == &entity_id
            }));
        }

        if any_stock_filter.n_items() == 0 {
            any_stock_filter.append(gtk::CustomFilter::new(|_| true));
        }

        if any_entity_filter.n_items() == 0 {
            any_entity_filter.append(gtk::CustomFilter::new(|_| true));
        }

        every_filter.append(any_stock_filter);
        every_filter.append(any_entity_filter);
        imp.filter_list_model.set_filter(Some(&every_filter));

        self.update_n_results_label();
    }

    fn handle_item_kind_dropdown_selected_item_notify(&self, dropdown: &gtk::DropDown) {
        let imp = self.imp();

        let selected_item = dropdown
            .selected_item()
            .unwrap()
            .downcast::<adw::EnumListItem>()
            .unwrap();

        let mut queries = imp.search_entry.queries();

        match selected_item.value().try_into().unwrap() {
            TimelineItemKindFilter::All => {
                queries.remove_all(S::IS, S::ENTRY);
                queries.remove_all(S::IS, S::EXIT);
            }
            TimelineItemKindFilter::Entry => {
                queries.replace_all_or_insert(S::IS, &[S::EXIT], S::ENTRY);
            }
            TimelineItemKindFilter::Exit => {
                queries.replace_all_or_insert(S::IS, &[S::ENTRY], S::EXIT);
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

    fn update_stack(&self) {
        let imp = self.imp();

        if imp.selection_model.n_items() == 0 {
            imp.stack.set_visible_child(&*imp.no_data_page);
        } else {
            imp.stack.set_visible_child(&*imp.main_page);
        }
    }

    fn update_n_results_label(&self) {
        let imp = self.imp();

        let n_total = imp.selection_model.n_items();
        let text = if imp.search_entry.queries().is_empty() {
            format!("Total: {}", n_total)
        } else {
            format!("Results: {}", n_total)
        };

        imp.n_results_label.set_label(&text);
    }

    fn update_scroll_to_bottom_revealer_reveal_child(&self) {
        let imp = self.imp();

        imp.scroll_to_bottom_revealer
            .set_reveal_child(!self.is_at_bottom() && !imp.is_auto_scrolling.get());
    }

    fn update_scroll_to_bottom_revealer_can_target(&self) {
        let imp = self.imp();

        imp.scroll_to_bottom_revealer
            .set_can_target(imp.scroll_to_bottom_revealer.is_child_revealed());
    }
}
