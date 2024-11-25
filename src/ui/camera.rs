use anyhow::{anyhow, Result};
use futures_channel::oneshot;
use gst::prelude::*;
use gtk::{
    gdk, gio,
    glib::{self, clone, closure_local},
    subclass::prelude::*,
};

mod imp {
    use std::{cell::RefCell, sync::OnceLock};

    use glib::subclass::Signal;
    use gst::bus::BusWatchGuard;

    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/seadve/Uets/ui/camera.ui")]
    pub struct Camera {
        #[template_child]
        pub(super) picture: TemplateChild<gtk::Picture>,

        pub(super) pipeline: RefCell<Option<(gst::Pipeline, BusWatchGuard)>>,
        pub(super) async_done_tx: RefCell<Option<oneshot::Sender<()>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Camera {
        const NAME: &'static str = "UetsCamera";
        type Type = super::Camera;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Camera {
        fn dispose(&self) {
            let obj = self.obj();

            if let Err(err) = obj.stop() {
                tracing::warn!("Failed to stop camera on stop: {:?}", err);
            }

            self.dispose_template();
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

    impl WidgetImpl for Camera {}
}

glib::wrapper! {
    pub struct Camera(ObjectSubclass<imp::Camera>)
        @extends gtk::Widget;
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

        if imp.pipeline.borrow().is_some() {
            tracing::warn!("Camera already started");
            return Ok(());
        }

        let v4l2_device_path = gio::spawn_blocking(v4l2_device_path).await.unwrap()?;
        let pipeline = gst::parse::launch(&format!(
            "v4l2src device={v4l2_device_path} ! tee name=t ! queue ! videoconvert ! zbar ! fakesink t. ! queue ! videoconvert ! gtk4paintablesink name=gtksink"
        ))?
        .downcast::<gst::Pipeline>()
        .unwrap();
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
        imp.pipeline
            .replace(Some((pipeline.clone(), bus_watch_guard)));

        let gtksink = pipeline.by_name("gtksink").unwrap();

        let paintable = gtksink.property::<gdk::Paintable>("paintable");
        imp.picture.set_paintable(Some(&paintable));

        let (async_done_tx, async_done_rx) = oneshot::channel();
        imp.async_done_tx.replace(Some(async_done_tx));

        let state_change = pipeline.set_state(gst::State::Playing)?;
        if state_change != gst::StateChangeSuccess::Async {
            if let Some(async_done_tx) = imp.async_done_tx.take() {
                let _ = async_done_tx.send(());
            }
        }

        async_done_rx.await.unwrap();

        Ok(())
    }

    pub fn stop(&self) -> Result<()> {
        let imp = self.imp();

        imp.picture.set_paintable(gdk::Paintable::NONE);

        if let Some((pipeline, _bus_watch_guard)) = imp.pipeline.take() {
            if let Err(err) = pipeline.set_state(gst::State::Null) {
                tracing::warn!("Failed to set pipeline to Null: {}", err);
            }
        }

        Ok(())
    }

    fn handle_bus_message(&self, message: &gst::Message) -> glib::ControlFlow {
        use gst::MessageView;

        let imp = self.imp();

        match message.view() {
            MessageView::AsyncDone(_) => {
                if let Some(async_done_tx) = imp.async_done_tx.take() {
                    let _ = async_done_tx.send(());
                }

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

                if let Some(async_done_tx) = imp.async_done_tx.take() {
                    let _ = async_done_tx.send(());
                }

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
