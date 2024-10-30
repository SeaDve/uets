use std::fmt;

use gtk::glib;
use serde::{Deserialize, Serialize};

/// This is unique to each stock. Different entities can have same stock id.
///
/// This is also referred to as stock name.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, glib::Boxed,
)]
#[serde(transparent)]
#[boxed_type(name = "UetsStockId")]
pub struct StockId(Box<str>);

impl StockId {
    pub fn new(id: impl Into<Box<str>>) -> Self {
        // TODO Disallow empty or blank stock id
        Self(id.into())
    }
}

impl fmt::Display for StockId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
