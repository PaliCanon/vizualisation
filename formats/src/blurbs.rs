//! `blurbs-{en,fr}.json`: SuttaCentral's summary of a text, in one language.

use crate::FormatError;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// The summaries of one language: uid to blurb, in file order.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Blurbs(pub IndexMap<String, String>);

impl Blurbs {
    /// Parse a blurbs file.
    ///
    /// # Errors
    /// When the bytes are not a JSON object of strings.
    pub fn from_json(bytes: &[u8]) -> Result<Blurbs, FormatError> {
        Ok(serde_json::from_slice(bytes)?)
    }

    /// The file's JSON, compact.
    ///
    /// # Errors
    /// When serialisation fails.
    pub fn to_json(&self) -> Result<String, FormatError> {
        Ok(serde_json::to_string(self)?)
    }

    /// The blurb of `uid`.
    #[must_use]
    pub fn get(&self, uid: &str) -> Option<&str> {
        self.0.get(uid).map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_uid_to_text_in_file_order() {
        let b = Blurbs::from_json(br#"{"an3.1":"Fools","dn1":"Net"}"#).unwrap();
        assert_eq!(b.get("an3.1"), Some("Fools"));
        assert_eq!(b.get("nope"), None);
        assert_eq!(b.0.keys().collect::<Vec<_>>(), ["an3.1", "dn1"]);
        assert_eq!(b.to_json().unwrap(), r#"{"an3.1":"Fools","dn1":"Net"}"#);
    }

    #[test]
    fn rejects_what_is_not_an_object_of_strings() {
        assert!(Blurbs::from_json(b"[]").is_err());
        assert!(Blurbs::from_json(br#"{"a":1}"#).is_err());
    }
}
