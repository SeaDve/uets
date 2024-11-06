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
        let now = Utc::now();
        let this = self.0;
        let this_local = self.0.with_timezone(&Local);

        if this.year() == now.year() && this.month() == now.month() && this.day() == now.day() {
            this_local.format("today at %r").to_string() // today at 13:21:16 AM
        } else if (now - this).num_hours() <= 30 {
            this_local.format("yesterday at %r").to_string() // yesterday at 13:21:16 AM
        } else {
            this_local.format("%a, %-d %b %Y at %r").to_string() // Sun, 3 Nov 2024 at 13:21:16 AM
        }
    }
}

impl fmt::Debug for DateTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}
