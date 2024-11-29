use adw::subclass::prelude::*;
use gtk::glib;

use crate::{camera::Camera, ui::camera_viewfinder::CameraViewfinder};

mod imp {
    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/seadve/Uets/ui/camera_live_feed_dialog.ui")]
    pub struct CameraLiveFeedDialog {
        #[template_child]
        pub(super) viewfinder: TemplateChild<CameraViewfinder>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for CameraLiveFeedDialog {
        const NAME: &'static str = "UetsCameraLiveFeedDialog";
        type Type = super::CameraLiveFeedDialog;
        type ParentType = adw::Dialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for CameraLiveFeedDialog {
        fn dispose(&self) {
            self.dispose_template();
        }
    }

    impl WidgetImpl for CameraLiveFeedDialog {}
    impl AdwDialogImpl for CameraLiveFeedDialog {}
}

glib::wrapper! {
    pub struct CameraLiveFeedDialog(ObjectSubclass<imp::CameraLiveFeedDialog>)
        @extends gtk::Widget, adw::Dialog;
}

impl CameraLiveFeedDialog {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_camera(&self, camera: Option<Camera>) {
        let imp = self.imp();

        imp.viewfinder.set_camera(camera);
    }
}
