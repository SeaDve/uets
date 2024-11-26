use serde::{Deserialize, Serialize};

use crate::{settings::OperationMode, stock_id::StockId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityDataField {
    StockId,
    Location,
    ExpirationDt,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityData {
    pub stock_id: Option<StockId>,
    pub location: Option<String>,
    pub expiration_dt: Option<String>, // FIXME use proper dt
}

impl EntityData {
    pub fn has_field(&self, field: EntityDataField) -> bool {
        match field {
            EntityDataField::StockId => self.stock_id.is_some(),
            EntityDataField::Location => self.location.is_some(),
            EntityDataField::ExpirationDt => self.expiration_dt.is_some(),
        }
    }
}

pub struct OperationModeValidFields(&'static [(EntityDataField, bool)]);

impl OperationModeValidFields {
    pub fn for_operation_mode(operation_mode: OperationMode) -> Self {
        macro_rules! f {
            ($field:expr) => {
                ($field, false)
            };
            (req $field:expr) => {
                ($field, true)
            };
        }

        Self(match operation_mode {
            OperationMode::Counter => &[],
            OperationMode::Attendance => &[],
            OperationMode::Parking => &[f!(EntityDataField::Location)],
            OperationMode::Inventory => &[
                f!(req EntityDataField::StockId),
                f!(EntityDataField::Location),
                f!(EntityDataField::ExpirationDt),
            ],
            OperationMode::Refrigerator => &[
                f!(req EntityDataField::StockId),
                f!(EntityDataField::ExpirationDt),
            ],
        })
    }

    pub fn contains(&self, field: EntityDataField) -> bool {
        self.0.iter().any(|&(f, _)| f == field)
    }

    pub fn is_valid_entity_data(&self, entity_data: &EntityData) -> bool {
        self.0.iter().all(|&(f, is_required)| {
            if is_required {
                entity_data.has_field(f)
            } else {
                true
            }
        })
    }
}
