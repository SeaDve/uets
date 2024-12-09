use gtk::{
    glib::{self, clone},
    prelude::*,
    subclass::prelude::*,
};

use crate::entity::Entity;

mod imp {
    use std::cell::{OnceCell, RefCell};

    use super::*;

    #[derive(Default, glib::Properties, gtk::CompositeTemplate)]
    #[properties(wrapper_type = super::EntityPhotoGalleryCell)]
    #[template(resource = "/io/github/seadve/Uets/ui/entity_photo_gallery_cell.ui")]
    pub struct EntityPhotoGalleryCell {
        #[property(get, set = Self::set_entity, explicit_notify)]
        pub(super) entity: RefCell<Option<Entity>>,

        #[template_child]
        pub(super) overlay: TemplateChild<gtk::Overlay>, // Unused
        #[template_child]
        pub(super) picture: TemplateChild<gtk::Picture>,
        #[template_child]
        pub(super) label: TemplateChild<gtk::Label>,

        pub(super) entity_signals: OnceCell<glib::SignalGroup>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EntityPhotoGalleryCell {
        const NAME: &'static str = "UetsEntityPhotoGalleryCell";
        type Type = super::EntityPhotoGalleryCell;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for EntityPhotoGalleryCell {
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
                        obj.update_picture();
                        obj.update_label();
                    }
                ),
            );
            self.entity_signals.set(entity_signals).unwrap();

            obj.update_picture();
            obj.update_label();
        }

        fn dispose(&self) {
            self.dispose_template();
        }
    }

    impl WidgetImpl for EntityPhotoGalleryCell {}

    impl EntityPhotoGalleryCell {
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
            obj.update_picture();
            obj.update_label();
            obj.notify_entity();
        }
    }
}

glib::wrapper! {
    pub struct EntityPhotoGalleryCell(ObjectSubclass<imp::EntityPhotoGalleryCell>)
        @extends gtk::Widget;
}

impl EntityPhotoGalleryCell {
    pub fn new() -> Self {
        glib::Object::new()
    }

    fn update_picture(&self) {
        let imp = self.imp();

        let paintable = self.entity().as_ref().and_then(|e| {
            e.data().photo().and_then(|p| {
                p.texture()
                    .inspect_err(|err| tracing::error!("Failed to load texture: {:?}", err))
                    .ok()
                    .cloned()
            })
        });
        imp.picture.set_paintable(paintable.as_ref());
    }

    fn update_label(&self) {
        let imp = self.imp();

        let text = self
            .entity()
            .as_ref()
            .map(|e| {
                e.data()
                    .name()
                    .map_or_else(|| e.id().to_string(), |n| n.clone())
            })
            .unwrap_or_default();
        imp.label.set_label(&text);
    }
}
