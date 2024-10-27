use std::fmt;

use chrono::{Datelike, Local, Utc};
use gtk::glib;
use serde::{Deserialize, Serialize};

/// A date time in UTC.
#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, glib::Boxed,
)]
#[serde(transparent)]
#[boxed_type(name = "UetsDateTime", nullable)]
pub struct DateTime(chrono::DateTime<Utc>);

impl DateTime {
    pub fn now() -> Self {
        Self(Utc::now())
    }

    pub fn inner(&self) -> chrono::DateTime<Utc> {
        self.0
    }

    pub fn local_fuzzy_display(&self) -> String {
        let now = Self::now();
        let this_local = self.0.with_timezone(&Local);

        if self.0.year() == now.0.year()
            && self.0.month() == self.0.month()
            && self.0.day() == self.0.day()
        {
            this_local.format("today at %R").to_string()
        } else if (now.0 - self.0).num_hours() <= 30 {
            this_local.format("yesterday at %R").to_string()
        } else {
            this_local.format("%F").to_string() // ISO 8601 (e.g., `2001-07-08`)
        }
    }
}

impl fmt::Debug for DateTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}
