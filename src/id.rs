use gtk::glib;

#[derive(Debug, Clone, PartialEq, Eq, glib::Boxed)]
#[boxed_type(name = "UetsId")]
pub struct Id(Box<str>);

impl Id {
    pub fn new(id: impl Into<Box<str>>) -> Self {
        Self(id.into())
    }
}
