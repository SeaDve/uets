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
use serde::Deserialize;

use crate::remote::Remote;

const PORT: u16 = 8888;

#[derive(Deserialize)]
enum SocketResponse {
    Code(String),
    Tag(String),
}

enum WsCommand {
    Close,
    SendMessage(tungstenite::Message),
}

mod imp {
    use std::{cell::RefCell, sync::OnceLock};

    use glib::subclass::Signal;

    use super::*;

    #[derive(Default)]
    pub struct RemoteApp {
        pub(super) ip_addr: RefCell<String>,

        pub(super) command_tx: Mutex<Option<Sender<WsCommand>>>,
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
        F: Fn(&Self, &str) + 'static,
    {
        self.connect_closure(
            "tag-detected",
            false,
            closure_local!(|obj: &Self, tag: &str| f(obj, tag)),
        )
    }

    pub async fn ws_send_text(&self, text: &str) -> Result<()> {
        let imp = self.imp();

        imp.command_tx
            .lock()
            .await
            .as_ref()
            .context("Command channel not initialized")?
            .send(WsCommand::SendMessage(tungstenite::Message::Text(
                text.into(),
            )))
            .await?;

        Ok(())
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
                                WsCommand::SendMessage(msg) => {
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
            match serde_json::from_str::<SocketResponse>(text.as_str())? {
                SocketResponse::Code(code) => {
                    self.emit_by_name::<()>("code-detected", &[&code]);
                }
                SocketResponse::Tag(tag) => {
                    self.emit_by_name::<()>("tag-detected", &[&tag]);
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
