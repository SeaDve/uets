use gtk::{
    glib::{self, clone, closure_local},
    prelude::*,
    subclass::prelude::*,
};

use crate::{
    entity::Entity,
    entity_id::EntityId,
    entity_list::EntityList,
    stock_id::StockId,
    ui::{entity_details_pane::EntityDetailsPane, entity_row::EntityRow},
};

mod imp {
    use std::sync::OnceLock;

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
        pub(super) search_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub(super) list_view: TemplateChild<gtk::ListView>,
        #[template_child]
        pub(super) selection_model: TemplateChild<gtk::SingleSelection>,
        #[template_child]
        pub(super) filter_list_model: TemplateChild<gtk::FilterListModel>,
        #[template_child]
        pub(super) details_pane: TemplateChild<EntityDetailsPane>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EntitiesView {
        const NAME: &'static str = "UetsEntitiesView";
        type Type = super::EntitiesView;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            EntityRow::ensure_type();

            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for EntitiesView {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            self.selection_model
                .bind_property("selected-item", &*self.flap, "reveal-flap")
                .transform_to(|_, entity: Option<Entity>| Some(entity.is_some()))
                .sync_create()
                .build();
            self.selection_model
                .bind_property("selected-item", &*self.details_pane, "entity")
                .sync_create()
                .build();

            self.search_entry.connect_search_changed(clone!(
                #[weak]
                obj,
                move |entry| {
                    obj.handle_search_entry_search_changed(entry);
                }
            ));

            self.filter_list_model.connect_items_changed(clone!(
                #[weak]
                obj,
                move |_, _, _, _| {
                    obj.update_stack();
                }
            ));

            self.details_pane.connect_show_stock_request(clone!(
                #[weak]
                obj,
                move |_, id| {
                    obj.emit_by_name::<()>("show-stock-request", &[&id]);
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
        }

        fn dispose(&self) {
            self.dispose_template();
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();

            SIGNALS.get_or_init(|| {
                vec![Signal::builder("show-stock-request")
                    .param_types([StockId::static_type()])
                    .build()]
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

    pub fn bind_entity_list(&self, entity_list: &EntityList) {
        let imp = self.imp();

        imp.filter_list_model.set_model(Some(entity_list));
    }

    pub fn show_entity(&self, entity_id: &EntityId) {
        let imp = self.imp();

        // Clear search filter so we can find the entity
        imp.search_entry.set_text("");

        let position = imp
            .filter_list_model
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

        imp.search_entry.set_text(&format!("stock:{}", stock_id));
    }

    fn handle_search_entry_search_changed(&self, entry: &gtk::SearchEntry) {
        let imp = self.imp();

        let text = entry.text();
        let kv_queries = text
            .split_whitespace()
            .filter_map(|part| part.split_once(':'))
            .collect::<Vec<_>>();

        if kv_queries.is_empty() {
            imp.filter_list_model.set_filter(gtk::Filter::NONE);
            return;
        }

        let every_filter = gtk::EveryFilter::new();
        let any_filter = gtk::AnyFilter::new();

        for (key, value) in kv_queries {
            match key {
                "is" => match value {
                    "inside" => {
                        every_filter.append(gtk::CustomFilter::new(|o| {
                            let entity = o.downcast_ref::<Entity>().unwrap();
                            entity.is_inside()
                        }));
                    }
                    "outside" => {
                        every_filter.append(gtk::CustomFilter::new(|o| {
                            let entity = o.downcast_ref::<Entity>().unwrap();
                            !entity.is_inside()
                        }));
                    }
                    _ => continue,
                },
                "stock" => {
                    let stock_id = StockId::new(value);
                    any_filter.append(gtk::CustomFilter::new(move |o| {
                        let entity = o.downcast_ref::<Entity>().unwrap();
                        entity.stock_id().is_some_and(|s_id| s_id == &stock_id)
                    }));
                }
                _ => {}
            }
        }

        every_filter.append(any_filter);
        imp.filter_list_model.set_filter(Some(&every_filter));
    }

    fn update_stack(&self) {
        let imp = self.imp();

        if imp.filter_list_model.n_items() == 0 {
            imp.stack.set_visible_child(&*imp.empty_page);
        } else {
            imp.stack.set_visible_child(&*imp.main_page);
        }
    }
}
