//! The views chip (App.tsx lines 502–515): SuttaCentral's parallels, the vectorial space and
//! its test views, each with its description as the hint.

use leptos::prelude::*;
use leptos_i18n::td_string;

use crate::catalogue::{Locale, strings};
use crate::map::ui::{ChipMenu, ChipOption};
use crate::route::View;

/// The description of `view` in `locale` (`views_<name>`).
pub(crate) fn view_hint(view: View, locale: Locale) -> &'static str {
    match view {
        View::Parallels => td_string!(locale, views_liens),
        View::Sim => td_string!(locale, views_sim),
        View::Pca => td_string!(locale, views_pca),
        View::Umap => td_string!(locale, views_umap),
        View::Tsne => td_string!(locale, views_tsne),
    }
}

/// The chip's options: every view in `View::ALL`'s order, its name and description.
pub(crate) fn view_options(locale: Locale) -> Vec<ChipOption> {
    View::ALL
        .into_iter()
        .map(|view| ChipOption {
            label: view.label(locale).to_owned(),
            hint: Some(view_hint(view, locale).to_owned()),
        })
        .collect()
}

/// The views chip: shows `view`, reports the one chosen.
#[component]
pub(crate) fn ViewMenu(
    /// The view shown.
    #[prop(into)]
    view: Signal<View>,
    /// A view was chosen.
    on_select: Callback<View>,
) -> impl IntoView {
    let t = strings();
    let selected = Signal::derive(move || {
        let current = view.get();
        View::ALL.iter().position(|v| *v == current).unwrap_or(0)
    });
    view! {
        <ChipMenu
            id="viz-views"
            name=Signal::derive(move || td_string!(t.locale(), views_name).to_owned())
            options=Signal::derive(move || view_options(t.locale()))
            selected=selected
            on_select=Callback::new(move |i: usize| {
                if let Some(view) = View::ALL.get(i) {
                    on_select.run(*view);
                }
            })
        />
    }
}

#[cfg(test)]
mod tests {
    use palicanon_ui_lib::Language;

    use super::*;

    #[test]
    fn every_view_is_offered_with_its_description() {
        // App.tsx:506 `MAPS.map(({ value }) => ({ value, label: t(`views.label_${value}`), hint: t(`views.${value}`) }))`
        let options = view_options(Locale::from(Language::En));
        assert_eq!(options.len(), View::ALL.len());
        assert_eq!(options[0].label, "SuttaCentral");
        assert_eq!(
            options[0].hint.as_deref(),
            Some("SuttaCentral parallels: positions computed from the links")
        );
        assert_eq!(options[1].label, "Vectorial space");
        assert_eq!(
            view_hint(View::Sim, Locale::from(Language::Fr)),
            "Espace vectoriel pāli : textes reliés quand leur similarité dépasse le seuil"
        );
        assert!(
            view_options(Locale::from(Language::Vi))
                .iter()
                .all(|o| o.hint.as_ref().is_some_and(|h| !h.is_empty()))
        );
    }

    #[cfg(feature = "ssr")]
    #[test]
    fn the_chip_names_the_view_in_each_language() {
        use crate::map::ui::test_support::{plain, render, tag_around};

        // App.tsx:503-504 `name={t("views.name")}`, `label={t(`views.label_${mapName}`)}`
        for (language, name, label) in [
            (Language::En, "View", "Vectorial space"),
            (Language::Fr, "Vue", "Espace vectoriel"),
            (Language::Vi, "Chế độ xem", "Không gian vectơ"),
        ] {
            let html = plain(&render(language, || {
                view! { <ViewMenu view=View::Sim on_select=Callback::new(|_| {}) /> }
            }));
            let chip = tag_around(&html, "aria-haspopup");
            assert!(chip.contains(&format!(r#"aria-label="{name}""#)), "{chip}");
            assert!(html.contains(label), "{language:?}: {html}");
            assert_eq!(html.matches(r#"role="option""#).count(), View::ALL.len());
        }
    }
}
