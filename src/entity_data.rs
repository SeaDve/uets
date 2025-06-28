use std::fmt;

use chrono::{DateTime, Utc};
use gtk::glib;
use indexmap::IndexMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{
    date_time_range::DateTimeRange, entity_id::EntityId, entity_kind::EntityKind,
    jpeg_image::JpegImage, sex::Sex, stock_id::StockId,
};

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
    Kind(EntityKind) => "Kind",
    StockId(StockId) => "Stock Name",
    Possessor(EntityId) => "Possessor",
    Location(String) => "Location",
    ExpirationDt(DateTime<Utc>) => "Expiration Date",
    AllowedDtRange(DateTimeRange) => "Allowed Date Range",
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

#[derive(Debug, Clone, glib::Boxed)]
#[boxed_type(name = "UetsEntityDataFieldVecBoxed")]
pub struct EntityDataFieldVecBoxed(pub Vec<EntityDataField>);

#[derive(Debug, Default, Clone, PartialEq, Eq, glib::Boxed)]
#[boxed_type(name = "UetsEntityData", nullable)]
pub struct EntityData(IndexMap<EntityDataFieldTy, EntityDataField>);

impl EntityData {
    pub fn from_fields(fields: impl IntoIterator<Item = EntityDataField>) -> Self {
        let mut this = {
            let mut inner = IndexMap::new();

            for field in fields {
                let field_ty = field.ty();
                if inner.insert(field_ty, field).is_some() {
                    tracing::warn!("Duplicate field: {:?}; removing previous value", field_ty);
                }
            }

            Self(inner)
        };

        let kind = this.kind();
        let valid_entity_fields = ValidEntityFields::for_entity_kind(kind);

        if tracing::enabled!(tracing::Level::ERROR) {
            for (field_ty, is_required) in valid_entity_fields.0.iter() {
                if *is_required && !this.has_field(*field_ty) {
                    tracing::error!(
                        "Entity data for kind {:?} is missing required field {:?}",
                        kind,
                        field_ty
                    );
                }
            }
        }

        this.0.retain(|field_ty, _| {
            let is_valid_field = valid_entity_fields.contains(*field_ty);

            if !is_valid_field {
                tracing::warn!(
                    "Entity data for kind {:?} contains field {:?} that is not valid; removing it",
                    kind,
                    field_ty
                );
            }

            is_valid_field
        });

        this
    }

    pub fn extended(self, fields: impl IntoIterator<Item = EntityDataField>) -> Self {
        let mut ret = self.0.clone();
        ret.extend(fields.into_iter().map(|field| (field.ty(), field)));
        Self(ret)
    }

    pub fn has_field(&self, field_ty: EntityDataFieldTy) -> bool {
        self.0.contains_key(&field_ty)
    }

    pub fn get(&self, field_ty: EntityDataFieldTy) -> Option<&EntityDataField> {
        self.0.get(&field_ty)
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

    pub fn with_possessor(self, possessor: Option<EntityId>) -> Self {
        Self::from_fields(
            self.0
                .into_values()
                .filter(|f| f.ty() != EntityDataFieldTy::Possessor)
                .chain(possessor.map(EntityDataField::Possessor)),
        )
    }

    pub fn kind(&self) -> EntityKind {
        *self
            .0
            .get(&EntityDataFieldTy::Kind)
            .map(|f| match f {
                EntityDataField::Kind(value) => value,
                _ => unreachable!(),
            })
            .expect("kind is required")
    }

    entity_data_getter!(stock_id, StockId, &StockId);
    entity_data_getter!(allowed_dt_range, AllowedDtRange, &DateTimeRange);
    entity_data_getter!(possessor, Possessor, &EntityId);
    entity_data_getter!(photo, Photo, &JpegImage);
    entity_data_getter!(name, Name, &String);
    entity_data_getter!(sex, Sex, &Sex);
    entity_data_getter!(email, Email, &String);
    entity_data_getter!(program, Program, &String);
    entity_data_getter!(location, Location, &String);
    entity_data_getter!(expiration_dt, ExpirationDt, &DateTime<Utc>);
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
    pub fn for_entity_kind(entity_kind: EntityKind) -> Self {
        macro_rules! f {
            ($field:expr) => {
                ($field, false)
            };
            (req $field:expr) => {
                ($field, true)
            };
        }

        Self(match entity_kind {
            EntityKind::Person => &[
                f!(req EntityDataFieldTy::Kind),
                f!(EntityDataFieldTy::AllowedDtRange),
                f!(EntityDataFieldTy::Photo),
                f!(EntityDataFieldTy::Name),
                f!(EntityDataFieldTy::Sex),
                f!(EntityDataFieldTy::Email),
                f!(EntityDataFieldTy::Program),
            ],
            EntityKind::Vehicle => &[
                f!(req EntityDataFieldTy::Kind),
                f!(EntityDataFieldTy::Possessor),
                f!(EntityDataFieldTy::AllowedDtRange),
                f!(EntityDataFieldTy::Photo),
                f!(EntityDataFieldTy::Name),
                f!(EntityDataFieldTy::Location),
            ],
            EntityKind::Item => &[
                f!(req EntityDataFieldTy::Kind),
                f!(req EntityDataFieldTy::StockId),
                f!(EntityDataFieldTy::Possessor),
                f!(EntityDataFieldTy::AllowedDtRange),
                f!(EntityDataFieldTy::Photo),
                f!(EntityDataFieldTy::Location),
                f!(EntityDataFieldTy::ExpirationDt),
            ],
        })
    }

    pub fn contains(&self, field: EntityDataFieldTy) -> bool {
        self.0.iter().any(|&(f, _)| f == field)
    }
}
