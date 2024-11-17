use chrono::{DateTime, Utc};
use gtk::glib;

#[derive(Debug, Clone, Copy, PartialEq, Eq, glib::Boxed)]
#[boxed_type(name = "UetsDateTimeBoxed", nullable)]
pub struct DateTimeBoxed(pub DateTime<Utc>);
