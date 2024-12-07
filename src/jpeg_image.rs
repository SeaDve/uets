use std::{cell::OnceCell, fmt, rc::Rc};

use gtk::{gdk, glib};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Clone, PartialEq, Eq, glib::Boxed)]
#[boxed_type(name = "UetsJpegImage", nullable)]
pub struct JpegImage {
    bytes: glib::Bytes,
    texture: Rc<OnceCell<Result<gdk::Texture, glib::Error>>>,
}

impl fmt::Debug for JpegImage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JpegImage")
            .field("size", &glib::format_size(self.bytes.len() as u64))
            .finish()
    }
}

impl fmt::Display for JpegImage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} JPEG Image",
            glib::format_size(self.bytes.len() as u64)
        )
    }
}

impl JpegImage {
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self {
            bytes: glib::Bytes::from_owned(bytes),
            texture: Rc::new(OnceCell::new()),
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.bytes.as_ref()
    }

    pub fn from_base64(string: &str) -> Self {
        Self::from_bytes(glib::base64_decode(string))
    }

    pub fn to_base64(&self) -> glib::GString {
        glib::base64_encode(&self.bytes)
    }

    pub fn texture(&self) -> Result<&gdk::Texture, glib::Error> {
        self.texture
            .get_or_init(|| gdk::Texture::from_bytes(&self.bytes))
            .as_ref()
            .map_err(|err| err.clone())
    }
}

impl Serialize for JpegImage {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.to_base64().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for JpegImage {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let string = String::deserialize(deserializer)?;
        Ok(Self::from_base64(&string))
    }
}
