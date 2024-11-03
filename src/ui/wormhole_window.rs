use std::future;

use adw::{prelude::*, subclass::prelude::*};
use anyhow::Result;
use gtk::{
    gdk, gio,
    glib::{self, clone},
};
use qrcode::{render::svg, QrCode};
use wormhole::{
    transfer, transit, uri::WormholeTransferUri, AppConfig, AppID, MailboxConnection, Wormhole,
};

use crate::format;

const WORMHOLE_APP_ID: &str = "lothar.com/wormhole/text-or-file-xfer";
const WORMHOLE_CODE_LENGTH: usize = 4;
const WORMHOLE_APP_RENDEZVOUS_URL: &str = "ws://relay.magic-wormhole.io:4000/v1";
const WORMHOLE_TRANSIT_RELAY_URL: &str = "tcp://transit.magic-wormhole.io:4001";
const WORMHOLE_TRANSIT_ABILITIES: transit::Abilities = transit::Abilities::FORCE_DIRECT;

mod imp {
    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/seadve/Uets/ui/wormhole_window.ui")]
    pub struct WormholeWindow {
        #[template_child]
        pub(super) stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) code_loading_page: TemplateChild<gtk::Spinner>,
        #[template_child]
        pub(super) code_loaded_page: TemplateChild<gtk::Box>,
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
    pub async fn send(
        bytes: Vec<u8>,
        dest_file_name: &str,
        parent: Option<&impl IsA<gtk::Window>>,
    ) -> Result<()> {
        let this = glib::Object::builder::<WormholeWindow>()
            .property("transient-for", parent)
            .property("modal", true)
            .build();
        this.present();

        if let Err(err) = this.start_send(bytes, dest_file_name).await {
            if !err.is::<gio::Cancelled>() {
                return Err(err);
            }
        }

        Ok(())
    }

    async fn start_send(&self, bytes: Vec<u8>, dest_file_name: &str) -> Result<()> {
        let imp = self.imp();

        imp.file_name_label.set_text(&format!(
            "“{dest_file_name}” ({})",
            glib::format_size(bytes.len() as u64)
        ));

        imp.stack.set_visible_child(&*imp.code_loading_page);
        imp.title_label.set_label("Loading Code");
        imp.close_button.set_label("Cancel");

        let app_config = AppConfig {
            id: AppID::new(WORMHOLE_APP_ID),
            rendezvous_url: WORMHOLE_APP_RENDEZVOUS_URL.into(),
            app_version: transfer::AppVersion::default(),
        };
        let connection = gio::CancellableFuture::new(
            MailboxConnection::create(app_config, WORMHOLE_CODE_LENGTH),
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

        imp.stack.set_visible_child(&*imp.code_loaded_page);
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