use anyhow::Result;
use gst::{prelude::*, subclass::prelude::*};
use gtk::{
    gdk,
    glib::{self, clone, closure_local},
};

const GTK_SINK_NAME: &str = "gtksink";
const RTSP_SRC_NAME: &str = "rtspsrc";

const PORT: u16 = 8080;

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

impl CameraState {
    pub fn is_running(&self) -> bool {
        matches!(self, CameraState::Loading | CameraState::Loaded)
    }
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
        pub(super) ip_addr: RefCell<String>,
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
    pub fn new(ip_addr: String) -> Self {
        let this = glib::Object::new::<Self>();

        let imp = this.imp();
        imp.ip_addr.replace(ip_addr);

        this
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

    pub fn set_ip_addr(&self, ip_addr: String) -> Result<()> {
        let imp = self.imp();

        imp.ip_addr.replace(ip_addr);

        self.restart()
    }

    pub fn start(&self) -> Result<()> {
        let imp = self.imp();

        self.dispose_pipeline();
        self.set_state(CameraState::Loading);

        let pipeline = match gst::parse::launch(&format!("rtspsrc latency=300 name={RTSP_SRC_NAME} ! decodebin ! tee name=t ! queue ! videoconvert ! zbar ! fakesink t. ! queue ! videoconvert ! gtk4paintablesink name={GTK_SINK_NAME}")) {
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

        let uri = format!("rtsp://{}:{PORT}/h264_ulaw.sdp", imp.ip_addr.borrow());
        let rtspsrc = pipeline.by_name(RTSP_SRC_NAME).unwrap();
        rtspsrc.set_property("location", uri);

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

        Ok(())
    }

    pub fn stop(&self) {
        self.dispose_pipeline();
        self.set_state(CameraState::Idle);
    }

    pub fn restart(&self) -> Result<()> {
        self.stop();

        self.start()?;

        Ok(())
    }

    pub async fn snapshot_jpg(&self) -> Result<Vec<u8>> {
        let imp = self.imp();

        let bytes = surf::get(format!("http://{}:{PORT}/shot.jpg", imp.ip_addr.borrow()))
            .recv_bytes()
            .await
            .map_err(|err| err.into_inner())?;

        Ok(bytes)
    }

    pub async fn set_flash(&self, is_enabled: bool) -> Result<()> {
        let imp = self.imp();

        let path = if is_enabled {
            "enabletorch"
        } else {
            "disabletorch"
        };

        surf::get(format!("http://{}:{PORT}/{path}", imp.ip_addr.borrow()))
            .recv_string()
            .await
            .map_err(|err| err.into_inner())?;

        Ok(())
    }

    fn set_state(&self, state: CameraState) {
        let imp = self.imp();

        if state == self.state() {
            return;
        }

        imp.state.replace(state);
        self.notify_state();
    }

    fn dispose_pipeline(&self) {
        let imp = self.imp();

        if let Some((pipeline, _bus_watch_guard)) = imp.pipeline.take() {
            if let Err(err) = pipeline.set_state(gst::State::Null) {
                tracing::warn!("Failed to set pipeline to Null: {}", err);
            }
            self.notify_paintable();
        }
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
