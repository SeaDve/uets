use gtk::{
    glib::{self, clone, closure_local},
    prelude::*,
    subclass::prelude::*,
};

use crate::{
    date_time, entity::Entity, entity_entry_tracker::EntityEntryTrackerSettingsExt,
    entity_id::EntityId, format, stock_id::StockId, timeline_item::TimelineItem,
    timeline_item_kind::TimelineItemKind, Application,
};

mod imp {
    use std::{
        cell::{OnceCell, RefCell},
        sync::OnceLock,
    };

    use glib::subclass::Signal;

    use super::*;

    #[derive(Default, glib::Properties, gtk::CompositeTemplate)]
    #[properties(wrapper_type = super::TimelineRow)]
    #[template(resource = "/io/github/seadve/Uets/ui/timeline_row.ui")]
    pub struct TimelineRow {
        #[property(get, set = Self::set_item, explicit_notify)]
        pub(super) item: RefCell<Option<TimelineItem>>,

        #[template_child]
        pub(super) hbox: TemplateChild<gtk::Box>, // Unused
        #[template_child]
        pub(super) image: TemplateChild<gtk::Image>,
        #[template_child]
        pub(super) dt_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) status_label: TemplateChild<gtk::Label>,

        pub(super) entity_signals: OnceCell<glib::SignalGroup>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TimelineRow {
        const NAME: &'static str = "UetsTimelineRow";
        type Type = super::TimelineRow;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for TimelineRow {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            let app = Application::get();
            let settings = app.settings();

            settings.connect_operation_mode_changed(clone!(
                #[weak]
                obj,
                move |_| {
                    obj.update_status_label();
                }
            ));
            settings.connect_max_entry_to_exit_duration_secs_changed(clone!(
                #[weak]
                obj,
                move |_| {
                    obj.update_status_label();
                }
            ));

            let entity_signals = glib::SignalGroup::new::<Entity>();
            entity_signals.connect_notify_local(
                Some("data"),
                clone!(
                    #[weak]
                    obj,
                    move |_, _| {
                        obj.update_status_label();
                    }
                ),
            );
            self.entity_signals.set(entity_signals).unwrap();

            self.status_label.connect_activate_link(clone!(
                #[weak]
                obj,
                #[upgrade_or_panic]
                move |_, uri| {
                    if let Some((scheme, raw_id)) = uri.split_once(":") {
                        match scheme {
                            "entity" => {
                                let entity_id = EntityId::new(raw_id);
                                obj.emit_by_name::<()>("show-entity-request", &[&entity_id]);
                            }
                            "stock" => {
                                let stock_id = StockId::new(raw_id);
                                obj.emit_by_name::<()>("show-stock-request", &[&stock_id]);
                            }
                            _ => unreachable!("invalid scheme `{scheme}`"),
                        }
                    }
                    glib::Propagation::Stop
                }
            ));

            obj.update_status_label();
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

    impl WidgetImpl for TimelineRow {}

    impl TimelineRow {
        fn set_item(&self, item: Option<TimelineItem>) {
            let obj = self.obj();

            if item == obj.item() {
                return;
            }

            if let Some(item) = &item {
                let dt_fuzzy_text = date_time::format::fuzzy(item.dt());
                self.dt_label.set_text(&dt_fuzzy_text);

                match item.kind() {
                    TimelineItemKind::Entry => {
                        self.image.set_icon_name(Some("arrow4-right-symbolic"));
                        self.image.remove_css_class("exit-icon");
                        self.image.add_css_class("entry-icon");
                    }
                    TimelineItemKind::Exit => {
                        self.image.set_icon_name(Some("arrow4-left-symbolic"));
                        self.image.remove_css_class("entry-icon");
                        self.image.add_css_class("exit-icon");
                    }
                }
            } else {
                self.dt_label.set_text("");
            }

            let entity = item.as_ref().map(|item| {
                Application::get()
                    .timeline()
                    .entity_list()
                    .get(item.entity_id())
                    .expect("entity must be known")
            });
            self.entity_signals
                .get()
                .unwrap()
                .set_target(entity.as_ref());

            self.item.replace(item);
            obj.update_status_label();
            obj.notify_item();
        }
    }
}

glib::wrapper! {
    pub struct TimelineRow(ObjectSubclass<imp::TimelineRow>)
        @extends gtk::Widget;
}

impl TimelineRow {
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

    fn update_status_label(&self) {
        let imp = self.imp();

        if let Some(item) = &self.item() {
            let entity_id = item.entity_id();

            let app = Application::get();

            let entity = app
                .timeline()
                .entity_list()
                .get(entity_id)
                .expect("entity must be known");

            let entity_id_escaped = glib::markup_escape_text(&entity_id.to_string());
            let entity_uri = format!("entity:{}", entity_id_escaped);
            let title = if let Some(stock_id) = entity.stock_id() {
                let stock_id_escaped = glib::markup_escape_text(&stock_id.to_string());
                let stock_uri = format!("stock:{}", stock_id_escaped);
                format!("<a href=\"{stock_uri}\">{stock_id_escaped}</a> (<a href=\"{entity_uri}\">{entity_id_escaped}</a>)")
            } else {
                let entity_display = &entity
                    .data()
                    .name()
                    .cloned()
                    .map_or_else(|| entity_id_escaped, |name| glib::markup_escape_text(&name));
                format!("<a href=\"{entity_uri}\">{entity_display}</a>")
            };

            let settings = app.settings();
            let operation_mode = settings.operation_mode();

            let text = match item.kind() {
                TimelineItemKind::Entry => {
                    format!("<b>{}</b> {}", title, operation_mode.enter_verb())
                }
                TimelineItemKind::Exit => {
                    let entry_to_exit_duration = item
                        .entry_to_exit_duration()
                        .expect("entry to exit duration must have been set on exit");
                    let entry_to_exit_duration_formatted = format::duration(entry_to_exit_duration);
                    format!(
                        "<b>{}</b> {} after <i>{}</i> {}",
                        title,
                        operation_mode.exit_verb(),
                        if settings.compute_overstayed(entry_to_exit_duration) {
                            format::red_markup(&entry_to_exit_duration_formatted)
                        } else {
                            entry_to_exit_duration_formatted
                        },
                        operation_mode.entry_to_exit_duration_suffix(),
                    )
                }
            };
            imp.status_label.set_markup(&text);
        } else {
            imp.status_label.set_text("");
        }
    }
}
