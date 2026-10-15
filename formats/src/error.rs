//! One error type for every data file.

use std::fmt;

/// Why a data file could not be read, or is inconsistent.
#[derive(Debug)]
pub enum FormatError {
    /// The bytes are not the JSON this file should hold.
    Json(serde_json::Error),
    /// The file parses but contradicts itself (an index out of range, a bad length).
    Invalid(String),
}

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(e) => write!(f, "not the expected JSON: {e}"),
            Self::Invalid(why) => write!(f, "inconsistent data: {why}"),
        }
    }
}

impl std::error::Error for FormatError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Json(e) => Some(e),
            Self::Invalid(_) => None,
        }
    }
}

impl From<serde_json::Error> for FormatError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}
