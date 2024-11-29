use adw::subclass::prelude::*;
use gtk::glib;

use crate::{camera::Camera, ui::camera_viewfinder::CameraViewfinder};

mod imp {
    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/seadve/Uets/ui/camera_live_feed_window.ui")]
    pub struct CameraLiveFeedWindow {
        #[template_child]
        pub(super) viewfinder: TemplateChild<CameraViewfinder>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for CameraLiveFeedWindow {
        const NAME: &'static str = "UetsCameraLiveFeedWindow";
        type Type = super::CameraLiveFeedWindow;
        type ParentType = adw::Dialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for CameraLiveFeedWindow {
        fn dispose(&self) {
            self.dispose_template();
        }
    }

    impl WidgetImpl for CameraLiveFeedWindow {}
    impl AdwDialogImpl for CameraLiveFeedWindow {}
}

glib::wrapper! {
    pub struct CameraLiveFeedWindow(ObjectSubclass<imp::CameraLiveFeedWindow>)
        @extends gtk::Widget, adw::Dialog;
}

impl CameraLiveFeedWindow {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_camera(&self, camera: Option<Camera>) {
        let imp = self.imp();

        imp.viewfinder.set_camera(camera);
    }
}
