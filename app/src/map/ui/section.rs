//! A titled section of the text panel that folds (`src/components/Section.tsx`): closed until
//! the reader opens it; open or closed is remembered per section in the browser's
//! localStorage (`suttasmap.section.v2.<id>`, the React page's key, so a reader's choices
//! carry over), and only once the reader has toggled it. The content is rendered folded too
//! (`hidden`), so the server's page lists every neighbour; the browser reads the remembered
//! state after hydrating, so the first render is the same on both sides.

use leptos::prelude::*;
use palicanon_ui_lib::{Icon, IconName};

/// The localStorage key of section `id` (Section.tsx line 17).
pub fn storage_key(id: &str) -> String {
    format!("suttasmap.section.v2.{id}")
}

/// Whether a stored value says open: `"1"` only (Section.tsx line 19); anything else, or
/// nothing, is closed.
pub fn stored_open(value: Option<&str>) -> bool {
    value == Some("1")
}

/// The value stored for `open` (Section.tsx line 22).
pub fn stored_value(open: bool) -> &'static str {
    if open { "1" } else { "0" }
}

/// A folding section: a button with a chevron, the title and the count, then the content.
#[component]
pub fn Section(
    /// Names the section in localStorage: the link type (`full`, `similar`…).
    #[prop(into)]
    id: String,
    /// The title.
    #[prop(into)]
    title: Signal<String>,
    /// How many items it holds, after the title.
    #[prop(optional)]
    count: Option<usize>,
    /// The content.
    children: Children,
) -> impl IntoView {
    let open = RwSignal::new(false);
    let key = storage_key(&id);
    #[cfg(feature = "hydrate")]
    {
        // After hydrating: the server rendered it closed, as the browser's first render.
        let key = key.clone();
        Effect::new(move |_| open.set(stored_open(storage::get(&key).as_deref())));
    }
    let toggle = move |_| {
        let next = !open.get_untracked();
        open.set(next);
        #[cfg(feature = "hydrate")]
        storage::set(&key, stored_value(next));
        #[cfg(not(feature = "hydrate"))]
        let _ = &key;
    };
    view! {
        <section>
            <button
                type="button"
                on:click=toggle
                aria-expanded=move || open.get().to_string()
                class="flex w-full cursor-pointer items-center gap-1.5 px-4 pb-1 pt-4 text-[11px] font-medium uppercase tracking-wider text-subtle transition hover:text-heading"
            >
                <span class="transition-transform" class:rotate-90=move || open.get()>
                    <Icon name=IconName::ChevronRight size=12 />
                </span>
                {move || title.get()}
                {count
                    .map(|count| {
                        view! { <span class="tabular-nums">{format!("· {count}")}</span> }
                    })}
            </button>
            <div hidden=move || !open.get()>{children()}</div>
        </section>
    }
}

/// The browser's localStorage, read and written as the React page did: a private window
/// that refuses it reads nothing and keeps nothing.
#[cfg(feature = "hydrate")]
mod storage {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(catch, js_namespace = localStorage, js_name = getItem)]
        fn get_item(key: &str) -> Result<Option<String>, JsValue>;
        #[wasm_bindgen(catch, js_namespace = localStorage, js_name = setItem)]
        fn set_item(key: &str, value: &str) -> Result<(), JsValue>;
    }

    /// The value stored under `key`, if any and readable.
    pub(super) fn get(key: &str) -> Option<String> {
        get_item(key).ok().flatten()
    }

    /// Stores `value` under `key`, when the browser lets it.
    pub(super) fn set(key: &str, value: &str) {
        let _ = set_item(key, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_section_is_closed_unless_stored_open() {
        // Section.tsx:17-19 `localStorage.getItem(`suttasmap.section.v2.${id}`) === "1"`
        assert_eq!(storage_key("full"), "suttasmap.section.v2.full");
        assert!(stored_open(Some("1")));
        assert!(!stored_open(Some("0")));
        assert!(!stored_open(Some("open")));
        assert!(!stored_open(None));
        // Section.tsx:22 `localStorage.setItem(key, o ? "0" : "1")`
        assert_eq!(stored_value(true), "1");
        assert_eq!(stored_value(false), "0");
    }

    #[cfg(feature = "ssr")]
    #[test]
    fn a_section_renders_folded_with_its_title_and_count() {
        use palicanon_ui_lib::Language;

        use crate::map::ui::test_support::{plain, render, tag_around};

        // Section.tsx:28-34 `aria-expanded={open}`, `{title}`, `· {count}`
        let html = plain(&render(Language::En, || {
            view! {
                <Section id="full" title="Full parallel".to_owned() count=2>
                    <p>"inside"</p>
                </Section>
            }
        }));
        let button = tag_around(&html, "aria-expanded");
        assert!(button.contains(r#"aria-expanded="false""#), "{button}");
        assert!(html.contains("Full parallel"), "{html}");
        assert!(html.contains("· 2"), "{html}");
        // Folded, yet in the page: the server lists every neighbour.
        assert!(html.contains("inside"), "{html}");
        assert!(html.contains("<div hidden"), "{html}");
    }
}
