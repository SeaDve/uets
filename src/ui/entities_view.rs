use anyhow::Result;
use gtk::{
    glib::{self, clone, closure, closure_local},
    prelude::*,
    subclass::prelude::*,
};

use crate::{
    date_time,
    date_time_range::DateTimeRange,
    entity::Entity,
    entity_data::{EntityDataField, EntityDataFieldTy, ValidEntityFields},
    entity_expiration::{EntityExpiration, EntityExpirationEntityExt},
    entity_id::EntityId,
    entity_list::EntityList,
    fuzzy_filter::FuzzyFilter,
    list_model_enum,
    report::{self, ReportKind},
    report_table,
    search_query::SearchQueries,
    search_query_ext::SearchQueriesDateTimeRangeExt,
    stock_id::StockId,
    ui::{
        date_time_range_button::DateTimeRangeButton, entity_details_pane::EntityDetailsPane,
        entity_row::EntityRow, search_entry::SearchEntry, send_dialog::SendDialog,
    },
    utils::{new_filter, new_sorter},
    Application,
};

struct S;

impl S {
    const IS: &str = "is";

    const ENTITY_ZONE_VALUES: &[&str] = &[Self::INSIDE, Self::OUTSIDE];
    const INSIDE: &str = "inside";
    const OUTSIDE: &str = "outside";

    const ENTITY_OVERSTAYED_VALUES: &[&str] = &[Self::OVERSTAYED];
    const OVERSTAYED: &str = "overstayed";

    const ENTITY_SEX_VALUES: &[&str] = &[Self::MALE, Self::FEMALE];
    const MALE: &str = "male";
    const FEMALE: &str = "female";

    const ENTITY_EXPIRATION_VALUES: &[&str] = &[
        Self::NO_EXPIRATION,
        Self::NOT_EXPIRING,
        Self::EXPIRING,
        Self::EXPIRED,
        Self::EXPIRING_OR_EXPIRED,
    ];
    const NO_EXPIRATION: &str = "no-expiration";
    const NOT_EXPIRING: &str = "not-expiring";
    const EXPIRING: &str = "expiring";
    const EXPIRED: &str = "expired";
    const EXPIRING_OR_EXPIRED: &str = "expiring-or-expired";

    const FROM: &str = "from";
    const TO: &str = "to";

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

#[derive(Debug, Clone, Copy, glib::Enum)]
#[enum_type(name = "UetsEntityOverstayedFilter")]
enum EntityOverstayedFilter {
    All,
    Overstayed,
}

list_model_enum!(EntityOverstayedFilter);

#[derive(Debug, Clone, Copy, glib::Enum)]
#[enum_type(name = "UetsEntityExpirationStateFilter")]
enum EntityExpirationFilter {
    All,
    NoExpiration,
    NotExpiring,
    Expiring,
    Expired,
    ExpiringOrExpired,
}

list_model_enum!(EntityExpirationFilter);

impl EntityExpirationFilter {
    fn display(&self) -> &'static str {
        match self {
            Self::All => "All",
            Self::NoExpiration => "No Expiration",
            Self::NotExpiring => "Not Expiring",
            Self::Expiring => "Expiring",
            Self::Expired => "Expired",
            Self::ExpiringOrExpired => "Expiring or Expired",
        }
    }
}

#[derive(Debug, Clone, Copy, glib::Enum)]
#[enum_type(name = "UetsEntitySexFilter")]
enum EntitySexFilter {
    All,
    Male,
    Female,
}

list_model_enum!(EntitySexFilter);

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
            Self::IdAsc => "A-Z",
            Self::IdDesc => "Z-A",
            Self::StockAsc => "Stock (A-Z)",
            Self::StockDesc => "Stock (Z-A)",
            Self::UpdatedAsc => "Least Recently Updated",
            Self::UpdatedDesc => "Recently Updated",
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
        pub(super) entity_overstayed_dropdown: TemplateChild<gtk::DropDown>,
        #[template_child]
        pub(super) entity_expiration_dropdown: TemplateChild<gtk::DropDown>,
        #[template_child]
        pub(super) entity_sex_dropdown: TemplateChild<gtk::DropDown>,
        #[template_child]
        pub(super) dt_range_button: TemplateChild<DateTimeRangeButton>,
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

        pub(super) dt_range: RefCell<DateTimeRange>,

        pub(super) entity_zone_dropdown_selected_item_id: OnceCell<glib::SignalHandlerId>,
        pub(super) entity_overstayed_dropdown_selected_item_id: OnceCell<glib::SignalHandlerId>,
        pub(super) entity_expiration_dropdown_selected_item_id: OnceCell<glib::SignalHandlerId>,
        pub(super) entity_sex_dropdown_selected_item_id: OnceCell<glib::SignalHandlerId>,
        pub(super) dt_range_button_range_notify_id: OnceCell<glib::SignalHandlerId>,
        pub(super) entity_sort_dropdown_selected_item_id: OnceCell<glib::SignalHandlerId>,

        pub(super) fuzzy_filter: OnceCell<FuzzyFilter>,

        pub(super) rows: RefCell<Vec<WeakRef<EntityRow>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EntitiesView {
        const NAME: &'static str = "UetsEntitiesView";
        type Type = super::EntitiesView;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action_async(
                "entities-view.share-report",
                Some(&ReportKind::static_variant_type()),
                |obj, _, kind| async move {
                    let kind = kind.unwrap().get::<ReportKind>().unwrap();

                    if let Err(err) = SendDialog::send(
                        &report::file_name("Entities Report", kind),
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

    impl ObjectImpl for EntitiesView {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            Application::get()
                .settings()
                .connect_operation_mode_changed(clone!(
                    #[weak]
                    obj,
                    move |_| {
                        obj.update_entity_expiration_dropdown_visibility();
                        obj.update_entity_sex_dropdown_visibility();
                    }
                ));

            self.search_entry.connect_search_changed(clone!(
                #[weak]
                obj,
                move |entry| {
                    obj.handle_search_entry_search_changed(entry);
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

            self.entity_overstayed_dropdown
                .set_expression(Some(&adw::EnumListItem::this_expression("name")));
            self.entity_overstayed_dropdown
                .set_model(Some(&EntityOverstayedFilter::new_model()));
            let entity_overstayed_dropdown_selected_item_notify_id = self
                .entity_overstayed_dropdown
                .connect_selected_item_notify(clone!(
                    #[weak]
                    obj,
                    move |dropdown| {
                        obj.handle_entity_overstayed_dropdown_selected_item_notify(dropdown);
                    }
                ));
            self.entity_overstayed_dropdown_selected_item_id
                .set(entity_overstayed_dropdown_selected_item_notify_id)
                .unwrap();

            self.entity_expiration_dropdown
                .set_expression(Some(&gtk::ClosureExpression::new::<String>(
                    &[] as &[gtk::Expression],
                    closure!(|list_item: adw::EnumListItem| {
                        EntityExpirationFilter::try_from(list_item.value())
                            .unwrap()
                            .display()
                    }),
                )));
            self.entity_expiration_dropdown
                .set_model(Some(&EntityExpirationFilter::new_model()));
            let entity_expiration_dropdown_selected_item_notify_id = self
                .entity_expiration_dropdown
                .connect_selected_item_notify(clone!(
                    #[weak]
                    obj,
                    move |dropdown| {
                        obj.handle_entity_expiration_dropdown_selected_item_notify(dropdown);
                    }
                ));
            self.entity_expiration_dropdown_selected_item_id
                .set(entity_expiration_dropdown_selected_item_notify_id)
                .unwrap();

            self.entity_sex_dropdown
                .set_expression(Some(&adw::EnumListItem::this_expression("name")));
            self.entity_sex_dropdown
                .set_model(Some(&EntitySexFilter::new_model()));
            let entity_sex_dropdown_selected_item_notify_id = self
                .entity_sex_dropdown
                .connect_selected_item_notify(clone!(
                    #[weak]
                    obj,
                    move |dropdown| {
                        obj.handle_entity_sex_dropdown_selected_item_notify(dropdown);
                    }
                ));
            self.entity_sex_dropdown_selected_item_id
                .set(entity_sex_dropdown_selected_item_notify_id)
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

            let factory = gtk::SignalListItemFactory::new();
            factory.connect_setup(clone!(
                #[weak]
                obj,
                move |_, list_item| {
                    let imp = obj.imp();

                    let list_item = list_item.downcast_ref::<gtk::ListItem>().unwrap();
                    let row = EntityRow::new();
                    list_item
                        .property_expression("item")
                        .bind(&row, "entity", glib::Object::NONE);
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
                        let row = row.downcast_ref::<EntityRow>().unwrap();

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
                let entity_data = entity.data();
                [
                    Some(entity.id().to_string()),
                    entity.stock_id().map(|s| s.to_string()),
                    entity_data.location().cloned(),
                    entity_data.name().cloned(),
                    entity_data.email().cloned(),
                    entity_data.program().cloned(),
                ]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(" ")
            });
            self.sort_list_model.set_sorter(Some(fuzzy_filter.sorter()));
            self.fuzzy_filter.set(fuzzy_filter).unwrap();

            obj.update_fallback_sorter();
            obj.update_entity_expiration_dropdown_visibility();
            obj.update_entity_sex_dropdown_visibility();
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
        imp.search_entry.set_queries(SearchQueries::new());

        let Some(position) = imp.selection_model.iter::<glib::Object>().position(|o| {
            let entity = o.unwrap().downcast::<Entity>().unwrap();
            entity.id() == entity_id
        }) else {
            tracing::warn!("Entity not found: {}", entity_id);
            return;
        };

        imp.selection_model.set_selected(position as u32);

        imp.list_view
            .activate_action("list.scroll-to-item", Some(&(position as u32).to_variant()))
            .unwrap();
    }

    pub fn show_entities_with_stock_id(&self, stock_id: &StockId) {
        let imp = self.imp();

        let mut queries = imp.search_entry.queries();
        queries.remove_all_standalones();
        queries.replace_all_iden_or_insert(S::STOCK, &stock_id.to_string());
        imp.search_entry.set_queries(queries);
    }

    pub async fn create_report(&self, kind: ReportKind) -> Result<Vec<u8>> {
        let imp = self.imp();

        let entities = imp
            .selection_model
            .iter::<glib::Object>()
            .map(|o| o.unwrap().downcast::<Entity>().unwrap())
            .collect::<Vec<_>>();

        let operation_mode = Application::get().settings().operation_mode();
        let valid_entity_field_tys = ValidEntityFields::for_operation_mode(operation_mode)
            .iter()
            .filter(|field_ty| !matches!(field_ty, EntityDataFieldTy::Photo))
            .collect::<Vec<_>>();

        let mut table = report_table::builder("Entities")
            .column("ID")
            .column("Status")
            .rows(entities.iter().map(|entity| {
                let status = entity.status_display(&imp.dt_range.borrow(), operation_mode, false);

                let mut cells = report_table::row_builder()
                    .cell(entity.id().to_string())
                    .cell(status.to_string())
                    .build();

                let data = entity.data();
                for field_ty in &valid_entity_field_tys {
                    match data.get(*field_ty) {
                        Some(field) => {
                            let string = match field {
                                EntityDataField::StockId(i) => i.to_string(),
                                EntityDataField::Location(l) => l.to_owned(),
                                EntityDataField::ExpirationDt(dt) => {
                                    date_time::format::human_readable_date(*dt)
                                }
                                EntityDataField::AllowedDtRange(dt_range) => dt_range.to_string(),
                                EntityDataField::Photo(_) => unreachable!(),
                                EntityDataField::Name(n) => n.to_owned(),
                                EntityDataField::Sex(s) => s.to_string(),
                                EntityDataField::Email(e) => e.to_owned(),
                                EntityDataField::Program(p) => p.to_owned(),
                            };
                            cells.push(string.into());
                        }
                        None if matches!(field_ty, EntityDataFieldTy::AllowedDtRange) => {
                            cells.push(DateTimeRange::default().to_string().into());
                        }
                        None => {
                            cells.push("".to_string().into());
                        }
                    }
                }

                cells
            }))
            .build();

        for field_ty in valid_entity_field_tys {
            table.columns.push(field_ty.to_string());
        }

        report::builder(kind, "Entities Report")
            .prop("Total Entities", entities.len())
            .prop("Search Query", imp.search_entry.queries())
            .table(table)
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
    }

    fn handle_search_entry_search_changed(&self, entry: &SearchEntry) {
        let imp = self.imp();

        let queries = entry.queries();

        let entity_zone = match queries.find_last_with_values(S::IS, S::ENTITY_ZONE_VALUES) {
            Some(S::INSIDE) => EntityZoneFilter::Inside,
            Some(S::OUTSIDE) => EntityZoneFilter::Outside,
            None => EntityZoneFilter::All,
            Some(_) => unreachable!(),
        };

        let selected_item_notify_id = imp.entity_zone_dropdown_selected_item_id.get().unwrap();
        imp.entity_zone_dropdown
            .block_signal(selected_item_notify_id);
        imp.entity_zone_dropdown
            .set_selected(entity_zone.model_position());
        imp.entity_zone_dropdown
            .unblock_signal(selected_item_notify_id);

        let entity_overstayed =
            match queries.find_last_with_values(S::IS, S::ENTITY_OVERSTAYED_VALUES) {
                Some(S::OVERSTAYED) => EntityOverstayedFilter::Overstayed,
                None => EntityOverstayedFilter::All,
                Some(_) => unreachable!(),
            };

        let selected_item_notify_id = imp
            .entity_overstayed_dropdown_selected_item_id
            .get()
            .unwrap();
        imp.entity_overstayed_dropdown
            .block_signal(selected_item_notify_id);
        imp.entity_overstayed_dropdown
            .set_selected(entity_overstayed.model_position());
        imp.entity_overstayed_dropdown
            .unblock_signal(selected_item_notify_id);

        let entity_expiration =
            match queries.find_last_with_values(S::IS, S::ENTITY_EXPIRATION_VALUES) {
                Some(S::NO_EXPIRATION) => EntityExpirationFilter::NoExpiration,
                Some(S::NOT_EXPIRING) => EntityExpirationFilter::NotExpiring,
                Some(S::EXPIRING) => EntityExpirationFilter::Expiring,
                Some(S::EXPIRED) => EntityExpirationFilter::Expired,
                Some(S::EXPIRING_OR_EXPIRED) => EntityExpirationFilter::ExpiringOrExpired,
                None => EntityExpirationFilter::All,
                Some(_) => unreachable!(),
            };

        let selected_item_notify_id = imp
            .entity_expiration_dropdown_selected_item_id
            .get()
            .unwrap();
        imp.entity_expiration_dropdown
            .block_signal(selected_item_notify_id);
        imp.entity_expiration_dropdown
            .set_selected(entity_expiration.model_position());
        imp.entity_expiration_dropdown
            .unblock_signal(selected_item_notify_id);

        let entity_sex = match queries.find_last_with_values(S::IS, S::ENTITY_SEX_VALUES) {
            Some(S::MALE) => EntitySexFilter::Male,
            Some(S::FEMALE) => EntitySexFilter::Female,
            None => EntitySexFilter::All,
            Some(_) => unreachable!(),
        };

        let selected_item_notify_id = imp.entity_sex_dropdown_selected_item_id.get().unwrap();
        imp.entity_sex_dropdown
            .block_signal(selected_item_notify_id);
        imp.entity_sex_dropdown
            .set_selected(entity_sex.model_position());
        imp.entity_sex_dropdown
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
            self.update_fallback_sorter();
            self.update_n_results_label();
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
                every_filter.append(new_filter(move |entity: &Entity| {
                    entity.is_inside_for_dt_range(&dt_range)
                }));
            }
            EntityZoneFilter::Outside => {
                every_filter.append(new_filter(move |entity: &Entity| {
                    !entity.is_inside_for_dt_range(&dt_range)
                }));
            }
        }

        match entity_overstayed {
            EntityOverstayedFilter::All => {}
            EntityOverstayedFilter::Overstayed => {
                every_filter.append(new_filter(|entity: &Entity| {
                    Application::get()
                        .timeline()
                        .entity_entry_tracker()
                        .is_overstayed(entity.id())
                }));
            }
        }

        match entity_expiration {
            EntityExpirationFilter::All => {}
            EntityExpirationFilter::NoExpiration => {
                every_filter.append(new_filter(|entity: &Entity| entity.expiration().is_none()));
            }
            EntityExpirationFilter::NotExpiring => {
                every_filter.append(new_filter(|entity: &Entity| {
                    entity
                        .expiration()
                        .is_some_and(|e| matches!(e, EntityExpiration::NotExpiring))
                }));
            }
            EntityExpirationFilter::Expiring => {
                every_filter.append(new_filter(|entity: &Entity| {
                    entity
                        .expiration()
                        .is_some_and(|e| matches!(e, EntityExpiration::Expiring))
                }));
            }
            EntityExpirationFilter::Expired => {
                every_filter.append(new_filter(|entity: &Entity| {
                    entity
                        .expiration()
                        .is_some_and(|e| matches!(e, EntityExpiration::Expired))
                }));
            }
            EntityExpirationFilter::ExpiringOrExpired => {
                every_filter.append(new_filter(|entity: &Entity| {
                    entity.expiration().is_some_and(|e| {
                        matches!(e, EntityExpiration::Expiring | EntityExpiration::Expired)
                    })
                }));
            }
        }

        match entity_sex {
            EntitySexFilter::All => {}
            EntitySexFilter::Male => {
                every_filter.append(new_filter(|entity: &Entity| {
                    entity.data().sex().is_some_and(|s| s.is_male())
                }));
            }
            EntitySexFilter::Female => {
                every_filter.append(new_filter(|entity: &Entity| {
                    entity.data().sex().is_some_and(|s| s.is_female())
                }));
            }
        }

        let any_stock_filter = gtk::AnyFilter::new();
        for stock_id in queries.all_values(S::STOCK).into_iter().map(StockId::new) {
            any_stock_filter.append(new_filter(move |entity: &Entity| {
                entity.stock_id().is_some_and(|s_id| s_id == stock_id)
            }));
        }

        if any_stock_filter.n_items() == 0 {
            any_stock_filter.append(new_filter(|_: &Entity| true));
        }

        every_filter.append(any_stock_filter);
        imp.filter_list_model.set_filter(Some(&every_filter));

        self.update_fallback_sorter();
        self.update_n_results_label();
    }

    fn handle_entity_overstayed_dropdown_selected_item_notify(&self, dropdown: &gtk::DropDown) {
        let imp = self.imp();

        let selected_item = dropdown
            .selected_item()
            .unwrap()
            .downcast::<adw::EnumListItem>()
            .unwrap();

        let mut queries = imp.search_entry.queries();

        match selected_item.value().try_into().unwrap() {
            EntityOverstayedFilter::All => {
                queries.remove_all(S::IS, S::ENTITY_OVERSTAYED_VALUES);
            }
            EntityOverstayedFilter::Overstayed => {
                queries.replace_all_or_insert(S::IS, S::ENTITY_OVERSTAYED_VALUES, S::OVERSTAYED);
            }
        }

        imp.search_entry.set_queries(queries);
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
                queries.remove_all(S::IS, S::ENTITY_ZONE_VALUES);
            }
            EntityZoneFilter::Inside => {
                queries.replace_all_or_insert(S::IS, S::ENTITY_ZONE_VALUES, S::INSIDE);
            }
            EntityZoneFilter::Outside => {
                queries.replace_all_or_insert(S::IS, S::ENTITY_ZONE_VALUES, S::OUTSIDE);
            }
        }

        imp.search_entry.set_queries(queries);
    }

    fn handle_entity_expiration_dropdown_selected_item_notify(&self, dropdown: &gtk::DropDown) {
        let imp = self.imp();

        let selected_item = dropdown
            .selected_item()
            .unwrap()
            .downcast::<adw::EnumListItem>()
            .unwrap();

        let mut queries = imp.search_entry.queries();

        match selected_item.value().try_into().unwrap() {
            EntityExpirationFilter::All => {
                queries.remove_all(S::IS, S::ENTITY_EXPIRATION_VALUES);
            }
            EntityExpirationFilter::NoExpiration => {
                queries.replace_all_or_insert(S::IS, S::ENTITY_EXPIRATION_VALUES, S::NO_EXPIRATION);
            }
            EntityExpirationFilter::NotExpiring => {
                queries.replace_all_or_insert(S::IS, S::ENTITY_EXPIRATION_VALUES, S::NOT_EXPIRING);
            }
            EntityExpirationFilter::Expiring => {
                queries.replace_all_or_insert(S::IS, S::ENTITY_EXPIRATION_VALUES, S::EXPIRING);
            }
            EntityExpirationFilter::Expired => {
                queries.replace_all_or_insert(S::IS, S::ENTITY_EXPIRATION_VALUES, S::EXPIRED);
            }
            EntityExpirationFilter::ExpiringOrExpired => {
                queries.replace_all_or_insert(
                    S::IS,
                    S::ENTITY_EXPIRATION_VALUES,
                    S::EXPIRING_OR_EXPIRED,
                );
            }
        }

        imp.search_entry.set_queries(queries);
    }

    fn handle_entity_sex_dropdown_selected_item_notify(&self, dropdown: &gtk::DropDown) {
        let imp = self.imp();

        let selected_item = dropdown
            .selected_item()
            .unwrap()
            .downcast::<adw::EnumListItem>()
            .unwrap();

        let mut queries = imp.search_entry.queries();

        match selected_item.value().try_into().unwrap() {
            EntitySexFilter::All => {
                queries.remove_all(S::IS, S::ENTITY_SEX_VALUES);
            }
            EntitySexFilter::Male => {
                queries.replace_all_or_insert(S::IS, S::ENTITY_SEX_VALUES, S::MALE);
            }
            EntitySexFilter::Female => {
                queries.replace_all_or_insert(S::IS, S::ENTITY_SEX_VALUES, S::FEMALE);
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

        imp.search_entry.set_queries(queries);
    }

    fn update_fallback_sorter(&self) {
        let imp = self.imp();

        let queries = imp.search_entry.queries();

        let entity_sort = match queries.find_last_with_values(S::SORT, S::SORT_VALUES) {
            Some(S::ID_ASC) => EntitySort::IdAsc,
            Some(S::ID_DESC) => EntitySort::IdDesc,
            Some(S::STOCK_ASC) => EntitySort::StockAsc,
            Some(S::STOCK_DESC) => EntitySort::StockDesc,
            Some(S::UPDATED_ASC) => EntitySort::UpdatedAsc,
            Some(S::UPDATED_DESC) => EntitySort::UpdatedDesc,
            None => EntitySort::default(),
            Some(_) => unreachable!(),
        };

        let selected_item_notify_id = imp.entity_sort_dropdown_selected_item_id.get().unwrap();
        imp.entity_sort_dropdown
            .block_signal(selected_item_notify_id);
        imp.entity_sort_dropdown
            .set_selected(entity_sort.model_position());
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
                |a: &Entity, b| a.last_action_dt().cmp(&b.last_action_dt()),
            ),
        };

        imp.fuzzy_filter
            .get()
            .unwrap()
            .sorter()
            .set_fallback_sorter(Some(sorter));
    }

    fn update_entity_expiration_dropdown_visibility(&self) {
        let imp = self.imp();

        let is_visible = Application::get()
            .settings()
            .operation_mode()
            .is_valid_entity_data_field_ty(EntityDataFieldTy::ExpirationDt);
        imp.entity_expiration_dropdown.set_visible(is_visible);
    }

    fn update_entity_sex_dropdown_visibility(&self) {
        let imp = self.imp();

        let is_visible = Application::get()
            .settings()
            .operation_mode()
            .is_valid_entity_data_field_ty(EntityDataFieldTy::Sex);
        imp.entity_sex_dropdown.set_visible(is_visible);
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
