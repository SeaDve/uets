use std::fmt;

use chrono::{DateTime, Utc};
use gtk::glib;
use indexmap::IndexMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{jpeg_image::JpegImage, settings::OperationMode, sex::Sex, stock_id::StockId};

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

        #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    ExpirationDt(DateTime<Utc>) => "Expiration Date",
    Photo(JpegImage) => "Photo",
    Name(String) => "Name",
    Sex(Sex) => "Sex",
    Email(String) => "Email",
    Program(String) => "Program"
}

macro_rules! entity_data_getter {
    ($fn_name:ident, $field:ident, $return_ty:ty) => {
        pub fn $fn_name(&self) -> Option<$return_ty> {
            self.0.get(&EntityDataFieldTy::$field).map(|f| match f {
                EntityDataField::$field(value) => value,
                _ => unreachable!(),
            })
        }
    };
}

#[derive(Debug, Default, Clone, PartialEq, Eq, glib::Boxed)]
#[boxed_type(name = "UetsEntityData", nullable)]
pub struct EntityData(IndexMap<EntityDataFieldTy, EntityDataField>);

impl EntityData {
    pub fn new() -> Self {
        Self(IndexMap::new())
    }

    pub fn from_fields(fields: impl IntoIterator<Item = EntityDataField>) -> Self {
        Self(fields.into_iter().map(|f| (f.ty(), f)).collect())
    }

    pub fn has_field(&self, field_ty: EntityDataFieldTy) -> bool {
        self.0.contains_key(&field_ty)
    }

    pub fn fields(&self) -> impl Iterator<Item = &EntityDataField> + '_ {
        self.0.values()
    }

    pub fn with_stock_id(self, stock_id: Option<StockId>) -> Self {
        Self::from_fields(
            self.0
                .into_values()
                .filter(|f| f.ty() != EntityDataFieldTy::StockId)
                .chain(stock_id.map(EntityDataField::StockId)),
        )
    }

    entity_data_getter!(stock_id, StockId, &StockId);
    entity_data_getter!(location, Location, &String);
    entity_data_getter!(expiration_dt, ExpirationDt, &DateTime<Utc>);
    entity_data_getter!(photo, Photo, &JpegImage);
    entity_data_getter!(name, Name, &String);
    entity_data_getter!(sex, Sex, &Sex);
    entity_data_getter!(email, Email, &String);
    entity_data_getter!(program, Program, &String);
}

impl Serialize for EntityData {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(self.0.values())
    }
}

impl<'de> Deserialize<'de> for EntityData {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let fields = Vec::<EntityDataField>::deserialize(deserializer)?;
        Ok(Self::from_fields(fields))
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

        let person_valid_entity_fields = &[
            f!(EntityDataFieldTy::Photo),
            f!(EntityDataFieldTy::Name),
            f!(EntityDataFieldTy::Sex),
            f!(EntityDataFieldTy::Email),
            f!(EntityDataFieldTy::Program),
        ];
        Self(match operation_mode {
            OperationMode::Counter => person_valid_entity_fields,
            OperationMode::Attendance => person_valid_entity_fields,
            OperationMode::Parking => &[
                f!(EntityDataFieldTy::Photo),
                f!(EntityDataFieldTy::Location),
            ],
            OperationMode::Inventory => &[
                f!(EntityDataFieldTy::Photo),
                f!(req EntityDataFieldTy::StockId),
                f!(EntityDataFieldTy::Location),
                f!(EntityDataFieldTy::ExpirationDt),
            ],
            OperationMode::Refrigerator => &[
                f!(EntityDataFieldTy::Photo),
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
                entity_data.has_field(*f)
            } else {
                true
            }
        }) && entity_data.fields().all(|f| self.contains(f.ty()))
    }
}
