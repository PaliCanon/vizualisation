//! The search box (App.tsx lines 416–433, `src/components/SearchBar.tsx`): ui-lib's
//! `SearchRow` (the same palicanon.app bar the React page copied), searched as the reader
//! types; the results under it, the arrows moving through them, Enter opening the active one,
//! Escape clearing; `/` or ⌘K (Ctrl+K) focusing the field from anywhere. The ranking is the
//! wiring's (`crate::map::search::hits`), given here as `hits`.
//!
//! Without JavaScript the form still submits (`GET ?q=`), as every `SearchRow` does.

use leptos::html;
use leptos::prelude::*;
use leptos_i18n::td_string;
use palicanon_ui_lib::SearchRow;

use crate::catalogue::strings;
use crate::map::ui::list_keys::list_move;
use crate::map::ui::text_ref::TextRow;
use crate::map::ui::{CARD, TextItem};

/// What a key pressed in the search box does (App.tsx lines 419–423).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchKey {
    /// Make this result the active one.
    Move(usize),
    /// Empty the field and leave it.
    Clear,
    /// Nothing of ours.
    Ignore,
}

/// What `key` does with result `active` of `hits` active: the arrows move as in the chips'
/// menus ([`list_move`]: wrapping around, nothing in an empty list); Escape clears.
pub fn search_key(key: &str, active: usize, hits: usize) -> SearchKey {
    if key == "Escape" {
        return SearchKey::Clear;
    }
    list_move(key, active, hits).map_or(SearchKey::Ignore, SearchKey::Move)
}

/// The node Enter opens: the active result's, if there is one (App.tsx line 425,
/// `hits[active] !== undefined && pick(hits[active])`).
pub fn submitted(active: usize, hits: &[TextItem]) -> Option<usize> {
    hits.get(active).map(|item| item.index)
}

/// Whether a key focuses the field from anywhere (SearchBar.tsx lines 22–24): ⌘K or Ctrl+K,
/// or `/` when the reader is not typing in a field.
pub fn focus_shortcut(key: &str, meta_or_ctrl: bool, typing: bool) -> bool {
    (key == "k" && meta_or_ctrl) || (key == "/" && !typing)
}

/// The search box and its results.
#[component]
pub fn SearchBox(
    /// The results of the current query, best first.
    #[prop(into)]
    hits: Signal<Vec<TextItem>>,
    /// The field's text changed (emptied after a pick or Escape).
    on_query: Callback<String>,
    /// A result was chosen: its node.
    on_pick: Callback<usize>,
) -> impl IntoView {
    let input: NodeRef<html::Input> = NodeRef::new();
    let active = RwSignal::new(0_usize);
    // New results: the first is the active one again (App.tsx line 374).
    Effect::new(move |_| {
        hits.track();
        active.set(0);
    });
    focus_on_shortcut(input);
    let clear = move || {
        clear_field(input);
        on_query.run(String::new());
    };
    let pick = Callback::new(move |i: usize| {
        on_pick.run(i);
        clear();
    });
    let on_key = move |ev: leptos::ev::KeyboardEvent| {
        let len = hits.with_untracked(Vec::len);
        match search_key(&ev.key(), active.get_untracked(), len) {
            SearchKey::Move(k) => {
                ev.prevent_default();
                active.set(k);
            }
            SearchKey::Clear => {
                // The field's own: the shell's rail and the open text stay as they are.
                ev.stop_propagation();
                clear();
            }
            SearchKey::Ignore => {}
        }
    };
    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        if let Some(i) = hits.with_untracked(|hits| submitted(active.get_untracked(), hits)) {
            pick.run(i);
        }
    };
    view! {
        <div
            class="relative min-w-0"
            on:keydown=on_key
            on:input=move |ev| on_query.run(event_target_value(&ev))
            on:submit=on_submit
        >
            <SearchField input=input />
            <SearchHits hits=hits active=active pick=pick />
        </div>
    }
}

/// The field: ui-lib's `SearchRow`, labelled from the catalogue.
#[component]
fn SearchField(input: NodeRef<html::Input>) -> impl IntoView {
    let t = strings();
    view! {
        <SearchRow
            value=String::new()
            label=Signal::derive(move || td_string!(t.locale(), search_label).to_owned())
            placeholder=Signal::derive(move || {
                td_string!(t.locale(), search_placeholder).to_owned()
            })
            submit_label=Signal::derive(move || {
                td_string!(t.locale(), search_submit).to_owned()
            })
            form_id="viz-search"
            input_ref=input
        />
    }
}

/// The results under the field, while there are any (App.tsx lines 426–432).
#[component]
fn SearchHits(
    hits: Signal<Vec<TextItem>>,
    active: RwSignal<usize>,
    pick: Callback<usize>,
) -> impl IntoView {
    move || {
        let items = hits.get();
        (!items.is_empty()).then(|| {
            let rows = items
                .into_iter()
                .enumerate()
                .map(|(k, item)| {
                    let i = item.index;
                    view! {
                        <TextRow
                            item=item
                            active=Signal::derive(move || active.get() == k)
                            on_click=Callback::new(move |()| pick.run(i))
                            on_enter=Callback::new(move |()| active.set(k))
                        />
                    }
                })
                .collect_view();
            view! {
                <ul
                    role="listbox"
                    class=format!(
                        "absolute inset-x-0 z-20 mt-2 max-h-[50vh] overflow-auto p-1 {CARD}",
                    )
                >
                    {rows}
                </ul>
            }
        })
    }
}

/// Empties the field and leaves it (App.tsx lines 381–385, `pick`; line 422, Escape).
fn clear_field(input: NodeRef<html::Input>) {
    #[cfg(feature = "hydrate")]
    if let Some(el) = input.get_untracked() {
        el.set_value("");
        let _ = el.blur();
    }
    #[cfg(not(feature = "hydrate"))]
    let _ = input;
}

/// `/` and ⌘K (Ctrl+K) focus the field from anywhere, its text selected (SearchBar.tsx lines
/// 20–31). The browser only.
fn focus_on_shortcut(input: NodeRef<html::Input>) {
    #[cfg(feature = "hydrate")]
    {
        let handle = window_event_listener(leptos::ev::keydown, move |ev| {
            let typing = ev
                .target()
                .and_then(|t| wasm_bindgen::JsCast::dyn_into::<leptos::web_sys::Element>(t).ok())
                .is_some_and(|el| {
                    matches!(el.tag_name().as_str(), "INPUT" | "TEXTAREA" | "SELECT")
                });
            if focus_shortcut(&ev.key(), ev.meta_key() || ev.ctrl_key(), typing)
                && let Some(el) = input.get_untracked()
            {
                ev.prevent_default();
                let _ = el.focus();
                el.select();
            }
        });
        on_cleanup(move || handle.remove());
    }
    #[cfg(not(feature = "hydrate"))]
    let _ = input;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arrows_move_inside_the_results_escape_clears() {
        // The menus' keys (006 review R4: one helper): the arrows move, wrapping around.
        assert_eq!(search_key("ArrowDown", 0, 3), SearchKey::Move(1));
        assert_eq!(search_key("ArrowDown", 2, 3), SearchKey::Move(0));
        assert_eq!(search_key("ArrowUp", 2, 3), SearchKey::Move(1));
        assert_eq!(search_key("ArrowUp", 0, 3), SearchKey::Move(2), "wraps");
        assert_eq!(
            search_key("ArrowDown", 0, 0),
            SearchKey::Ignore,
            "no results"
        );
        assert_eq!(search_key("ArrowUp", 0, 0), SearchKey::Ignore, "no results");
        // App.tsx:422 `else if (e.key === "Escape") { setQuery(""); input.current?.blur(); }`
        assert_eq!(search_key("Escape", 1, 3), SearchKey::Clear);
        assert_eq!(
            search_key("Enter", 1, 3),
            SearchKey::Ignore,
            "the form's submit"
        );
        assert_eq!(search_key("m", 1, 3), SearchKey::Ignore);
    }

    #[test]
    fn enter_opens_the_active_result() {
        use palicanon_ui_lib::Language;

        use crate::map::ui::test_support::Fixture;

        // App.tsx:425 `onSubmit={() => hits[active] !== undefined && pick(hits[active]!)}`
        let f = Fixture::new();
        let texts = f.texts(Language::En);
        let hits = vec![texts.item(1), texts.item(2)];
        assert_eq!(submitted(0, &hits), Some(1));
        assert_eq!(submitted(1, &hits), Some(2));
        assert_eq!(submitted(2, &hits), None);
        assert_eq!(submitted(0, &[]), None);
    }

    #[test]
    fn slash_or_command_k_focuses_the_field() {
        // SearchBar.tsx:23 `(e.key === "k" && (e.metaKey || e.ctrlKey)) || (e.key === "/" && !typing)`
        assert!(focus_shortcut("k", true, false));
        assert!(focus_shortcut("k", true, true));
        assert!(!focus_shortcut("k", false, false));
        assert!(focus_shortcut("/", false, false));
        assert!(!focus_shortcut("/", false, true), "typing a slash");
        assert!(!focus_shortcut("a", true, false));
    }

    #[cfg(feature = "ssr")]
    fn render_box(language: palicanon_ui_lib::Language, hits: Vec<TextItem>) -> String {
        use crate::map::ui::test_support::{plain, render};

        plain(&render(language, move || {
            view! { <SearchBox hits=hits on_query=Callback::new(|_| {}) on_pick=Callback::new(|_| {}) /> }
        }))
    }

    #[cfg(feature = "ssr")]
    #[test]
    fn the_box_is_labelled_in_each_language() {
        use palicanon_ui_lib::Language;

        // SearchBar.tsx:47 `t("search.label")`, 56 `t("search.placeholder")`, 64 `t("search.submit")`
        for (language, words) in [
            (
                Language::En,
                [
                    "Search the suttas",
                    "Find a text: MN 10, sn56.11, Satipaṭṭhāna…",
                    "Search",
                ],
            ),
            (
                Language::Fr,
                [
                    "Chercher dans les suttas",
                    "Trouver un texte : MN 10, sn56.11, Satipaṭṭhāna…",
                    "Chercher",
                ],
            ),
            (
                Language::Vi,
                [
                    "Tìm trong các bài kinh",
                    "Tìm một bài kinh: MN 10, sn56.11, Satipaṭṭhāna…",
                    "Tìm",
                ],
            ),
        ] {
            let html = render_box(language, Vec::new());
            for word in words {
                assert!(html.contains(word), "{language:?} {word}: {html}");
            }
            assert!(html.contains(r#"role="search""#), "{html}");
            assert!(!html.contains("<ul"), "no results, no list: {html}");
        }
    }

    #[cfg(feature = "ssr")]
    #[test]
    fn the_results_are_listed_the_first_active() {
        use palicanon_ui_lib::Language;

        use crate::map::ui::test_support::{Fixture, classes, tag_around};

        // App.tsx:427-431 one `TextRow` per hit, `active={k === active}`
        let f = Fixture::new();
        let texts = f.texts(Language::Fr);
        let html = render_box(Language::Fr, vec![texts.item(1), texts.item(3)]);
        assert_eq!(html.matches("<li").count(), 2, "{html}");
        // 006 review R4: each result an option of the results' listbox.
        assert!(
            crate::map::ui::test_support::options_in_listboxes(&html),
            "{html}"
        );
        assert!(html.contains(">MN 1<") && html.contains(">MN 2<"), "{html}");
        assert!(html.contains("Les souillures"), "{html}");
        let rows: Vec<&str> = html
            .match_indices("<li")
            .map(|(at, _)| &html[at..])
            .collect();
        assert!(classes(&tag_around(rows[0], "<li")).contains(&"bg-chip-hover"));
        assert!(!classes(&tag_around(rows[1], "<li")).contains(&"bg-chip-hover"));
    }
}
