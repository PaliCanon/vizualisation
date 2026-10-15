//! The not-found page: rendered for any path the route table does not know
//! (`crate::route::Route::parse` gives `None`). A centred column with the
//! lotus and the brand linking to the map, the eyebrow, the title, the
//! body, and a gold underlined link to the map.
//!
//! From search-front/app/src/components/not_found.rs (commit 6d370b4).
//! What differs: the brand is the catalogue's ("Suttas Map"), as the React
//! `NotFoundPage` (`src/components/Pages.tsx`); the head title is
//! `head_notfound_title`; on the server it sets the response's status to
//! 404 itself, since a path the router matches (`/:uid`) can still be
//! unknown to the table (`/MN10`).

use leptos::prelude::*;
use leptos_i18n::td_string;
use leptos_meta::{Meta, Title};

use crate::catalogue::strings;

/// The not-found page: a centred column with the brand, eyebrow, title,
/// body, and home link. Sets the page title, disallows indexing.
#[component]
pub(crate) fn NotFoundPage() -> impl IntoView {
    #[cfg(feature = "ssr")]
    if let Some(response) = use_context::<leptos_axum::ResponseOptions>() {
        response.set_status(axum::http::StatusCode::NOT_FOUND);
    }
    let t = strings();
    move || {
        let locale = t.locale();
        view! {
            <Title text=td_string!(locale, head_notfound_title) />
            <Meta name="robots" content="noindex" />
            <div class="mx-auto flex min-h-dvh max-w-[64rem] flex-col items-center justify-center px-6 text-center">
                <a
                    href="/"
                    class="mb-6 flex items-center gap-2 font-serif text-lg font-semibold text-heading"
                >
                    <img src="/lotus.png" alt="" width="22" height="22" />
                    {td_string!(locale, brand)}
                </a>
                <p class="mb-2 font-mono text-xs tracking-widest text-subtle">
                    {td_string!(locale, notfound_eyebrow)}
                </p>
                <h1 class="font-serif text-4xl font-semibold text-heading">
                    {td_string!(locale, notfound_title)}
                </h1>
                <p class="mt-4 max-w-[42ch] text-[15px] leading-7 text-ink">
                    {td_string!(locale, notfound_body)}
                </p>
                <a
                    href="/"
                    class="mt-6 text-gold underline underline-offset-4 hover:text-gold-hover"
                >
                    {td_string!(locale, notfound_home)}
                </a>
            </div>
        }
    }
}
