use std::{cell::RefCell, error, rc::Rc};

use adw::{prelude::*, subclass::prelude::*};
use anyhow::{ensure, Result};
use futures_channel::oneshot;
use gtk::{
    gio,
    glib::{self, clone},
};
use wormhole::{transfer, uri::WormholeTransferUri, Code, MailboxConnection, Wormhole};

use crate::{camera::Camera, format, ui::camera_viewfinder::CameraViewfinder, wormhole_ext};

#[derive(Debug)]
pub struct InvalidFileExtension;

impl std::fmt::Display for InvalidFileExtension {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Invalid file extension")
    }
}

impl error::Error for InvalidFileExtension {}

mod imp {
    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/seadve/Uets/ui/receive_dialog.ui")]
    pub struct ReceiveDialog {
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
    impl ObjectSubclass for ReceiveDialog {
        const NAME: &'static str = "UetsReceiveDialog";
        type Type = super::ReceiveDialog;
        type ParentType = adw::Dialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ReceiveDialog {
        fn constructed(&self) {
            self.parent_constructed();

            self.code_camera_viewfinder
                .set_camera(Some(self.code_camera.clone()));
        }
    }

    impl WidgetImpl for ReceiveDialog {}

    impl AdwDialogImpl for ReceiveDialog {
        fn closed(&self) {
            tracing::trace!("Close request");

            self.cancellable.cancel();

            self.code_camera.stop();

            self.parent_closed();
        }
    }
}

glib::wrapper! {
    pub struct ReceiveDialog(ObjectSubclass<imp::ReceiveDialog>)
        @extends gtk::Widget, adw::Dialog;
}

impl ReceiveDialog {
    pub async fn receive(
        valid_file_extensions: &[&str],
        parent: Option<&impl IsA<gtk::Widget>>,
    ) -> Result<(String, Vec<u8>)> {
        let this = glib::Object::new::<Self>();
        this.present(parent);

        let ret = this.start_receive(valid_file_extensions).await;

        this.close();

        ret
    }

    async fn start_receive(&self, valid_file_extensions: &[&str]) -> Result<(String, Vec<u8>)> {
        let imp = self.imp();

        imp.stack.set_visible_child(&*imp.code_page);
        imp.title_label.set_label("Starting Camera");
        imp.file_name_label.set_label(&format!(
            "Valid file extensions: {}",
            valid_file_extensions
                .iter()
                .map(|extension| format!("<b>{}</b>", glib::markup_escape_text(extension)))
                .collect::<Vec<_>>()
                .join(", ")
        ));
        imp.close_button.set_label("Cancel");

        let (tx, rx) = oneshot::channel();
        let tx = Rc::new(RefCell::new(Some(tx)));

        if let Err(err) = imp.code_camera.start() {
            tracing::warn!("Failed to start camera: {:?}", err);

            imp.title_label.set_label("Enter Code");
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

        if !valid_file_extensions
            .iter()
            .any(|file_extension| request_file_name.ends_with(file_extension))
        {
            return Err(InvalidFileExtension.into());
        }

        imp.file_name_label.set_label(&format!(
            "{} ({})",
            glib::markup_escape_text(&request_file_name),
            glib::markup_escape_text(&glib::format_size(request_file_size))
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
