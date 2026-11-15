//! The five pages that live on palicanon.app: Acknowledgements, Terms, Privacy, Legal and
//! Support (the user's decision: the legal and info pages are the main site's, GDPR-updated
//! there; search-front's task 037 did the same).
//!
//! The footer links to them directly ([`url`]), and the server answers their old addresses
//! on this site with a 301 to the same page in the visitor's language ([`routes`], `ssr`
//! only): the language every page of the site is rendered in (ui-lib's `resolve_language`:
//! cookie `pc_lang`, then `Accept-Language`, then English). palicanon.app localises by path
//! prefix (`/fr/`, `/vi/`; English has none) and serves each page under a trailing slash
//! (its nginx redirects the form without one), so the target is the slash form.

use leptos_i18n::td_string;
use palicanon_ui_lib::Language;

use crate::catalogue::Locale;

/// One moved page, in the footer's order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Page {
    /// Who made the texts the map shows, and their licences.
    Acknowledgements,
    /// The terms of use.
    Terms,
    /// The privacy notice.
    Privacy,
    /// The legal notice: the publisher and the host.
    Legal,
    /// The costs, and how to help.
    Support,
}

impl Page {
    /// Every page, in the footer's order.
    pub(crate) const ALL: [Self; 5] = [
        Self::Acknowledgements,
        Self::Terms,
        Self::Privacy,
        Self::Legal,
        Self::Support,
    ];

    /// The page's name in a path, here and on palicanon.app.
    #[cfg(any(feature = "ssr", test))]
    pub(crate) const fn slug(self) -> &'static str {
        match self {
            Self::Acknowledgements => "acknowledgements",
            Self::Terms => "terms",
            Self::Privacy => "privacy",
            Self::Legal => "legal",
            Self::Support => "support",
        }
    }

    /// The page's link label in the footer (`footer_nav_*`).
    pub(crate) fn label(self, locale: Locale) -> &'static str {
        match self {
            Self::Acknowledgements => td_string!(locale, footer_nav_acknowledgements),
            Self::Terms => td_string!(locale, footer_nav_terms),
            Self::Privacy => td_string!(locale, footer_nav_privacy),
            Self::Legal => td_string!(locale, footer_nav_legal),
            Self::Support => td_string!(locale, footer_nav_support),
        }
    }
}

/// The page on palicanon.app in `language`: absolute, with its trailing slash. Exhaustive:
/// a new [`Language`] does not build until it has its prefix on the main site.
pub(crate) const fn url(language: Language, page: Page) -> &'static str {
    match (language, page) {
        (Language::En, Page::Acknowledgements) => "https://palicanon.app/acknowledgements/",
        (Language::En, Page::Terms) => "https://palicanon.app/terms/",
        (Language::En, Page::Privacy) => "https://palicanon.app/privacy/",
        (Language::En, Page::Legal) => "https://palicanon.app/legal/",
        (Language::En, Page::Support) => "https://palicanon.app/support/",
        (Language::Fr, Page::Acknowledgements) => "https://palicanon.app/fr/acknowledgements/",
        (Language::Fr, Page::Terms) => "https://palicanon.app/fr/terms/",
        (Language::Fr, Page::Privacy) => "https://palicanon.app/fr/privacy/",
        (Language::Fr, Page::Legal) => "https://palicanon.app/fr/legal/",
        (Language::Fr, Page::Support) => "https://palicanon.app/fr/support/",
        (Language::Vi, Page::Acknowledgements) => "https://palicanon.app/vi/acknowledgements/",
        (Language::Vi, Page::Terms) => "https://palicanon.app/vi/terms/",
        (Language::Vi, Page::Privacy) => "https://palicanon.app/vi/privacy/",
        (Language::Vi, Page::Legal) => "https://palicanon.app/vi/legal/",
        (Language::Vi, Page::Support) => "https://palicanon.app/vi/support/",
    }
}

/// The 301s: each page's path, with and without a trailing slash, for GET (and HEAD),
/// answered before any page is rendered.
#[cfg(feature = "ssr")]
pub(crate) fn routes<S>() -> axum::Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    use axum::routing::get;

    Page::ALL
        .into_iter()
        .fold(axum::Router::new(), |router, page| {
            let slug = page.slug();
            router
                .route(
                    &format!("/{slug}"),
                    get(move |request| async move { moved(page, request) }),
                )
                .route(
                    &format!("/{slug}/"),
                    get(move |request| async move { moved(page, request) }),
                )
        })
}

/// The redirect: a permanent one, but one that depends on the request's language, so a
/// cache must ask again (`no-cache`, `Vary`). The query string is not carried over.
#[cfg(feature = "ssr")]
fn moved(page: Page, request: axum::extract::Request) -> axum::response::Response {
    use axum::http::{StatusCode, header};
    use axum::response::IntoResponse;

    let (parts, _body) = request.into_parts();
    let language = palicanon_ui_lib::language::resolve_language(&parts);
    (
        StatusCode::MOVED_PERMANENTLY,
        [
            (header::LOCATION, url(language, page)),
            (header::CACHE_CONTROL, "no-cache"),
            (header::VARY, "Cookie, Accept-Language"),
        ],
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The table agrees with the rule it abbreviates: the prefix by language (none for
    /// English), the slug, the slash, one host.
    #[test]
    fn every_url_is_the_prefix_the_slug_and_a_trailing_slash() {
        for language in Language::ALL {
            for page in Page::ALL {
                let prefix = match language {
                    Language::En => String::new(),
                    other => format!("/{}", other.code()),
                };
                let want = format!("https://palicanon.app{prefix}/{}/", page.slug());
                assert_eq!(url(language, page), want, "{language:?} {page:?}");
            }
        }
    }
}
