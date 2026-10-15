//! The texts' language chip (App.tsx lines 389–404): Pāli, the whole canon, or a language
//! the texts are translated into, with how many it has.

use std::collections::BTreeMap;

use leptos::prelude::*;
use leptos_i18n::td_string;

use crate::catalogue::{Locale, strings};
use crate::map::model::Translations;
use crate::map::texts_lang::TextsLang;
use crate::map::ui::{ChipMenu, ChipOption, interpolate};

/// How many texts each language of `translations.json` has a title for, by code (App.tsx
/// line 400, `Object.keys(translations?.[value]?.titles ?? {}).length`).
pub fn translated_counts(translations: &Translations) -> BTreeMap<String, usize> {
    translations
        .0
        .iter()
        .map(|(code, tr)| (code.clone(), tr.titles.len()))
        .collect()
}

/// The chip's options, in `TextsLang::ALL`'s order: Pāli "the whole canon", each language
/// "N texts translated" (0 for a language `counts` lacks).
pub(crate) fn texts_options(counts: &BTreeMap<String, usize>, locale: Locale) -> Vec<ChipOption> {
    TextsLang::ALL
        .into_iter()
        .map(|texts| {
            let hint = match texts {
                TextsLang::Pali => td_string!(locale, texts_all).to_owned(),
                TextsLang::Translated(language) => {
                    let count = counts.get(language.code()).copied().unwrap_or(0);
                    interpolate(locale, "texts_translated", &[("count", &count.to_string())])
                }
            };
            ChipOption {
                label: texts.label().to_owned(),
                hint: Some(hint),
            }
        })
        .collect()
}

/// The texts' language chip: shows `texts`, reports the choice.
#[component]
pub fn TextsMenu(
    /// The texts' language.
    #[prop(into)]
    texts: Signal<TextsLang>,
    /// [`translated_counts`].
    #[prop(into)]
    counts: Signal<BTreeMap<String, usize>>,
    /// A language was chosen.
    on_select: Callback<TextsLang>,
) -> impl IntoView {
    let t = strings();
    let selected = Signal::derive(move || {
        let current = texts.get();
        TextsLang::ALL
            .iter()
            .position(|v| *v == current)
            .unwrap_or(0)
    });
    view! {
        <ChipMenu
            id="viz-texts"
            name=Signal::derive(move || td_string!(t.locale(), texts_name).to_owned())
            options=Signal::derive(move || counts.with(|counts| texts_options(counts, t.locale())))
            selected=selected
            on_select=Callback::new(move |i: usize| {
                if let Some(texts) = TextsLang::ALL.get(i) {
                    on_select.run(*texts);
                }
            })
        />
    }
}

#[cfg(test)]
mod tests {
    use palicanon_ui_lib::Language;

    use super::*;

    fn counts() -> BTreeMap<String, usize> {
        [("en".to_owned(), 4210), ("fr".to_owned(), 312)].into()
    }

    #[test]
    fn languages_are_counted_from_the_translations() {
        let tr = Translations::from_json(
            br#"{"fr":{"titles":{"mn1":"a","mn2":"b"},"by":{},"translators":[]},"vi":{"titles":{},"by":{},"translators":[]}}"#,
        )
        .expect("translations");
        assert_eq!(
            translated_counts(&tr),
            BTreeMap::from([("fr".to_owned(), 2), ("vi".to_owned(), 0)])
        );
    }

    #[test]
    fn pali_is_the_whole_canon_a_language_its_count() {
        // App.tsx:398-400 `hint: value === "pli" ? t("texts.all") : t("texts.translated", { count })`
        let options = texts_options(&counts(), Locale::from(Language::En));
        let shown: Vec<(&str, Option<&str>)> = options
            .iter()
            .map(|o| (o.label.as_str(), o.hint.as_deref()))
            .collect();
        assert_eq!(
            shown,
            [
                ("Pāli", Some("The whole canon")),
                ("English", Some("4210 texts translated")),
                ("Français", Some("312 texts translated")),
                ("Tiếng Việt", Some("0 texts translated")),
            ]
        );
    }

    #[cfg(feature = "ssr")]
    #[test]
    fn the_chip_names_the_texts_in_each_language() {
        use crate::map::ui::test_support::{plain, render, tag_around};

        // App.tsx:392-393 `name={t("texts.name")}`, `label={TEXT_LANGS.find(...).label}`
        for (language, name, hint) in [
            (Language::En, "Texts", "312 texts translated"),
            (Language::Fr, "Textes", "312 textes traduits"),
            (Language::Vi, "Văn bản", "312 bài kinh đã dịch"),
        ] {
            let html = plain(&render(language, || {
                view! {
                    <TextsMenu
                        texts=TextsLang::Translated(Language::Fr)
                        counts=counts()
                        on_select=Callback::new(|_| {})
                    />
                }
            }));
            let chip = tag_around(&html, "aria-haspopup");
            assert!(chip.contains(&format!(r#"aria-label="{name}""#)), "{chip}");
            assert!(
                html.contains("Français") && html.contains(hint),
                "{language:?}: {html}"
            );
        }
    }
}
