//! `translations.json`: the texts translated on SuttaCentral, per language.

use crate::FormatError;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// What one language has: the title of each translated text and who translated it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LangTranslations {
    /// uid to the translated title.
    pub titles: IndexMap<String, String>,
    /// uid to the index of its translator in [`LangTranslations::translators`].
    pub by: IndexMap<String, u32>,
    /// The translators' names.
    pub translators: Vec<String>,
}

/// Language code (`en`, `fr`, `vi`) to its translations, in file order.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Translations(pub IndexMap<String, LangTranslations>);

impl Translations {
    /// Parse the file.
    ///
    /// # Errors
    /// When the bytes are not a translations file, or it contradicts itself
    /// ([`Translations::check`]).
    pub fn from_json(bytes: &[u8]) -> Result<Translations, FormatError> {
        let t: Translations = serde_json::from_slice(bytes)?;
        t.check()?;
        Ok(t)
    }

    /// The file's JSON, compact.
    ///
    /// # Errors
    /// When serialisation fails.
    pub fn to_json(&self) -> Result<String, FormatError> {
        Ok(serde_json::to_string(self)?)
    }

    /// Every translator index exists, and every credited text has a title.
    ///
    /// # Errors
    /// With the first inconsistency found.
    pub fn check(&self) -> Result<(), FormatError> {
        for (lang, l) in &self.0 {
            for (uid, &who) in &l.by {
                if who as usize >= l.translators.len() {
                    return Err(FormatError::Invalid(format!(
                        "{lang}: {uid} credits translator {who}, only {} translators",
                        l.translators.len()
                    )));
                }
                if !l.titles.contains_key(uid) {
                    return Err(FormatError::Invalid(format!(
                        "{lang}: {uid} is credited but has no title"
                    )));
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILE: &str = r#"{"en":{"titles":{"dn1":"The Net","dn2":"Fruits"},"by":{"dn1":0,"dn2":1},"translators":["A","B"]},"fr":{"titles":{},"by":{},"translators":[]}}"#;

    #[test]
    fn reads_titles_and_translators_per_language() {
        let t = Translations::from_json(FILE.as_bytes()).unwrap();
        assert_eq!(t.0.keys().collect::<Vec<_>>(), ["en", "fr"]);
        let en = &t.0["en"];
        assert_eq!(en.titles["dn1"], "The Net");
        assert_eq!(en.translators[en.by["dn2"] as usize], "B");
        assert_eq!(t.to_json().unwrap(), FILE);
    }

    #[test]
    fn by_index_is_in_range() {
        let bad = FILE.replace(r#""dn2":1}"#, r#""dn2":2}"#);
        let err = Translations::from_json(bad.as_bytes())
            .unwrap_err()
            .to_string();
        assert!(err.contains("dn2") && err.contains("translator 2"), "{err}");
    }

    #[test]
    fn a_credited_text_has_a_title() {
        let bad = FILE.replace(r#""by":{"dn1":0,"dn2":1}"#, r#""by":{"dn1":0,"dn9":1}"#);
        let err = Translations::from_json(bad.as_bytes())
            .unwrap_err()
            .to_string();
        assert!(err.contains("dn9"), "{err}");
    }

    #[test]
    fn rejects_a_file_of_the_wrong_shape() {
        assert!(Translations::from_json(b"[]").is_err());
        assert!(Translations::from_json(br#"{"en":{"titles":{}}}"#).is_err());
    }

    #[test]
    fn credits_with_no_translators_are_an_error() {
        let file = r#"{"en":{"titles":{"dn1":"The Net"},"by":{"dn1":0},"translators":[]}}"#;
        let err = Translations::from_json(file.as_bytes()).unwrap_err();
        assert!(matches!(err, FormatError::Invalid(_)), "{err}");
        assert!(err.to_string().contains("dn1"), "{err}");
    }
}
