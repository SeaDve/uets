use std::fmt;

use chrono::{Datelike, Local, SecondsFormat, TimeDelta, Utc};
use gtk::glib;
use serde::{Deserialize, Serialize};

/// A date time in UTC.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, glib::Boxed)]
#[serde(transparent)]
#[boxed_type(name = "UetsDateTime", nullable)]
pub struct DateTime(chrono::DateTime<Utc>);

impl DateTime {
    pub fn now() -> Self {
        Self(Utc::now())
    }

    pub fn difference(&self, other: &Self) -> TimeDelta {
        self.0 - other.0
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
        f.debug_tuple("DateTime")
            .field(&self.0.to_rfc3339_opts(SecondsFormat::Secs, true))
            .finish()
    }
}
