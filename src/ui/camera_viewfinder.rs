use gtk::{
    glib::{self, clone},
    prelude::*,
    subclass::prelude::*,
};

use crate::camera::{Camera, CameraState};

mod imp {
    use std::cell::{OnceCell, RefCell};

    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/seadve/Uets/ui/camera_viewfinder.ui")]
    pub struct CameraViewfinder {
        #[template_child]
        pub(super) stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) spinner: TemplateChild<gtk::Spinner>,
        #[template_child]
        pub(super) loaded: TemplateChild<gtk::Overlay>,
        #[template_child]
        pub(super) picture: TemplateChild<gtk::Picture>,
        #[template_child]
        pub(super) flash_toggle_button: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub(super) message_label: TemplateChild<gtk::Label>,

        pub(super) camera: RefCell<Option<Camera>>,
        pub(super) camera_bindings: glib::BindingGroup,
        pub(super) camera_signals: OnceCell<glib::SignalGroup>,
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

            let obj = self.obj();

            self.flash_toggle_button.connect_active_notify(clone!(
                #[weak]
                obj,
                move |button| {
                    let imp = obj.imp();

                    let camera = imp.camera.borrow().clone();
                    if let Some(camera) = camera {
                        let is_enabled = button.is_active();
                        glib::spawn_future_local(async move {
                            if let Err(err) = camera.set_flash(is_enabled).await {
                                tracing::error!("Failed to set flash: {:?}", err);
                            }
                        });
                    }
                }
            ));

            self.camera_bindings
                .bind("paintable", &*self.picture, "paintable")
                .sync_create()
                .build();

            let signals = glib::SignalGroup::new::<Camera>();
            signals.connect_notify_local(
                Some("state"),
                clone!(
                    #[weak]
                    obj,
                    move |_, _| {
                        obj.update_stack();
                    }
                ),
            );
            self.camera_signals.set(signals).unwrap();

            obj.update_stack();
        }

        fn dispose(&self) {
            self.dispose_template();
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

        imp.camera.replace(camera.clone());

        imp.camera_bindings.set_source(camera.as_ref());
        imp.camera_signals
            .get()
            .unwrap()
            .set_target(camera.as_ref());

        if let Some(camera) = camera {
            if !camera.state().is_running() {
                if let Err(err) = camera.start() {
                    tracing::error!("Failed to start camera: {:?}", err);
                }
            }
        }

        self.update_stack();
    }

    fn update_stack(&self) {
        let imp = self.imp();

        match imp.camera.borrow().as_ref().map(|c| c.state()) {
            None => {
                imp.message_label.set_label("No camera");
                imp.stack.set_visible_child(&*imp.message_label)
            }
            Some(CameraState::Idle) => {
                imp.message_label.set_label("Camera is idle");
                imp.stack.set_visible_child(&*imp.message_label)
            }
            Some(CameraState::Loading) => imp.stack.set_visible_child(&*imp.spinner),
            Some(CameraState::Loaded) => imp.stack.set_visible_child(&*imp.loaded),
            Some(CameraState::Error { message }) => {
                imp.message_label.set_label(&message);
                imp.stack.set_visible_child(&*imp.message_label)
            }
        }
    }
}
