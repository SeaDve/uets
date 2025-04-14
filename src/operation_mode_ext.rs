use std::fmt;

use crate::{
    entity_data::{EntityData, EntityDataFieldTy, ValidEntityFields},
    entity_kind::EntityKind,
    settings::OperationMode,
};

impl fmt::Display for OperationMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OperationMode::Counter => write!(f, "Counter"),
            OperationMode::Attendance => write!(f, "Attendance"),
            OperationMode::Parking => write!(f, "Parking"),
            OperationMode::Inventory => write!(f, "Inventory"),
            OperationMode::Refrigerator => write!(f, "Refrigerator"),
        }
    }
}

impl OperationMode {
    pub fn all() -> &'static [OperationMode] {
        &[
            OperationMode::Counter,
            OperationMode::Attendance,
            OperationMode::Parking,
            OperationMode::Inventory,
            OperationMode::Refrigerator,
        ]
    }

    pub fn description(&self) -> &str {
        match self {
            OperationMode::Counter => {
                "Used for counting entities without data attached (e.g., mall entry counter)"
            }
            OperationMode::Attendance => {
                "Used for tracking people attendance, alerting when unauthorized entity is detected (e.g., meeting rooms, establishment entry)"
            },
            OperationMode::Parking => "Used for tracking parking spaces and vehicles (e.g., parking lots)",
            OperationMode::Inventory => "Used for tracking inventory items with lifetime, location, and quantity (e.g., stock rooms)",
            OperationMode::Refrigerator => "Used for tracking food items with lifetime, quantity, and recipe suggestions",
        }
    }

    pub fn is_valid_entity_data_field_ty(&self, entity_field: EntityDataFieldTy) -> bool {
        ValidEntityFields::for_entity_kind(self.entity_kind()).contains(entity_field)
    }

    pub fn is_valid_entity_data(&self, entity_data: &EntityData) -> bool {
        ValidEntityFields::for_entity_kind(self.entity_kind()).is_valid_entity_data(entity_data)
    }

    pub fn entity_kind(&self) -> EntityKind {
        match self {
            OperationMode::Counter => EntityKind::General,
            OperationMode::Attendance => EntityKind::Person,
            OperationMode::Parking => EntityKind::Vehicle,
            OperationMode::Inventory => EntityKind::StockItem,
            OperationMode::Refrigerator => EntityKind::FoodItem,
        }
    }

    pub fn entities_view_icon_name(&self) -> &str {
        match self {
            OperationMode::Counter => "person-symbolic",
            OperationMode::Attendance => "person-symbolic",
            OperationMode::Parking => "driving-symbolic",
            OperationMode::Inventory => "tag-outline-symbolic",
            OperationMode::Refrigerator => "tag-outline-symbolic",
        }
    }

    pub fn stocks_view_icon_name(&self) -> Option<&str> {
        let ret = match self {
            OperationMode::Counter => None,
            OperationMode::Attendance => None,
            OperationMode::Parking => None,
            OperationMode::Inventory => Some("preferences-desktop-apps-symbolic"),
            OperationMode::Refrigerator => Some("egg-symbolic"),
        };

        debug_assert_eq!(
            ret.is_some(),
            self.is_valid_entity_data_field_ty(EntityDataFieldTy::StockId)
        );

        ret
    }

    pub fn n_inside_term(&self) -> &str {
        match self {
            OperationMode::Counter => "Inside Count",
            OperationMode::Attendance => "Total People Inside",
            OperationMode::Parking => "Total Vehicles Inside",
            OperationMode::Inventory => "Total Stocks Count",
            OperationMode::Refrigerator => "Total Food Items Count",
        }
    }

    pub fn enter_verb(&self) -> &str {
        match self {
            OperationMode::Counter | OperationMode::Attendance => "enters",
            OperationMode::Parking => "drives in",
            OperationMode::Inventory | OperationMode::Refrigerator => "added",
        }
    }

    pub fn exit_verb(&self) -> &str {
        match self {
            OperationMode::Counter | OperationMode::Attendance => "exits",
            OperationMode::Parking => "drives out",
            OperationMode::Inventory | OperationMode::Refrigerator => "removed",
        }
    }

    pub fn entry_to_exit_duration_suffix(&self) -> &str {
        match self {
            OperationMode::Counter | OperationMode::Attendance => "of stay",
            OperationMode::Parking => "of parking",
            OperationMode::Inventory | OperationMode::Refrigerator => "of being kept",
        }
    }
}
