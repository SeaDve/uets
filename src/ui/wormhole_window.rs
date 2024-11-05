use std::future::{self, Future};

use adw::{prelude::*, subclass::prelude::*};
use anyhow::Result;
use async_lock::Mutex;
use gtk::{
    gdk, gio,
    glib::{self, clone},
};
use qrcode::{render::svg, QrCode};
use wormhole::{
    rendezvous, transfer, transit, uri::WormholeTransferUri, AppConfig, AppID, MailboxConnection,
    Wormhole,
};

use crate::format;

const WORMHOLE_CODE_LENGTH: usize = 4;
const WORMHOLE_APP_ID: &str = "lothar.com/wormhole/text-or-file-xfer";
const WORMHOLE_APP_RENDEZVOUS_URL: &str = rendezvous::DEFAULT_RENDEZVOUS_SERVER;
const WORMHOLE_TRANSIT_RELAY_URL: &str = transit::DEFAULT_RELAY_SERVER;
const WORMHOLE_TRANSIT_ABILITIES: transit::Abilities = transit::Abilities::FORCE_DIRECT;

static PREMADE_CONNECTION: Mutex<Option<MailboxConnection<transfer::AppVersion>>> =
    Mutex::new(None);

mod imp {
    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/seadve/Uets/ui/wormhole_window.ui")]
    pub struct WormholeWindow {
        #[template_child]
        pub(super) stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) loading_page: TemplateChild<gtk::Spinner>,
        #[template_child]
        pub(super) loaded_page: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) qrcode_image: TemplateChild<gtk::Image>,
        #[template_child]
        pub(super) file_name_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) code_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) sending_page: TemplateChild<gtk::ProgressBar>,
        #[template_child]
        pub(super) title_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) close_button: TemplateChild<gtk::Button>,

        pub(super) cancellable: gio::Cancellable,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for WormholeWindow {
        const NAME: &'static str = "UetsWormholeWindow";
        type Type = super::WormholeWindow;
        type ParentType = adw::Window;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for WormholeWindow {
        fn dispose(&self) {
            self.cancellable.cancel();

            tracing::debug!("Window disposed");
        }
    }

    impl WidgetImpl for WormholeWindow {}
    impl WindowImpl for WormholeWindow {}
    impl AdwWindowImpl for WormholeWindow {}
}

glib::wrapper! {
    pub struct WormholeWindow(ObjectSubclass<imp::WormholeWindow>)
        @extends gtk::Widget, gtk::Window, adw::Window;
}

impl WormholeWindow {
    pub fn init_premade_connection() {
        glib::spawn_future_local(async {
            if let Err(err) = init_premade_connection_inner().await {
                tracing::error!("Failed to init premade connection: {:?}", err);
            }
        });
    }

    pub async fn send(
        bytes_fut: impl Future<Output = Result<Vec<u8>>>,
        dest_file_name: &str,
        parent: &impl IsA<gtk::Widget>,
    ) -> Result<()> {
        let root = parent.root().map(|r| r.downcast::<gtk::Window>().unwrap());

        let this = glib::Object::builder::<WormholeWindow>()
            .property("transient-for", root)
            .property("modal", true)
            .build();
        this.present();

        if let Err(err) = this.start_send(bytes_fut, dest_file_name).await {
            if !err.is::<gio::Cancelled>() {
                this.close();
                return Err(err);
            }
        }

        Ok(())
    }

    async fn start_send(
        &self,
        bytes_fut: impl Future<Output = Result<Vec<u8>>>,
        dest_file_name: &str,
    ) -> Result<()> {
        let imp = self.imp();

        imp.file_name_label.set_text(dest_file_name);

        imp.stack.set_visible_child(&*imp.loading_page);
        imp.title_label.set_label("Loading Data");
        imp.close_button.set_label("Cancel");

        let bytes = gio::CancellableFuture::new(bytes_fut, imp.cancellable.clone()).await??;

        imp.title_label.set_label("Loading Code");

        imp.file_name_label.set_text(&format!(
            "{dest_file_name} ({})",
            glib::format_size(bytes.len() as u64)
        ));

        let connection = gio::CancellableFuture::new(
            take_and_replace_premade_connection(),
            imp.cancellable.clone(),
        )
        .await??;

        let uri = {
            let mut uri = WormholeTransferUri::new(connection.code().clone());
            uri.is_leader = true;
            uri
        };

        let qrcode_texture = qrcode_texture_for_uri(&uri)?;
        imp.qrcode_image.set_paintable(Some(&qrcode_texture));
        imp.code_label.set_text(connection.code().as_str());

        imp.stack.set_visible_child(&*imp.loaded_page);
        imp.title_label.set_label("Scan or Type Code");

        let wormhole =
            gio::CancellableFuture::new(Wormhole::connect(connection), imp.cancellable.clone())
                .await??;
        let relay_hints = vec![transit::RelayHint::from_urls(
            None,
            [WORMHOLE_TRANSIT_RELAY_URL.parse().unwrap()],
        )
        .unwrap()];

        imp.sending_page.set_fraction(0.0);
        imp.sending_page
            .set_text(Some(&format::transfer_progress(0, bytes.len() as u64)));
        imp.stack.set_visible_child(&*imp.sending_page);
        imp.title_label.set_label("Sending Report");

        gio::CancellableFuture::new(
            transfer::send_file(
                wormhole,
                relay_hints,
                &mut bytes.as_slice(),
                dest_file_name,
                bytes.len() as u64,
                WORMHOLE_TRANSIT_ABILITIES,
                |transit_info| {
                    tracing::debug!("Wormhole transit info: {:?}", transit_info);
                },
                clone!(
                    #[weak(rename_to = obj)]
                    self,
                    move |sent_bytes, total_bytes| {
                        let imp = obj.imp();

                        imp.sending_page
                            .set_fraction(sent_bytes as f64 / total_bytes as f64);
                        imp.sending_page
                            .set_text(Some(&format::transfer_progress(sent_bytes, total_bytes)));
                    }
                ),
                future::pending(),
            ),
            imp.cancellable.clone(),
        )
        .await??;

        imp.title_label.set_label("Report Sent");
        imp.stack.set_visible(false);
        imp.close_button.set_label("Close");
        imp.close_button.add_css_class("suggested-action");

        Ok(())
    }
}

fn qrcode_texture_for_uri(uri: &WormholeTransferUri) -> Result<gdk::Texture> {
    let qrcode = QrCode::new(uri.to_string())?;
    let svg_bytes = qrcode.render::<svg::Color<'_>>().build();
    let texture = gdk::Texture::from_bytes(&svg_bytes.as_bytes().into())?;
    Ok(texture)
}

async fn take_and_replace_premade_connection() -> Result<MailboxConnection<transfer::AppVersion>> {
    if let Some(connection) = PREMADE_CONNECTION.lock().await.take() {
        // Reinitialize a new premade connection for the next time.
        WormholeWindow::init_premade_connection();

        tracing::trace!("Connection taken");

        return Ok(connection);
    }

    init_premade_connection_inner().await?;
    let connection = PREMADE_CONNECTION
        .lock()
        .await
        .take()
        .expect("premade connection must have been initialized");

    // Reinitialize a new premade connection for the next time.
    WormholeWindow::init_premade_connection();

    tracing::trace!("Connection taken");

    Ok(connection)
}

async fn init_premade_connection_inner() -> Result<()> {
    let mut stored = PREMADE_CONNECTION.lock().await;

    if stored.is_none() {
        let app_config = AppConfig {
            id: AppID::new(WORMHOLE_APP_ID),
            rendezvous_url: WORMHOLE_APP_RENDEZVOUS_URL.into(),
            app_version: transfer::AppVersion::default(),
        };
        let connection = MailboxConnection::create(app_config, WORMHOLE_CODE_LENGTH).await?;

        tracing::trace!("Connection initialized");

        *stored = Some(connection);
    }

    Ok(())
}
