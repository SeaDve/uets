use std::net::Shutdown;

use anyhow::Result;
use async_net::TcpStream;
use futures_util::{io::BufReader, AsyncBufReadExt, StreamExt};
use gtk::{
    glib::{self, clone, closure_local},
    prelude::*,
    subclass::prelude::*,
};

use crate::remote::Remote;

const PORT: u16 = 8888;

mod imp {
    use std::{cell::RefCell, sync::OnceLock};

    use glib::{subclass::Signal, JoinHandle};

    use super::*;

    #[derive(Default)]
    pub struct RfidReader {
        pub(super) stream: RefCell<Option<TcpStream>>,
        pub(super) handle: RefCell<Option<JoinHandle<()>>>,
        pub(super) ip_addr: RefCell<String>,
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
    pub fn new(ip_addr: String) -> Self {
        let this = glib::Object::new::<Self>();

        let imp = this.imp();
        imp.ip_addr.replace(ip_addr);

        this
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

    pub fn set_ip_addr(&self, ip_addr: String) {
        let imp = self.imp();

        imp.ip_addr.replace(ip_addr);

        self.reconnect();
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

        let ip_addr = imp.ip_addr.borrow().clone();
        tracing::debug!("Trying to connect to {}", ip_addr);

        let stream = TcpStream::connect((ip_addr, PORT)).await?;
        imp.stream.replace(Some(stream.clone()));

        tracing::debug!("Connected to {:?}", stream.peer_addr());

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

impl Remote for RfidReader {
    fn ip_addr(&self) -> String {
        self.imp().ip_addr.borrow().clone()
    }

    fn port(&self) -> u16 {
        PORT
    }
}
