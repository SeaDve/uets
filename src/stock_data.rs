use gtk::glib;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize, glib::Boxed)]
#[boxed_type(name = "UetsStockData")]
pub struct StockData {}
