use adw::{prelude::*, subclass::prelude::*};
use gtk::{gio, glib};
use std::process::Command;

use crate::Application;

mod imp {
    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/seadve/Uets/ui/settings_view.ui")]
    pub struct SettingsView {
        #[template_child]
        pub(super) page: TemplateChild<adw::PreferencesPage>, // Unused
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
        pub(super) quit_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) shutdown_button: TemplateChild<gtk::Button>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SettingsView {
        const NAME: &'static str = "UetsSettingsView";
        type Type = super::SettingsView;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
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
            obj.insert_action_group("settings-view", Some(&action_group));

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

            self.quit_button.connect_clicked(|_| {
                Application::get().quit();
            });

            self.shutdown_button.connect_clicked(|_| {
                if let Err(err) = Command::new("shutdown").arg("now").spawn() {
                    tracing::error!("Failed to run shutdown command: {:?}", err);

                    Application::get().add_message_toast("Failed to start shutdown process");
                }
            });
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
}
