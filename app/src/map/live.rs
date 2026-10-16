//! `LiveMap`: the map screen live (task 006, M5) — `src/App.tsx`'s state, derived values and
//! effects as signals, wired to M3's `EngineView` and M4's components in M4's `MapScreen`.
//!
//! **The address is the state** ([`crate::map::address`]): the path names the view and the
//! open text, `?th=` the threshold, the history entry the trail. Choosing a text pushes an
//! entry (a linked text extends the trail, any other starts one: `model::next_trail`); the
//! browser's back and forward walk them; another view, or the threshold settling, replaces
//! the entry. Escape, the panel's close, or a click on the empty map, closes the text: the
//! address goes back to the view's, a new entry. A uid the browser finds in no view is the
//! 404, as the server answers it; a data file that does not arrive is said over the map.
//!
//! **The first render is the server's** ([`FirstRender`]): the server has the data in memory
//! and renders the open text's panel and its head; the browser hydrates from the very same
//! decisions, then fetches its own copy of the data (the manifest's hashed URLs) and takes
//! over: from then on the panel, the legend, the search and the engine are computed here.
//! The cookies `viz_legend` and `viz_textlang` are read on both sides for the first render,
//! and written by the browser.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use leptos::prelude::*;
use leptos_i18n::td_string;
use leptos_router::NavigateOptions;
use leptos_router::hooks::{use_location, use_navigate};
use leptos_router::location::State;
use palicanon_ui_lib::{Language, language_signal, theme_signal};

use crate::catalogue::strings;
use crate::components::not_found::NotFoundPage;
use crate::head::PageHead;
use crate::map::address::{address, trail_at, trail_state, view_address};
use crate::map::cookies;
use crate::map::draw::Scene;
use crate::map::engine_view::{EngineInput, EngineView, Hover as Pointed, LinksAt, take_links};
use crate::map::first::{FirstOpen, FirstRender, panel_without_links, text_title};
use crate::map::legend::{self, Legend as LegendState};
use crate::map::model::{Blurbs, Graph, Pairs, Translations, index_of, next_trail, text_ref};
use crate::map::open_text::highlighted;
use crate::map::search;
use crate::map::texts_lang::{self, TextsLang, available, gloss_lang, masks, texts_lang};
use crate::map::theme::{MapTheme, is_light};
use crate::map::ui::legend::{LegendRows, legend_rows};
use crate::map::ui::panel::{PanelText, panel_text};
use crate::map::ui::screen::{MapError, MapLoading};
use crate::map::ui::texts_menu::translated_counts;
use crate::map::ui::view_menu::ViewMenu;
use crate::map::ui::{
    Hover, Legend, MapScreen, SearchBox, SimControls, TextItem, TextPanel, Texts, TextsMenu,
    Tooltip,
};
use crate::route::{Route, View};

/// A view's data in the browser: its graph, and a vectorial view's similarity pairs.
#[derive(Clone, Debug)]
pub(crate) struct ViewData {
    /// Its graph.
    pub graph: Arc<Graph>,
    /// The pairs its links are cut from (`graph.sim`'s file).
    pub pairs: Option<Arc<Pairs>>,
}

/// The summaries of the glosses' language, and in English.
#[derive(Clone, Debug)]
struct Summaries {
    /// The glosses' language.
    language: Language,
    /// Its summaries (empty: none in that language).
    lang: Arc<Blurbs>,
    /// SuttaCentral's English ones, the fallback.
    en: Arc<Blurbs>,
}

/// Whether an `Option<Arc<_>>` memo changed: another allocation, not other contents (the
/// graphs are megabytes; comparing them would cost more than recomputing).
#[allow(
    clippy::ref_option,
    reason = "the shape `Memo::new_with_compare` asks for"
)]
fn other_arc<T>(a: Option<&Option<Arc<T>>>, b: Option<&Option<Arc<T>>>) -> bool {
    match (a, b) {
        (Some(Some(a)), Some(Some(b))) => !Arc::ptr_eq(a, b),
        (Some(None), Some(None)) => false,
        _ => true,
    }
}

/// The map's address: the view and the text the path names (the parallels, nothing, on a
/// path that is not the map's).
fn map_route(path: &str) -> (View, Option<String>) {
    match Route::parse(path) {
        Some(Route::Map { view, uid }) => (view, uid),
        _ => (View::Parallels, None),
    }
}

/// Whether Escape (`key`) closes the open text: no field has the focus (`typing`), a text is
/// open, and no dialog is (`dialog_open`: the shell's Settings, its languages, its phone
/// navigation, a chip's list). The map then takes the key, and stops it: ui-lib's shell does
/// not also fold its rail. Otherwise the key is the shell's, as on every other page.
#[cfg_attr(
    not(all(feature = "hydrate", target_arch = "wasm32")),
    allow(dead_code, reason = "read by the browser part only")
)]
pub(crate) fn escape_closes_text(
    key: &str,
    typing: bool,
    text_open: bool,
    dialog_open: bool,
) -> bool {
    key == "Escape" && !typing && text_open && !dialog_open
}

/// Whether the slider's `value` (hundredths) is to be applied once it settles: it differs
/// from the `applied` threshold, and the view did not just change (`view_changed`).
#[cfg_attr(
    not(all(feature = "hydrate", target_arch = "wasm32")),
    allow(dead_code, reason = "read by the browser part only")
)]
pub(crate) fn threshold_settles(view_changed: bool, applied: Option<f64>, value: u32) -> bool {
    !view_changed && applied.is_some_and(|th| crate::map::ui::sim_controls::hundredths(th) != value)
}

/// Whether the browser knows `uid` is in no view: `has(view)` says whether a view's data has
/// it, `None` while that data is not in. The projections (PCA, UMAP, t-SNE) hold the
/// vectorial space's texts, so the parallels and the vectorial space decide.
pub(crate) fn in_no_view(has: impl Fn(View) -> Option<bool>) -> bool {
    has(View::Parallels) == Some(false) && has(View::Sim) == Some(false)
}

/// The map at every map address: the 404 when the server found the address's text in no
/// view (and the reader is still there), else the map screen.
#[component]
pub(crate) fn LiveMap() -> impl IntoView {
    let first = use_context::<Arc<FirstRender>>().unwrap_or_default();
    let location = use_location();
    let not_found = {
        let first = Arc::clone(&first);
        Memo::new(move |_| {
            matches!(first.open, FirstOpen::NotFound) && location.pathname.get() == first.path
        })
    };
    move || {
        if not_found.get() {
            view! { <NotFoundPage /> }.into_any()
        } else {
            view! { <MapView first=Arc::clone(&first) /> }.into_any()
        }
    }
}

/// The trail kept in the current history entry, in the browser.
fn history_state() -> Option<String> {
    #[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
    {
        window().history().ok()?.state().ok()?.as_string()
    }
    #[cfg(not(all(feature = "hydrate", target_arch = "wasm32")))]
    None
}

/// The device prefers a light page (`prefers-color-scheme`), in the browser; the server
/// cannot know.
fn prefers_light() -> bool {
    #[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
    {
        window()
            .match_media("(prefers-color-scheme: light)")
            .ok()
            .flatten()
            .is_some_and(|query| query.matches())
    }
    #[cfg(not(all(feature = "hydrate", target_arch = "wasm32")))]
    false
}

/// The map screen and its state.
#[component]
#[allow(
    clippy::too_many_lines,
    reason = "App.tsx's state, one signal after the other"
)]
fn MapView(first: Arc<FirstRender>) -> impl IntoView {
    let t = strings();
    let language = language_signal();
    let theme_choice = theme_signal();
    let location = use_location();
    let navigate = use_navigate();

    // ---------- The address ----------
    let route = Memo::new(move |_| map_route(&location.pathname.get()));
    let view = Memo::new(move |_| route.with(|r| r.0));
    let uid = Memo::new(move |_| route.with(|r| r.1.clone()));
    let th_query = Memo::new(move |_| location.query.with(|q| q.get("th")));
    let trail = RwSignal::new(trail_at(
        &[],
        uid.get_untracked().as_deref(),
        history_state().as_deref(),
    ));
    // Back, forward, a link: the history entry's trail, when it ends at the address's text.
    Effect::new(move |_| {
        let uid = uid.get();
        let current = trail.get_untracked();
        let next = trail_at(&current, uid.as_deref(), history_state().as_deref());
        if next != current {
            trail.set(next);
        }
    });
    let go = Callback::new(move |(path, next, replace): (String, Vec<String>, bool)| {
        navigate(
            &path,
            NavigateOptions {
                resolve: false,
                replace,
                scroll: false,
                state: State::from(trail_state(&next)),
            },
        );
    });

    // ---------- The data ----------
    let data = RwSignal::new(None::<ViewData>);
    let translations = RwSignal::new(None::<Arc<Translations>>);
    let summaries = RwSignal::new(None::<Summaries>);

    // ---------- The reader's choices ----------
    let texts = RwSignal::new(texts_lang(
        cookies::read(texts_lang::COOKIE).as_deref(),
        language.get_untracked(),
    ));
    let legend_state = RwSignal::new(LegendState::parse(cookies::read(legend::COOKIE).as_deref()));
    let legend_open = RwSignal::new(legend_state.with_untracked(|l| l.open));
    let drawer_open = RwSignal::new(false);
    let type_on = RwSignal::new(Vec::<bool>::new());
    let applied = RwSignal::new(None::<f64>);
    let slider = RwSignal::new(0_u32);
    let hide_alone = RwSignal::new(true);
    let query = RwSignal::new(String::new());
    let pointed = RwSignal::new(None::<Pointed>);
    let no_gpu = RwSignal::new(false);
    // A data file did not arrive: the map says so instead of loading for ever.
    let failed = RwSignal::new(false);
    // Bumped each time a view's data is kept (`views` is not a signal).
    let views_in = RwSignal::new(0_usize);

    // ---------- Derived ----------
    let graph = Memo::new_with_compare(
        move |_| data.with(|d| d.as_ref().map(|d| Arc::clone(&d.graph))),
        other_arc::<Graph>,
    );
    let light = Memo::new(move |_| is_light(theme_choice.get(), prefers_light()));
    // The legend as the server rendered it, until the browser's data is in.
    let first_legend = SharedValue::new(move || {
        server_legend(
            view.get_untracked(),
            th_query.get_untracked().as_deref(),
            texts.get_untracked(),
            light.get_untracked(),
        )
    })
    .into_inner();
    if let Some(range) = first_legend.as_ref().and_then(|rows| rows.sim) {
        slider.set(range.value);
    }
    let theme = Memo::new(move |_| graph.get().map(|g| MapTheme::new(light.get(), &g)));
    let gloss = Memo::new(move |_| gloss_lang(texts.get(), language.get()));
    let index = Memo::new_with_compare(
        move |_| graph.get().map(|g| Arc::new(index_of(&g))),
        other_arc::<HashMap<String, usize>>,
    );
    let all_masks = Memo::new_with_compare(
        move |_| {
            let g = graph.get()?;
            let tr = translations.get()?;
            Some(Arc::new(masks(&g, &tr)))
        },
        other_arc::<BTreeMap<String, Vec<bool>>>,
    );
    let mask = Memo::new(move |_| {
        all_masks.with(|m| {
            m.as_ref()
                .and_then(|m| available(texts.get(), m).map(<[bool]>::to_vec))
        })
    });
    let links = Memo::new_with_compare(
        move |_| {
            let d = data.get()?;
            let sim = d.graph.sim.as_ref()?;
            let pairs = d.pairs.as_ref()?;
            let threshold = applied.get()?;
            Some(Arc::new(LinksAt {
                threshold,
                links: crate::map::draw::links_at(pairs, sim, threshold),
            }))
        },
        other_arc::<LinksAt>,
    );
    let scene = Memo::new_with_compare(
        move |_| {
            let g = graph.get()?;
            let mut scene = Scene::new(&g);
            if let Some(links) = links.get() {
                // As the engine takes them: refused links leave none, not stale counts.
                take_links(&mut scene, &links, g.nodes.len());
            }
            scene.available = mask.get();
            Some(Arc::new(scene))
        },
        other_arc::<Scene>,
    );
    // Everything the panel and the search read is in: the view, the titles, the summaries.
    let ready = Memo::new(move |_| {
        graph.with(Option::is_some)
            && translations.with(Option::is_some)
            && summaries.with(|s| s.as_ref().is_some_and(|s| s.language == gloss.get()))
    });
    let selected = Memo::new(move |_| {
        let at = uid.with(|uid| {
            let uid = uid.as_ref()?;
            index.with(|index| index.as_ref()?.get(uid).copied())
        });
        mask.with(|mask| highlighted(at, mask.as_deref()))
    });

    let views = StoredValue::new(HashMap::<View, ViewData>::new());
    // A uid in no view, reached in the browser: the 404, as the server answers it.
    let missing = Memo::new(move |_| {
        views_in.track();
        uid.with(|uid| {
            let Some(uid) = uid.as_deref() else {
                return false;
            };
            views.with_value(|views| {
                in_no_view(|v| {
                    views
                        .get(&v)
                        .map(|d| d.graph.nodes.iter().any(|n| n.uid == uid))
                })
            })
        })
    });

    // Everything a card is read from, once it is all in.
    let loaded = move || -> Option<Loaded> {
        if !ready.get() {
            return None;
        }
        Some(Loaded {
            graph: graph.get()?,
            theme: theme.get()?,
            translations: translations.get()?,
            summaries: summaries.get()?,
            scene: scene.get()?,
            index: index.get()?,
            gloss: gloss.get(),
        })
    };

    // ---------- The open text ----------
    let opened = {
        let first = Arc::clone(&first);
        move |uid: &str| {
            first
                .opened()
                .filter(|text| text.panel.item.uid == uid)
                .cloned()
        }
    };
    let panel = {
        let opened = opened.clone();
        Memo::new(move |_| -> Option<PanelText> {
            let uid = uid.get()?;
            let Some(l) = loaded() else {
                return opened(&uid).map(|text| text.panel);
            };
            let trail = trail.get();
            if let Some(&i) = l.index.get(&uid) {
                return Some(panel_text(&l.texts(), &l.scene, &l.index, i, &trail));
            }
            // The view lacks the text: another view the browser has shows it, without links.
            let other = views.with_value(|views| {
                View::ALL.into_iter().find_map(|v| {
                    let data = views.get(&v)?;
                    let j = data.graph.nodes.iter().position(|n| n.uid == uid)?;
                    Some((Arc::clone(&data.graph), j))
                })
            });
            if let Some((g, j)) = other {
                let theme = MapTheme::new(light.get(), &g);
                let texts = Texts {
                    graph: &g,
                    theme: &theme,
                    ..l.texts()
                };
                return Some(panel_without_links(&texts, j, &trail, &index_of(&g)));
            }
            opened(&uid).map(|text| text.panel)
        })
    };
    // The panel is the open text's: closing it closes the text (the address goes back to the
    // view's), so a reload or a shared link shows what the reader sees.
    let panel_shown = Signal::derive(move || panel.with(Option::is_some));

    // ---------- The head ----------
    let canonical = move |uid: &str| -> View {
        if let Some(text) = opened(uid) {
            return text.canonical;
        }
        views
            .with_value(|views| {
                View::ALL.into_iter().find(|v| {
                    views
                        .get(v)
                        .is_some_and(|d| d.graph.nodes.iter().any(|n| n.uid == uid))
                })
            })
            .unwrap_or_else(|| view.get())
    };
    let head = Memo::new(move |_| {
        let locale = t.locale();
        let description = td_string!(locale, head_home_description).to_owned();
        let open = uid.get().and_then(|uid| {
            panel.with(|panel| {
                let item = &panel.as_ref()?.item;
                Some((
                    text_title(&item.reference),
                    item.blurb.clone().unwrap_or_else(|| description.clone()),
                    address(canonical(&uid), Some(&uid), None),
                ))
            })
        });
        open.unwrap_or_else(|| {
            (
                td_string!(locale, head_home_title).to_owned(),
                description,
                address(view.get(), None, None),
            )
        })
    });

    // ---------- Choosing ----------
    // Open node `i`, or close the open text (`None`): App.tsx lines 244–256, `choose`.
    let choose = Callback::new(move |picked: Option<usize>| {
        let current_view = view.get_untracked();
        let th = th_query.get_untracked();
        let Some(i) = picked else {
            if uid.with_untracked(Option::is_some) {
                trail.set(Vec::new());
                go.run((
                    address(current_view, None, th.as_deref()),
                    Vec::new(),
                    false,
                ));
            }
            return;
        };
        let Some(picked_uid) =
            graph.with_untracked(|g| Some(g.as_ref()?.nodes.get(i)?.uid.clone()))
        else {
            return;
        };
        let current = trail.get_untracked();
        let open = current
            .last()
            .and_then(|last| index.with_untracked(|index| index.as_ref()?.get(last).copied()));
        let linked = open.is_some_and(|open| {
            scene.with_untracked(|scene| {
                scene
                    .as_ref()
                    .and_then(|scene| scene.adj.get(open))
                    .is_some_and(|links| links.iter().any(|&(j, _)| j == i))
            })
        });
        let Some(next) = next_trail(&current, &picked_uid, linked) else {
            return;
        };
        trail.set(next.clone());
        go.run((
            address(current_view, Some(&picked_uid), th.as_deref()),
            next,
            false,
        ));
    });
    // A text of the trail: the trail up to it (TextPanel's `onJump`).
    let jump = Callback::new(move |k: usize| {
        let next: Vec<String> =
            trail.with_untracked(|trail| trail.iter().take(k + 1).cloned().collect());
        let Some(last) = next.last().cloned() else {
            return;
        };
        trail.set(next.clone());
        let th = th_query.get_untracked();
        go.run((
            address(view.get_untracked(), Some(&last), th.as_deref()),
            next,
            false,
        ));
    });
    // Another view: the same text there, the entry replaced (App.tsx lines 505–511).
    let on_view = Callback::new(move |v: View| {
        if v != view.get_untracked() {
            let path = view_address(v, uid.get_untracked().as_deref());
            go.run((path, trail.get_untracked(), true));
        }
    });
    let on_texts = Callback::new(move |choice: TextsLang| {
        texts.set(choice);
        cookies::write(
            texts_lang::COOKIE,
            &texts_lang::cookie_value(choice, language.get_untracked()),
        );
    });

    // ---------- What the browser keeps ----------
    // The legend as the reader leaves it, from their first change on.
    Effect::new(move |previous: Option<()>| {
        let mut state = legend_state.get();
        state.open = legend_open.get();
        if previous.is_some() {
            cookies::write(legend::COOKIE, &state.to_cookie());
        }
    });
    // A new interface language shows the texts translated into it (App.tsx lines 89–94).
    Effect::new(move |previous: Option<Language>| {
        let now = language.get();
        if previous.is_some_and(|before| before != now) {
            let choice = TextsLang::Translated(now);
            texts.set(choice);
            cookies::write(texts_lang::COOKIE, &texts_lang::cookie_value(choice, now));
        }
        now
    });

    #[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
    browser::start(&browser::Wiring {
        manifest: Arc::new(first.manifest.clone()),
        view,
        uid,
        th_query,
        trail,
        views,
        views_in,
        data,
        translations,
        summaries,
        gloss,
        type_on,
        applied,
        slider,
        pointed,
        failed,
        go,
        choose,
    });

    // ---------- The slots ----------
    let input = EngineInput {
        graph: graph.into(),
        theme: theme.into(),
        links: links.into(),
        colored: Signal::derive(move || {
            graph.with(|g| {
                g.as_ref()
                    .map(|g| legend_state.with(|l| l.colored_mask(g)))
                    .unwrap_or_default()
            })
        }),
        visible: Signal::derive(move || {
            graph.with(|g| {
                g.as_ref()
                    .map(|g| legend_state.with(|l| l.visible_mask(g)))
                    .unwrap_or_default()
            })
        }),
        types: type_on.into(),
        available: mask.into(),
        hide_alone: hide_alone.into(),
        selected: selected.into(),
    };
    let search_index = Memo::new_with_compare(
        move |_| {
            let g = graph.get()?;
            let tr = translations.get()?;
            let titles = match texts.get() {
                TextsLang::Pali => None,
                TextsLang::Translated(lang) => tr.0.get(lang.code()),
            };
            Some(Arc::new(search::index(&g, titles)))
        },
        other_arc::<Vec<search::Entry>>,
    );
    let hits = Memo::new(move |_| -> Vec<TextItem> {
        let query = query.get();
        let (Some(l), Some(entries)) = (loaded(), search_index.get()) else {
            return Vec::new();
        };
        let degree = |i: usize| l.scene.adj.get(i).map_or(0, Vec::len);
        let found = mask.with(|mask| search::hits(&query, &entries, mask.as_deref(), degree));
        let texts = l.texts();
        found.into_iter().map(|i| texts.item(i)).collect()
    });
    let tooltip = Signal::derive(move || {
        let pointed = pointed.get()?;
        let g = graph.get()?;
        if pointed.index >= g.nodes.len() {
            return None;
        }
        let reference = translations.with(|tr| {
            let gloss = tr.as_ref().and_then(|tr| tr.0.get(gloss.get().code()));
            text_ref(&g, pointed.index, gloss)
        });
        let links = scene.with(|s| {
            s.as_ref()
                .and_then(|s| s.adj.get(pointed.index))
                .map_or(0, Vec::len)
        });
        Some(Hover {
            reference,
            links,
            x: pointed.x,
            y: pointed.y,
        })
    });
    // The legend: the browser's own rows once its data is in, the server's until then.
    let legend_live = Memo::new(move |_| {
        let g = graph.get()?;
        let th = theme.get()?;
        let scene = scene.get()?;
        let threshold = applied.get();
        Some(mask.with(|m| legend_rows(&g, &th, &scene, m.as_deref(), threshold)))
    });
    let legend_now = Memo::new(move |_| legend_live.get().or_else(|| first_legend.clone()));
    // What the legend is built for: shown or not, and a vectorial view's range; its rows
    // and counts then follow `legend_now` in place.
    let legend_shape = Memo::new(move |_| {
        legend_now.with(|rows| rows.as_ref().map(|rows| rows.sim.map(|s| (s.min, s.max))))
    });
    let rows_of = move |pick: fn(&LegendRows) -> usize| {
        Signal::derive(move || legend_now.with(|rows| rows.as_ref().map_or(0, pick)))
    };
    let legend_view = move || {
        let sim = legend_shape.get()?;
        let collections = Signal::derive(move || {
            legend_now.with(|rows| {
                rows.as_ref()
                    .map(|rows| rows.collections.clone())
                    .unwrap_or_default()
            })
        });
        let types = Signal::derive(move || {
            legend_now.with(|rows| {
                rows.as_ref()
                    .map(|rows| rows.types.clone())
                    .unwrap_or_default()
            })
        });
        let links_count = rows_of(|rows| rows.links);
        let alone = rows_of(|rows| rows.alone);
        let controls = sim.map(|(min, max)| {
            view! {
                <SimControls
                    min=min
                    max=max
                    value=slider
                    links=links_count
                    alone=alone
                    hide_alone=hide_alone
                    on_hide_alone=Callback::new(move |on| hide_alone.set(on))
                />
            }
        });
        Some(view! {
            <Legend
                collections=collections
                types=types
                legend=legend_state
                type_on=type_on
                on_color=Callback::new(move |iso: String| {
                    legend_state.update(|l| l.toggle_colored(&iso));
                })
                on_hide=Callback::new(move |iso: String| {
                    legend_state.update(|l| l.toggle_hidden(&iso));
                })
                on_type=Callback::new(move |k: usize| {
                    type_on
                        .update(|on| {
                            if let Some(on) = on.get_mut(k) {
                                *on = !*on;
                            }
                        });
                })
            >
                {controls}
            </Legend>
        })
    };
    let panel_view = move || {
        panel.get().map(|text| {
            view! {
                <TextPanel
                    text=text
                    on_pick=Callback::new(move |j: usize| choose.run(Some(j)))
                    on_close=Callback::new(move |()| choose.run(None))
                    on_jump=jump
                />
            }
        })
    };
    let first_counts = first.counts.clone();
    let counts = Signal::derive(move || {
        translations.with(|tr| {
            tr.as_ref()
                .map_or_else(|| first_counts.clone(), |tr| translated_counts(tr))
        })
    });

    // `MapScreen` runs each slot once: a slot that changes (the legend, the overlay) hands it
    // a closure, which the view keeps reactive.
    // The map stays mounted under a 404 (its engine and data with it), hidden.
    view! {
        {move || {
            if missing.get() {
                return view! { <NotFoundPage /> }.into_any();
            }
            let (title, description, path) = head.get();
            view! { <PageHead title=title description=description path=path /> }.into_any()
        }}
        <div class="contents" hidden=move || missing.get()>
            <h1 class="sr-only">{move || view.get().label(t.locale())}</h1>
            <MapScreen
                map=move || {
                    view! {
                        <EngineView
                            input=input
                            on_hover=Callback::new(move |h: Option<Pointed>| pointed.set(h))
                            on_select=Callback::new(move |i: Option<usize>| {
                                drawer_open.set(false);
                                choose.run(i);
                            })
                            on_no_gpu=Callback::new(move |()| no_gpu.set(true))
                        />
                    }
                }
                search=move || {
                    view! {
                        <SearchBox
                            hits=hits
                            on_query=Callback::new(move |q: String| query.set(q))
                            on_pick=Callback::new(move |i: usize| choose.run(Some(i)))
                        />
                    }
                }
                chips=move || {
                    view! {
                        <ViewMenu view=view on_select=on_view />
                        <TextsMenu texts=texts counts=counts on_select=on_texts />
                    }
                }
                legend=move || legend_view
                legend_open=legend_open
                drawer_open=drawer_open
                panel=panel_view
                panel_open=panel_shown
                overlay=move || {
                    move || {
                        if no_gpu.get() {
                            None
                        } else if failed.get() {
                            Some(view! { <MapError /> }.into_any())
                        } else {
                            graph.with(Option::is_none).then(|| view! { <MapLoading /> }.into_any())
                        }
                    }
                }
                tooltip=move || view! { <Tooltip hover=tooltip /> }
            />
        </div>
    }
}

/// The legend of the address's view as the server renders it: from the data in context
/// (`Served`), the texts' language and the theme of the request, the threshold of its
/// `?th=` — the rows and counts the browser computes once its own data is in. `None`
/// without data (a page rendered alone) and in the browser (it reads the server's).
fn server_legend(
    view: View,
    th: Option<&str>,
    texts: TextsLang,
    light: bool,
) -> Option<LegendRows> {
    #[cfg(feature = "ssr")]
    {
        use crate::map::address::threshold;
        use crate::map::draw::links_at;

        let served = use_context::<crate::data::Served>()?;
        let data = &served.data;
        let graph = data.graph(view);
        let theme = MapTheme::new(light, graph);
        let mask = available(texts, &masks(graph, &data.translations)).map(<[bool]>::to_vec);
        let mut scene = Scene::new(graph);
        let at = graph.sim.as_ref().map(|sim| threshold(th, sim));
        if let (Some(sim), Some(at)) = (&graph.sim, at) {
            scene.set_links(links_at(&data.pairs, sim, at));
        }
        scene.available.clone_from(&mask);
        Some(legend_rows(graph, &theme, &scene, mask.as_deref(), at))
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = (view, th, texts, light);
        None
    }
}

/// Everything a card is read from, owned: the view's data, its colours, the titles and
/// summaries of the glosses' language, its links, its uids.
#[derive(Clone)]
struct Loaded {
    graph: Arc<Graph>,
    theme: MapTheme,
    translations: Arc<Translations>,
    summaries: Summaries,
    scene: Arc<Scene>,
    index: Arc<HashMap<String, usize>>,
    gloss: Language,
}

impl Loaded {
    /// The texts as cards show them.
    fn texts(&self) -> Texts<'_> {
        Texts {
            graph: &self.graph,
            theme: &self.theme,
            gloss: self.translations.0.get(self.gloss.code()),
            blurbs: &self.summaries.lang,
            blurbs_en: &self.summaries.en,
        }
    }
}

/// The browser's part: the data fetched, the threshold's debounce, Escape.
#[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
mod browser {
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::Duration;

    use leptos::prelude::*;
    use leptos::task::spawn_local;
    use palicanon_ui_lib::Language;
    use wasm_bindgen::JsCast;

    use super::{Summaries, ViewData, escape_closes_text, threshold_settles};
    use crate::manifest::Manifest;
    use crate::map::address::{address, threshold};
    use crate::map::engine_view::{Hover as Pointed, is_typing};
    use crate::map::fetch;
    use crate::map::model::{Blurbs, Pairs, Translations};
    use crate::map::ui::sim_controls::{hundredths, threshold_label};
    use crate::route::View;
    use wasm_bindgen::closure::Closure;

    /// The data file of `view`'s graph (`src/map/data.ts`'s `MAPS`).
    const fn graph_file(view: View) -> &'static str {
        match view {
            View::Parallels => "graph.json",
            View::Sim => "graph-sim.json",
            View::Pca => "graph-sim-pca.json",
            View::Umap => "graph-sim-umap.json",
            View::Tsne => "graph-sim-tsne.json",
        }
    }

    /// The signals the browser's part reads and writes.
    pub(super) struct Wiring {
        pub manifest: Arc<Manifest>,
        pub view: Memo<View>,
        pub uid: Memo<Option<String>>,
        pub th_query: Memo<Option<String>>,
        pub trail: RwSignal<Vec<String>>,
        pub views: StoredValue<HashMap<View, ViewData>>,
        pub views_in: RwSignal<usize>,
        pub data: RwSignal<Option<ViewData>>,
        pub translations: RwSignal<Option<Arc<Translations>>>,
        pub summaries: RwSignal<Option<Summaries>>,
        pub gloss: Memo<Language>,
        pub type_on: RwSignal<Vec<bool>>,
        pub applied: RwSignal<Option<f64>>,
        pub slider: RwSignal<u32>,
        pub pointed: RwSignal<Option<Pointed>>,
        pub failed: RwSignal<bool>,
        pub go: Callback<(String, Vec<String>, bool)>,
        pub choose: Callback<Option<usize>>,
    }

    /// Starts the browser's part.
    pub(super) fn start(w: &Wiring) {
        load_views(w);
        load_glosses(w);
        debounce_threshold(w);
        escape_closes(w.uid, w.choose);
    }

    /// The view's data, fetched once per page and kept (App.tsx lines 61–71, `loadView`); the
    /// view on screen stays until the next is in; then the parallels and the vectorial space,
    /// ahead of time (App.tsx line 192).
    fn load_views(w: &Wiring) {
        let Wiring {
            view,
            views,
            views_in,
            data,
            type_on,
            applied,
            slider,
            pointed,
            th_query,
            failed,
            ..
        } = *w;
        let pairs = StoredValue::new(None::<Arc<Pairs>>);
        let manifest = Arc::clone(&w.manifest);
        let load = move |v: View| load_view(Arc::clone(&manifest), views, views_in, pairs, v);
        // A view's data on screen: its link types all shown, its threshold the address's or
        // its default (App.tsx lines 182–191).
        let install = move |loaded: ViewData| {
            type_on.set(vec![true; loaded.graph.types.len()]);
            match &loaded.graph.sim {
                Some(sim) => {
                    let th = threshold(th_query.get_untracked().as_deref(), sim);
                    applied.set(Some(th));
                    slider.set(hundredths(th));
                }
                None => applied.set(None),
            }
            pointed.set(None);
            failed.set(false);
            data.set(Some(loaded));
        };
        Effect::new(move |_| {
            let v = view.get();
            let load = load.clone();
            spawn_local(async move {
                let Some(loaded) = load(v).await else {
                    // The view's data did not arrive: said over the map (fetch.rs logged why).
                    if view.get_untracked() == v {
                        failed.set(true);
                    }
                    return;
                };
                if view.get_untracked() == v {
                    install(loaded);
                }
                for ahead in [View::Parallels, View::Sim] {
                    let _ = load(ahead).await;
                }
            });
        });
    }

    /// The data of view `v`: kept, else fetched and kept; the pairs fetched once for every
    /// vectorial view.
    async fn load_view(
        manifest: Arc<Manifest>,
        views: StoredValue<HashMap<View, ViewData>>,
        views_in: RwSignal<usize>,
        pairs: StoredValue<Option<Arc<Pairs>>>,
        v: View,
    ) -> Option<ViewData> {
        if let Some(cached) = views.with_value(|views| views.get(&v).cloned()) {
            return Some(cached);
        }
        let graph = fetch::graph(&manifest, graph_file(v)).await?;
        let view_pairs = if let Some(sim) = &graph.sim {
            if let Some(kept) = pairs.get_value() {
                Some(kept)
            } else {
                let fetched = fetch::pairs(&manifest, &sim.url).await?;
                pairs.set_value(Some(Arc::clone(&fetched)));
                Some(fetched)
            }
        } else {
            None
        };
        let loaded = ViewData {
            graph,
            pairs: view_pairs,
        };
        views.update_value(|views| {
            views.insert(v, loaded.clone());
        });
        views_in.update(|n| *n += 1);
        Some(loaded)
    }

    /// The translations, once; the summaries of the glosses' language and of English, each
    /// fetched once (App.tsx lines 78 and 106–112).
    fn load_glosses(w: &Wiring) {
        let Wiring {
            translations,
            summaries,
            gloss,
            failed,
            ..
        } = *w;
        let manifest = Arc::clone(&w.manifest);
        {
            let manifest = Arc::clone(&manifest);
            spawn_local(async move {
                match fetch::translations(&manifest).await {
                    Some(tr) => translations.set(Some(tr)),
                    // Without the titles nothing can be shown or searched: said over the map.
                    None => failed.set(true),
                }
            });
        }
        let kept = StoredValue::new(Vec::<(&'static str, Arc<Blurbs>)>::new());
        let blurbs = move |code: &'static str| {
            let manifest = Arc::clone(&manifest);
            async move {
                let cached = kept.with_value(|kept| {
                    kept.iter()
                        .find(|(c, _)| *c == code)
                        .map(|(_, b)| Arc::clone(b))
                });
                if let Some(cached) = cached {
                    return cached;
                }
                let fetched = fetch::blurbs(&manifest, code).await;
                kept.update_value(|kept| kept.push((code, Arc::clone(&fetched))));
                fetched
            }
        };
        Effect::new(move |_| {
            let language = gloss.get();
            let blurbs = blurbs.clone();
            spawn_local(async move {
                let en = blurbs("en").await;
                let lang = if language == Language::En {
                    Arc::clone(&en)
                } else {
                    blurbs(language.code()).await
                };
                if gloss.get_untracked() == language {
                    summaries.set(Some(Summaries { language, lang, en }));
                }
            });
        });
    }

    /// The threshold applied 200 ms after the slider settles, and written in the address
    /// (App.tsx lines 327–337). A view change cancels a pending one ([`threshold_settles`]):
    /// the old view's threshold is never applied to the new one, nor written in its address.
    fn debounce_threshold(w: &Wiring) {
        let Wiring {
            view,
            uid,
            trail,
            applied,
            slider,
            go,
            ..
        } = *w;
        Effect::new(move |previous: Option<View>| {
            let value = slider.get();
            let now = view.get();
            let view_changed = previous.is_some_and(|before| before != now);
            // On a view change the pending timer is cleared (`on_cleanup` below, as the
            // effect runs again) and none is set.
            if threshold_settles(view_changed, applied.get_untracked(), value) {
                let timer = set_timeout_with_handle(
                    move || {
                        if view.get_untracked() != now {
                            return;
                        }
                        applied.set(Some(f64::from(value) / 100.0));
                        let th = threshold_label(value);
                        let path = address(now, uid.get_untracked().as_deref(), Some(&th));
                        go.run((path, trail.get_untracked(), true));
                    },
                    Duration::from_millis(200),
                )
                .ok();
                on_cleanup(move || {
                    if let Some(timer) = timer {
                        timer.clear();
                    }
                });
            }
            now
        });
    }

    /// Escape closes the open text (App.tsx lines 340–346), unless a field has the focus or
    /// a dialog is open ([`escape_closes_text`]). The listener is on the document, which the
    /// key reaches before the window, where ui-lib's `AppShell` listens: when the map takes
    /// the key it stops it there, so the shell does not also fold its rail (`pc_rail`).
    fn escape_closes(uid: Memo<Option<String>>, choose: Callback<Option<usize>>) {
        let listener =
            Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |ev: web_sys::KeyboardEvent| {
                let text_open = uid.try_with_untracked(Option::is_some).unwrap_or(false);
                if escape_closes_text(&ev.key(), typing(), text_open, dialog_open()) {
                    ev.stop_propagation();
                    choose.run(None);
                }
            });
        let added = document()
            .add_event_listener_with_callback("keydown", listener.as_ref().unchecked_ref())
            .is_ok();
        if added {
            let kept = StoredValue::new_local(Some(listener));
            on_cleanup(move || {
                if let Some(Some(listener)) = kept.try_update_value(Option::take) {
                    let _ = document().remove_event_listener_with_callback(
                        "keydown",
                        listener.as_ref().unchecked_ref(),
                    );
                }
            });
        }
    }

    /// Whether a dialog is open on the page — ui-lib's Settings, its languages, its phone
    /// navigation, a chip's list (each a `role="dialog"`, hidden while closed): one with a
    /// box on screen.
    fn dialog_open() -> bool {
        let Ok(dialogs) = document().query_selector_all(r#"[role="dialog"]"#) else {
            return false;
        };
        (0..dialogs.length()).any(|k| {
            dialogs
                .item(k)
                .and_then(|node| node.dyn_into::<web_sys::Element>().ok())
                .is_some_and(|element| element.get_bounding_client_rect().width() > 0.0)
        })
    }

    /// Whether the focused element takes typing.
    fn typing() -> bool {
        document().active_element().is_some_and(|element| {
            let editable = element
                .dyn_ref::<web_sys::HtmlElement>()
                .is_some_and(web_sys::HtmlElement::is_content_editable);
            is_typing(&element.tag_name(), editable)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 006 review: Escape closed the text and, unstopped, also folded the shell's rail.
    #[test]
    fn escape_closes_an_open_text_only() {
        assert!(escape_closes_text("Escape", false, true, false));
        assert!(
            !escape_closes_text("Escape", true, true, false),
            "a field's own"
        );
        assert!(
            !escape_closes_text("Escape", false, false, false),
            "no text: the shell's"
        );
        assert!(
            !escape_closes_text("Escape", false, true, true),
            "a dialog open: the shell's"
        );
        assert!(!escape_closes_text("Enter", false, true, false));
    }

    /// 006 review R3: a view change left the old view's timer, which wrote its `?th` into
    /// the new view.
    #[test]
    fn a_view_change_never_applies_the_old_threshold() {
        assert!(threshold_settles(false, Some(0.56), 70));
        assert!(!threshold_settles(false, Some(0.56), 56), "already applied");
        assert!(
            !threshold_settles(false, None, 70),
            "a view without a threshold"
        );
        assert!(!threshold_settles(true, Some(0.56), 70), "another view now");
    }

    /// 006 review R3: a uid in no view, reached in the browser, is the 404 as on the server.
    #[test]
    fn a_uid_is_in_no_view_once_both_sets_say_so() {
        let views = |parallels: Option<bool>, sim: Option<bool>| {
            move |v: View| match v {
                View::Parallels => parallels,
                _ => sim,
            }
        };
        assert!(in_no_view(views(Some(false), Some(false))));
        assert!(!in_no_view(views(Some(true), Some(false))));
        assert!(!in_no_view(views(Some(false), Some(true))));
        assert!(
            !in_no_view(views(None, Some(false))),
            "the parallels not in yet"
        );
        assert!(
            !in_no_view(views(Some(false), None)),
            "the vectorial space not in yet"
        );
    }
}
