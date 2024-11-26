use crate::settings::OperationMode;

impl OperationMode {
    pub fn entities_view_icon_name(&self) -> &str {
        match self {
            OperationMode::Counter => "people-symbolic",
            OperationMode::Attendance => "people-symbolic",
            OperationMode::Parking => "driving-symbolic",
            OperationMode::Inventory => "tag-outline-symbolic",
            OperationMode::Refrigerator => "tag-outline-symbolic",
        }
    }

    pub fn has_stocks(&self) -> bool {
        self.stocks_view_icon_name().is_some()
    }

    pub fn stocks_view_icon_name(&self) -> Option<&str> {
        match self {
            OperationMode::Counter => None,
            OperationMode::Attendance => None,
            OperationMode::Parking => None,
            OperationMode::Inventory => Some("preferences-desktop-apps-symbolic"),
            OperationMode::Refrigerator => Some("egg-symbolic"),
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

    pub fn stay_suffix(&self) -> &str {
        match self {
            OperationMode::Counter | OperationMode::Attendance => "of stay",
            OperationMode::Parking => "of parking",
            OperationMode::Inventory | OperationMode::Refrigerator => "of being kept",
        }
    }
}
