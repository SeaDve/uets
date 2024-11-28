use gtk::{glib, subclass::prelude::*};

use crate::camera::Camera;

mod imp {
    use std::cell::RefCell;

    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/seadve/Uets/ui/camera_viewfinder.ui")]
    pub struct CameraViewfinder {
        #[template_child]
        pub(super) picture: TemplateChild<gtk::Picture>,

        pub(super) camera: RefCell<Option<Camera>>,
        pub(super) camera_bindings: glib::BindingGroup,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for CameraViewfinder {
        const NAME: &'static str = "UetsCameraViewfinder";
        type Type = super::CameraViewfinder;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for CameraViewfinder {
        fn constructed(&self) {
            self.parent_constructed();

            self.camera_bindings
                .bind("paintable", &*self.picture, "paintable")
                .sync_create()
                .build();
        }
    }

    impl WidgetImpl for CameraViewfinder {}
}

glib::wrapper! {
    pub struct CameraViewfinder(ObjectSubclass<imp::CameraViewfinder>)
        @extends gtk::Widget;
}

impl CameraViewfinder {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_camera(&self, camera: Option<Camera>) {
        let imp = self.imp();

        imp.camera_bindings.set_source(camera.as_ref());
        imp.camera.replace(camera);
    }
}
