use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntityKind {
    General,
    Person,
    Vehicle,
    Item,
    Food,
}

impl fmt::Display for EntityKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EntityKind::General => write!(f, "General"),
            EntityKind::Person => write!(f, "Person"),
            EntityKind::Vehicle => write!(f, "Vehicle"),
            EntityKind::Item => write!(f, "Item"),
            EntityKind::Food => write!(f, "Food"),
        }
    }
}
