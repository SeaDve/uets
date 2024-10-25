use gtk::{gio, glib, prelude::*, subclass::prelude::*};
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

            let action_group = gio::SimpleActionGroup::new();
            action_group.add_action(&Application::get().settings().create_profile_action());
            obj.insert_action_group("settings-view", Some(&action_group));

            self.quit_button.connect_clicked(|_| {
                Application::get().quit();
            });
            self.shutdown_button.connect_clicked(|_| {
                if let Err(err) = Command::new("shutdown").arg("now").spawn() {
                    tracing::error!("Failed to run shutdown command: {:?}", err);
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