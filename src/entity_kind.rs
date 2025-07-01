use std::{error, fmt, str::FromStr};

use gtk::glib;
use serde::{Deserialize, Serialize};

use crate::entity_data::{EntityDataFieldTy, ValidEntityFields};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, glib::Enum)]
#[enum_type(name = "UetsEntityKind")]
pub enum EntityKind {
    #[default]
    Person,
    Vehicle,
    Item,
}

impl EntityKind {
    pub fn is_valid_entity_data_field_ty(&self, entity_field: EntityDataFieldTy) -> bool {
        ValidEntityFields::for_entity_kind(*self).contains(entity_field)
    }

    pub fn enter_verb(&self) -> &str {
        match self {
            EntityKind::Person => "enters",
            EntityKind::Vehicle => "is driven in",
            EntityKind::Item => "is added",
        }
    }

    pub fn enter_verb_with_possessor(&self, possessor_display: &str) -> String {
        match self {
            EntityKind::Person => self.enter_verb().to_string(),
            EntityKind::Vehicle => format!("is driven in by {possessor_display}"),
            EntityKind::Item => format!("is added by {possessor_display}"),
        }
    }

    pub fn exit_verb(&self) -> &str {
        match self {
            EntityKind::Person => "exits",
            EntityKind::Vehicle => "is driven out",
            EntityKind::Item => "is removed",
        }
    }

    pub fn exit_verb_with_possessor(&self, possessor_display: &str) -> String {
        match self {
            EntityKind::Person => self.exit_verb().to_string(),
            EntityKind::Vehicle => format!("is driven out by {possessor_display}"),
            EntityKind::Item => format!("is removed by {possessor_display}"),
        }
    }

    pub fn enter_status(&self) -> &str {
        match self {
            EntityKind::Person => "Entered",
            EntityKind::Vehicle => "Driven in",
            EntityKind::Item => "Added",
        }
    }

    pub fn enter_status_with_possessor(&self, possessor_display: &str) -> String {
        match self {
            EntityKind::Person => self.enter_status().to_string(),
            EntityKind::Vehicle => format!("Driven in by {possessor_display}"),
            EntityKind::Item => format!("Added by {possessor_display}"),
        }
    }

    pub fn exit_status(&self) -> &str {
        match self {
            EntityKind::Person => "Exited",
            EntityKind::Vehicle => "Driven out",
            EntityKind::Item => "Removed",
        }
    }

    pub fn exit_status_with_possessor(&self, possessor_display: &str) -> String {
        match self {
            EntityKind::Person => self.exit_status().to_string(),
            EntityKind::Vehicle => format!("Driven out by {possessor_display}"),
            EntityKind::Item => format!("Removed by {possessor_display}"),
        }
    }

    pub fn default_status(&self) -> &str {
        match self {
            EntityKind::Person => "Never entered",
            EntityKind::Vehicle => "Never driven in",
            EntityKind::Item => "Never added",
        }
    }

    pub fn default_status_with_possessor(&self, possessor_display: &str) -> String {
        match self {
            EntityKind::Person => self.default_status().to_string(),
            EntityKind::Vehicle => format!("Never driven in by {possessor_display}"),
            EntityKind::Item => format!("Never added by {possessor_display}"),
        }
    }

    pub fn entry_to_exit_duration_suffix(&self) -> &str {
        match self {
            EntityKind::Person => "of stay",
            EntityKind::Vehicle => "of parking",
            EntityKind::Item => "of being kept",
        }
    }

    pub fn entities_view_icon_name(&self) -> &str {
        match self {
            Self::Person => "person-symbolic",
            Self::Vehicle => "driving-symbolic",
            Self::Item => "package-x-generic-symbolic",
        }
    }
}

impl fmt::Display for EntityKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EntityKind::Person => write!(f, "Person"),
            EntityKind::Vehicle => write!(f, "Vehicle"),
            EntityKind::Item => write!(f, "Item"),
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
            "person" => Ok(Self::Person),
            "vehicle" => Ok(Self::Vehicle),
            "item" => Ok(Self::Item),
            _ => Err(EntityKindParseError),
        }
    }
}
