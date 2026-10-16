//! The text panel (`src/components/TextPanel.tsx`, `Credits.tsx`): a header bar — close, then
//! the texts visited —, the open text's reference, summary, place and "Read on
//! SuttaCentral", its neighbours by link type in folding sections, and the credits.
//!
//! What it shows is [`PanelText`], built by [`panel_text`] from the view's data: the same on
//! the server (the panel of `/:view/:uid`) and in the browser.

use std::collections::HashMap;
use std::hash::BuildHasher;
use std::ops::Range;

use leptos::prelude::*;
use leptos_i18n::td_string;
use palicanon_ui_lib::{Icon, IconName, language_signal};
use serde::{Deserialize, Serialize};

use crate::catalogue::strings;
use crate::map::draw::Scene;
use crate::map::model::{panel_translators, place};
use crate::map::ui::section::Section;
use crate::map::ui::text_ref::{ReadLink, RefChip, RefVariant, SuttaRef, TextCard};
use crate::map::ui::{TextItem, Texts, conjunction, encode_component, type_label};

/// How many texts of the trail the header shows, the last ones (TextPanel.tsx line 53,
/// `trail.slice(-6)`).
pub const TRAIL_SHOWN: usize = 6;

/// One text of the trail: its uid and the acronym shown.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrailStep {
    /// The uid.
    pub uid: String,
    /// Its acronym in this view, else the uid (App.tsx lines 554–557).
    pub label: String,
}

/// The neighbours by one link type.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NeighbourSection {
    /// The link type's name (`full`, `similar`…): the section's title and its id.
    pub type_name: String,
    /// The neighbours, most similar first, then by collection and acronym.
    pub items: Vec<TextItem>,
}

/// Everything the panel shows of one text.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PanelText {
    /// The text: reference, summary.
    pub item: TextItem,
    /// Its collection and group (`crate::map::model::place`).
    pub place: String,
    /// The texts visited, the last one this.
    pub trail: Vec<TrailStep>,
    /// The neighbours by link type, the types without any left out.
    pub sections: Vec<NeighbourSection>,
    /// Who translated the titles shown, this text's first.
    pub translators: Vec<String>,
    /// A vectorial view: the Digital Pāḷi Dictionary is credited.
    pub dpd: bool,
}

/// What the panel shows of node `i` (TextPanel.tsx lines 29–37): its neighbours in `scene`
/// (the texts' language's only) by type, the trail's labels (`index`: each uid's node).
pub fn panel_text<S: BuildHasher>(
    texts: &Texts<'_>,
    scene: &Scene,
    index: &HashMap<String, usize, S>,
    i: usize,
    trail: &[String],
) -> PanelText {
    let graph = texts.graph;
    let sections = scene
        .neighbours_by_type(graph, i)
        .into_iter()
        .enumerate()
        .filter(|(_, list)| !list.is_empty())
        .map(|(t, list)| NeighbourSection {
            type_name: graph.types.get(t).cloned().unwrap_or_default(),
            items: list
                .into_iter()
                .map(|j| TextItem {
                    score: scene.sim_of(i, j),
                    ..texts.item(j)
                })
                .collect(),
        })
        .collect();
    let neighbours: Vec<usize> = scene.neighbours(i).into_iter().map(|(j, _)| j).collect();
    PanelText {
        item: texts.item(i),
        place: place(graph, i),
        trail: trail
            .iter()
            .map(|uid| TrailStep {
                uid: uid.clone(),
                label: index
                    .get(uid)
                    .and_then(|&k| graph.nodes.get(k))
                    .map_or_else(|| uid.clone(), |node| node.acronym.clone()),
            })
            .collect(),
        sections,
        translators: panel_translators(graph, texts.gloss, i, &neighbours),
        dpd: graph.sim.is_some(),
    }
}

/// The trail's positions the header shows: the last [`TRAIL_SHOWN`].
pub fn trail_shown(len: usize) -> Range<usize> {
    len.saturating_sub(TRAIL_SHOWN)..len
}

/// SuttaCentral's page of `uid`.
pub fn read_href(uid: &str) -> String {
    format!("https://suttacentral.net/{}", encode_component(uid))
}

/// The text panel.
#[component]
pub fn TextPanel(
    /// What it shows.
    text: PanelText,
    /// A neighbour was chosen: its node.
    on_pick: Callback<usize>,
    /// The close button.
    on_close: Callback<()>,
    /// A text of the trail was chosen: its position in the trail.
    on_jump: Callback<usize>,
    /// At the right of the header bar.
    #[prop(optional, into)]
    actions: Option<ViewFn>,
) -> impl IntoView {
    let PanelText {
        item,
        place,
        trail,
        sections,
        translators,
        dpd,
    } = text;
    view! {
        <div class="relative flex h-full flex-col">
            <PanelBar trail=trail on_close=on_close on_jump=on_jump actions=actions />
            <div class="min-h-0 flex-1 overflow-y-auto">
                <PanelHeading item=item place=place />
                <div class="pb-4">
                    {sections
                        .into_iter()
                        .map(|section| view! { <NeighbourList section=section on_pick=on_pick /> })
                        .collect_view()}
                </div>
            </div>
            <div class="border-t border-line px-4 py-3">
                <Credits translators=translators dpd=dpd class="text-[11px]" />
            </div>
        </div>
    }
}

/// The panel's header bar, like palicanon.app's side sheets: the close chevron, then the
/// texts visited, the last [`TRAIL_SHOWN`] (TextPanel.tsx lines 41–63).
#[component]
fn PanelBar(
    trail: Vec<TrailStep>,
    on_close: Callback<()>,
    on_jump: Callback<usize>,
    actions: Option<ViewFn>,
) -> impl IntoView {
    let t = strings();
    let shown = trail_shown(trail.len());
    let last = shown.end.saturating_sub(1);
    let chips = trail[shown.clone()]
        .iter()
        .zip(shown.clone())
        .map(|(step, k)| {
            view! {
                <li class="flex items-center gap-1">
                    {(k > shown.start)
                        .then(|| {
                            view! {
                                <span class="text-subtle" aria-hidden="true">
                                    "›"
                                </span>
                            }
                        })}
                    <RefChip
                        label=step.label.clone()
                        current=k == last
                        on_click=Callback::new(move |()| on_jump.run(k))
                    />
                </li>
            }
        })
        .collect_view();
    view! {
        <div class="flex shrink-0 items-center gap-2 border-b border-line px-3 py-2.5">
            <button
                type="button"
                on:click=move |_| on_close.run(())
                aria-label=move || td_string!(t.locale(), panel_close)
                title=move || td_string!(t.locale(), panel_close)
                class="grid size-9 shrink-0 cursor-pointer place-items-center rounded-full text-muted transition hover:bg-chip hover:text-heading"
            >
                <Icon name=IconName::ChevronRight />
            </button>
            <ol
                aria-label=move || td_string!(t.locale(), panel_trail)
                class="flex min-w-0 items-center gap-1 overflow-x-auto"
            >
                {chips}
            </ol>
            {actions.map(|actions| view! { <div class="ml-auto shrink-0">{actions.run()}</div> })}
        </div>
    }
}

/// The open text: its reference, summary, place and "Read on SuttaCentral" (TextPanel.tsx
/// lines 66–76).
#[component]
fn PanelHeading(item: TextItem, place: String) -> impl IntoView {
    let t = strings();
    let href = read_href(&item.uid);
    view! {
        <div class="px-4 pt-4">
            <div>
                <SuttaRef reference=item.reference variant=RefVariant::Heading />
            </div>
            {item
                .blurb
                .map(|blurb| view! { <p class="mt-2 text-sm leading-6 text-muted">{blurb}</p> })}
            <div class="mt-3 flex flex-wrap items-center gap-x-3 gap-y-2 text-xs text-muted">
                <span>{place}</span>
                <ReadLink href=href>{move || td_string!(t.locale(), panel_read)}</ReadLink>
            </div>
        </div>
    }
}

/// One link type's neighbours, a folding section of cards (TextPanel.tsx lines 78–88).
#[component]
fn NeighbourList(section: NeighbourSection, on_pick: Callback<usize>) -> impl IntoView {
    let t = strings();
    let name = section.type_name.clone();
    let title = Signal::derive(move || type_label(&name, t.locale()).to_owned());
    let count = section.items.len();
    let cards = section
        .items
        .into_iter()
        .map(|item| {
            let j = item.index;
            view! { <TextCard item=item on_click=Callback::new(move |()| on_pick.run(j)) /> }
        })
        .collect_view();
    view! {
        <Section id=section.type_name title=title count=count>
            <ul class="divide-y divide-line px-2">{cards}</ul>
        </Section>
    }
}

/// A credits link's classes.
const LINK: &str = "hover:text-gold";

/// The credits, gold lines (palicanon.app's `Credits`, `Credits.tsx`): SuttaCentral, the
/// translators of the titles shown, the DPD on a vectorial view.
#[component]
pub fn Credits(
    /// The translators, each once.
    translators: Vec<String>,
    /// Credit the Digital Pāḷi Dictionary.
    dpd: bool,
    /// Classes added to the paragraph's.
    #[prop(optional, into)]
    class: String,
) -> impl IntoView {
    let t = strings();
    let names = move || {
        (!translators.is_empty()).then(|| {
            let names = conjunction(&translators, language_signal().get());
            td_string!(t.locale(), credits_translated, names = names).to_string()
        })
    };
    view! {
        <p class=format!("text-[13px] leading-5 text-gold/80 {class}")>
            <a href="https://suttacentral.net" target="_blank" rel="noopener" class=LINK>
                {move || td_string!(t.locale(), credits_sources)}
            </a>
            {move || {
                names()
                    .map(|names| {
                        view! {
                            <br />
                            {names}
                        }
                    })
            }}
            {dpd
                .then(|| {
                    view! {
                        <br />
                        <a
                            href="https://digitalpalidictionary.github.io"
                            target="_blank"
                            rel="noopener"
                            class=LINK
                        >
                            {move || td_string!(t.locale(), credits_dpd)}
                        </a>
                    }
                })}
        </p>
    }
}

#[cfg(test)]
mod tests {
    use palicanon_ui_lib::Language;

    use super::*;
    use crate::map::draw::Links;
    use crate::map::model::index_of;
    use crate::map::ui::test_support::Fixture;

    fn trail(uids: &[&str]) -> Vec<String> {
        uids.iter().map(|s| (*s).to_owned()).collect()
    }

    fn mn1(f: &Fixture, language: Language, scene: &Scene) -> PanelText {
        let index = index_of(&f.graph);
        panel_text(
            &f.texts(language),
            scene,
            &index,
            1,
            &trail(&["dn1", "an9", "mn1"]),
        )
    }

    fn uids(section: &NeighbourSection) -> Vec<&str> {
        section.items.iter().map(|item| item.uid.as_str()).collect()
    }

    #[test]
    fn the_panel_has_the_neighbours_by_type_and_the_trail() {
        // TextPanel.tsx:31-32 `byType[ty]!.push(j)`, 79 a section per type with neighbours
        let f = Fixture::new();
        let p = mn1(&f, Language::Fr, &Scene::new(&f.graph));
        assert_eq!(p.item.uid, "mn1");
        assert_eq!(p.item.reference.title, "La racine de toutes choses");
        assert_eq!(p.place, "Majjhima Nikāya");
        let types: Vec<&str> = p.sections.iter().map(|s| s.type_name.as_str()).collect();
        assert_eq!(types, ["full", "resembling"]);
        // TextPanel.tsx:83 by collection (dn before mn), then acronym
        assert_eq!(uids(&p.sections[0]), ["dn1", "mn2"]);
        assert_eq!(uids(&p.sections[1]), ["mn10"]);
        // App.tsx:556 `label: k !== undefined ? graph.nodes[k]![1] : uid`
        let labels: Vec<&str> = p.trail.iter().map(|s| s.label.as_str()).collect();
        assert_eq!(labels, ["DN 1", "an9", "MN 1"]);
        // TextPanel.tsx:36 this text's translator, then its neighbours'
        assert_eq!(p.translators, ["Môhan Wijayaratna", "Christian Maës"]);
        assert!(!p.dpd);
    }

    #[test]
    fn neighbours_outside_the_texts_language_are_left_out() {
        // App.tsx:537 `(engine.adj[selected] ?? []).filter(([j]) => !available || available[j])`
        let f = Fixture::new();
        let mut scene = Scene::new(&f.graph);
        scene.available = Some(vec![true, true, false, false, true]);
        let p = mn1(&f, Language::En, &scene);
        assert_eq!(p.sections.len(), 1, "no resembling neighbour left");
        assert_eq!(uids(&p.sections[0]), ["dn1"]);
        assert!(p.translators.is_empty(), "English: SuttaCentral's titles");
    }

    #[test]
    fn a_vectorial_view_scores_its_neighbours_and_credits_the_dpd() {
        // TextPanel.tsx:83 `(simOf(index, b) ?? 0) - (simOf(index, a) ?? 0)`; App.tsx:546 `dpd={!!graph.sim}`
        let mut f = Fixture::new();
        f.graph.types = vec!["similar".to_owned()];
        f.graph.sim = Some(crate::map::model::SimInfo {
            url: "pairs.bin".to_owned(),
            min: 0.5,
            max: 0.9,
            default: 0.6,
            range: None,
            sha256: None,
        });
        let mut scene = Scene::new(&f.graph);
        scene.set_links(Links {
            edges: vec![0, 1, 0, 1, 2, 0, 1, 3, 0],
            sims: [((0, 1), 0.61), ((1, 2), 0.83), ((1, 3), 0.7)].into(),
        });
        let p = mn1(&f, Language::En, &scene);
        assert_eq!(uids(&p.sections[0]), ["mn10", "mn2", "dn1"]);
        assert_eq!(p.sections[0].items[0].score, Some(0.83));
        assert!(p.dpd);
    }

    #[test]
    fn the_header_shows_the_last_six_texts() {
        // TextPanel.tsx:53-54 `trail.slice(-6)`, `real = trail.length - last.length + k`
        assert_eq!(trail_shown(3), 0..3);
        assert_eq!(trail_shown(6), 0..6);
        assert_eq!(trail_shown(9), 3..9);
        assert_eq!(trail_shown(0), 0..0);
        // TextPanel.tsx:73 `https://suttacentral.net/${encodeURIComponent(uid)}`
        assert_eq!(read_href("sn56.11"), "https://suttacentral.net/sn56.11");
    }

    #[cfg(feature = "ssr")]
    fn render_panel(language: Language, text: PanelText) -> String {
        use crate::map::ui::test_support::{plain, render};

        plain(&render(language, move || {
            view! {
                <TextPanel
                    text=text
                    on_pick=Callback::new(|_| {})
                    on_close=Callback::new(|()| {})
                    on_jump=Callback::new(|_| {})
                />
            }
        }))
    }

    #[cfg(feature = "ssr")]
    #[test]
    fn the_panel_is_labelled_in_each_language() {
        use crate::map::ui::test_support::tag_around;

        // TextPanel.tsx:46 `t("panel.close")`, 52 `t("panel.trail")`, 73 `t("panel.read")`,
        // 80 `t(`types.${...}`)`; Credits.tsx:11 `t("credits.sources")`, 15 `t("credits.translated", { names })`
        let f = Fixture::new();
        for (language, words) in [
            (
                Language::En,
                [
                    "Close the panel",
                    "Texts visited",
                    "Read on SuttaCentral",
                    "Full parallel",
                    "Resembling",
                    "Texts and parallels from SuttaCentral.",
                ],
            ),
            (
                Language::Fr,
                [
                    "Fermer le panneau",
                    "Textes visités",
                    "Lire sur SuttaCentral",
                    "Parallèle complet",
                    "Ressemblant",
                    "Traduit par Môhan Wijayaratna et Christian Maës.",
                ],
            ),
            (
                Language::Vi,
                [
                    "Đóng bảng",
                    "Các bài kinh đã xem",
                    "Đọc trên SuttaCentral",
                    "Tương đương hoàn toàn",
                    "Tương tự",
                    "Kinh văn và các bài tương đương từ SuttaCentral.",
                ],
            ),
        ] {
            let html = render_panel(language, mn1(&f, language, &Scene::new(&f.graph)));
            for word in words {
                assert!(html.contains(word), "{language:?} {word}: {html}");
            }
            let read = tag_around(&html, "https://suttacentral.net/");
            assert!(
                read.contains(r#"href="https://suttacentral.net/mn1""#),
                "{read}"
            );
        }
    }

    #[cfg(feature = "ssr")]
    #[test]
    fn the_panel_shows_the_trail_the_text_and_its_neighbours() {
        use crate::map::ui::test_support::tag_around;

        let f = Fixture::new();
        let html = render_panel(Language::En, mn1(&f, Language::En, &Scene::new(&f.graph)));
        // TextPanel.tsx:57 the trail: the last chip is the current text
        let trail = &html[html.find("<ol").expect(&html)..html.find("</ol>").expect(&html)];
        assert!(
            trail.contains(">DN 1<") && trail.contains(">an9<"),
            "{trail}"
        );
        assert!(
            tag_around(trail, ">MN 1<").contains(r#"aria-current="page""#),
            "{trail}"
        );
        // TextPanel.tsx:68-70 the reference and the summary
        assert!(html.contains(">Mūlapariyāya<"), "{html}");
        assert!(html.contains(">The Buddha teaches the root.<"), "{html}");
        assert!(html.contains(">Majjhima Nikāya<"), "the place: {html}");
        // TextPanel.tsx:80 each section's count, every neighbour a card
        assert!(html.contains("· 2") && html.contains("· 1"), "{html}");
        for acronym in [">DN 1<", ">MN 2<", ">MN 10<"] {
            assert!(html.matches(acronym).count() >= 1, "{acronym}: {html}");
        }
        assert_eq!(
            html.matches("aria-expanded=\"false\"").count(),
            2,
            "folded: {html}"
        );
        // Credits.tsx:13 no translator line in SuttaCentral's English; no DPD outside a vectorial view
        assert!(!html.contains("Translated by"), "{html}");
        assert!(!html.contains("Digital Pāḷi Dictionary"), "{html}");
    }
}
