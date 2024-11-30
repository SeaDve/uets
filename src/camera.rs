use anyhow::{anyhow, ensure, Result};
use gst::{prelude::*, subclass::prelude::*};
use gtk::{
    gdk, gio,
    glib::{self, clone, closure_local},
};

const GTK_SINK_NAME: &str = "gtksink";
const V4L2_SRC_NAME: &str = "v4l2src";

#[derive(Debug, Default, Clone, PartialEq, Eq, glib::Boxed)]
#[boxed_type(name = "UetsCameraState")]
pub enum CameraState {
    #[default]
    Idle,
    Loading,
    Loaded,
    Error {
        message: String,
    },
}

mod imp {
    use std::{cell::RefCell, marker::PhantomData, sync::OnceLock};

    use glib::subclass::Signal;
    use gst::bus::BusWatchGuard;

    use super::*;

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::Camera)]
    pub struct Camera {
        #[property(get = Self::paintable)]
        pub(super) paintable: PhantomData<Option<gdk::Paintable>>,
        #[property(get)]
        pub(super) state: RefCell<CameraState>,

        pub(super) pipeline: RefCell<Option<(gst::Pipeline, BusWatchGuard)>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Camera {
        const NAME: &'static str = "UetsCamera";
        type Type = super::Camera;
    }

    #[glib::derived_properties]
    impl ObjectImpl for Camera {
        fn dispose(&self) {
            let obj = self.obj();

            obj.stop();
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();

            SIGNALS.get_or_init(|| {
                vec![Signal::builder("code-detected")
                    .param_types([String::static_type()])
                    .build()]
            })
        }
    }

    impl Camera {
        fn paintable(&self) -> Option<gdk::Paintable> {
            self.pipeline.borrow().as_ref().map(|(pipeline, _)| {
                let gtksink = pipeline.by_name(GTK_SINK_NAME).unwrap();
                gtksink.property::<gdk::Paintable>("paintable")
            })
        }
    }
}

glib::wrapper! {
    pub struct Camera(ObjectSubclass<imp::Camera>);
}

impl Camera {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn connect_code_detected<F>(&self, f: F) -> glib::SignalHandlerId
    where
        F: Fn(&Self, &str) + 'static,
    {
        self.connect_closure(
            "code-detected",
            false,
            closure_local!(|obj: &Self, code: &str| f(obj, code)),
        )
    }

    pub async fn start(&self) -> Result<()> {
        let imp = self.imp();

        ensure!(
            matches!(self.state(), CameraState::Idle | CameraState::Error { .. }),
            "Camera is already loading or loaded"
        );

        self.set_state(CameraState::Loading);

        let v4l2_device_path = gio::spawn_blocking(v4l2_device_path).await.unwrap();

        let pipeline = match gst::parse::launch(&format!("v4l2src name={V4L2_SRC_NAME} ! tee name=t ! queue ! videoconvert ! zbar ! fakesink t. ! queue ! videoconvert ! gtk4paintablesink name={GTK_SINK_NAME}")) {
            Ok(pipeline) => pipeline.downcast::<gst::Pipeline>().unwrap(),
            Err(err) => {
                self.set_state(CameraState::Error {
                    message: err.to_string(),
                });
                return Err(err.into());
            }
        };
        let bus_watch_guard = pipeline
            .bus()
            .unwrap()
            .add_watch_local(clone!(
                #[weak(rename_to = obj)]
                self,
                #[upgrade_or_panic]
                move |_, message| obj.handle_bus_message(message)
            ))
            .unwrap();

        match v4l2_device_path {
            Ok(device_path) => {
                let v4l2src = pipeline.by_name(V4L2_SRC_NAME).unwrap();
                v4l2src.set_property("device", &device_path);
            }
            Err(err) => {
                tracing::debug!("Failed to get v4l2 device path: {:?}", err);
            }
        }

        imp.pipeline
            .replace(Some((pipeline.clone(), bus_watch_guard)));
        self.notify_paintable();

        let state_change = match pipeline.set_state(gst::State::Playing) {
            Ok(state_change) => state_change,
            Err(err) => {
                self.set_state(CameraState::Error {
                    message: err.to_string(),
                });
                return Err(err.into());
            }
        };
        if state_change != gst::StateChangeSuccess::Async {
            self.set_state(CameraState::Loaded);
        }

        tracing::debug!("Camera started");

        Ok(())
    }

    pub fn stop(&self) {
        let imp = self.imp();

        if let Some((pipeline, _bus_watch_guard)) = imp.pipeline.take() {
            if let Err(err) = pipeline.set_state(gst::State::Null) {
                tracing::warn!("Failed to set pipeline to Null: {}", err);
            }
            self.notify_paintable();
        }

        self.set_state(CameraState::Idle);

        tracing::debug!("Camera stopped");
    }

    fn set_state(&self, state: CameraState) {
        let imp = self.imp();

        if state == self.state() {
            return;
        }

        imp.state.replace(state);
        self.notify_state();
    }

    fn handle_bus_message(&self, message: &gst::Message) -> glib::ControlFlow {
        use gst::MessageView;

        let imp = self.imp();

        match message.view() {
            MessageView::AsyncDone(_) => {
                self.set_state(CameraState::Loaded);

                glib::ControlFlow::Continue
            }
            MessageView::Element(e) => {
                if e.has_name("barcode") {
                    let structure = e.structure().unwrap();
                    let symbol = structure.get::<String>("symbol").unwrap();
                    let symbol_type = structure.get::<String>("type").unwrap();

                    tracing::debug!("Detected barcode: {} ({})", symbol, symbol_type);
                    self.emit_by_name::<()>("code-detected", &[&symbol]);
                }

                glib::ControlFlow::Continue
            }
            MessageView::Eos(_) => {
                tracing::debug!("Eos signal received from record bus");

                glib::ControlFlow::Break
            }
            MessageView::StateChanged(sc) => {
                let new_state = sc.current();

                if message.src()
                    != imp
                        .pipeline
                        .borrow()
                        .as_ref()
                        .map(|(pipeline, _)| pipeline.upcast_ref::<gst::Object>())
                {
                    tracing::trace!(
                        "`{}` changed state from `{:?}` -> `{:?}`",
                        message
                            .src()
                            .map_or_else(|| "<unknown source>".into(), |e| e.name()),
                        sc.old(),
                        new_state,
                    );
                    return glib::ControlFlow::Continue;
                }

                tracing::trace!(
                    "Pipeline changed state from `{:?}` -> `{:?}`",
                    sc.old(),
                    new_state,
                );

                glib::ControlFlow::Continue
            }
            MessageView::Error(e) => {
                tracing::error!("Received error message on bus: {:?}", e);

                self.set_state(CameraState::Error {
                    message: e.error().to_string(),
                });

                glib::ControlFlow::Break
            }
            MessageView::Warning(w) => {
                tracing::warn!("Received warning message on bus: {:?}", w);

                glib::ControlFlow::Continue
            }
            MessageView::Info(i) => {
                tracing::debug!("Received info message on bus: {:?}", i);

                glib::ControlFlow::Continue
            }
            other => {
                tracing::trace!("Received other message on bus: {:?}", other);

                glib::ControlFlow::Continue
            }
        }
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self::new()
    }
}

fn v4l2_device_path() -> Result<String> {
    let m = gst::DeviceMonitor::new();

    m.start()?;
    let devices = m.devices();
    m.stop();

    for device in devices {
        if !device.has_classes("Video/Source") {
            continue;
        }

        let Some(properties) = device.properties() else {
            continue;
        };

        if !properties
            .get::<String>("device.api")
            .is_ok_and(|api| api == "v4l2")
        {
            continue;
        }

        if let Ok(path) = properties.get::<String>("device.path") {
            return Ok(path);
        };
    }

    Err(anyhow!("Failed to find a v4l2 device"))
}
