//! The texts' language: Pāli (the whole canon) or a language the texts are translated into
//! (only those texts, with their translated titles). Design v4's TEXTLANG node: Pāli on a
//! first visit; the interface language once it changes; else the reader's choice (the
//! chip), kept in the `viz_textlang` cookie as `<choice>|<interface language at the
//! choice>` (`fr|en`), written by the browser only. The React page kept the choice in
//! localStorage and followed the interface language with a ref (`src/App.tsx` lines 83–101).

use std::collections::BTreeMap;

use palicanon_ui_lib::Language;

use crate::map::model::{Graph, Translations};

/// The cookie's name.
pub const COOKIE: &str = "viz_textlang";

/// The language the map's texts are shown in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextsLang {
    /// The Pāli canon, every text.
    Pali,
    /// Only the texts translated into this language.
    Translated(Language),
}

impl TextsLang {
    /// Every choice, in the chip's order (App.tsx lines 34–39 `TEXT_LANGS`).
    pub const ALL: [TextsLang; 4] = [
        TextsLang::Pali,
        TextsLang::Translated(Language::En),
        TextsLang::Translated(Language::Fr),
        TextsLang::Translated(Language::Vi),
    ];

    /// The code: `pli`, else the language's (`translations.json`'s keys).
    pub fn code(self) -> &'static str {
        match self {
            TextsLang::Pali => "pli",
            TextsLang::Translated(language) => language.code(),
        }
    }

    /// The choice whose [`TextsLang::code`] is `code`.
    pub fn from_code(code: &str) -> Option<TextsLang> {
        TextsLang::ALL.into_iter().find(|t| t.code() == code)
    }

    /// The chip's label: `Pāli`, else the language's own name (App.tsx lines 35–38).
    pub fn label(self) -> &'static str {
        match self {
            TextsLang::Pali => "Pāli",
            TextsLang::Translated(language) => language.name(),
        }
    }
}

/// The texts' language for a `viz_textlang` cookie and the interface language: absent (or
/// garbled) → Pāli; the interface language differs from the one recorded at the choice →
/// the interface language; else the choice. App.tsx line 101 (`textLangChoice ?? "pli"`)
/// and lines 89–94 (an interface change sets the choice to it).
pub fn texts_lang(viz_textlang: Option<&str>, interface: Language) -> TextsLang {
    let Some((choice, at_choice)) = viz_textlang.and_then(parse_cookie) else {
        return TextsLang::Pali;
    };
    if at_choice == interface {
        choice
    } else {
        TextsLang::Translated(interface)
    }
}

/// The choice and the interface language a `viz_textlang` value records.
fn parse_cookie(value: &str) -> Option<(TextsLang, Language)> {
    let (choice, at_choice) = value.split_once('|')?;
    Some((TextsLang::from_code(choice)?, at_choice.parse().ok()?))
}

/// The `viz_textlang` value recording `choice` made while the interface is in `interface`.
pub fn cookie_value(choice: TextsLang, interface: Language) -> String {
    format!("{}|{}", choice.code(), interface.code())
}

/// The language of glosses and summaries: the texts' language, the interface language when
/// the texts are in Pāli (App.tsx line 105; SuttaCentral's English is the fallback the
/// callers apply per text).
pub fn gloss_lang(texts: TextsLang, interface: Language) -> Language {
    match texts {
        TextsLang::Pali => interface,
        TextsLang::Translated(language) => language,
    }
}

/// One mask per language of `translations`: for each node of `graph`, whether the language
/// has it (App.tsx lines 274–279, `masks`).
pub fn masks(graph: &Graph, translations: &Translations) -> BTreeMap<String, Vec<bool>> {
    translations
        .0
        .iter()
        .map(|(lang, tr)| {
            let mask = graph
                .nodes
                .iter()
                .map(|node| tr.titles.contains_key(&node.uid))
                .collect();
            (lang.clone(), mask)
        })
        .collect()
}

/// The texts `texts` shows: none filtered in Pāli, else its mask; a language without
/// translations shows nothing filtered (App.tsx line 280, `masks?.get(textLang) ?? null`).
pub fn available(texts: TextsLang, masks: &BTreeMap<String, Vec<bool>>) -> Option<&[bool]> {
    match texts {
        TextsLang::Pali => None,
        TextsLang::Translated(language) => masks.get(language.code()).map(Vec::as_slice),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::{fixture, public_file};

    const FR: TextsLang = TextsLang::Translated(Language::Fr);
    const EN: TextsLang = TextsLang::Translated(Language::En);

    #[test]
    fn codes_and_labels_round_trip() {
        // App.tsx:35-38 `{ value: "pli", label: "Pāli" }`, `{ value: "fr", label: "Français" }`
        assert_eq!(
            TextsLang::ALL.map(TextsLang::code),
            ["pli", "en", "fr", "vi"]
        );
        assert_eq!(TextsLang::Pali.label(), "Pāli");
        assert_eq!(FR.label(), "Français");
        for t in TextsLang::ALL {
            assert_eq!(TextsLang::from_code(t.code()), Some(t));
        }
        assert_eq!(TextsLang::from_code("de"), None);
    }

    #[test]
    fn pali_on_a_first_visit() {
        // App.tsx:101 `const textLang: TextLang = textLangChoice ?? "pli";`
        for interface in Language::ALL {
            assert_eq!(texts_lang(None, interface), TextsLang::Pali);
        }
        for garbled in ["", "fr", "de|en", "fr|de", "fr|en|vi", "Fr|en"] {
            assert_eq!(
                texts_lang(Some(garbled), Language::En),
                TextsLang::Pali,
                "{garbled}"
            );
        }
    }

    #[test]
    fn a_cookie_naming_an_unknown_language_is_pali() {
        // A language the site does not have, as the choice or as the interface at the
        // choice (a cookie from another version, or edited): Pāli, as a first visit.
        for interface in Language::ALL {
            for value in ["de|en", "fr|de", "pli|xx", "de|de", "|en", "fr|"] {
                assert_eq!(
                    texts_lang(Some(value), interface),
                    TextsLang::Pali,
                    "{value} in {interface:?}"
                );
            }
        }
    }

    #[test]
    fn the_choice_holds_while_the_interface_language_does() {
        // App.tsx:91 `if (uiLang === lastUiLang.current) return;`
        assert_eq!(texts_lang(Some("fr|en"), Language::En), FR);
        assert_eq!(texts_lang(Some("pli|fr"), Language::Fr), TextsLang::Pali);
        assert_eq!(
            texts_lang(Some(&cookie_value(EN, Language::Vi)), Language::Vi),
            EN
        );
    }

    #[test]
    fn a_new_interface_language_shows_its_texts() {
        // App.tsx:92-93 `lastUiLang.current = uiLang; setTextLangChoice(uiLang);`
        assert_eq!(
            texts_lang(Some("fr|en"), Language::Vi),
            TextsLang::Translated(Language::Vi)
        );
        assert_eq!(texts_lang(Some("pli|en"), Language::Fr), FR);
    }

    #[test]
    fn the_cookie_records_the_choice_and_the_interface() {
        assert_eq!(cookie_value(FR, Language::En), "fr|en");
        assert_eq!(cookie_value(TextsLang::Pali, Language::Vi), "pli|vi");
    }

    #[test]
    fn glosses_follow_the_texts_else_the_interface() {
        // App.tsx:105 `const glossLang = textLang === "pli" ? (uiLang === "pli" ? "en" : uiLang) : textLang;`
        assert_eq!(gloss_lang(FR, Language::En), Language::Fr);
        assert_eq!(gloss_lang(TextsLang::Pali, Language::Vi), Language::Vi);
    }

    #[test]
    fn a_mask_per_language_and_none_in_pali() {
        // App.tsx:277 `graph.nodes.map((n) => n[0] in tr.titles)`; :280 `textLang === "pli" ? null : (masks?.get(textLang) ?? null)`
        let g = fixture();
        let tr = Translations::from_json(
            br#"{"en":{"titles":{"dn1":"a","mn2":"b"},"by":{},"translators":[]},
                 "fr":{"titles":{"mn10":"c"},"by":{},"translators":[]}}"#,
        )
        .expect("translations");
        let m = masks(&g, &tr);
        assert_eq!(m["en"], [true, false, false, true, false]);
        assert_eq!(m["fr"], [false, false, true, false, false]);
        assert_eq!(available(TextsLang::Pali, &m), None);
        assert_eq!(
            available(FR, &m),
            Some(&[false, false, true, false, false][..])
        );
        assert_eq!(available(TextsLang::Translated(Language::Vi), &m), None);
    }

    #[test]
    fn every_language_of_the_committed_translations_has_a_mask() {
        let g =
            crate::map::model::Graph::from_json(&public_file("graph.json")).expect("graph.json");
        let tr =
            Translations::from_json(&public_file("translations.json")).expect("translations.json");
        let m = masks(&g, &tr);
        for t in TextsLang::ALL.into_iter().skip(1) {
            let mask = available(t, &m).unwrap_or_else(|| panic!("{}", t.code()));
            assert_eq!(mask.len(), g.nodes.len());
            assert!(mask.iter().any(|&on| on), "{} has texts", t.code());
        }
    }
}
