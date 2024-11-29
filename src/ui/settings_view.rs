use gtk::{gio, glib, prelude::*, subclass::prelude::*};
use std::process::Command;

use crate::{ui::receive_window::ReceiveWindow, Application};

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

            klass.install_action_async(
                "settings-view.register-entity-data",
                None,
                |obj, _, _| async move {
                    let app = Application::get();

                    match ReceiveWindow::receive(&obj).await {
                        Ok((file_name, bytes)) => {
                            if [".xls", ".xlsx", ".xlsm", ".xlsb", ".xla", ".xlam", ".ods"]
                                .iter()
                                .any(|ext| file_name.ends_with(ext))
                            {
                                if let Err(err) =
                                    app.timeline().insert_entities_from_workbook_bytes(&bytes)
                                {
                                    tracing::error!("Failed to register entity data: {:?}", err);

                                    app.add_message_toast("Failed to register entity data");
                                } else {
                                    app.add_message_toast("Entity data registered");
                                }
                            } else {
                                app.add_message_toast("Unknown file type");
                            }
                        }
                        Err(err) => {
                            app.add_message_toast("Failed to receive file");

                            tracing::error!("Failed to receive file: {:?}", err)
                        }
                    }
                },
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

            let action_group = gio::SimpleActionGroup::new();
            action_group.add_action(&Application::get().settings().create_operation_mode_action());
            obj.insert_action_group("settings-view", Some(&action_group));

            self.fullscreen_window_button.connect_clicked(|_| {
                Application::get().window().fullscreen();
            });
            self.show_test_window_button.connect_clicked(|_| {
                Application::get().present_test_window();
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
