use adw::subclass::prelude::*;
use gtk::{glib, prelude::*};

use crate::{camera::Camera, ui::camera_viewfinder::CameraViewfinder};

mod imp {
    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/seadve/Uets/ui/camera_live_feed_dialog.ui")]
    pub struct CameraLiveFeedDialog {
        #[template_child]
        pub(super) list_box: TemplateChild<gtk::ListBox>,
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

    pub fn set_cameras(&self, cameras: Vec<(String, Camera)>) {
        let imp = self.imp();

        for (name, camera) in cameras {
            let label = gtk::Label::builder().xalign(0.0).label(name).build();

            let vf = CameraViewfinder::new();
            vf.set_width_request(240);
            vf.set_height_request(150);
            vf.set_overflow(gtk::Overflow::Hidden);
            vf.set_camera(Some(camera.clone()));
            vf.add_css_class("card");

            let vbox = gtk::Box::builder()
                .orientation(gtk::Orientation::Vertical)
                .margin_top(6)
                .margin_bottom(6)
                .margin_start(6)
                .margin_end(6)
                .spacing(6)
                .build();
            vbox.append(&label);
            vbox.append(&vf);

            let row = gtk::ListBoxRow::builder()
                .activatable(false)
                .selectable(false)
                .child(&vbox)
                .build();

            imp.list_box.append(&row);
        }
    }
}
