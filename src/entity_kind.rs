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
    Item,
    FoodItem,
}

impl EntityKind {
    pub fn is_valid_entity_data_field_ty(&self, entity_field: EntityDataFieldTy) -> bool {
        ValidEntityFields::for_entity_kind(*self).contains(entity_field)
    }

    pub fn is_valid_entity_data(&self, entity_data: &EntityData) -> bool {
        ValidEntityFields::for_entity_kind(*self).is_valid_entity_data(entity_data)
    }

    pub fn enter_verb(&self) -> &str {
        match self {
            EntityKind::General | EntityKind::Person => "enters",
            EntityKind::Vehicle => "drives in",
            EntityKind::Item | EntityKind::FoodItem => "added",
        }
    }

    pub fn exit_verb(&self) -> &str {
        match self {
            EntityKind::General | EntityKind::Person => "exits",
            EntityKind::Vehicle => "drives out",
            EntityKind::Item | EntityKind::FoodItem => "removed",
        }
    }

    pub fn entry_to_exit_duration_suffix(&self) -> &str {
        match self {
            EntityKind::General | EntityKind::Person => "of stay",
            EntityKind::Vehicle => "of parking",
            EntityKind::Item | EntityKind::FoodItem => "of being kept",
        }
    }

    pub fn entities_view_icon_name(&self) -> &str {
        match self {
            Self::General => "tag-outline-symbolic",
            Self::Person => "person-symbolic",
            Self::Vehicle => "driving-symbolic",
            Self::Item => "package-x-generic-symbolic",
            Self::FoodItem => "egg-symbolic",
        }
    }
}

impl fmt::Display for EntityKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EntityKind::General => write!(f, "General"),
            EntityKind::Person => write!(f, "Person"),
            EntityKind::Vehicle => write!(f, "Vehicle"),
            EntityKind::Item => write!(f, "Item"),
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
            "item" => Ok(Self::Item),
            "food" => Ok(Self::FoodItem),
            _ => Err(EntityKindParseError),
        }
    }
}
