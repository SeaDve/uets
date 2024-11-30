use crate::{
    entity_data::{EntityData, EntityDataFieldTy, ValidEntityFields},
    settings::OperationMode,
};

impl OperationMode {
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
