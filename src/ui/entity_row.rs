use gtk::{
    glib::{self, clone},
    prelude::*,
    subclass::prelude::*,
};

use crate::{date_time_range::DateTimeRange, entity::Entity, Application};

mod imp {
    use std::cell::{OnceCell, RefCell};

    use super::*;

    #[derive(Default, glib::Properties, gtk::CompositeTemplate)]
    #[properties(wrapper_type = super::EntityRow)]
    #[template(resource = "/io/github/seadve/Uets/ui/entity_row.ui")]
    pub struct EntityRow {
        #[property(get, set = Self::set_entity, explicit_notify)]
        pub(super) entity: RefCell<Option<Entity>>,

        #[template_child]
        pub(super) hbox: TemplateChild<gtk::Box>, // Unused
        #[template_child]
        pub(super) avatar: TemplateChild<adw::Avatar>,
        #[template_child]
        pub(super) title_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) zone_label: TemplateChild<gtk::Label>,

        pub(super) dt_range: RefCell<DateTimeRange>,

        pub(super) entity_signals: OnceCell<glib::SignalGroup>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EntityRow {
        const NAME: &'static str = "UetsEntityRow";
        type Type = super::EntityRow;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for EntityRow {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            let entity_signals = glib::SignalGroup::new::<Entity>();
            entity_signals.connect_notify_local(
                Some("is-inside"),
                clone!(
                    #[weak]
                    obj,
                    move |_, _| {
                        obj.update_zone_label();
                    }
                ),
            );
            self.entity_signals.set(entity_signals).unwrap();

            Application::get()
                .settings()
                .connect_operation_mode_changed(clone!(
                    #[weak]
                    obj,
                    move |_| {
                        obj.update_avatar_icon_name();
                    }
                ));

            obj.update_zone_label();
            obj.update_avatar_icon_name();
        }

        fn dispose(&self) {
            self.dispose_template();
        }
    }

    impl WidgetImpl for EntityRow {}

    impl EntityRow {
        fn set_entity(&self, entity: Option<Entity>) {
            let obj = self.obj();

            if entity == obj.entity() {
                return;
            }

            if let Some(entity) = &entity {
                let text = if let Some(stock_id) = entity.stock_id() {
                    format!("{} ({})", entity.id(), stock_id)
                } else {
                    entity.id().to_string()
                };
                self.title_label.set_label(&text);

                self.avatar.set_text(Some(&entity.id().to_string()));
            } else {
                self.title_label.set_label("");

                self.avatar.set_text(None);
            }

            self.entity_signals
                .get()
                .unwrap()
                .set_target(entity.as_ref());

            self.entity.replace(entity);
            obj.update_zone_label();
            obj.notify_entity();
        }
    }
}

glib::wrapper! {
    pub struct EntityRow(ObjectSubclass<imp::EntityRow>)
        @extends gtk::Widget;
}

impl EntityRow {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_dt_range(&self, dt_range: DateTimeRange) {
        let imp = self.imp();
        imp.dt_range.replace(dt_range);
        self.update_zone_label();
    }

    fn update_zone_label(&self) {
        let imp = self.imp();

        if let Some(entity) = self.entity() {
            let text = if entity.is_inside_for_dt_range(&imp.dt_range.borrow()) {
                "Inside"
            } else {
                "Outside"
            };
            imp.zone_label.set_label(text);
        } else {
            imp.zone_label.set_label("");
        }
    }

    fn update_avatar_icon_name(&self) {
        let imp = self.imp();

        imp.avatar.set_icon_name(Some(
            Application::get()
                .settings()
                .operation_mode()
                .entities_view_icon_name(),
        ));
    }
}
