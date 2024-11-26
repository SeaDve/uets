use crate::settings::OperationMode;

impl OperationMode {
    pub fn entities_view_icon_name(&self) -> &str {
        match self {
            OperationMode::Counter => "people-symbolic",
            OperationMode::Inventory => "tag-outline-symbolic",
            OperationMode::Refrigerator => "tag-outline-symbolic",
            OperationMode::Attendance => "people-symbolic",
        }
    }

    pub fn has_stocks(&self) -> bool {
        self.stocks_view_icon_name().is_some()
    }

    pub fn stocks_view_icon_name(&self) -> Option<&str> {
        match self {
            OperationMode::Counter => None,
            OperationMode::Inventory => Some("preferences-desktop-apps-symbolic"),
            OperationMode::Refrigerator => Some("egg-symbolic"),
            OperationMode::Attendance => None,
        }
    }
}
