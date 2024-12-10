use std::fmt;

use chrono::Utc;

use crate::{
    date_time,
    date_time_range::DateTimeRange,
    entity::Entity,
    entity_data::{EntityData, EntityDataFieldTy, ValidEntityFields},
    format,
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
        ValidEntityFields::for_operation_mode(*self).contains(entity_field)
    }

    pub fn is_valid_entity_data(&self, entity_data: &EntityData) -> bool {
        ValidEntityFields::for_operation_mode(*self).is_valid_entity_data(entity_data)
    }

    pub fn is_for_person(&self) -> bool {
        match self {
            OperationMode::Counter => true,
            OperationMode::Attendance => true,
            OperationMode::Parking => false,
            OperationMode::Inventory => false,
            OperationMode::Refrigerator => false,
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

pub trait OperationModeEntityExt {
    fn status_display(
        &self,
        for_dt_range: &DateTimeRange,
        operation_mode: OperationMode,
        use_red_markup_on_entry_to_exit_duration: bool,
    ) -> String;
}

impl OperationModeEntityExt for Entity {
    fn status_display(
        &self,
        for_dt_range: &DateTimeRange,
        operation_mode: OperationMode,
        use_red_markup_on_entry_to_exit_duration: bool,
    ) -> String {
        match self.is_inside_for_dt_range_full(for_dt_range) {
            Some((dt, true)) => {
                let verb = match operation_mode {
                    OperationMode::Counter | OperationMode::Attendance => "Entered",
                    OperationMode::Parking => "Drove in",
                    OperationMode::Inventory | OperationMode::Refrigerator => "Added",
                };
                let entry_to_exit_duration_prefix = match operation_mode {
                    OperationMode::Counter | OperationMode::Attendance => "stayed",
                    OperationMode::Parking => "parked",
                    OperationMode::Inventory | OperationMode::Refrigerator => "kept",
                };

                let duration_start = if let Some(start) = for_dt_range.start {
                    start.max(dt)
                } else {
                    dt
                };
                let duration_end = if let Some(end) = for_dt_range.end {
                    end
                } else {
                    Utc::now()
                };
                let formatted_duration = format::duration(duration_end - duration_start);

                format!(
                    "{verb} {} and {entry_to_exit_duration_prefix} for {}",
                    date_time::format::fuzzy(dt),
                    if use_red_markup_on_entry_to_exit_duration {
                        format::red_markup(&formatted_duration)
                    } else {
                        formatted_duration
                    },
                )
            }
            Some((dt, false)) => {
                let verb = match operation_mode {
                    OperationMode::Counter | OperationMode::Attendance => "Exited",
                    OperationMode::Parking => "Drove out",
                    OperationMode::Inventory | OperationMode::Refrigerator => "Removed",
                };
                format!("{verb} {}", date_time::format::fuzzy(dt))
            }
            None => "Never entered".to_string(),
        }
    }
}
