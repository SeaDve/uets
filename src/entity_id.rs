use gtk::glib;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, glib::Boxed)]
#[boxed_type(name = "UetsEntityId")]
pub struct EntityId(Box<str>);

impl EntityId {
    pub fn new(id: impl Into<Box<str>>) -> Self {
        Self(id.into())
    }
}
