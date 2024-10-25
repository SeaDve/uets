use anyhow::{Context, Result};
use gtk::glib;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

/// A [`glib::DateTime`] that implements [`Serialize`] and [`Deserialize`]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, glib::ValueDelegate)]
#[value_delegate(nullable)]
pub struct DateTime(glib::DateTime);

impl DateTime {
    pub fn now_utc() -> Self {
        Self(glib::DateTime::now_utc().unwrap())
    }

    pub fn to_local(&self) -> Self {
        Self(self.0.to_local().unwrap())
    }

    pub fn from_iso8601(string: &str) -> Result<Self> {
        glib::DateTime::from_iso8601(string, None)
            .map(Self)
            .with_context(|| format!("Invalid iso8601 datetime `{}`", string))
    }

    pub fn format_iso8601(&self) -> glib::GString {
        self.0.format_iso8601().unwrap()
    }

    pub fn fuzzy_display(&self) -> glib::GString {
        let now = Self::now_utc();

        if self.0.ymd() == now.0.ymd() {
            // Translators: `%R` will be replaced with 24-hour formatted datetime (e.g., `13:21`)
            self.0.format("today at %R")
        } else if now.0.difference(&self.0).as_hours() <= 30 {
            // Translators: `%R` will be replaced with 24-hour formatted datetime (e.g., `13:21`)
            self.0.format("yesterday at %R")
        } else {
            self.0.format("%F") // ISO 8601 (e.g., `2001-07-08`)
        }
        .expect("format must be correct")
    }
}

impl Serialize for DateTime {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.format_iso8601().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for DateTime {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let string = <&str>::deserialize(deserializer)?;
        DateTime::from_iso8601(string).map_err(de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize() {
        let dt = DateTime::from_iso8601("2022-07-28T08:23:28.623259+08").unwrap();
        assert_eq!(
            serde_json::to_string(&dt).unwrap(),
            "\"2022-07-28T08:23:28.623259+08\"",
        );

        assert_eq!(dt.format_iso8601(), "2022-07-28T08:23:28.623259+08");
    }

    #[test]
    fn deserialize() {
        assert_eq!(
            DateTime::from_iso8601("2022-07-28T08:23:28.623259+08").unwrap(),
            serde_json::from_str("\"2022-07-28T08:23:28.623259+08\"").unwrap()
        );

        assert!(DateTime::from_iso8601("2022").is_err());
        assert!(serde_json::from_str::<DateTime>("\"2022\"").is_err());
    }
}
