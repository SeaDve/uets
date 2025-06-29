use std::{collections::HashMap, time::Duration};

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
    entity_id::EntityId,
    entity_kind::EntityKind,
    remote::Remote,
    timeline::Timeline,
    utils,
};

const PORT: u16 = 8888;

const REQUEST_REPLY_IDLE_INTERVAL: Duration = Duration::from_millis(100);

// FIXME:
// Ideally, we should use HTTP for requesting properties and entity data to improve
// efficiency. We should only use WebSocket for notifying about changes, and
// client can request properties and entity data when needed.

#[derive(Debug, Serialize)]
struct PseudoEntityData {
    kind: EntityKind,
    /// This can be a stock ID or name.
    name: Option<String>,
    possessor: Option<EntityId>,
    is_inside: bool,
}

#[derive(Debug, Deserialize)]
enum SocketIncoming {
    Code(String),
    Entity(EntityId, Vec<EntityDataField>),
    RequestAllEntityData,
}

#[derive(Debug, Serialize)]
enum SocketOutgoing {
    Message(String),
    AllEntityData(HashMap<EntityId, PseudoEntityData>),
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

        pub(super) send_all_entity_data: RefCell<Option<glib::JoinHandle<()>>>,
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

            utils::spawn_future_local_idle(clone!(
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
                    Signal::builder("entity-detected")
                        .param_types([
                            EntityId::static_type(),
                            EntityDataFieldVecBoxed::static_type(),
                        ])
                        .build(),
                    Signal::builder("code-detected")
                        .param_types([String::static_type()])
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

        timeline.entity_list().connect_items_changed(clone!(
            #[weak(rename_to = obj)]
            self,
            move |_, _, _, _| {
                obj.queue_send_all_entity_kinds_and_names();
            }
        ));

        imp.timeline.set(timeline.clone()).unwrap();
    }

    pub fn connect_entity_detected<F>(&self, f: F) -> glib::SignalHandlerId
    where
        F: Fn(&Self, &EntityId, &EntityDataFieldVecBoxed) + 'static,
    {
        self.connect_closure(
            "entity-detected",
            false,
            closure_local!(
                |obj: &Self, id: &EntityId, data_fields_boxed: &EntityDataFieldVecBoxed| f(
                    obj,
                    id,
                    data_fields_boxed
                )
            ),
        )
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

    async fn ws_send_all_entity_kinds_and_names(&self) -> Result<()> {
        let imp = self.imp();

        if let Some(timeline) = imp.timeline.get() {
            let ret = timeline
                .entity_list()
                .iter()
                .map(|entity| {
                    (
                        entity.id().clone(),
                        PseudoEntityData {
                            kind: entity.kind(),
                            name: entity
                                .data()
                                .stock_id()
                                .map(|s| s.to_string())
                                .or_else(|| entity.data().name().cloned()),
                            possessor: entity.data().possessor().cloned(),
                            is_inside: entity.is_inside(),
                        },
                    )
                })
                .collect::<HashMap<_, _>>();
            self.ws_send(&SocketOutgoing::AllEntityData(ret)).await?;
        } else {
            tracing::warn!("Failed to send entity kinds and names: Timeline not set");
        }

        Ok(())
    }

    fn queue_send_all_entity_kinds_and_names(&self) {
        let imp = self.imp();

        if let Some(handle) = imp.send_all_entity_data.take() {
            handle.abort();
        }

        let source_id = glib::spawn_future_local(clone!(
            #[weak(rename_to = obj)]
            self,
            async move {
                glib::timeout_future(REQUEST_REPLY_IDLE_INTERVAL).await;

                if let Err(err) = obj.ws_send_all_entity_kinds_and_names().await {
                    tracing::error!("Failed to send all entity kinds and names: {:?}", err);
                }
            }
        ));
        imp.send_all_entity_data.replace(Some(source_id));
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
                            if let Err(err) = self.handle_message(raw_message) {
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

    fn handle_message(&self, raw_message: tungstenite::Result<tungstenite::Message>) -> Result<()> {
        if let tungstenite::Message::Text(text) = raw_message? {
            let incoming = serde_json::from_str::<SocketIncoming>(text.as_str())?;

            tracing::debug!("Received message: {:?}", incoming);

            match incoming {
                SocketIncoming::Code(code) => {
                    self.emit_by_name::<()>("code-detected", &[&code]);
                }
                SocketIncoming::Entity(id, data_fields) => {
                    self.emit_by_name::<()>(
                        "entity-detected",
                        &[&id, &EntityDataFieldVecBoxed(data_fields)],
                    );
                }
                SocketIncoming::RequestAllEntityData => {
                    self.queue_send_all_entity_kinds_and_names();
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
