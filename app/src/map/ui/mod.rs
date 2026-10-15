//! The map's UI around the canvas, ported from the React page (`src/App.tsx`'s map screen and
//! `src/components/{Legend,SimControls,SearchBar,TextPanel,Section,Credits,SuttaRef,TextCard,
//! Menu}.tsx`) to Leptos components. They know no engine and hold no global state: each takes
//! what it shows as props or signals and reports what the reader does through callbacks; the
//! map's wiring (task 006, M5) owns the state, the URL and the cookies.
//!
//! - [`screen`]: [`MapScreen`], the layout — search, the legend (an aside on a desktop, a
//!   drawer on a phone), the canvas (a slot), the chips over it, the text panel, the tooltip.
//! - [`legend`]: the collections (colour, hide) and the link types (show, hide), with counts.
//! - [`chip_menu`]: a chip opening a list of choices, each with a hint (`Menu.tsx`).
//! - `view_menu` (crate-wide: it takes the crate's `View`), [`texts_menu`]: the views chip
//!   and the texts' language chip.
//! - [`sim_controls`]: the vectorial view's threshold and "hide texts without a neighbour".
//! - [`search`]: the search box, its results and its keys.
//! - [`panel`]: the open text — trail, reference, summary, place, neighbours by link type,
//!   credits.
//! - [`section`]: a folding section of the panel, closed until opened, remembered.
//! - [`text_ref`](mod@text_ref): a text's reference as rows, cards, chips and links show it.
//! - [`tooltip`]: the text under the pointer.
//! - [`translators`]: the Acknowledgements page's table of translators.
//!
//! The data each component shows is built by pure functions here ([`Texts::item`],
//! `panel::panel_text`, `legend::collection_rows`…), tested without a DOM.

pub mod chip_menu;
pub mod legend;
pub mod panel;
pub mod screen;
pub mod search;
pub mod section;
pub mod sim_controls;
pub mod text_ref;
pub mod texts_menu;
pub mod tooltip;
pub mod translators;
// The map's wiring (task 006, M5) mounts `ViewMenu` in `MapScreen`'s chips; until then only
// its tests use it. Crate-wide: it takes the crate's own `View`.
#[allow(dead_code)]
pub(crate) mod view_menu;

use leptos_i18n::td_string;
use palicanon_ui_lib::Language;

use crate::catalogue::Locale;
use crate::map::model::{Blurbs, Graph, LangTranslations, TextRef, blurb_of, text_ref};
use crate::map::theme::{MapTheme, NO_GROUP_COLOR};

pub use chip_menu::{ChipMenu, ChipOption};
pub use legend::{CollectionRow, Legend, TypeRow};
pub use panel::{PanelText, TextPanel};
pub use screen::MapScreen;
pub use search::SearchBox;
pub use section::Section;
pub use sim_controls::SimControls;
pub use texts_menu::TextsMenu;
pub use tooltip::{Hover, Tooltip};

/// A floating card: the search results, the tooltip (App.tsx line 19, `card`).
pub const CARD: &str =
    "rounded-xl border border-card-edge bg-card shadow-[0_16px_40px_-16px_rgb(0_0_0/0.7)]";

/// One text as a row, a card or the tooltip shows it.
#[derive(Clone, Debug, PartialEq)]
pub struct TextItem {
    /// Its index in the view's graph.
    pub index: usize,
    /// Its uid.
    pub uid: String,
    /// Acronym, Pāli title, title in the glosses' language.
    pub reference: TextRef,
    /// Its collection's colour, `#rrggbb`.
    pub color: String,
    /// SuttaCentral's summary, in the glosses' language else English.
    pub blurb: Option<String>,
    /// Its similarity to the open text (vectorial views).
    pub score: Option<f32>,
}

/// What a text's reference, colour and summary are read from: the view's graph and colours,
/// and the glosses' language (its translations and summaries, English summaries as fallback).
#[derive(Clone, Copy, Debug)]
pub struct Texts<'a> {
    /// The view's graph.
    pub graph: &'a Graph,
    /// The map's colours.
    pub theme: &'a MapTheme,
    /// The translations of the glosses' language (`None`: SuttaCentral's English).
    pub gloss: Option<&'a LangTranslations>,
    /// The summaries in the glosses' language.
    pub blurbs: &'a Blurbs,
    /// The summaries in English.
    pub blurbs_en: &'a Blurbs,
}

impl Texts<'_> {
    /// Node `i` as a [`TextItem`], without a score (App.tsx lines 295–305, `refOf`,
    /// `blurbOf`; the colour is its collection's, `theme.groups[node[5]]`).
    pub fn item(&self, i: usize) -> TextItem {
        let node = &self.graph.nodes[i];
        TextItem {
            index: i,
            uid: node.uid.clone(),
            reference: text_ref(self.graph, i, self.gloss),
            color: self
                .theme
                .groups
                .get(node.group as usize)
                .map_or(NO_GROUP_COLOR, String::as_str)
                .to_owned(),
            blurb: blurb_of(self.blurbs, self.blurbs_en, &node.uid).map(str::to_owned),
            score: None,
        }
    }
}

/// `n` with its thousands grouped as the language writes them (`toLocaleString`): `7,286`,
/// `7 286` (a narrow no-break space), `7.286`.
pub fn number(n: usize, language: Language) -> String {
    let separator = match language {
        Language::En => ',',
        Language::Fr => '\u{202f}',
        Language::Vi => '.',
    };
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3 * 3);
    for (k, digit) in digits.chars().enumerate() {
        if k > 0 && (digits.len() - k).is_multiple_of(3) {
            out.push(separator);
        }
        out.push(digit);
    }
    out
}

/// `names` joined as a sentence lists them (`Intl.ListFormat`, conjunction): `A, B, and C`,
/// `A, B et C`, `A, B và C`; two names take no comma.
pub fn conjunction(names: &[String], language: Language) -> String {
    let and = match language {
        Language::En => "and",
        Language::Fr => "et",
        Language::Vi => "và",
    };
    match names {
        [] => String::new(),
        [one] => one.clone(),
        [first @ .., last] => {
            // English's serial comma before "and" once there are three names or more.
            let comma = if language == Language::En && first.len() > 1 {
                ","
            } else {
                ""
            };
            format!("{}{comma} {and} {last}", first.join(", "))
        }
    }
}

/// `"3 links"`, `"1 link"`: the count, then the word, singular when the count is 1
/// (`tooltip_link` / `tooltip_links`, app/STRINGS.md's plurals).
pub(crate) fn links_label(count: usize, locale: Locale) -> String {
    let word = if count == 1 {
        td_string!(locale, tooltip_link)
    } else {
        td_string!(locale, tooltip_links)
    };
    format!("{count} {word}")
}

/// A link type's name in `locale` (`types_<name>`); an unknown type keeps its own name.
pub(crate) fn type_label(name: &str, locale: Locale) -> &str {
    match name {
        "full" => td_string!(locale, types_full),
        "sections" => td_string!(locale, types_sections),
        "resembling" => td_string!(locale, types_resembling),
        "mentions" => td_string!(locale, types_mentions),
        "retells" => td_string!(locale, types_retells),
        "similar" => td_string!(locale, types_similar),
        _ => name,
    }
}

/// The catalogue's `key` in `locale`, its `{{name}}` placeholders replaced by `values`.
///
/// `td_string!` takes no arguments in this crate: `build.rs` generates the catalogue without
/// `leptos_i18n`'s `interpolate_display` option (learn-pali's `build.rs` enables it). Until it
/// does, the three sentences with a placeholder the map shows (`credits_translated`,
/// `sim_stats`, `texts_translated`) read their template from the catalogue files themselves,
/// the very files `build.rs` checks; a missing key reads as the key.
pub(crate) fn interpolate(locale: Locale, key: &str, values: &[(&str, &str)]) -> String {
    let file = match locale {
        Locale::en => include_str!("../../../locales/en.json"),
        Locale::fr => include_str!("../../../locales/fr.json"),
        Locale::vi => include_str!("../../../locales/vi.json"),
    };
    let mut text = template(file, key).unwrap_or_else(|| key.to_owned());
    for (name, value) in values {
        text = text
            .replace(&format!("{{{{{name}}}}}"), value)
            .replace(&format!("{{{{ {name} }}}}"), value);
    }
    text
}

/// The string value of `key` in a flat JSON object as the catalogue files write it
/// (`"key": "value"`), its escapes (`\"`, `\\`, `\n`, `\/`) read.
fn template(file: &str, key: &str) -> Option<String> {
    let start = file.find(&format!("\"{key}\": \""))? + key.len() + 5;
    let mut out = String::new();
    let mut chars = file[start..].chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => return Some(out),
            '\\' => match chars.next()? {
                'n' => out.push('\n'),
                other => out.push(other),
            },
            _ => out.push(c),
        }
    }
    None
}

/// `s` as a URL path segment (`encodeURIComponent`): every byte but the unreserved ones
/// (`A–Z a–z 0–9 - _ . ! ~ * ' ( )`) percent-encoded.
pub fn encode_component(s: &str) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(s.len());
    for byte in s.bytes() {
        if byte.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&byte) {
            out.push(char::from(byte));
        } else {
            let _ = write!(out, "%{byte:02X}");
        }
    }
    out
}

/// What the components' tests share: rendering in a language, reading the markup, and the
/// map's fixture with translations and summaries.
#[cfg(test)]
pub(crate) mod test_support {
    use palicanon_ui_lib::Language;

    use crate::map::model::{Blurbs, Graph, Translations};
    use crate::map::theme::MapTheme;

    use super::Texts;

    /// `view` rendered by the server with the interface in `language`.
    #[cfg(feature = "ssr")]
    pub(crate) fn render<V: leptos::prelude::IntoView + 'static>(
        language: Language,
        view: impl FnOnce() -> V,
    ) -> String {
        use leptos::prelude::*;
        Owner::new().with(|| {
            palicanon_ui_lib::provide_language().set(language);
            view().to_html()
        })
    }

    /// `html` without the hydration markers (`<!--…-->`).
    pub(crate) fn plain(html: &str) -> String {
        let mut out = html.to_owned();
        while let Some(start) = out.find("<!--") {
            let end = start + out[start..].find("-->").expect("a comment's end") + 3;
            out.replace_range(start..end, "");
        }
        out
    }

    /// The opening tag that contains `needle`, from its `<` to its `>`.
    pub(crate) fn tag_around(html: &str, needle: &str) -> String {
        let at = html
            .find(needle)
            .unwrap_or_else(|| panic!("{needle}: {html}"));
        let start = html[..=at].rfind('<').expect(html);
        let end = at + html[at..].find('>').expect(html);
        html[start..=end].to_owned()
    }

    /// The classes of a tag, one per item.
    pub(crate) fn classes(tag: &str) -> Vec<&str> {
        let Some(at) = tag.find("class=\"") else {
            return Vec::new();
        };
        let rest = &tag[at + 7..];
        rest[..rest.find('"').expect(tag)]
            .split_whitespace()
            .collect()
    }

    /// The map's fixture with French titles and translators, and summaries.
    pub(crate) struct Fixture {
        /// `crate::map::fixture()`.
        pub(crate) graph: Graph,
        /// Its dark colours.
        pub(crate) theme: MapTheme,
        /// French titles for mn1 and mn2, two translators.
        pub(crate) translations: Translations,
        /// French summaries (mn1).
        pub(crate) blurbs_fr: Blurbs,
        /// English summaries (mn1, mn10).
        pub(crate) blurbs_en: Blurbs,
    }

    impl Fixture {
        /// The fixture.
        pub(crate) fn new() -> Fixture {
            let graph = crate::map::fixture();
            let theme = MapTheme::new(false, &graph);
            let translations = Translations::from_json(
                r#"{"fr":{"titles":{"mn1":"La racine de toutes choses","mn2":"Les souillures"},
                          "by":{"mn1":0,"mn2":1},"translators":["Môhan Wijayaratna","Christian Maës"]}}"#
                    .as_bytes(),
            )
            .expect("translations");
            Fixture {
                graph,
                theme,
                translations,
                blurbs_fr: Blurbs::from_json(br#"{"mn1":"Le Bouddha enseigne la racine."}"#)
                    .expect("blurbs"),
                blurbs_en: Blurbs::from_json(
                    br#"{"mn1":"The Buddha teaches the root.","mn10":"The four kinds of mindfulness."}"#,
                )
                .expect("blurbs"),
            }
        }

        /// The texts with glosses in `language` (English: SuttaCentral's titles).
        pub(crate) fn texts(&self, language: Language) -> Texts<'_> {
            let gloss = self.translations.0.get(language.code());
            Texts {
                graph: &self.graph,
                theme: &self.theme,
                gloss,
                blurbs: if language == Language::Fr {
                    &self.blurbs_fr
                } else {
                    &self.blurbs_en
                },
                blurbs_en: &self.blurbs_en,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use palicanon_ui_lib::Language;

    use super::test_support::Fixture;
    use super::*;

    #[test]
    fn a_text_item_has_its_reference_colour_and_summary() {
        // App.tsx:298 `title: translations?.[glossLang]?.titles[n[0]] || n[2]`;
        // App.tsx:304 `return blurbs.lang[uid] ?? blurbs.en[uid];`
        let f = Fixture::new();
        let mn1 = f.texts(Language::Fr).item(1);
        assert_eq!(mn1.uid, "mn1");
        assert_eq!(mn1.reference.acronym, "MN 1");
        assert_eq!(mn1.reference.title, "La racine de toutes choses");
        assert_eq!(mn1.color, "#ff6f8e");
        assert_eq!(mn1.blurb.as_deref(), Some("Le Bouddha enseigne la racine."));
        let mn10 = f.texts(Language::Fr).item(2);
        assert_eq!(mn10.reference.title, "Mindfulness Meditation");
        assert_eq!(
            mn10.blurb.as_deref(),
            Some("The four kinds of mindfulness."),
            "English when the language has none"
        );
        assert_eq!(f.texts(Language::En).item(0).blurb, None);
        assert_eq!(mn1.score, None);
    }

    #[test]
    fn numbers_are_grouped_per_language() {
        // Legend.tsx:23 `n.toLocaleString(i18n.language)`
        assert_eq!(number(7286, Language::En), "7,286");
        assert_eq!(number(7286, Language::Fr), "7\u{202f}286");
        assert_eq!(number(7286, Language::Vi), "7.286");
        assert_eq!(number(1_234_567, Language::En), "1,234,567");
        assert_eq!(number(999, Language::Fr), "999");
        assert_eq!(number(0, Language::Vi), "0");
    }

    #[test]
    fn names_are_listed_as_a_sentence() {
        // Credits.tsx:9 `new Intl.ListFormat(i18n.language, { type: "conjunction" })`
        let names = |list: &[&str]| list.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
        assert_eq!(conjunction(&names(&["A"]), Language::En), "A");
        assert_eq!(conjunction(&names(&["A", "B"]), Language::En), "A and B");
        assert_eq!(
            conjunction(&names(&["A", "B", "C"]), Language::En),
            "A, B, and C"
        );
        assert_eq!(
            conjunction(&names(&["A", "B", "C"]), Language::Fr),
            "A, B et C"
        );
        assert_eq!(conjunction(&names(&["A", "B"]), Language::Vi), "A và B");
        assert_eq!(conjunction(&[], Language::En), "");
    }

    #[test]
    fn the_link_count_puts_the_number_before_the_word() {
        // App.tsx:564 `t("tooltip.links", { count })`, STRINGS.md: `count == 1` → singular
        let english = Locale::from(Language::En);
        assert_eq!(links_label(1, english), "1 link");
        assert_eq!(links_label(0, english), "0 links");
        assert_eq!(links_label(3, english), "3 links");
        assert_eq!(links_label(1, Locale::from(Language::Fr)), "1 lien");
        assert_eq!(links_label(2, Locale::from(Language::Fr)), "2 liens");
        assert_eq!(links_label(2, Locale::from(Language::Vi)), "2 liên kết");
    }

    #[test]
    fn placeholders_are_filled_from_the_catalogue() {
        // Credits.tsx:15 `t("credits.translated", { names })`
        assert_eq!(
            interpolate(
                Locale::from(Language::Fr),
                "credits_translated",
                &[("names", "A et B")]
            ),
            "Traduit par A et B."
        );
        assert_eq!(
            interpolate(
                Locale::from(Language::En),
                "sim_stats",
                &[("links", "1,234"), ("alone", "7")]
            ),
            "1,234 links · 7 texts without a neighbour"
        );
        assert_eq!(
            interpolate(Locale::from(Language::En), "no_such_key", &[]),
            "no_such_key"
        );
        assert_eq!(
            template(r#"{"a": "say \"hi\"\\n"}"#, "a").as_deref(),
            Some("say \"hi\"\\n")
        );
    }

    #[test]
    fn link_types_are_named_in_the_language() {
        // Legend.tsx:72 `t(`types.${type}`)`
        assert_eq!(
            type_label("full", Locale::from(Language::En)),
            "Full parallel"
        );
        assert_eq!(
            type_label("similar", Locale::from(Language::Fr)),
            "Similarité"
        );
        assert_eq!(type_label("unknown", Locale::from(Language::En)), "unknown");
    }

    #[test]
    fn a_uid_is_encoded_as_a_path_segment() {
        // TextPanel.tsx:73 `encodeURIComponent(uid)`
        assert_eq!(encode_component("mn10"), "mn10");
        assert_eq!(encode_component("sn56.11"), "sn56.11");
        assert_eq!(encode_component("dn1 a/b"), "dn1%20a%2Fb");
        assert_eq!(encode_component("ā"), "%C4%81");
    }
}
