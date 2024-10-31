use gtk::{
    glib::{self, clone, closure_local},
    prelude::*,
    subclass::prelude::*,
};

use crate::{entity::Entity, ui::information_row::InformationRow};

mod imp {
    use std::{cell::RefCell, sync::OnceLock};

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

        pub(super) entity_bindings: glib::BindingGroup,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EntityDetailsPane {
        const NAME: &'static str = "UetsEntityDetailsPane";
        type Type = super::EntityDetailsPane;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
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

            self.entity_bindings
                .bind("is-inside", &*self.is_inside_row, "value")
                .transform_to(|_, n_inside| {
                    let is_inside = n_inside.get::<bool>().unwrap();
                    let ret = if is_inside { "Yes" } else { "No" };
                    Some(ret.to_string().into())
                })
                .flags(glib::BindingFlags::SYNC_CREATE)
                .build();
        }

        fn dispose(&self) {
            self.dispose_template();
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();

            SIGNALS.get_or_init(|| vec![Signal::builder("close-request").build()])
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
                    .and_then(|s| s.stock_id().map(|s_id| s_id.to_string()))
                    .unwrap_or_default(),
            );

            self.entity_bindings.set_source(entity.as_ref());

            self.entity.replace(entity);
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

    pub fn connect_close_request<F>(&self, f: F) -> glib::SignalHandlerId
    where
        F: Fn(&Self) + 'static,
    {
        self.connect_closure("close-request", false, closure_local!(|obj: &Self| f(obj)))
    }
}
