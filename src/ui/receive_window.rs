use std::{cell::RefCell, rc::Rc};

use adw::{prelude::*, subclass::prelude::*};
use anyhow::{ensure, Result};
use futures_channel::oneshot;
use gtk::{
    gio,
    glib::{self, clone},
};
use wormhole::{transfer, uri::WormholeTransferUri, Code, MailboxConnection, Wormhole};

use crate::{camera::Camera, format, ui::camera_viewfinder::CameraViewfinder, wormhole_ext};

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
        pub(super) code_camera_viewfinder: TemplateChild<CameraViewfinder>,
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
        pub(super) code_camera: Camera,
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
        fn constructed(&self) {
            self.parent_constructed();

            self.code_camera_viewfinder
                .set_camera(Some(self.code_camera.clone()));
        }
    }

    impl WidgetImpl for ReceiveWindow {}

    impl WindowImpl for ReceiveWindow {
        fn close_request(&self) -> glib::Propagation {
            tracing::trace!("Close request");

            self.cancellable.cancel();

            self.code_camera.stop();

            self.parent_close_request()
        }
    }

    impl AdwWindowImpl for ReceiveWindow {}
}

glib::wrapper! {
    pub struct ReceiveWindow(ObjectSubclass<imp::ReceiveWindow>)
        @extends gtk::Widget, gtk::Window, adw::Window;
}

impl ReceiveWindow {
    pub async fn receive(parent: &impl IsA<gtk::Widget>) -> Result<(String, Vec<u8>)> {
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

    async fn start_receive(&self) -> Result<(String, Vec<u8>)> {
        let imp = self.imp();

        imp.stack.set_visible_child(&*imp.code_page);
        imp.title_label.set_label("Starting Camera");
        imp.close_button.set_label("Cancel");

        let (tx, rx) = oneshot::channel();
        let tx = Rc::new(RefCell::new(Some(tx)));

        if let Err(err) = imp.code_camera.start().await {
            tracing::warn!("Failed to start camera: {:?}", err);

            imp.title_label.set_label("Enter Code");

            imp.code_camera_viewfinder.set_visible(false);
        } else {
            imp.title_label.set_label("Show or Enter Code");

            imp.code_camera.connect_code_detected(clone!(
                #[strong]
                tx,
                move |_, qrcode| {
                    match qrcode.parse::<WormholeTransferUri>() {
                        Ok(uri) => {
                            let _ = tx.take().unwrap().send(uri.code);
                        }
                        Err(err) => {
                            tracing::warn!("Failed to parse QR code to uri: {:?}", err);
                        }
                    }
                }
            ));
        }

        imp.code_entry.connect_entry_activated(move |entry| {
            let code = Code(entry.text().trim().to_string());
            let _ = tx.take().unwrap().send(code);
        });
        let code = rx.await.unwrap();

        imp.code_camera.stop();

        imp.stack.set_visible_child(&*imp.receiving_page);
        imp.title_label.set_label("Receiving");

        let app_config = wormhole_ext::app_config();
        let connection = gio::CancellableFuture::new(
            MailboxConnection::connect(app_config, code, false),
            imp.cancellable.clone(),
        )
        .await??;

        let wormhole =
            gio::CancellableFuture::new(Wormhole::connect(connection), imp.cancellable.clone())
                .await??;
        let relay_hints = wormhole_ext::relay_hints();

        let request = gio::CancellableFuture::new(
            transfer::request_file(
                wormhole,
                relay_hints,
                wormhole_ext::DEFAULT_TRANSIT_ABILITIES,
                self.cancellable_cancel_fut(),
            ),
            imp.cancellable.clone(),
        )
        .await??
        .ok_or(gio::Cancelled)?;
        let request_file_size = request.file_size();
        let request_file_name = request.file_name();

        imp.file_name_label.set_label(&format!(
            "{} ({})",
            request_file_name,
            glib::format_size(request_file_size)
        ));

        let mut bytes = Vec::new();
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
                &mut bytes,
                self.cancellable_cancel_fut(),
            ),
            imp.cancellable.clone(),
        )
        .await??;

        ensure!(
            bytes.len() == request_file_size as usize,
            "Received file size mismatch"
        );

        imp.title_label.set_label("File Received");
        imp.stack.set_visible(false);
        imp.close_button.set_label("Close");
        imp.close_button.add_css_class("suggested-action");

        Ok((request_file_name, bytes))
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
