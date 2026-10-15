//! The translators table of the Acknowledgements page (`src/components/Translators.tsx`):
//! everyone whose work the map shows, one row per language, the names alike and in
//! alphabetical order, with how many texts are theirs — read from `translations.json`, so it
//! never drifts from the data. Mounting it on the page is the pages' (`crate::pages`).

use leptos::prelude::*;
use leptos_i18n::td_string;
use palicanon_ui_lib::{Language, language_signal};
use unicode_normalization::UnicodeNormalization;

use crate::catalogue::strings;
use crate::map::model::Translations;
use crate::map::ui::number;

/// A header cell's classes (Translators.tsx line 5).
const TH: &str = "py-2 pr-4 text-left text-xs font-semibold uppercase tracking-wide text-subtle";
/// A cell's classes (Translators.tsx line 6).
const TD: &str = "py-2 pr-4 align-top text-[14px] leading-6 text-ink break-words";

/// One language of the table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TranslatorRow {
    /// The language's own name.
    pub language: &'static str,
    /// The translators, each once, alphabetically, comma-separated.
    pub authors: String,
    /// How many texts the language has.
    pub texts: usize,
}

/// One name per person, sorted for `language` (Translators.tsx lines 22–24): a credit naming
/// several (`E.B. Cowell, W.H.D. Rouse`) is split at its commas, but a degree stays with its
/// name (`…, D. Litt`).
pub fn people(credits: &[String], language: Language) -> Vec<String> {
    let _ = language; // The three languages sort Latin names alike: diacritics, then case.
    let mut out: Vec<String> = Vec::new();
    for credit in credits {
        for name in split_credit(credit) {
            if !name.is_empty() && !out.iter().any(|o| o == name) {
                out.push(name.to_owned());
            }
        }
    }
    out.sort_by(|a, b| {
        collation_key(a)
            .cmp(&collation_key(b))
            .then_with(|| a.cmp(b))
    });
    out
}

/// `credit` split at each comma not followed by `D. Litt`, each part trimmed.
fn split_credit(credit: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    for (at, _) in credit.match_indices(',') {
        if !credit[at + 1..].trim_start().starts_with("D. Litt") {
            parts.push(credit[start..at].trim());
            start = at + 1;
        }
    }
    parts.push(credit[start..].trim());
    parts
}

/// `name` as `localeCompare` orders it, first: without diacritics, lower-cased.
fn collation_key(name: &str) -> String {
    name.nfd()
        .filter(|c| !('\u{300}'..='\u{36f}').contains(c))
        .flat_map(char::to_lowercase)
        .collect()
}

/// The table's rows after Pāli: each interface language `translations` has, in
/// `Language::ALL`'s order (Translators.tsx lines 45–56).
pub fn translator_rows(translations: &Translations, language: Language) -> Vec<TranslatorRow> {
    Language::ALL
        .into_iter()
        .filter_map(|code| {
            let tr = translations.0.get(code.code())?;
            Some(TranslatorRow {
                language: code.name(),
                authors: people(&tr.translators, language).join(", "),
                texts: tr.titles.len(),
            })
        })
        .collect()
}

/// The table: Pāli (its edition, `pali_texts` texts), then [`translator_rows`].
#[component]
pub fn TranslatorsTable(
    /// The languages' rows.
    rows: Vec<TranslatorRow>,
    /// How many texts the Pāli edition has.
    pali_texts: usize,
) -> impl IntoView {
    let t = strings();
    let num = move |n: usize| move || number(n, language_signal().get());
    let rows = rows
        .into_iter()
        .map(|row| {
            view! {
                <tr>
                    <td class=TD>{row.language}</td>
                    <td class=TD>{row.authors}</td>
                    <td class=format!("{TD} tabular-nums")>{num(row.texts)}</td>
                </tr>
            }
        })
        .collect_view();
    view! {
        <section>
            <h2 class="mt-10 font-serif text-xl font-semibold text-heading">
                {move || td_string!(t.locale(), credits_table_title)}
            </h2>
            <p class="mt-3 max-w-[66ch] text-[15px] leading-7 text-ink">
                {move || td_string!(t.locale(), credits_table_intro)}
            </p>
            <div class="mt-3 overflow-x-auto">
                <table class="w-full max-w-[66ch] border-collapse">
                    <thead>
                        <tr class="border-b border-line">
                            <th class=TH>{move || td_string!(t.locale(), credits_col_lang)}</th>
                            <th class=TH>{move || td_string!(t.locale(), credits_col_authors)}</th>
                            <th class=TH>{move || td_string!(t.locale(), credits_col_texts)}</th>
                        </tr>
                    </thead>
                    <tbody class="divide-y divide-line">
                        <tr>
                            <td class=TD>"Pāli"</td>
                            <td class=TD>{move || td_string!(t.locale(), credits_pali_edition)}</td>
                            <td class=format!("{TD} tabular-nums")>{num(pali_texts)}</td>
                        </tr>
                        {rows}
                    </tbody>
                </table>
            </div>
        </section>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn credits_are_split_into_people_each_once_sorted() {
        // Translators.tsx:23 `c.split(/,(?!\s*D\. Litt)\s*/)`, `new Set`, `localeCompare`
        let credits = names(&[
            "Bhikkhu Sujato",
            "E.B. Cowell, W.H.D. Rouse",
            "T.W. Rhys Davids, D. Litt",
            "Bhikkhu Sujato",
            "Ánanda",
            " ",
        ]);
        assert_eq!(
            people(&credits, Language::En),
            [
                "Ánanda",
                "Bhikkhu Sujato",
                "E.B. Cowell",
                "T.W. Rhys Davids, D. Litt",
                "W.H.D. Rouse",
            ]
        );
    }

    #[test]
    fn a_row_per_language_the_translations_have() {
        // Translators.tsx:47-55 `{name}`, `sort(l.translators).join(", ")`, `Object.keys(l.titles).length`
        let tr = Translations::from_json(
            r#"{"vi":{"titles":{"mn1":"a"},"by":{},"translators":["Thích Minh Châu"]},
                "fr":{"titles":{"mn1":"a","mn2":"b"},"by":{},"translators":["Môhan Wijayaratna","Christian Maës"]}}"#
                .as_bytes(),
        )
        .expect("translations");
        assert_eq!(
            translator_rows(&tr, Language::Fr),
            [
                TranslatorRow {
                    language: "Français",
                    authors: "Christian Maës, Môhan Wijayaratna".to_owned(),
                    texts: 2,
                },
                TranslatorRow {
                    language: "Tiếng Việt",
                    authors: "Thích Minh Châu".to_owned(),
                    texts: 1,
                },
            ]
        );
    }

    #[cfg(feature = "ssr")]
    #[test]
    fn the_table_is_labelled_in_each_language() {
        use crate::map::ui::test_support::{plain, render};

        // Translators.tsx:29-43 title, intro, the columns, the Pāli edition
        for (language, words) in [
            (
                Language::En,
                [
                    "Texts and translators",
                    "Language",
                    "Edition and translators",
                    "7,286",
                ],
            ),
            (
                Language::Fr,
                [
                    "Textes et traducteurs",
                    "Langue",
                    "Édition et traducteurs",
                    "7\u{202f}286",
                ],
            ),
            (
                Language::Vi,
                [
                    "Kinh văn và dịch giả",
                    "Ngôn ngữ",
                    "Ấn bản và dịch giả",
                    "7.286",
                ],
            ),
        ] {
            let rows = vec![TranslatorRow {
                language: "Français",
                authors: "Christian Maës, Môhan Wijayaratna".to_owned(),
                texts: 312,
            }];
            let html = plain(&render(language, || {
                view! { <TranslatorsTable rows=rows pali_texts=7286 /> }
            }));
            for word in words {
                assert!(html.contains(word), "{language:?} {word}: {html}");
            }
            assert!(
                html.contains("Mahāsaṅgīti Tipiṭaka, Dhamma Society"),
                "{html}"
            );
            assert!(html.contains("Christian Maës, Môhan Wijayaratna"), "{html}");
            assert_eq!(html.matches("<tr").count(), 3, "head, Pāli, French: {html}");
        }
    }
}
