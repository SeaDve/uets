use std::{borrow::Cow, fmt};

use gtk::glib;
use heed::types::Str;

/// This must be universally unique for each entity, regardless if they have the same name (e.g., entity of same
/// model must have different IDs).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, glib::Boxed)]
#[boxed_type(name = "UetsEntityId")]
pub struct EntityId(Box<str>);

impl EntityId {
    pub fn new(id: impl Into<Box<str>>) -> Self {
        Self(id.into())
    }
}

impl fmt::Display for EntityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub struct EntityIdCodec;

impl heed::BytesEncode<'_> for EntityIdCodec {
    type EItem = EntityId;

    fn bytes_encode(item: &Self::EItem) -> Result<Cow<'_, [u8]>, heed::BoxedError> {
        Str::bytes_encode(&item.0)
    }
}

impl<'a> heed::BytesDecode<'a> for EntityIdCodec {
    type DItem = EntityId;

    fn bytes_decode(bytes: &'a [u8]) -> Result<Self::DItem, heed::BoxedError> {
        Str::bytes_decode(bytes).map(EntityId::new)
    }
}
