use gtk::{
    glib::{self, clone, closure_local, translate::TryFromGlib},
    prelude::*,
    subclass::prelude::*,
};

use crate::{
    entity_id::EntityId,
    fuzzy_filter::FuzzyFilter,
    search_query::SearchQuery,
    stock_id::StockId,
    timeline::Timeline,
    timeline_item::TimelineItem,
    ui::{search_entry::SearchEntry, timeline_row::TimelineRow},
    Application,
};

#[derive(Debug, Clone, Copy, glib::Enum)]
#[enum_type(name = "UetsItemKind")]
enum ItemKind {
    All,
    Entry,
    Exit,
}

impl ItemKind {
    fn position(&self) -> u32 {
        *self as u32
    }
}

impl TryFrom<i32> for ItemKind {
    type Error = i32;

    fn try_from(val: i32) -> Result<Self, Self::Error> {
        unsafe { Self::try_from_glib(val) }
    }
}

mod imp {
    use std::{
        cell::{Cell, OnceCell},
        sync::OnceLock,
    };

    use glib::subclass::Signal;

    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/seadve/Uets/ui/timeline_view.ui")]
    pub struct TimelineView {
        #[template_child]
        pub(super) vbox: TemplateChild<gtk::Box>, // Unused
        #[template_child]
        pub(super) stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) search_entry: TemplateChild<SearchEntry>,
        #[template_child]
        pub(super) item_kind_dropdown: TemplateChild<gtk::DropDown>,
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

        pub(super) is_sticky: Cell<bool>,
        pub(super) is_auto_scrolling: Cell<bool>,

        pub(super) item_kind_dropdown_selected_item_id: OnceCell<glib::SignalHandlerId>,

        pub(super) fuzzy_filter: OnceCell<FuzzyFilter>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TimelineView {
        const NAME: &'static str = "UetsTimelineView";
        type Type = super::TimelineView;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action("timeline-view.scroll-to-bottom", None, move |obj, _, _| {
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
                .set_model(Some(&adw::EnumListModel::new(ItemKind::static_type())));
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

            self.selection_model.connect_items_changed(clone!(
                #[weak]
                obj,
                move |_, _, _, _| {
                    obj.update_stack();
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
                ]
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

    pub fn show_stock(&self, stock_id: &StockId) {
        let imp = self.imp();

        let mut queries = imp.search_entry.queries();
        queries.remove_all_standlones();
        queries.replace_all_iden_or_insert("stock", &stock_id.to_string());
        imp.search_entry.set_queries(&queries);
    }

    pub fn show_entity(&self, entity_id: &EntityId) {
        let imp = self.imp();

        let mut queries = imp.search_entry.queries();
        queries.remove_all_standlones();
        queries.replace_all_iden_or_insert("entity", &entity_id.to_string());
        imp.search_entry.set_queries(&queries);
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

    fn handle_search_entry_search_changed(&self, entry: &SearchEntry) {
        let imp = self.imp();

        let queries = entry.queries();

        let item_kind = if let Some(SearchQuery::IdenValue(iden, value)) =
            queries.find_last_match(&["is:entry", "is:exit"])
        {
            debug_assert_eq!(iden, "is");
            match value.as_str() {
                "entry" => ItemKind::Entry,
                "exit" => ItemKind::Exit,
                _ => unreachable!(),
            }
        } else {
            ItemKind::All
        };

        let selected_item_notify_id = imp.item_kind_dropdown_selected_item_id.get().unwrap();
        imp.item_kind_dropdown.block_signal(selected_item_notify_id);
        imp.item_kind_dropdown.set_selected(item_kind.position());
        imp.item_kind_dropdown
            .unblock_signal(selected_item_notify_id);

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
            ItemKind::All => {}
            ItemKind::Entry => {
                every_filter.append(gtk::CustomFilter::new(|o| {
                    let entity = o.downcast_ref::<TimelineItem>().unwrap();
                    entity.kind().is_entry()
                }));
            }
            ItemKind::Exit => {
                every_filter.append(gtk::CustomFilter::new(|o| {
                    let entity = o.downcast_ref::<TimelineItem>().unwrap();
                    entity.kind().is_exit()
                }));
            }
        }

        let any_stock_filter = gtk::AnyFilter::new();
        for stock_id in queries.all_values("stock").into_iter().map(StockId::new) {
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
        for entity_id in queries.all_values("entity").into_iter().map(EntityId::new) {
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
            ItemKind::All => {
                queries.remove_all("is", "entry");
                queries.remove_all("is", "exit");
            }
            ItemKind::Entry => {
                queries.replace_all_or_insert("is", "exit", "entry");
            }
            ItemKind::Exit => {
                queries.replace_all_or_insert("is", "entry", "exit");
            }
        }

        imp.search_entry.set_queries(&queries);
    }

    fn update_stack(&self) {
        let imp = self.imp();

        if imp.selection_model.n_items() == 0 {
            imp.stack.set_visible_child(&*imp.no_data_page);
        } else {
            imp.stack.set_visible_child(&*imp.main_page);
        }
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
