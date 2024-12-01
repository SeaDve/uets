use std::net::Shutdown;

use anyhow::Result;
use async_net::TcpStream;
use futures_util::{io::BufReader, AsyncBufReadExt, StreamExt};
use gtk::{
    glib::{self, clone, closure_local},
    prelude::*,
    subclass::prelude::*,
};

mod imp {
    use std::{cell::RefCell, sync::OnceLock};

    use glib::{subclass::Signal, JoinHandle};

    use super::*;

    #[derive(Default)]
    pub struct RfidReader {
        pub(super) stream: RefCell<Option<TcpStream>>,
        pub(super) handle: RefCell<Option<JoinHandle<()>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for RfidReader {
        const NAME: &'static str = "UetsRfidReader";
        type Type = super::RfidReader;
    }

    impl ObjectImpl for RfidReader {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            obj.connect();
        }

        fn dispose(&self) {
            let obj = self.obj();

            obj.disconnect();
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();

            SIGNALS.get_or_init(|| {
                vec![Signal::builder("detected")
                    .param_types([String::static_type()])
                    .build()]
            })
        }
    }
}

glib::wrapper! {
    pub struct RfidReader(ObjectSubclass<imp::RfidReader>);
}

impl RfidReader {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn connect_detected<F>(&self, f: F) -> glib::SignalHandlerId
    where
        F: Fn(&Self, &str) + 'static,
    {
        self.connect_closure(
            "detected",
            false,
            closure_local!(|obj: &Self, id: &str| f(obj, id)),
        )
    }

    pub fn reconnect(&self) {
        self.disconnect();
        self.connect();
    }

    fn connect(&self) {
        let imp = self.imp();

        let handle = glib::spawn_future_local(clone!(
            #[strong(rename_to = obj)]
            self,
            async move {
                if let Err(err) = obj.connect_inner().await {
                    tracing::error!("Failed to connect: {:?}", err);
                }
            }
        ));
        imp.handle.replace(Some(handle));
    }

    async fn connect_inner(&self) -> Result<()> {
        let imp = self.imp();

        // FIXME Find IP Address via mdns or tcp
        let addr = "192.168.100.203:8888";
        let stream = TcpStream::connect(addr).await?;
        imp.stream.replace(Some(stream.clone()));

        tracing::debug!("Connected to {}", addr);

        let reader = BufReader::new(stream);

        let mut lines = reader.lines();
        while let Some(line) = lines.next().await {
            let id = line?;
            self.emit_by_name::<()>("detected", &[&id]);
        }

        Ok(())
    }

    fn disconnect(&self) {
        let imp = self.imp();

        if let Some(stream) = imp.stream.take() {
            if let Err(err) = stream.shutdown(Shutdown::Both) {
                tracing::error!("Failed to shutdown stream: {:?}", err);
            }
        }

        if let Some(prev_handle) = imp.handle.take() {
            prev_handle.abort();
        }
    }
}

impl Default for RfidReader {
    fn default() -> Self {
        Self::new()
    }
}
