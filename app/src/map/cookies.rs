//! The map's own preferences, `viz_legend` and `viz_textlang` (design v4), where ui-lib's
//! `Store` says ([`read_in`], [`write_in`]; task 012). On the web (`Store::Cookie`) they are
//! cookies: read by the server for the first render (from the request) and by the browser
//! when it hydrates (from `document.cookie`), so both start alike; written by the browser
//! only. Host-only, unlike ui-lib's `pc_*` cookies shared across PaliCanon's subdomains:
//! they are this map's (never `Store::Cookie.write`, which adds `Domain=palicanon.app`). In
//! the PaliCanon app (`Store::Local`, `csr::mount`) they are `localStorage` entries under the
//! same names.

use palicanon_ui_lib::Store;

/// A year, as ui-lib keeps its preferences.
#[cfg_attr(
    all(feature = "ssr", not(test)),
    expect(dead_code, reason = "the browser writes the cookies")
)]
const MAX_AGE_SECONDS: u32 = 365 * 24 * 3600;

/// The `document.cookie` assignment storing `value` under `name`: a year, the whole site,
/// sent on same-site navigation, for this host only.
#[cfg_attr(
    all(feature = "ssr", not(test)),
    expect(dead_code, reason = "the browser writes the cookies")
)]
pub(crate) fn assignment(name: &str, value: &str) -> String {
    format!("{name}={value}; Path=/; Max-Age={MAX_AGE_SECONDS}; SameSite=Lax")
}

/// The value of the cookie `name` of the request being rendered (the server) or of the page
/// (the browser); `None` when there is none.
pub(crate) fn read(name: &str) -> Option<String> {
    let header = cookie_header()?;
    palicanon_ui_lib::persisted::cookie_value(&header, name).map(str::to_owned)
}

/// The request's `Cookie` header.
#[cfg(feature = "ssr")]
fn cookie_header() -> Option<String> {
    let parts = leptos::prelude::use_context::<axum::http::request::Parts>()?;
    let header = parts.headers.get(axum::http::header::COOKIE)?;
    header.to_str().ok().map(str::to_owned)
}

/// `document.cookie`.
#[cfg(all(feature = "browser", not(feature = "ssr")))]
fn cookie_header() -> Option<String> {
    use wasm_bindgen::JsCast;
    leptos::prelude::document()
        .dyn_into::<web_sys::HtmlDocument>()
        .ok()?
        .cookie()
        .ok()
}

/// Neither a request nor a page (a plain test).
#[cfg(not(any(feature = "ssr", feature = "browser")))]
fn cookie_header() -> Option<String> {
    None
}

/// The preference `name` kept in `store` (`Store::current()` of the map): the cookie
/// ([`read`]), or the `localStorage` entry; `None` when there is none.
pub(crate) fn read_in(store: Store, name: &str) -> Option<String> {
    match store {
        Store::Cookie => read(name),
        Store::Local => Store::Local.read(name),
    }
}

/// Keeps the preference `value` under `name` in `store`: the host-only cookie ([`write`]), or
/// the `localStorage` entry (storage that refuses it keeps the choice for the page only).
pub(crate) fn write_in(store: Store, name: &str, value: &str) {
    match store {
        Store::Cookie => write(name, value),
        Store::Local => {
            let _ = Store::Local.write(name, value);
        }
    }
}

/// Stores `value` under `name`, in the browser; nothing on the server.
fn write(name: &str, value: &str) {
    #[cfg(all(feature = "browser", not(feature = "ssr")))]
    {
        use wasm_bindgen::JsCast;
        if let Ok(document) = leptos::prelude::document().dyn_into::<web_sys::HtmlDocument>() {
            let _ = document.set_cookie(&assignment(name, value));
        }
    }
    #[cfg(not(all(feature = "browser", not(feature = "ssr"))))]
    let _ = (name, value);
}

#[cfg(test)]
mod tests {
    use palicanon_ui_lib::Language;
    use palicanon_ui_lib::persisted::cookie_value;

    use super::*;
    use crate::map::legend::{self, Legend};
    use crate::map::texts_lang::{self, TextsLang, cookie_value as textlang_value, texts_lang};

    /// What the browser stores is what the server reads back: each cookie written, then found
    /// in a `Cookie` header among others.
    #[test]
    fn a_written_cookie_reads_back() {
        let mut state = Legend::default();
        state.toggle_colored("mn");
        state.toggle_hidden("vinaya");
        state.open = false;
        let legend_set = assignment(legend::COOKIE, &state.to_cookie());
        assert_eq!(
            legend_set,
            "viz_legend=mn|vinaya|0; Path=/; Max-Age=31536000; SameSite=Lax"
        );
        let choice = TextsLang::Translated(Language::Fr);
        let textlang_set = assignment(texts_lang::COOKIE, &textlang_value(choice, Language::En));
        // A browser sends back `name=value` of each assignment.
        let pair = |set: &str| set.split(';').next().unwrap_or_default().to_owned();
        let header = format!("pc_lang=en; {}; {}", pair(&legend_set), pair(&textlang_set));
        assert_eq!(Legend::parse(cookie_value(&header, legend::COOKIE)), state);
        assert_eq!(
            texts_lang(cookie_value(&header, texts_lang::COOKIE), Language::En),
            choice
        );
        assert!(!legend_set.contains("Domain"), "host-only: {legend_set}");
    }
}
