use std::time::Duration;

use gtk::{
    glib::{self, clone, closure_local},
    prelude::*,
    subclass::prelude::*,
};

use crate::{camera::Camera, entity_id::EntityId};

mod imp {
    use std::sync::OnceLock;

    use gtk::glib::subclass::Signal;

    use super::*;

    #[derive(Default)]
    pub struct Detector {
        pub(super) camera: Camera,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Detector {
        const NAME: &'static str = "UetsDetector";
        type Type = super::Detector;
    }

    impl ObjectImpl for Detector {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            glib::spawn_future_local(clone!(
                #[weak]
                obj,
                async move {
                    let imp = obj.imp();

                    loop {
                        if !imp.camera.has_started() {
                            if let Err(err) = imp.camera.start().await {
                                tracing::error!("Failed to start camera: {:?}", err);
                            }
                        }

                        glib::timeout_future(Duration::from_secs(3)).await;
                    }
                }
            ));
        }

        fn dispose(&self) {
            self.camera.stop();
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();

            SIGNALS.get_or_init(|| {
                vec![Signal::builder("detected")
                    .param_types([EntityId::static_type()])
                    .build()]
            })
        }
    }
}

glib::wrapper! {
    pub struct Detector(ObjectSubclass<imp::Detector>);
}

impl Detector {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn connect_detected<F>(&self, f: F) -> glib::SignalHandlerId
    where
        F: Fn(&Self, &EntityId) + 'static,
    {
        self.connect_closure(
            "detected",
            false,
            closure_local!(|obj: &Self, id: &EntityId| f(obj, id)),
        )
    }

    pub fn camera(&self) -> &Camera {
        &self.imp().camera
    }

    pub fn simulate_detected(&self, id: &EntityId) {
        self.emit_detected(id);
    }

    fn emit_detected(&self, id: &EntityId) {
        self.emit_by_name("detected", &[id])
    }
}

impl Default for Detector {
    fn default() -> Self {
        Self::new()
    }
}
