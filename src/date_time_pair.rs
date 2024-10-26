use gtk::glib;

use crate::{date_time::DateTime, db};

#[derive(Debug, Clone)]
pub struct DateTimePair {
    pub entry: DateTime,
    pub exit: Option<DateTime>,
}

impl DateTimePair {
    pub fn from_db(raw: db::RawDateTimePair) -> Self {
        Self {
            entry: raw.entry,
            exit: raw.exit,
        }
    }

    pub fn to_db(&self) -> db::RawDateTimePair {
        db::RawDateTimePair {
            entry: self.entry.clone(),
            exit: self.exit.clone(),
        }
    }

    pub fn inside_duration(&self) -> Option<glib::TimeSpan> {
        self.exit.as_ref().map(|exit| exit.difference(&self.entry))
    }
}
