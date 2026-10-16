//! What the server decides for a map address's first render and hands to the browser
//! (design v4's OPEN node): the data files' URLs, whether the address opens a text — its
//! panel, rendered by the server — or is a 404, and the texts' counts per language. The
//! browser hydrates from the very same value ([`FirstRender`], a Leptos `SharedValue`), then
//! takes over once its own copy of the data is in.

use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};

use crate::manifest::Manifest;
use crate::map::draw::{Links, Scene};
use palicanon_ui_lib::Language;

use crate::map::model::TextRef;
use crate::map::ui::Texts;
use crate::map::ui::panel::{PanelText, panel_text};
use crate::map::ui::translators::{TranslatorRow, people};
use crate::route::View;

/// What a map address opened on the server.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub(crate) enum FirstOpen {
    /// No text in the address.
    #[default]
    Nothing,
    /// A uid no view has: the 404.
    NotFound,
    /// A text: its panel.
    Text(Box<OpenedText>),
}

/// A text an address opened.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct OpenedText {
    /// The panel, as the server rendered it.
    pub panel: PanelText,
    /// The first view that has the text: the address the head names as canonical.
    pub canonical: View,
    /// Its node in the address's view; `None` when the view lacks it.
    pub index_in_view: Option<usize>,
    /// The node the map highlights: none when the view or the texts' language lacks it.
    pub highlighted: Option<usize>,
}

/// The server's decisions for one map address.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub(crate) struct FirstRender {
    /// The path rendered (`/sim/mn10`): the browser keeps these decisions while it shows it.
    pub path: String,
    /// The data files' URLs.
    pub manifest: Manifest,
    /// What the address opens.
    pub open: FirstOpen,
    /// How many texts each language has a title for (the texts chip's hints).
    pub counts: BTreeMap<String, usize>,
    /// The Acknowledgements page's translators table (with every page: the browser may reach
    /// that page by its own navigation, without a server render of it; under a kilobyte).
    pub credits: Option<Credits>,
}

/// What the Acknowledgements' translators table shows (`Translators.tsx`), read from the
/// data: the Pāli texts' count, then each language's translators and texts.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub(crate) struct Credits {
    /// How many texts the Pāli edition has: the vectorial space's, every Pāli text.
    pub pali_texts: usize,
    /// Each language of `translations.json`: its code, its credits, how many texts it has.
    pub languages: Vec<(String, Vec<String>, usize)>,
}

#[cfg(feature = "ssr")]
impl Credits {
    /// The credits of `translations`, with `pali_texts` Pāli texts.
    pub(crate) fn of(translations: &crate::map::model::Translations, pali_texts: usize) -> Credits {
        Credits {
            pali_texts,
            languages: translations
                .0
                .iter()
                .map(|(code, tr)| (code.clone(), tr.translators.clone(), tr.titles.len()))
                .collect(),
        }
    }
}

impl Credits {
    /// The table's rows after Pāli, in `language`
    /// ([`translator_rows`](crate::map::ui::translators::translator_rows)'s, from these
    /// credits): each interface language they have, in `Language::ALL`'s order.
    pub(crate) fn rows(&self, language: Language) -> Vec<TranslatorRow> {
        Language::ALL
            .into_iter()
            .filter_map(|code| {
                let (_, translators, texts) =
                    self.languages.iter().find(|(c, ..)| c == code.code())?;
                Some(TranslatorRow {
                    language: code.name(),
                    authors: people(translators, language).join(", "),
                    texts: *texts,
                })
            })
            .collect()
    }
}

impl FirstRender {
    /// What the browser runs on when the page holds no first render (a `SharedValue` lost:
    /// the page cut, a serialization failure): the empty decisions, and an error in the
    /// console, not a silently empty map.
    #[cfg_attr(
        feature = "ssr",
        allow(
            dead_code,
            reason = "app.rs's first_render calls it in the browser only"
        )
    )]
    pub(crate) fn not_in_page() -> FirstRender {
        Self::not_in_page_with(|message| leptos::logging::error!("{message}"))
    }

    /// [`FirstRender::not_in_page`], the error given to `log`.
    fn not_in_page_with(log: impl FnOnce(&str)) -> FirstRender {
        log(
            "vizualisation: the page holds no first render (its SharedValue is missing): \
             the map starts empty",
        );
        FirstRender::default()
    }

    /// The opened text, when the address opened one.
    pub(crate) fn opened(&self) -> Option<&OpenedText> {
        match &self.open {
            FirstOpen::Text(text) => Some(text),
            _ => None,
        }
    }
}

/// The panel of node `i` of `texts`' graph when the view shown lacks the text: its reference,
/// summary and place, no neighbours (it has no link there), the trail as given.
pub(crate) fn panel_without_links(
    texts: &Texts<'_>,
    i: usize,
    trail: &[String],
    index: &HashMap<String, usize>,
) -> PanelText {
    let mut scene = Scene::new(texts.graph);
    scene.set_links(Links::default());
    panel_text(texts, &scene, index, i, trail)
}

/// The document title of an open text (`head.ts`'s title, the panel's reference): `MN 10
/// Satipaṭṭhāna: Mindfulness Meditation — Suttas Map`.
pub(crate) fn text_title(reference: &TextRef) -> String {
    let name = [reference.acronym.as_str(), reference.pali.as_str()]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if reference.title.is_empty() || reference.title == reference.pali {
        format!("{name} — Suttas Map")
    } else {
        format!("{name}: {} — Suttas Map", reference.title)
    }
}

/// The summaries of `gloss` and of English the data has (French and English only; another
/// language has none, its texts fall back to English).
#[cfg(feature = "ssr")]
fn blurbs_of(
    data: &crate::data::Data,
    gloss: Language,
) -> (&crate::map::model::Blurbs, &crate::map::model::Blurbs) {
    use crate::map::model::Blurbs;
    static NONE: std::sync::OnceLock<Blurbs> = std::sync::OnceLock::new();
    let [en, fr] = &data.blurbs;
    match gloss {
        Language::En => (en, en),
        Language::Fr => (fr, en),
        Language::Vi => (NONE.get_or_init(Blurbs::default), en),
    }
}

/// What the server asks of an address.
#[cfg(feature = "ssr")]
#[derive(Clone, Debug)]
pub(crate) struct FirstRequest<'a> {
    /// The path rendered.
    pub path: &'a str,
    /// The address's view.
    pub view: View,
    /// The address's text.
    pub uid: Option<&'a str>,
    /// `?th=`.
    pub th: Option<&'a str>,
    /// The interface language.
    pub language: Language,
    /// The texts' language (`viz_textlang`).
    pub texts: crate::map::texts_lang::TextsLang,
    /// The page is light (`pc_theme`); the device's choice is unknown to the server: dark.
    pub light: bool,
}

/// The server's decisions for `request` over the data `served`: `open_text`'s answer, and for
/// a text its panel in the address's view (its neighbours at the address's threshold, of the
/// texts' language), else in its first view without links.
#[cfg(feature = "ssr")]
pub(crate) fn first_render(
    served: &crate::data::Served,
    request: &FirstRequest<'_>,
) -> FirstRender {
    use crate::map::open_text::{Open, highlighted, open_text};
    use crate::map::ui::texts_menu::translated_counts;

    let data = &served.data;
    let mut first = FirstRender {
        path: request.path.to_owned(),
        manifest: (*served.manifest).clone(),
        open: FirstOpen::Nothing,
        counts: translated_counts(&data.translations),
        credits: None,
    };
    let Some(uid) = request.uid else {
        return first;
    };
    let position = |uid: &str, view: View| served.texts.position(uid, view);
    let (index_in_view, canonical) = match open_text(uid, request.view, &View::ALL, &position) {
        Open::NotFound => {
            first.open = FirstOpen::NotFound;
            return first;
        }
        Open::Panel {
            index_in_view,
            canonical_view,
        } => (index_in_view, canonical_view),
    };
    let shown_in = if index_in_view.is_some() {
        request.view
    } else {
        canonical
    };
    let Some(i) = position(uid, shown_in) else {
        return first;
    };
    let (panel, mask) = panel_of(served, request, i, shown_in, index_in_view.is_some());
    first.open = FirstOpen::Text(Box::new(OpenedText {
        panel,
        canonical,
        index_in_view,
        highlighted: highlighted(index_in_view, mask.as_deref()),
    }));
    first
}

/// The panel of node `i` of `shown_in` for `request`, and the texts' language's mask of that
/// view: the address's view has the text (`in_view`) — its neighbours there, at the
/// address's threshold, of the texts' language —, or not — no neighbours.
#[cfg(feature = "ssr")]
fn panel_of(
    served: &crate::data::Served,
    request: &FirstRequest<'_>,
    i: usize,
    shown_in: View,
    in_view: bool,
) -> (PanelText, Option<Vec<bool>>) {
    use crate::map::texts_lang::{available, gloss_lang, masks};
    use crate::map::theme::MapTheme;

    let data = &served.data;
    let graph = data.graph(shown_in);
    let uid = graph.nodes[i].uid.clone();
    let gloss_language = gloss_lang(request.texts, request.language);
    let (blurbs, blurbs_en) = blurbs_of(data, gloss_language);
    let theme = MapTheme::new(request.light, graph);
    let texts = Texts {
        graph,
        theme: &theme,
        gloss: data.translations.0.get(gloss_language.code()),
        blurbs,
        blurbs_en,
    };
    let trail = vec![uid.clone()];
    let index = HashMap::from([(uid, i)]);
    let masks = masks(graph, &data.translations);
    let mask = available(request.texts, &masks).map(<[bool]>::to_vec);
    let panel = if in_view {
        let mut scene = Scene::new(graph);
        if let Some(sim) = &graph.sim {
            let threshold = crate::map::address::threshold(request.th, sim);
            scene.set_links(crate::map::draw::links_at(&data.pairs, sim, threshold));
        }
        scene.available.clone_from(&mask);
        panel_text(&texts, &scene, &index, i, &trail)
    } else {
        panel_without_links(&texts, i, &trail, &index)
    };
    (panel, mask)
}

/// The first render's decisions for the request being rendered: on the server, from the data
/// in context (`Served`), the request's path, query and cookies, and the interface language;
/// empty without data (a page rendered alone, the route list).
#[cfg(feature = "ssr")]
pub(crate) fn of_request() -> FirstRender {
    use leptos::prelude::{GetUntracked, use_context};

    use crate::map::texts_lang::{self, texts_lang};
    use crate::route::Route;

    let Some(served) = use_context::<crate::data::Served>() else {
        return FirstRender::default();
    };
    let Some(parts) = use_context::<axum::http::request::Parts>() else {
        return FirstRender::default();
    };
    let path = parts.uri.path();
    let language = palicanon_ui_lib::language_signal().get_untracked();
    let texts = texts_lang(
        crate::map::cookies::read(texts_lang::COOKIE).as_deref(),
        language,
    );
    let light = palicanon_ui_lib::theme_attribute() == Some("light");
    let (view, uid) = match Route::parse(path) {
        Some(Route::Map { view, uid }) => (view, uid),
        _ => (View::Parallels, None),
    };
    let th = crate::map::address::th_param(parts.uri.query(), view);
    let mut first = first_render(
        &served,
        &FirstRequest {
            path,
            view,
            uid: uid.as_deref(),
            th: th.as_deref(),
            language,
            texts,
            light,
        },
    );
    first.credits = Some(Credits::of(
        &served.data.translations,
        served.data.sim.nodes.len(),
    ));
    first
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_page_without_its_first_render_says_so() {
        let mut logged = Vec::new();
        let first = FirstRender::not_in_page_with(|message| logged.push(message.to_owned()));
        assert_eq!(first, FirstRender::default());
        assert_eq!(logged.len(), 1, "one error: {logged:?}");
        assert!(logged[0].contains("first render"), "{logged:?}");
    }

    /// The committed data as the server serves it.
    #[cfg(feature = "ssr")]
    fn served() -> crate::data::Served {
        use std::sync::{Arc, OnceLock};
        static SERVED: OnceLock<crate::data::Served> = OnceLock::new();
        SERVED
            .get_or_init(|| {
                let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../public");
                let data = crate::data::Data::load(&dir).expect("the committed data");
                crate::data::Served::new(Arc::new(data))
            })
            .clone()
    }

    /// The first render of `/mn10` with its glosses in `language`.
    #[cfg(feature = "ssr")]
    fn mn10_in(language: Language) -> PanelText {
        let first = first_render(
            &served(),
            &FirstRequest {
                path: "/mn10",
                view: View::Parallels,
                uid: Some("mn10"),
                th: None,
                language,
                texts: crate::map::texts_lang::TextsLang::Pali,
                light: false,
            },
        );
        first.opened().expect("mn10 opens").panel.clone()
    }

    /// Vietnamese has no blurbs file: a text's summary is SuttaCentral's English, as in
    /// English; French has its own.
    #[cfg(feature = "ssr")]
    #[test]
    fn a_language_without_blurbs_shows_the_english_one() {
        let served = served();
        let en = served.data.blurbs[0].get("mn10").expect("mn10 has a blurb");
        assert_eq!(mn10_in(Language::En).item.blurb.as_deref(), Some(en));
        assert_eq!(mn10_in(Language::Vi).item.blurb.as_deref(), Some(en), "vi");
        let fr = served.data.blurbs[1].get("mn10").expect("mn10 in French");
        assert_eq!(mn10_in(Language::Fr).item.blurb.as_deref(), Some(fr));
    }
}
