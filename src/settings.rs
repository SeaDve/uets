use anyhow::Result;
use gsettings_macro::gen_settings;
use gtk::{gio, glib};
use serde::Deserialize;

use crate::APP_ID;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize, glib::Enum)]
#[serde(rename_all = "kebab-case")]
#[enum_type(name = "UetsAccessMode")]
pub enum AccessMode {
    #[default]
    EntryAndExit,
    EntryOnly,
    ExitOnly,
}

#[derive(Debug, Deserialize)]
pub struct DetectorConfig {
    pub name: String,
    pub access_mode: AccessMode,
    #[serde(rename = "camera")]
    pub camera_ip_addr: Option<String>,
    #[serde(rename = "rfid_reader")]
    pub rfid_reader_ip_addr: Option<String>,
    #[serde(rename = "remote_app")]
    pub remote_app_ip_addr: Option<String>,
}

#[gen_settings(file = "./data/io.github.seadve.Uets.gschema.xml")]
pub struct Settings;

impl Settings {
    pub fn detector_config_parsed(&self) -> Result<Vec<DetectorConfig>> {
        let string = self.detector_config();
        Ok(serde_yaml::from_str(&string)?)
    }
}

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
