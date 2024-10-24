use std::fmt;

use gtk::glib;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, glib::Boxed)]
#[boxed_type(name = "UetsEntityId")]
pub struct EntityId(Box<str>);

impl EntityId {
    pub fn new(id: impl Into<Box<str>>) -> Self {
        Self(id.into())
    }
}

impl fmt::Display for EntityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
