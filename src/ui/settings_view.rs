use adw::{prelude::*, subclass::prelude::*};
use anyhow::Result;
use gtk::{
    gio,
    glib::{self, clone},
    pango,
};
use std::process::Command;

use crate::{remote::Remote, Application};

mod imp {
    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/seadve/Uets/ui/settings_view.ui")]
    pub struct SettingsView {
        #[template_child]
        pub(super) page: TemplateChild<adw::PreferencesPage>, // Unused
        #[template_child]
        pub(super) enable_n_inside_hook_row: TemplateChild<adw::ExpanderRow>,
        #[template_child]
        pub(super) n_inside_hook_threshold_row: TemplateChild<adw::SpinRow>,
        #[template_child]
        pub(super) lower_limit_reached_threshold_row: TemplateChild<adw::SpinRow>,
        #[template_child]
        pub(super) upper_limit_reached_threshold_row: TemplateChild<adw::SpinRow>,
        #[template_child]
        pub(super) fullscreen_window_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) show_test_window_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) camera_ip_addr_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) aux_camera_ip_addrs_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) rfid_reader_ip_addr_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) relay_ip_addr_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) quit_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) shutdown_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) remote_status_box: TemplateChild<gtk::ListBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SettingsView {
        const NAME: &'static str = "UetsSettingsView";
        type Type = super::SettingsView;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action("settings-view.reload-camera", None, move |_, _, _| {
                let app = Application::get();
                if let Err(err) = app.camera().restart() {
                    tracing::error!("Failed to restart camera: {:?}", err);
                    app.add_message_toast("Failed to restart camera");
                }
            });
            klass.install_action("settings-view.reload-aux-cameras", None, move |_, _, _| {
                let app = Application::get();
                for camera in app.detector().aux_cameras() {
                    if let Err(err) = camera.restart() {
                        tracing::error!("Failed to restart aux camera: {:?}", err);
                        app.add_message_toast("Failed to restart aux camera");
                    }
                }
            });
            klass.install_action("settings-view.reload-rfid-reader", None, move |_, _, _| {
                Application::get().rfid_reader().reconnect();
            });
            klass.install_action(
                "settings-view.reload-remote-status",
                None,
                move |obj, _, _| obj.update_remote_status_box(),
            );
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SettingsView {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            let app = Application::get();
            let settings = app.settings();

            let action_group = gio::SimpleActionGroup::new();
            action_group.add_action(&settings.create_operation_mode_action());
            action_group.add_action(&settings.create_enable_lower_limit_reached_alert_action());
            action_group.add_action(&settings.create_enable_upper_limit_reached_alert_action());
            action_group.add_action(&settings.create_enable_detection_wo_id_action());
            obj.insert_action_group("settings-view", Some(&action_group));

            settings
                .bind_lower_limit_reached_threshold(
                    &*self.lower_limit_reached_threshold_row,
                    "value",
                )
                .build();
            settings
                .bind_upper_limit_reached_threshold(
                    &*self.upper_limit_reached_threshold_row,
                    "value",
                )
                .build();

            settings
                .bind_enable_n_inside_hook(&*self.enable_n_inside_hook_row, "enable-expansion")
                .build();
            settings
                .bind_n_inside_hook_threshold(&*self.n_inside_hook_threshold_row, "value")
                .build();

            self.fullscreen_window_button.connect_clicked(|_| {
                Application::get().window().fullscreen();
            });

            self.show_test_window_button.connect_clicked(|_| {
                Application::get().present_test_window();
            });

            self.camera_ip_addr_row.set_text(&settings.camera_ip_addr());
            self.camera_ip_addr_row.connect_apply(|entry| {
                Application::get()
                    .settings()
                    .set_camera_ip_addr(entry.text().trim());
            });

            self.aux_camera_ip_addrs_row
                .set_text(&settings.aux_camera_ip_addrs().join(", "));
            self.aux_camera_ip_addrs_row.connect_apply(|entry| {
                Application::get().settings().set_aux_camera_ip_addrs(
                    &entry
                        .text()
                        .split(",")
                        .map(|s| s.trim())
                        .collect::<Vec<_>>(),
                );
            });

            self.rfid_reader_ip_addr_row
                .set_text(&settings.rfid_reader_ip_addr());
            self.rfid_reader_ip_addr_row.connect_apply(|entry| {
                Application::get()
                    .settings()
                    .set_rfid_reader_ip_addr(entry.text().trim());
            });

            self.relay_ip_addr_row.set_text(&settings.relay_ip_addr());
            self.relay_ip_addr_row.connect_apply(|entry| {
                Application::get()
                    .settings()
                    .set_relay_ip_addr(entry.text().trim());
            });

            self.quit_button.connect_clicked(|_| {
                Application::get().quit();
            });

            self.shutdown_button.connect_clicked(|_| {
                if let Err(err) = Command::new("shutdown").arg("now").spawn() {
                    tracing::error!("Failed to run shutdown command: {:?}", err);

                    Application::get().add_message_toast("Failed to start shutdown process");
                }
            });

            obj.update_remote_status_box();
        }

        fn dispose(&self) {
            self.dispose_template();
        }
    }

    impl WidgetImpl for SettingsView {}
}

glib::wrapper! {
    pub struct SettingsView(ObjectSubclass<imp::SettingsView>)
        @extends gtk::Widget;
}

impl SettingsView {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn update_remote_status_box(&self) {
        glib::spawn_future_local(clone!(
            #[strong(rename_to = obj)]
            self,
            async move {
                obj.action_set_enabled("settings-view.reload-remote-status", false);
                obj.update_remote_status_inner().await;
                obj.action_set_enabled("settings-view.reload-remote-status", true);
            }
        ));
    }

    async fn update_remote_status_inner(&self) {
        struct RemoteStatus {
            name: &'static str,
            ip_addr: String,
            port: u16,
            port_reachability: Result<()>,
        }

        let imp = self.imp();

        imp.remote_status_box.remove_all();
        imp.remote_status_box.append(
            &gtk::Spinner::builder()
                .width_request(24)
                .height_request(24)
                .margin_top(6)
                .margin_bottom(6)
                .spinning(true)
                .build(),
        );

        let app = Application::get();
        let mut statuses = vec![
            RemoteStatus {
                name: "Camera",
                ip_addr: app.camera().ip_addr(),
                port: app.camera().port(),
                port_reachability: app.camera().check_port_reachability().await,
            },
            RemoteStatus {
                name: "RFID Reader",
                ip_addr: app.rfid_reader().ip_addr(),
                port: app.rfid_reader().port(),
                port_reachability: app.rfid_reader().check_port_reachability().await,
            },
            RemoteStatus {
                name: "Relay",
                ip_addr: app.relay().ip_addr(),
                port: app.relay().port(),
                port_reachability: app.relay().check_port_reachability().await,
            },
        ];
        for camera in app.detector().aux_cameras() {
            statuses.push(RemoteStatus {
                name: "Aux Camera",
                ip_addr: camera.ip_addr(),
                port: camera.port(),
                port_reachability: camera.check_port_reachability().await,
            });
        }

        if statuses.is_empty() {
            imp.remote_status_box.append(
                &gtk::Label::builder()
                    .label("No remote status to show")
                    .build(),
            );
        }

        imp.remote_status_box.remove_all();

        for status in statuses {
            let row = adw::ActionRow::builder()
                .activatable(false)
                .selectable(false)
                .title(status.name)
                .subtitle(format!("{}:{}", status.ip_addr, status.port))
                .build();

            let label = gtk::Label::builder()
                .xalign(1.0)
                .ellipsize(pango::EllipsizeMode::End)
                .selectable(true)
                .build();
            if let Err(err) = &status.port_reachability {
                label.set_text(&err.to_string());
                label.add_css_class("error");
            } else {
                label.set_text("OK");
                label.add_css_class("success");
            }
            row.add_suffix(&label);

            imp.remote_status_box.append(&row);
        }
    }
}
