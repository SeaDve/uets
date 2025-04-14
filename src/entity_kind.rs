use std::{error, fmt, str::FromStr};

use gtk::glib;
use serde::{Deserialize, Serialize};

use crate::entity_data::{EntityData, EntityDataFieldTy, ValidEntityFields};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, glib::Enum)]
#[enum_type(name = "UetsEntityKind")]
pub enum EntityKind {
    #[default]
    General,
    Person,
    Vehicle,
    StockItem,
    FoodItem,
}

impl EntityKind {
    pub fn is_valid_entity_data_field_ty(&self, entity_field: EntityDataFieldTy) -> bool {
        ValidEntityFields::for_entity_kind(*self).contains(entity_field)
    }

    pub fn is_valid_entity_data(&self, entity_data: &EntityData) -> bool {
        ValidEntityFields::for_entity_kind(*self).is_valid_entity_data(entity_data)
    }
}

impl fmt::Display for EntityKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EntityKind::General => write!(f, "General"),
            EntityKind::Person => write!(f, "Person"),
            EntityKind::Vehicle => write!(f, "Vehicle"),
            EntityKind::StockItem => write!(f, "Stock Item"),
            EntityKind::FoodItem => write!(f, "Food Item"),
        }
    }
}

#[derive(Debug)]
pub struct EntityKindParseError;

impl fmt::Display for EntityKindParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Failed to parse entity kind")
    }
}

impl error::Error for EntityKindParseError {}

impl FromStr for EntityKind {
    type Err = EntityKindParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().trim() {
            "general" => Ok(Self::General),
            "person" => Ok(Self::Person),
            "vehicle" => Ok(Self::Vehicle),
            "item" => Ok(Self::StockItem),
            "food" => Ok(Self::FoodItem),
            _ => Err(EntityKindParseError),
        }
    }
}
