use std::{cell::RefCell, rc::Rc};

use adw::{prelude::*, subclass::prelude::*};
use anyhow::Result;
use futures_channel::oneshot;
use gtk::{
    gio,
    glib::{self, clone},
};
use wormhole::{
    rendezvous, transfer, transit, AppConfig, AppID, Code, MailboxConnection, Wormhole,
};

use crate::{format, ui::camera::Camera};

const WORMHOLE_APP_ID: &str = "lothar.com/wormhole/text-or-file-xfer";
const WORMHOLE_APP_RENDEZVOUS_URL: &str = rendezvous::DEFAULT_RENDEZVOUS_SERVER;
const WORMHOLE_TRANSIT_RELAY_URL: &str = transit::DEFAULT_RELAY_SERVER;
const WORMHOLE_TRANSIT_ABILITIES: transit::Abilities = transit::Abilities::FORCE_DIRECT;

mod imp {
    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/seadve/Uets/ui/receive_window.ui")]
    pub struct ReceiveWindow {
        #[template_child]
        pub(super) stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) code_page: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) code_camera: TemplateChild<Camera>,
        #[template_child]
        pub(super) code_entry: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) receiving_page: TemplateChild<gtk::ProgressBar>,
        #[template_child]
        pub(super) title_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) file_name_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) close_button: TemplateChild<gtk::Button>,

        pub(super) cancellable: gio::Cancellable,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ReceiveWindow {
        const NAME: &'static str = "UetsReceiveWindow";
        type Type = super::ReceiveWindow;
        type ParentType = adw::Window;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ReceiveWindow {
        fn dispose(&self) {
            self.cancellable.cancel();

            tracing::debug!("Window disposed");
        }
    }

    impl WidgetImpl for ReceiveWindow {}
    impl WindowImpl for ReceiveWindow {}
    impl AdwWindowImpl for ReceiveWindow {}
}

glib::wrapper! {
    pub struct ReceiveWindow(ObjectSubclass<imp::ReceiveWindow>)
        @extends gtk::Widget, gtk::Window, adw::Window;
}

impl ReceiveWindow {
    pub async fn receive(parent: &impl IsA<gtk::Widget>) -> Result<Vec<u8>> {
        let root = parent.root().map(|r| r.downcast::<gtk::Window>().unwrap());

        let this = glib::Object::builder::<Self>()
            .property("transient-for", root)
            .property("modal", true)
            .build();
        this.present();

        let ret = this.start_receive().await;

        this.close();

        ret
    }

    async fn start_receive(&self) -> Result<Vec<u8>> {
        let imp = self.imp();

        imp.stack.set_visible_child(&*imp.code_page);
        imp.title_label.set_label("Show or Enter Code");
        imp.close_button.set_label("Cancel");

        imp.code_camera.start()?;

        let (tx, rx) = oneshot::channel();
        let tx = Rc::new(RefCell::new(Some(tx)));
        imp.code_camera.connect_code_detected(clone!(
            #[strong]
            tx,
            move |_, code| {
                let tx = tx.take().unwrap();
                let _ = tx.send(code.to_string());
            }
        ));
        imp.code_entry.connect_entry_activated(move |entry| {
            let tx = tx.take().unwrap();
            let _ = tx.send(entry.text().to_string());
        });
        let code = rx.await.unwrap();

        imp.code_camera.stop()?;

        imp.stack.set_visible_child(&*imp.receiving_page);
        imp.title_label.set_label("Receiving");

        let app_config = AppConfig {
            id: AppID::new(WORMHOLE_APP_ID),
            rendezvous_url: WORMHOLE_APP_RENDEZVOUS_URL.into(),
            app_version: transfer::AppVersion::default(),
        };
        let connection = gio::CancellableFuture::new(
            MailboxConnection::connect(app_config, Code(code), false),
            imp.cancellable.clone(),
        )
        .await??;

        let wormhole =
            gio::CancellableFuture::new(Wormhole::connect(connection), imp.cancellable.clone())
                .await??;
        let relay_hints = vec![transit::RelayHint::from_urls(
            None,
            [WORMHOLE_TRANSIT_RELAY_URL.parse().unwrap()],
        )
        .unwrap()];

        let request = gio::CancellableFuture::new(
            transfer::request_file(
                wormhole,
                relay_hints,
                WORMHOLE_TRANSIT_ABILITIES,
                self.cancellable_cancel_fut(),
            ),
            imp.cancellable.clone(),
        )
        .await??
        .ok_or(gio::Cancelled)?;

        imp.file_name_label.set_label(&format!(
            "{} ({})",
            request.file_name(),
            glib::format_size(request.file_size() as u64)
        ));

        let mut ret = Vec::new();
        gio::CancellableFuture::new(
            request.accept(
                |transit_info| {
                    tracing::debug!("Wormhole transit info: {:?}", transit_info);
                },
                clone!(
                    #[weak(rename_to = obj)]
                    self,
                    move |sent_bytes, total_bytes| {
                        let imp = obj.imp();

                        imp.receiving_page
                            .set_fraction(sent_bytes as f64 / total_bytes as f64);
                        imp.receiving_page
                            .set_text(Some(&format::transfer_progress(sent_bytes, total_bytes)));
                    }
                ),
                &mut ret,
                self.cancellable_cancel_fut(),
            ),
            imp.cancellable.clone(),
        )
        .await??;

        imp.title_label.set_label("File Received");
        imp.stack.set_visible(false);
        imp.close_button.set_label("Close");
        imp.close_button.add_css_class("suggested-action");

        Ok(ret)
    }

    async fn cancellable_cancel_fut(&self) {
        let imp = self.imp();

        let (tx, rx) = oneshot::channel();
        imp.cancellable.connect_cancelled(|_| {
            let _ = tx.send(());
        });
        rx.await.unwrap()
    }
}
