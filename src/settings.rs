use gsettings_macro::gen_settings;
use gtk::{gio, glib};

use crate::APP_ID;

#[gen_settings(file = "./data/io.github.seadve.Uets.gschema.xml")]
pub struct Settings;

impl Default for Settings {
    fn default() -> Self {
        let schema_source = gio::SettingsSchemaSource::from_directory(
            "data/",
            gio::SettingsSchemaSource::default().as_ref(),
            false,
        )
        .unwrap();
        let schema = schema_source.lookup(APP_ID, false).unwrap();

        Self(gio::Settings::new_full(
            &schema,
            gio::SettingsBackend::NONE,
            None,
        ))
    }
}
