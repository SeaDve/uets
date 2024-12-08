use adw::{prelude::*, subclass::prelude::*};
use gtk::glib::{self, clone, closure_local};

use crate::{
    entity::Entity, entity_id::EntityId, entity_list::EntityList,
    ui::entity_photo_gallery_cell::EntityPhotoGalleryCell, utils,
};

mod imp {
    use std::sync::OnceLock;

    use glib::subclass::Signal;

    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/seadve/Uets/ui/entity_photo_gallery_dialog.ui")]
    pub struct EntityPhotoGalleryDialog {
        #[template_child]
        pub(super) grid_view: TemplateChild<gtk::GridView>,
        #[template_child]
        pub(super) selection_model: TemplateChild<gtk::NoSelection>,
        #[template_child]
        pub(super) filter_list_model: TemplateChild<gtk::FilterListModel>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EntityPhotoGalleryDialog {
        const NAME: &'static str = "UetsEntityPhotoGalleryDialog";
        type Type = super::EntityPhotoGalleryDialog;
        type ParentType = adw::Dialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for EntityPhotoGalleryDialog {
        fn constructed(&self) {
            self.parent_constructed();

            self.grid_view.remove_css_class("view");

            let obj = self.obj();

            let factory = gtk::SignalListItemFactory::new();
            factory.connect_setup(|_, list_item| {
                let list_item = list_item.downcast_ref::<gtk::ListItem>().unwrap();
                list_item.set_selectable(false);

                let cell = EntityPhotoGalleryCell::new();

                list_item
                    .property_expression("item")
                    .bind(&cell, "entity", glib::Object::NONE);

                list_item.set_child(Some(&cell));
            });
            self.grid_view.set_factory(Some(&factory));

            self.grid_view.connect_activate(clone!(
                #[weak]
                obj,
                move |_, position| {
                    let imp = obj.imp();

                    let entity = imp
                        .selection_model
                        .item(position)
                        .expect("position must be valid")
                        .downcast::<Entity>()
                        .unwrap();
                    obj.emit_by_name::<()>("show-entity-request", &[entity.id()]);

                    obj.close();
                }
            ));

            let filter = utils::new_filter(|entity: &Entity| entity.data().photo().is_some());
            self.filter_list_model.set_filter(Some(&filter));
        }

        fn dispose(&self) {
            self.dispose_template();
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();

            SIGNALS.get_or_init(|| {
                vec![Signal::builder("show-entity-request")
                    .param_types([EntityId::static_type()])
                    .build()]
            })
        }
    }

    impl WidgetImpl for EntityPhotoGalleryDialog {}
    impl AdwDialogImpl for EntityPhotoGalleryDialog {}
}

glib::wrapper! {
    pub struct EntityPhotoGalleryDialog(ObjectSubclass<imp::EntityPhotoGalleryDialog>)
        @extends gtk::Widget, adw::Dialog;
}

impl EntityPhotoGalleryDialog {
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

    pub fn set_model(&self, list: Option<&EntityList>) {
        let imp = self.imp();
        imp.filter_list_model.set_model(list);
    }
}
