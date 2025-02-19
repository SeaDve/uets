use std::fmt;

use gtk::glib;

#[derive(Debug, Clone, Copy, PartialEq, Eq, glib::Enum)]
#[enum_type(name = "UetsTimelineItemKind")]
pub enum TimelineItemKind {
    Entry,
    Exit,
}

impl TimelineItemKind {
    pub fn is_entry(&self) -> bool {
        matches!(self, Self::Entry)
    }

    pub fn is_exit(&self) -> bool {
        matches!(self, Self::Exit)
    }
}

impl fmt::Display for TimelineItemKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Entry => write!(f, "Entry"),
            Self::Exit => write!(f, "Exit"),
        }
    }
}
