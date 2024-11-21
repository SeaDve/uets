use std::fmt;

use gtk::glib;
use serde::{Deserialize, Serialize};

/// This must be universally unique for each entity, even if they have the same stock id.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, glib::Boxed)]
#[serde(transparent)]
#[boxed_type(name = "UetsEntityId")]
pub struct EntityId(Box<str>);

impl EntityId {
    pub fn new(id: impl Into<Box<str>>) -> Self {
        Self(id.into())
    }
}

impl fmt::Debug for EntityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl fmt::Display for EntityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}
