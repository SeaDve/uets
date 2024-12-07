use std::{error, fmt, str::FromStr};

use gtk::glib;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, glib::Enum)]
#[enum_type(name = "UetsSex")]
pub enum Sex {
    Male,
    Female,
}

impl fmt::Display for Sex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Sex::Male => write!(f, "Male"),
            Sex::Female => write!(f, "Female"),
        }
    }
}

#[derive(Debug)]
pub struct SexParseError;

impl fmt::Display for SexParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Failed to parse sex")
    }
}

impl error::Error for SexParseError {}

impl FromStr for Sex {
    type Err = SexParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().trim() {
            "male" | "m" => Ok(Self::Male),
            "female" | "f" => Ok(Self::Female),
            _ => Err(SexParseError),
        }
    }
}
