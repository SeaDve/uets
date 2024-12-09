use adw::{prelude::*, subclass::prelude::*};
use futures_channel::oneshot;
use gtk::{
    gdk,
    glib::{self, clone, closure_local},
};

use crate::{
    date_time,
    date_time_range::DateTimeRange,
    entity::Entity,
    entity_data::{EntityDataField, EntityDataFieldTy},
    entity_entry_tracker::EntityIdSet,
    entity_expiration::EntityExpiration,
    format,
    stock_id::StockId,
    ui::{entity_data_dialog::EntityDataDialog, information_row::InformationRow},
    Application,
};

mod imp {
    use std::{
        cell::{OnceCell, RefCell},
        sync::OnceLock,
    };

    use glib::subclass::Signal;

    use super::*;

    #[derive(Default, glib::Properties, gtk::CompositeTemplate)]
    #[properties(wrapper_type = super::EntityDetailsPane)]
    #[template(resource = "/io/github/seadve/Uets/ui/entity_details_pane.ui")]
    pub struct EntityDetailsPane {
        #[property(get, set = Self::set_entity, explicit_notify)]
        pub(super) entity: RefCell<Option<Entity>>,

        #[template_child]
        pub(super) vbox: TemplateChild<gtk::Box>, // Unused
        #[template_child]
        pub(super) close_image: TemplateChild<gtk::Image>,
        #[template_child]
        pub(super) id_row: TemplateChild<InformationRow>,
        #[template_child]
        pub(super) stock_id_row: TemplateChild<InformationRow>,
        #[template_child]
        pub(super) is_inside_row: TemplateChild<InformationRow>,
        #[template_child]
        pub(super) data_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub(super) photo_picture_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub(super) photo_picture: TemplateChild<gtk::Picture>,

        pub(super) dt_range: RefCell<DateTimeRange>,
        pub(super) data_group_rows: RefCell<Vec<InformationRow>>,

        pub(super) entity_signals: OnceCell<glib::SignalGroup>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EntityDetailsPane {
        const NAME: &'static str = "UetsEntityDetailsPane";
        type Type = super::EntityDetailsPane;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action("entity-details-pane.show-timeline", None, |obj, _, _| {
                obj.emit_by_name::<()>("show-timeline-request", &[]);
            });
            klass.install_action_async(
                "entity-details-pane.edit-data",
                None,
                |obj, _, _| async move {
                    let Some(entity) = obj.entity() else {
                        return;
                    };

                    let updated_data = match EntityDataDialog::gather_data(
                        entity.id(),
                        &entity.data(),
                        [EntityDataFieldTy::StockId], // FIXME Allow changing stock ID
                        Some(&obj),
                    )
                    .await
                    {
                        Ok(data) => data,
                        Err(oneshot::Canceled) => {
                            return;
                        }
                    };

                    if let Err(err) = Application::get()
                        .timeline()
                        .replace_entity_data(entity.id(), updated_data)
                    {
                        tracing::error!("Failed to update entity data: {:?}", err);
                    }
                },
            );
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for EntityDetailsPane {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            Application::get()
                .timeline()
                .entity_entry_tracker()
                .connect_overstayed_changed(clone!(
                    #[weak]
                    obj,
                    move |_, EntityIdSet(entity_ids)| {
                        if obj.entity().is_some_and(|e| entity_ids.contains(e.id())) {
                            obj.update_is_inside_row();
                        }
                    }
                ));

            let gesture_click = gtk::GestureClick::new();
            gesture_click.connect_released(clone!(
                #[weak]
                obj,
                move |_, n_clicked, _, _| {
                    if n_clicked == 1 {
                        obj.emit_by_name::<()>("close-request", &[]);
                    }
                }
            ));
            self.close_image.add_controller(gesture_click);

            self.stock_id_row.connect_activate_value_link(clone!(
                #[weak]
                obj,
                #[upgrade_or_panic]
                move |_, raw_stock_id| {
                    debug_assert_eq!(
                        obj.entity().unwrap().stock_id(),
                        Some(StockId::new(raw_stock_id))
                    );

                    obj.emit_by_name::<()>("show-stock-request", &[]);
                    glib::Propagation::Stop
                }
            ));

            let entity_signals = glib::SignalGroup::new::<Entity>();
            entity_signals.connect_notify_local(
                Some("data"),
                clone!(
                    #[weak]
                    obj,
                    move |_, _| {
                        obj.update_data_group_rows();
                        obj.update_photo_picture_group();
                    }
                ),
            );
            entity_signals.connect_notify_local(
                Some("is-inside"),
                clone!(
                    #[weak]
                    obj,
                    move |_, _| {
                        obj.update_is_inside_row();
                    }
                ),
            );
            self.entity_signals.set(entity_signals).unwrap();

            obj.update_data_group_rows();
            obj.update_photo_picture_group();
            obj.update_is_inside_row();
        }

        fn dispose(&self) {
            self.dispose_template();
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();

            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("show-stock-request").build(),
                    Signal::builder("show-timeline-request").build(),
                    Signal::builder("close-request").build(),
                ]
            })
        }
    }

    impl WidgetImpl for EntityDetailsPane {}

    impl EntityDetailsPane {
        fn set_entity(&self, entity: Option<Entity>) {
            let obj = self.obj();

            if entity == obj.entity() {
                return;
            }

            self.id_row.set_value(
                entity
                    .as_ref()
                    .map(|s| s.id().to_string())
                    .unwrap_or_default(),
            );
            self.stock_id_row.set_value(
                entity
                    .as_ref()
                    .and_then(|s| {
                        s.stock_id()
                            .map(|s_id| format!("<a href=\"{s_id}\">{s_id}</a>",))
                    })
                    .unwrap_or_default(),
            );

            self.entity_signals
                .get()
                .unwrap()
                .set_target(entity.as_ref());

            self.entity.replace(entity);
            obj.update_data_group_rows();
            obj.update_photo_picture_group();
            obj.update_is_inside_row();
            obj.notify_entity();
        }
    }
}

glib::wrapper! {
    pub struct EntityDetailsPane(ObjectSubclass<imp::EntityDetailsPane>)
        @extends gtk::Widget;
}

impl EntityDetailsPane {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn connect_show_stock_request<F>(&self, f: F) -> glib::SignalHandlerId
    where
        F: Fn(&Self) + 'static,
    {
        self.connect_closure(
            "show-stock-request",
            false,
            closure_local!(|obj: &Self| f(obj)),
        )
    }

    pub fn connect_show_timeline_request<F>(&self, f: F) -> glib::SignalHandlerId
    where
        F: Fn(&Self) + 'static,
    {
        self.connect_closure(
            "show-timeline-request",
            false,
            closure_local!(|obj: &Self| f(obj)),
        )
    }

    pub fn connect_close_request<F>(&self, f: F) -> glib::SignalHandlerId
    where
        F: Fn(&Self) + 'static,
    {
        self.connect_closure("close-request", false, closure_local!(|obj: &Self| f(obj)))
    }

    pub fn set_dt_range(&self, dt_range: DateTimeRange) {
        let imp = self.imp();
        imp.dt_range.replace(dt_range);
        self.update_is_inside_row();
    }

    fn update_data_group_rows(&self) {
        let imp = self.imp();

        for row in imp.data_group_rows.take() {
            imp.data_group.remove(&row);
        }

        if let Some(entity) = self.entity() {
            let entity_data = entity.data();
            let mut fields = entity_data.fields().collect::<Vec<_>>();

            let default_allowed_dt_range_field =
                EntityDataField::AllowedDtRange(DateTimeRange::default());
            if !entity_data.has_field(EntityDataFieldTy::AllowedDtRange) {
                fields.push(&default_allowed_dt_range_field);
            }

            for field in fields {
                if matches!(
                    field.ty(),
                    EntityDataFieldTy::StockId | EntityDataFieldTy::Photo
                ) {
                    continue;
                }

                let row = InformationRow::new();
                row.set_title(&field.ty().to_string());

                let value = match field {
                    EntityDataField::ExpirationDt(dt) => {
                        let date_fmt = date_time::format::human_readable_date(*dt);
                        if EntityExpiration::for_expiration_dt(*dt).is_some_and(|e| {
                            matches!(e, EntityExpiration::Expiring | EntityExpiration::Expired)
                        }) {
                            format::red_markup(&date_fmt)
                        } else {
                            glib::markup_escape_text(&date_fmt).to_string()
                        }
                    }
                    _ => glib::markup_escape_text(&field.to_string()).to_string(),
                };
                row.set_value_use_markup(true);
                row.set_value(value);

                imp.data_group.add(&row);
                imp.data_group_rows.borrow_mut().push(row);
            }
        }
    }

    fn update_photo_picture_group(&self) {
        let imp = self.imp();

        if let Some(photo) = self.entity().and_then(|e| e.data().photo().cloned()) {
            imp.photo_picture.set_paintable(
                photo
                    .texture()
                    .inspect_err(|err| {
                        tracing::debug!("Failed to load photo texture: {:?}", err);
                    })
                    .ok(),
            );
            imp.photo_picture_group.set_visible(true);
        } else {
            imp.photo_picture.set_paintable(gdk::Paintable::NONE);
            imp.photo_picture_group.set_visible(false);
        }
    }

    fn update_is_inside_row(&self) {
        let imp = self.imp();

        if let Some(entity) = self.entity() {
            let value = if entity.is_inside_for_dt_range(&imp.dt_range.borrow()) {
                "Yes"
            } else {
                "No"
            };
            if Application::get()
                .timeline()
                .entity_entry_tracker()
                .is_overstayed(entity.id())
            {
                imp.is_inside_row.set_value_use_markup(true);
                imp.is_inside_row.set_value(format::red_markup(value));
            } else {
                imp.is_inside_row.set_value_use_markup(false);
                imp.is_inside_row.set_value(value);
            }
        } else {
            imp.is_inside_row.set_value_use_markup(false);
            imp.is_inside_row.set_value("");
        }
    }
}
