use gtk::{
    glib::{self, clone},
    prelude::*,
    subclass::prelude::*,
};

use crate::{
    camera::{Camera, CameraState},
    jpeg_image::JpegImage,
    Application,
};

mod imp {
    use std::cell::{Cell, OnceCell, RefCell};

    use super::*;

    #[derive(Default, glib::Properties, gtk::CompositeTemplate)]
    #[properties(wrapper_type = super::CameraViewfinder)]
    #[template(resource = "/io/github/seadve/Uets/ui/camera_viewfinder.ui")]
    pub struct CameraViewfinder {
        #[property(get, set = Self::set_enables_capture, explicit_notify)]
        pub(super) enables_capture: Cell<bool>,

        #[template_child]
        pub(super) stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) spinner_page: TemplateChild<gtk::Spinner>,
        #[template_child]
        pub(super) loaded_page: TemplateChild<gtk::Overlay>,
        #[template_child]
        pub(super) picture: TemplateChild<gtk::Picture>,
        #[template_child]
        pub(super) flash_toggle_button: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub(super) capture_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) message_label_page: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) capture_page: TemplateChild<gtk::Overlay>,
        #[template_child]
        pub(super) capture_picture: TemplateChild<gtk::Picture>,

        pub(super) camera: RefCell<Option<Camera>>,
        pub(super) camera_bindings: glib::BindingGroup,
        pub(super) camera_signals: OnceCell<glib::SignalGroup>,

        pub(super) capture_image: RefCell<Option<JpegImage>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for CameraViewfinder {
        const NAME: &'static str = "UetsCameraViewfinder";
        type Type = super::CameraViewfinder;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action_async("camera-viewfinder.capture", None, |obj, _, _| async move {
                let imp = obj.imp();

                let Some(camera) = imp.camera.borrow().clone() else {
                    return;
                };

                obj.action_set_enabled("camera-viewfinder.capture", false);
                let ret = camera.capture_jpeg().await;
                obj.action_set_enabled("camera-viewfinder.capture", true);

                match ret {
                    Ok(image) => obj.set_capture_image(Some(image)),
                    Err(err) => {
                        tracing::error!("Failed to capture image: {:?}", err);

                        Application::get().add_message_toast("Can't capture image");
                    }
                }
            });

            klass.install_action("camera-viewfinder.capture-reset", None, |obj, _, _| {
                obj.set_capture_image(None);
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
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
            obj.update_capture_button_visibility();
        }

        fn dispose(&self) {
            self.dispose_template();
        }
    }

    impl WidgetImpl for CameraViewfinder {}

    impl CameraViewfinder {
        fn set_enables_capture(&self, enables_capture: bool) {
            let obj = self.obj();

            if enables_capture == obj.enables_capture() {
                return;
            }

            self.enables_capture.set(enables_capture);
            obj.update_capture_button_visibility();
            obj.notify_enables_capture();
        }
    }
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

    pub fn capture_image(&self) -> Option<JpegImage> {
        self.imp().capture_image.borrow().clone()
    }

    pub fn set_capture_image(&self, image: Option<JpegImage>) {
        let imp = self.imp();

        imp.capture_picture
            .set_paintable(image.as_ref().and_then(|i| {
                i.texture()
                    .inspect_err(|err| tracing::debug!("Failed to load texture: {:?}", err))
                    .ok()
            }));
        imp.capture_image.replace(image);

        self.update_stack();
    }

    fn update_stack(&self) {
        let imp = self.imp();

        if imp.capture_image.borrow().is_some() {
            imp.stack.set_visible_child(&*imp.capture_page);
            return;
        }

        match imp.camera.borrow().as_ref().map(|c| c.state()) {
            None => {
                imp.message_label_page.set_label("No camera");
                imp.stack.set_visible_child(&*imp.message_label_page)
            }
            Some(CameraState::Idle) => {
                imp.message_label_page.set_label("Camera is idle");
                imp.stack.set_visible_child(&*imp.message_label_page)
            }
            Some(CameraState::Loading) => imp.stack.set_visible_child(&*imp.spinner_page),
            Some(CameraState::Loaded) => imp.stack.set_visible_child(&*imp.loaded_page),
            Some(CameraState::Error { message }) => {
                imp.message_label_page.set_label(&message);
                imp.stack.set_visible_child(&*imp.message_label_page)
            }
        }
    }

    fn update_capture_button_visibility(&self) {
        let imp = self.imp();

        let enables_capture = self.enables_capture();
        imp.capture_button.set_visible(enables_capture);
    }
}
