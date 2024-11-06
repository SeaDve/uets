use anyhow::Result;
use gtk::{
    glib::{self, clone, closure, closure_local},
    prelude::*,
    subclass::prelude::*,
};

use crate::{
    entity::Entity,
    entity_id::EntityId,
    entity_list::EntityList,
    fuzzy_filter::FuzzyFilter,
    list_model_enum,
    report::{self, ReportKind},
    search_query::SearchQueries,
    stock_id::StockId,
    ui::{
        entity_details_pane::EntityDetailsPane, entity_row::EntityRow, search_entry::SearchEntry,
        wormhole_window::WormholeWindow,
    },
    utils::new_sorter,
    Application,
};

struct S;

impl S {
    const IS: &str = "is";

    const INSIDE: &str = "inside";
    const OUTSIDE: &str = "outside";

    const STOCK: &str = "stock";

    const SORT: &str = "sort";
    const SORT_VALUES: &[&str] = &[
        Self::ID_ASC,
        Self::ID_DESC,
        Self::STOCK_ASC,
        Self::STOCK_DESC,
        Self::UPDATED_ASC,
        Self::UPDATED_DESC,
    ];

    const ID_ASC: &str = "id-asc";
    const ID_DESC: &str = "id-desc";
    const STOCK_ASC: &str = "stock-asc";
    const STOCK_DESC: &str = "stock-desc";
    const UPDATED_ASC: &str = "updated-asc";
    const UPDATED_DESC: &str = "updated-desc";
}

#[derive(Debug, Clone, Copy, glib::Enum)]
#[enum_type(name = "UetsEntityZoneFilter")]
enum EntityZoneFilter {
    All,
    Inside,
    Outside,
}

list_model_enum!(EntityZoneFilter);

#[derive(Debug, Default, Clone, Copy, glib::Enum)]
#[enum_type(name = "UetsEntitySort")]
enum EntitySort {
    #[default]
    IdAsc,
    IdDesc,
    StockAsc,
    StockDesc,
    UpdatedAsc,
    UpdatedDesc,
}

list_model_enum!(EntitySort);

impl EntitySort {
    fn display(&self) -> &'static str {
        match self {
            EntitySort::IdAsc => "A-Z",
            EntitySort::IdDesc => "Z-A",
            EntitySort::StockAsc => "Stock (A-Z)",
            EntitySort::StockDesc => "Stock (Z-A)",
            EntitySort::UpdatedAsc => "Least Recently Updated",
            EntitySort::UpdatedDesc => "Recently Updated",
        }
    }
}

mod imp {
    use std::{cell::OnceCell, sync::OnceLock};

    use glib::subclass::Signal;

    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/seadve/Uets/ui/entities_view.ui")]
    pub struct EntitiesView {
        #[template_child]
        pub(super) flap: TemplateChild<adw::Flap>,
        #[template_child]
        pub(super) stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) empty_page: TemplateChild<adw::StatusPage>,
        #[template_child]
        pub(super) main_page: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub(super) search_entry: TemplateChild<SearchEntry>,
        #[template_child]
        pub(super) entity_zone_dropdown: TemplateChild<gtk::DropDown>,
        #[template_child]
        pub(super) entity_sort_dropdown: TemplateChild<gtk::DropDown>,
        #[template_child]
        pub(super) n_results_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) list_view: TemplateChild<gtk::ListView>,
        #[template_child]
        pub(super) selection_model: TemplateChild<gtk::SingleSelection>,
        #[template_child]
        pub(super) sort_list_model: TemplateChild<gtk::SortListModel>,
        #[template_child]
        pub(super) filter_list_model: TemplateChild<gtk::FilterListModel>,
        #[template_child]
        pub(super) details_pane: TemplateChild<EntityDetailsPane>,

        pub(super) entity_zone_dropdown_selected_item_id: OnceCell<glib::SignalHandlerId>,
        pub(super) entity_sort_dropdown_selected_item_id: OnceCell<glib::SignalHandlerId>,

        pub(super) fuzzy_filter: OnceCell<FuzzyFilter>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EntitiesView {
        const NAME: &'static str = "UetsEntitiesView";
        type Type = super::EntitiesView;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            EntityRow::ensure_type();

            klass.bind_template();

            klass.install_action_async(
                "entities-view.share-report",
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

    impl ObjectImpl for EntitiesView {
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

            self.entity_zone_dropdown
                .set_expression(Some(&adw::EnumListItem::this_expression("name")));
            self.entity_zone_dropdown
                .set_model(Some(&EntityZoneFilter::new_model()));
            let entity_zone_dropdown_selected_item_notify_id = self
                .entity_zone_dropdown
                .connect_selected_item_notify(clone!(
                    #[weak]
                    obj,
                    move |dropdown| {
                        obj.handle_entity_zone_dropdown_selected_item_notify(dropdown);
                    }
                ));
            self.entity_zone_dropdown_selected_item_id
                .set(entity_zone_dropdown_selected_item_notify_id)
                .unwrap();

            self.entity_sort_dropdown
                .set_expression(Some(&gtk::ClosureExpression::new::<String>(
                    &[] as &[gtk::Expression],
                    closure!(|list_item: adw::EnumListItem| {
                        EntitySort::try_from(list_item.value()).unwrap().display()
                    }),
                )));
            self.entity_sort_dropdown
                .set_model(Some(&EntitySort::new_model()));
            let entity_sort_dropdown_selected_item_notify_id = self
                .entity_sort_dropdown
                .connect_selected_item_notify(clone!(
                    #[weak]
                    obj,
                    move |dropdown| {
                        obj.handle_entity_sort_dropdown_selected_item_notify(dropdown);
                    }
                ));
            self.entity_sort_dropdown_selected_item_id
                .set(entity_sort_dropdown_selected_item_notify_id)
                .unwrap();

            self.selection_model
                .bind_property("selected-item", &*self.flap, "reveal-flap")
                .transform_to(|_, entity: Option<Entity>| Some(entity.is_some()))
                .sync_create()
                .build();
            self.selection_model
                .bind_property("selected-item", &*self.details_pane, "entity")
                .sync_create()
                .build();
            self.selection_model.connect_items_changed(clone!(
                #[weak]
                obj,
                move |_, _, _, _| {
                    obj.update_stack();
                    obj.update_n_results_label();
                }
            ));

            self.details_pane.connect_show_stock_request(clone!(
                #[weak]
                obj,
                move |details_pane| {
                    let entity = details_pane.entity().expect("entity must exist");
                    let stock_id = entity.stock_id().expect("stock must exist");
                    obj.emit_by_name::<()>("show-stock-request", &[&stock_id]);
                }
            ));
            self.details_pane.connect_show_timeline_request(clone!(
                #[weak]
                obj,
                move |details_pane| {
                    let entity = details_pane.entity().expect("entity must exist");
                    obj.emit_by_name::<()>("show-timeline-request", &[entity.id()]);
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
                let entity = o.downcast_ref::<Entity>().unwrap();
                [
                    Some(entity.id().to_string()),
                    entity.stock_id().map(|s| s.to_string()),
                ]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(" ")
            });
            self.sort_list_model.set_sorter(Some(fuzzy_filter.sorter()));
            self.fuzzy_filter.set(fuzzy_filter).unwrap();

            obj.update_fallback_sorter();
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
                    Signal::builder("show-stock-request")
                        .param_types([StockId::static_type()])
                        .build(),
                    Signal::builder("show-timeline-request")
                        .param_types([EntityId::static_type()])
                        .build(),
                ]
            })
        }
    }

    impl WidgetImpl for EntitiesView {}
}

glib::wrapper! {
    pub struct EntitiesView(ObjectSubclass<imp::EntitiesView>)
        @extends gtk::Widget;
}

impl EntitiesView {
    pub fn new() -> Self {
        glib::Object::new()
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

    pub fn connect_show_timeline_request<F>(&self, f: F) -> glib::SignalHandlerId
    where
        F: Fn(&Self, &EntityId) + 'static,
    {
        self.connect_closure(
            "show-timeline-request",
            false,
            closure_local!(move |obj: &Self, id: &EntityId| f(obj, id)),
        )
    }

    pub fn bind_entity_list(&self, entity_list: &EntityList) {
        let imp = self.imp();

        imp.filter_list_model.set_model(Some(entity_list));
    }

    pub fn show_entity(&self, entity_id: &EntityId) {
        let imp = self.imp();

        // Clear search filter so we can find the entity
        imp.search_entry.set_queries(&SearchQueries::new());

        let position = imp
            .selection_model
            .iter::<glib::Object>()
            .position(|o| {
                let entity = o.unwrap().downcast::<Entity>().unwrap();
                entity.id() == entity_id
            })
            .expect("entity must exist") as u32;

        imp.selection_model.set_selected(position);

        imp.list_view
            .activate_action("list.scroll-to-item", Some(&position.to_variant()))
            .unwrap();
    }

    pub fn show_entities_with_stock_id(&self, stock_id: &StockId) {
        let imp = self.imp();

        let mut queries = imp.search_entry.queries();
        queries.remove_all_standalones();
        queries.replace_all_iden_or_insert(S::STOCK, &stock_id.to_string());
        imp.search_entry.set_queries(&queries);
    }

    async fn handle_share_report(&self, kind: ReportKind) {
        let imp = self.imp();

        let entities = imp
            .selection_model
            .iter::<glib::Object>()
            .map(|o| o.unwrap().downcast::<Entity>().unwrap())
            .collect::<Vec<_>>();

        let bytes_fut = report::builder(kind, "Entities Report")
            .prop("Total Entities", entities.len())
            .prop("Search Query", imp.search_entry.queries())
            .table(
                "Entities",
                ["ID", "Stock ID", "Zone"],
                entities.iter().map(|entity| {
                    [
                        entity.id().to_string(),
                        entity
                            .stock_id()
                            .map(|id| id.to_string())
                            .unwrap_or_default(),
                        if entity.is_inside() {
                            "Inside"
                        } else {
                            "Outside"
                        }
                        .to_string(),
                    ]
                }),
            )
            .build();

        if let Err(err) =
            WormholeWindow::send(bytes_fut, &report::file_name("Entities Report", kind), self).await
        {
            tracing::error!("Failed to send report: {:?}", err);

            Application::get().add_message_toast("Failed to share report");
        }
    }

    fn handle_search_entry_search_changed(&self, entry: &SearchEntry) {
        let imp = self.imp();

        let queries = entry.queries();

        let entity_zone = match queries.find_last_match(S::IS, &[S::INSIDE, S::OUTSIDE]) {
            Some(S::INSIDE) => EntityZoneFilter::Inside,
            Some(S::OUTSIDE) => EntityZoneFilter::Outside,
            _ => EntityZoneFilter::All,
        };

        let selected_item_notify_id = imp.entity_zone_dropdown_selected_item_id.get().unwrap();
        imp.entity_zone_dropdown
            .block_signal(selected_item_notify_id);
        imp.entity_zone_dropdown
            .set_selected(entity_zone.position());
        imp.entity_zone_dropdown
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

        match entity_zone {
            EntityZoneFilter::All => {}
            EntityZoneFilter::Inside => {
                every_filter.append(gtk::CustomFilter::new(|o| {
                    let entity = o.downcast_ref::<Entity>().unwrap();
                    entity.is_inside()
                }));
            }
            EntityZoneFilter::Outside => {
                every_filter.append(gtk::CustomFilter::new(|o| {
                    let entity = o.downcast_ref::<Entity>().unwrap();
                    !entity.is_inside()
                }));
            }
        }

        let any_stock_filter = gtk::AnyFilter::new();
        for stock_id in queries.all_values(S::STOCK).into_iter().map(StockId::new) {
            any_stock_filter.append(gtk::CustomFilter::new(move |o| {
                let entity = o.downcast_ref::<Entity>().unwrap();
                entity.stock_id().is_some_and(|s_id| s_id == &stock_id)
            }));
        }

        if any_stock_filter.n_items() == 0 {
            any_stock_filter.append(gtk::CustomFilter::new(|_| true));
        }

        every_filter.append(any_stock_filter);
        imp.filter_list_model.set_filter(Some(&every_filter));
    }

    fn handle_entity_zone_dropdown_selected_item_notify(&self, dropdown: &gtk::DropDown) {
        let imp = self.imp();

        let selected_item = dropdown
            .selected_item()
            .unwrap()
            .downcast::<adw::EnumListItem>()
            .unwrap();

        let mut queries = imp.search_entry.queries();

        match selected_item.value().try_into().unwrap() {
            EntityZoneFilter::All => {
                queries.remove_all(S::IS, S::INSIDE);
                queries.remove_all(S::IS, S::OUTSIDE);
            }
            EntityZoneFilter::Inside => {
                queries.replace_all_or_insert(S::IS, &[S::OUTSIDE], S::INSIDE);
            }
            EntityZoneFilter::Outside => {
                queries.replace_all_or_insert(S::IS, &[S::INSIDE], S::OUTSIDE);
            }
        }

        imp.search_entry.set_queries(&queries);
    }

    fn handle_entity_sort_dropdown_selected_item_notify(&self, dropdown: &gtk::DropDown) {
        let imp = self.imp();

        let selected_item = dropdown
            .selected_item()
            .unwrap()
            .downcast::<adw::EnumListItem>()
            .unwrap();

        let mut queries = imp.search_entry.queries();

        match selected_item.value().try_into().unwrap() {
            EntitySort::IdAsc => queries.replace_all_or_insert(S::SORT, S::SORT_VALUES, S::ID_ASC),
            EntitySort::IdDesc => {
                queries.replace_all_or_insert(S::SORT, S::SORT_VALUES, S::ID_DESC)
            }
            EntitySort::StockAsc => {
                queries.replace_all_or_insert(S::SORT, S::SORT_VALUES, S::STOCK_ASC)
            }
            EntitySort::StockDesc => {
                queries.replace_all_or_insert(S::SORT, S::SORT_VALUES, S::STOCK_DESC)
            }
            EntitySort::UpdatedAsc => {
                queries.replace_all_or_insert(S::SORT, S::SORT_VALUES, S::UPDATED_ASC)
            }
            EntitySort::UpdatedDesc => {
                queries.replace_all_or_insert(S::SORT, S::SORT_VALUES, S::UPDATED_DESC)
            }
        }

        imp.search_entry.set_queries(&queries);
    }

    fn update_fallback_sorter(&self) {
        let imp = self.imp();

        let queries = imp.search_entry.queries();

        let entity_sort = match queries.find_last_match(S::SORT, S::SORT_VALUES) {
            Some(S::ID_ASC) => EntitySort::IdAsc,
            Some(S::ID_DESC) => EntitySort::IdDesc,
            Some(S::STOCK_ASC) => EntitySort::StockAsc,
            Some(S::STOCK_DESC) => EntitySort::StockDesc,
            Some(S::UPDATED_ASC) => EntitySort::UpdatedAsc,
            Some(S::UPDATED_DESC) => EntitySort::UpdatedDesc,
            _ => EntitySort::default(),
        };

        let selected_item_notify_id = imp.entity_sort_dropdown_selected_item_id.get().unwrap();
        imp.entity_sort_dropdown
            .block_signal(selected_item_notify_id);
        imp.entity_sort_dropdown
            .set_selected(entity_sort.position());
        imp.entity_sort_dropdown
            .unblock_signal(selected_item_notify_id);

        let sorter = match entity_sort {
            EntitySort::IdAsc | EntitySort::IdDesc => new_sorter(
                matches!(entity_sort, EntitySort::IdDesc),
                |a: &Entity, b| a.id().cmp(b.id()),
            ),
            EntitySort::StockAsc | EntitySort::StockDesc => new_sorter(
                matches!(entity_sort, EntitySort::StockDesc),
                |a: &Entity, b| a.stock_id().cmp(&b.stock_id()),
            ),
            EntitySort::UpdatedAsc | EntitySort::UpdatedDesc => new_sorter(
                matches!(entity_sort, EntitySort::UpdatedDesc),
                |a: &Entity, b| {
                    a.last_dt_pair()
                        .map(|pair| pair.last_dt())
                        .cmp(&b.last_dt_pair().map(|pair| pair.last_dt()))
                },
            ),
        };

        imp.fuzzy_filter
            .get()
            .unwrap()
            .sorter()
            .set_fallback_sorter(Some(sorter));
    }

    fn update_stack(&self) {
        let imp = self.imp();

        if imp.selection_model.n_items() == 0 {
            imp.stack.set_visible_child(&*imp.empty_page);
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
}
