use std::{collections::HashMap, future::Future};

use anyhow::{Context, Result};
use async_channel::Sender;
use async_lock::Mutex;
use async_tungstenite::tungstenite;
use futures_util::{select, FutureExt, StreamExt};
use gtk::{
    glib::{self, clone, closure_local},
    prelude::*,
    subclass::prelude::*,
};
use serde::{Deserialize, Serialize};

use crate::{
    entity_data::{EntityDataField, EntityDataFieldVecBoxed},
    remote::Remote,
    timeline::Timeline,
};

const PORT: u16 = 8888;

#[derive(Debug, PartialEq, Eq, Hash, Serialize)]
enum Property {
    NInside,
}

#[derive(Debug, Deserialize)]
enum SocketIncoming {
    Code(String),
    Tag(String, Vec<EntityDataField>),
    RequestProperties,
}

#[derive(Debug, Serialize)]
enum SocketOutgoing {
    Message(String),
    Properties(HashMap<Property, serde_json::Value>),
}

enum WsCommand {
    Close,
    Send(tungstenite::Message),
}

mod imp {
    use std::{
        cell::{OnceCell, RefCell},
        sync::OnceLock,
    };

    use glib::subclass::Signal;

    use super::*;

    #[derive(Default)]
    pub struct RemoteApp {
        pub(super) ip_addr: RefCell<String>,

        pub(super) command_tx: Mutex<Option<Sender<WsCommand>>>,

        pub(super) timeline: OnceCell<Timeline>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for RemoteApp {
        const NAME: &'static str = "UetsRemoteApp";
        type Type = super::RemoteApp;
    }

    impl ObjectImpl for RemoteApp {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            glib::spawn_future_local(clone!(
                #[weak]
                obj,
                async move {
                    if let Err(err) = obj.run_client().await {
                        tracing::error!("Failed to run_client: {:?}", err);
                    }
                }
            ));
        }

        fn dispose(&self) {
            let obj = self.obj();

            obj.stop();
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();

            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("code-detected")
                        .param_types([String::static_type()])
                        .build(),
                    Signal::builder("tag-detected")
                        .param_types([
                            String::static_type(),
                            EntityDataFieldVecBoxed::static_type(),
                        ])
                        .build(),
                ]
            })
        }
    }
}

glib::wrapper! {
    pub struct RemoteApp(ObjectSubclass<imp::RemoteApp>);
}

impl RemoteApp {
    pub fn new(ip_addr: String) -> Self {
        let this = glib::Object::new::<Self>();

        let imp = this.imp();
        imp.ip_addr.replace(ip_addr);

        this
    }

    pub fn bind_timeline(&self, timeline: &Timeline) {
        let imp = self.imp();

        timeline.connect_n_inside_notify(clone!(
            #[weak(rename_to = obj)]
            self,
            move |timeline| {
                let n_inside = timeline.n_inside();
                obj.ws_send_properties_helper(async move {
                    let mut props = HashMap::new();
                    props.insert(Property::NInside, serde_json::to_value(n_inside)?);
                    Ok(props)
                });
            }
        ));

        imp.timeline.set(timeline.clone()).unwrap();
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

    pub fn connect_tag_detected<F>(&self, f: F) -> glib::SignalHandlerId
    where
        F: Fn(&Self, &str, &EntityDataFieldVecBoxed) + 'static,
    {
        self.connect_closure(
            "tag-detected",
            false,
            closure_local!(
                |obj: &Self, tag: &str, data_fields_boxed: &EntityDataFieldVecBoxed| f(
                    obj,
                    tag,
                    data_fields_boxed
                )
            ),
        )
    }

    pub fn stop(&self) {
        glib::spawn_future_local(clone!(
            #[weak(rename_to = obj)]
            self,
            async move {
                let imp = obj.imp();

                if let Some(tx) = imp.command_tx.lock().await.take() {
                    let _ = tx.send(WsCommand::Close).await;
                };
            }
        ));
    }

    pub async fn ws_send_message(&self, message: &str) -> Result<()> {
        self.ws_send(&SocketOutgoing::Message(message.to_string()))
            .await
    }

    async fn ws_send_all_properties(&self) -> Result<()> {
        let imp = self.imp();

        let mut props = HashMap::new();

        if let Some(timeline) = imp.timeline.get() {
            props.insert(
                Property::NInside,
                serde_json::to_value(timeline.n_inside())?,
            );
        }

        self.ws_send_properties(props).await
    }

    fn ws_send_properties_helper(
        &self,
        props: impl Future<Output = Result<HashMap<Property, serde_json::Value>>> + 'static,
    ) {
        glib::spawn_future_local(clone!(
            #[weak(rename_to = obj)]
            self,
            async move {
                match props.await {
                    Ok(props) => {
                        if let Err(err) = obj.ws_send_properties(props).await {
                            tracing::error!("Failed to send properties: {:?}", err);
                        }
                    }
                    Err(err) => {
                        tracing::error!("Failed to get properties: {:?}", err);
                    }
                }
            }
        ));
    }

    async fn ws_send_properties(&self, props: HashMap<Property, serde_json::Value>) -> Result<()> {
        self.ws_send(&SocketOutgoing::Properties(props)).await
    }

    async fn ws_send(&self, outgoing: &SocketOutgoing) -> Result<()> {
        let imp = self.imp();

        let text = serde_json::to_string(outgoing)?;
        imp.command_tx
            .lock()
            .await
            .as_ref()
            .context("Command channel not initialized")?
            .send(WsCommand::Send(tungstenite::Message::Text(text.into())))
            .await?;

        tracing::debug!("Sent outgoing: {:?}", outgoing);

        Ok(())
    }

    async fn run_client(&self) -> Result<()> {
        let imp = self.imp();

        let (tx, rx) = async_channel::bounded::<WsCommand>(1);
        imp.command_tx.lock().await.replace(tx);

        let url = format!("ws://{}:{}/ws", imp.ip_addr.borrow(), PORT);
        let (mut ws_stream, _) = async_tungstenite::gio::connect_async(&url).await?;

        loop {
            select! {
                ws_message = ws_stream.next().fuse() => {
                    match ws_message {
                        Some(raw_message) => {
                            if let Err(err) = self.handle_message(raw_message).await {
                                tracing::error!("Error handling message: {:?}", err);
                                break;
                            }
                        }
                        None => {
                            tracing::info!("WebSocket stream closed");
                            break;
                        }
                    }
                },
                channel_message = rx.recv().fuse() => {
                    match channel_message {
                        Ok(message) => {
                            match message {
                                WsCommand::Close => {
                                    tracing::info!("Closing WebSocket stream");
                                    break;
                                }
                                WsCommand::Send(msg) => {
                                    if let Err(err) = ws_stream.send(msg).await {
                                        tracing::error!("Error sending message: {:?}", err);
                                        break;
                                    }
                                }
                            }
                        }
                        Err(err) => {
                            tracing::error!("Channel receive error: {:?}", err);
                            break;
                        }
                    }
                }
            }
        }

        ws_stream.close(None).await?;

        Ok(())
    }

    async fn handle_message(
        &self,
        raw_message: tungstenite::Result<tungstenite::Message>,
    ) -> Result<()> {
        if let tungstenite::Message::Text(text) = raw_message? {
            let incoming = serde_json::from_str::<SocketIncoming>(text.as_str())?;

            tracing::debug!("Received message: {:?}", incoming);

            match incoming {
                SocketIncoming::Code(code) => {
                    self.emit_by_name::<()>("code-detected", &[&code]);
                }
                SocketIncoming::Tag(tag, data_fields) => {
                    self.emit_by_name::<()>(
                        "tag-detected",
                        &[&tag, &EntityDataFieldVecBoxed(data_fields)],
                    );
                }
                SocketIncoming::RequestProperties => {
                    self.ws_send_all_properties().await?;
                }
            }
        }

        Ok(())
    }
}

impl Remote for RemoteApp {
    fn ip_addr(&self) -> String {
        self.imp().ip_addr.borrow().clone()
    }

    fn port(&self) -> u16 {
        PORT
    }
}
