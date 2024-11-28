use std::fmt;

use gtk::glib;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{settings::OperationMode, stock_id::StockId};

macro_rules! entity_data_field {
    ($($field:ident($ty:ty) => $display:expr),*) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub enum EntityDataFieldTy {
            $($field),*
        }

        impl fmt::Display for EntityDataFieldTy {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match self {
                    $(EntityDataFieldTy::$field => write!(f, $display)),*
                }
            }
        }

        impl EntityDataFieldTy {
            pub fn all() -> &'static [EntityDataFieldTy] {
                &[$(Self::$field),*]
            }
        }

        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub enum EntityDataField {
            $($field($ty)),*
        }

        impl fmt::Display for EntityDataField {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match self {
                    $(EntityDataField::$field(value) => fmt::Display::fmt(value, f)),*
                }
            }
        }

        impl EntityDataField {
            pub fn ty(&self) -> EntityDataFieldTy {
                match self {
                    $(Self::$field(_) => EntityDataFieldTy::$field),*
                }
            }
        }
    };
}

entity_data_field! {
    StockId(StockId) => "Stock Name",
    Location(String) => "Location",
    ExpirationDt(String) => "Expiration Date"
}

// TODO more efficient ser-de
#[derive(Debug, Clone, Serialize, Deserialize, glib::Boxed)]
#[boxed_type(name = "UetsEntityData", nullable)]
pub struct EntityData(IndexMap<EntityDataFieldTy, EntityDataField>);

impl EntityData {
    pub fn new() -> Self {
        Self(IndexMap::new())
    }

    pub fn from_fields(fields: impl IntoIterator<Item = EntityDataField>) -> Self {
        Self(fields.into_iter().map(|f| (f.ty(), f)).collect())
    }

    pub fn has_field(&self, field_ty: &EntityDataFieldTy) -> bool {
        self.0.contains_key(field_ty)
    }

    pub fn fields(&self) -> impl Iterator<Item = &EntityDataField> + '_ {
        self.0.values()
    }

    pub fn stock_id(&self) -> Option<&StockId> {
        self.0.get(&EntityDataFieldTy::StockId).map(|f| match f {
            EntityDataField::StockId(stock_id) => stock_id,
            _ => panic!(),
        })
    }
}

pub struct ValidEntityFields(&'static [(EntityDataFieldTy, bool)]);

impl ValidEntityFields {
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
            OperationMode::Parking => &[f!(EntityDataFieldTy::Location)],
            OperationMode::Inventory => &[
                f!(req EntityDataFieldTy::StockId),
                f!(EntityDataFieldTy::Location),
                f!(EntityDataFieldTy::ExpirationDt),
            ],
            OperationMode::Refrigerator => &[
                f!(req EntityDataFieldTy::StockId),
                f!(EntityDataFieldTy::ExpirationDt),
            ],
        })
    }

    pub fn contains(&self, field: EntityDataFieldTy) -> bool {
        self.0.iter().any(|&(f, _)| f == field)
    }

    pub fn is_valid_entity_data(&self, entity_data: &EntityData) -> bool {
        self.0.iter().all(|(f, is_required)| {
            if *is_required {
                entity_data.has_field(f)
            } else {
                true
            }
        })
    }
}
