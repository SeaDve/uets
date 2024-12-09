use adw::{prelude::*, subclass::prelude::*};
use gtk::{
    gdk,
    glib::{self, clone},
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
        pub(super) subtitle_label: TemplateChild<gtk::Label>,

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
                Some("data"),
                clone!(
                    #[weak]
                    obj,
                    move |_, _| {
                        obj.update_title_label_and_avatar();
                    }
                ),
            );
            entity_signals.connect_notify_local(
                Some("is-inside"),
                clone!(
                    #[weak]
                    obj,
                    move |_, _| {
                        obj.update_subtitle_label();
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

            obj.update_title_label_and_avatar();
            obj.update_subtitle_label();
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

            self.entity_signals
                .get()
                .unwrap()
                .set_target(entity.as_ref());

            self.entity.replace(entity);
            obj.update_title_label_and_avatar();
            obj.update_subtitle_label();
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
        self.update_subtitle_label();
    }

    fn update_title_label_and_avatar(&self) {
        let imp = self.imp();

        if let Some(entity) = self.entity() {
            if let Some(name) = entity.data().name() {
                imp.title_label.set_text(name);

                imp.avatar.set_text(Some(name));
                imp.avatar.set_show_initials(true);
            } else {
                if let Some(stock_id) = entity.stock_id() {
                    imp.title_label
                        .set_markup(&format!("{} (<i>{}</i>)", entity.id(), stock_id));
                } else {
                    imp.title_label.set_text(&entity.id().to_string());
                };

                imp.avatar.set_text(Some(&entity.id().to_string()));
                imp.avatar.set_show_initials(false);
            }

            imp.avatar
                .set_custom_image(entity.data().photo().and_then(|p| {
                    p.texture()
                        .inspect_err(|err| tracing::error!("Failed to load texture: {:?}", err))
                        .ok()
                }));
        } else {
            imp.title_label.set_text("");

            imp.avatar.set_text(None);
            imp.avatar.set_custom_image(gdk::Paintable::NONE);
            imp.avatar.set_show_initials(false);
        }
    }

    fn update_subtitle_label(&self) {
        let imp = self.imp();

        if let Some(entity) = self.entity() {
            let text = if entity.is_inside_for_dt_range(&imp.dt_range.borrow()) {
                "Inside"
            } else {
                "Outside"
            };
            imp.subtitle_label.set_label(text);
        } else {
            imp.subtitle_label.set_label("");
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
