//! The document head of every route (from the React site's `src/head.ts` and
//! `index.html`): [`PageHead`] sets a route's title, description, canonical
//! link and their Open Graph mirrors; [`SiteHead`] holds what every page shares
//! (icons, the share image, the JSON-LD). [`sitemap`] lists the indexable
//! paths. Every canonical is built from [`CANONICAL_BASE`]: there is only ever
//! one production host.

#[cfg(feature = "ssr")]
use std::fmt::Write as _;

use leptos::prelude::*;
use leptos_meta::{Link, Meta, Script, Title};

#[cfg(feature = "ssr")]
use crate::pages::PageKind;

/// The production origin every canonical and `og:url` starts with.
pub const CANONICAL_BASE: &str = "https://map.palicanon.app";

/// A route's title, description and canonical path (`/`, `/terms`, `/sim/mn10`),
/// with their `og:*` mirrors.
#[component]
pub(crate) fn PageHead(title: String, description: String, path: String) -> impl IntoView {
    let url = format!("{CANONICAL_BASE}{path}");
    view! {
        <Title text=title.clone() />
        <Meta name="description" content=description.clone() />
        <Link rel="canonical" href=url.clone() />
        <Meta property="og:title" content=title />
        <Meta property="og:description" content=description />
        <Meta property="og:url" content=url />
    }
}

/// What every page's head shares: the icons, the share image and card, the app
/// banner, the theme colour and the JSON-LD (React's `index.html`).
#[component]
pub(crate) fn SiteHead() -> impl IntoView {
    view! {
        // Google wants a square icon whose side is a multiple of 48 px.
        <Link rel="icon" href="/favicon.ico" sizes="48x48" />
        <Link rel="icon" type_="image/png" sizes="48x48" href="/favicon-48.png" />
        <Link rel="icon" type_="image/png" sizes="96x96" href="/favicon-96.png" />
        <Link rel="icon" type_="image/png" sizes="192x192" href="/favicon-192.png" />
        <Link rel="apple-touch-icon" href="/apple-touch-icon.png" />
        <Meta name="apple-itunes-app" content="app-id=6760995639" />
        <Meta name="theme-color" content="#161a22" />
        <Meta property="og:type" content="website" />
        <Meta property="og:site_name" content="Suttas Map" />
        <Meta property="og:image" content=format!("{CANONICAL_BASE}/og.png") />
        <Meta property="og:image:width" content="1200" />
        <Meta property="og:image:height" content="630" />
        <Meta
            property="og:image:alt"
            content="The suttas of the Pāli Canon as golden points, joined by SuttaCentral's parallels."
        />
        <Meta name="twitter:card" content="summary_large_image" />
        <Script type_="application/ld+json">{JSON_LD}</Script>
    }
}

/// The site's structured data (React's `index.html`), one line.
const JSON_LD: &str = concat!(
    r#"{"@context":"https://schema.org","@type":"WebApplication","name":"Suttas Map","#,
    r#""url":"https://map.palicanon.app/","description":"A 3D map of the Pāli Canon: "#,
    r#"SuttaCentral's parallels between the suttas, and the texts that resemble each other "#,
    r#"in Pāli.","applicationCategory":"EducationalApplication","operatingSystem":"Any","#,
    r#""inLanguage":["en","fr","vi","pi"],"isAccessibleForFree":true,"#,
    r#""offers":{"@type":"Offer","price":"0","priceCurrency":"USD"},"#,
    r#""isPartOf":{"@type":"WebSite","name":"PaliCanon","url":"https://palicanon.app/"}}"#,
);

/// `sitemap.xml`: the map's home and the footer pages (the texts come with the
/// map screen, task 006), each at its canonical URL.
#[cfg(feature = "ssr")]
#[must_use]
pub fn sitemap() -> String {
    let paths = std::iter::once("/").chain(PageKind::ALL.map(PageKind::path));
    let urls = paths.fold(String::new(), |mut urls, path| {
        let _ = writeln!(
            urls,
            "  <url>\n    <loc>{CANONICAL_BASE}{path}</loc>\n  </url>"
        );
        urls
    });
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n{urls}</urlset>\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The public host is map.palicanon.app (user, 2026-10-16): the canonical
    /// base, the JSON-LD's url and robots.txt's sitemap all name it.
    #[test]
    fn the_canonical_host_is_map_palicanon_app() {
        assert_eq!(CANONICAL_BASE, "https://map.palicanon.app");
        assert!(
            JSON_LD.contains(&format!(r#""url":"{CANONICAL_BASE}/""#)),
            "{JSON_LD}"
        );
        let robots = include_str!("../public/robots.txt");
        assert!(
            robots
                .lines()
                .any(|line| line == format!("Sitemap: {CANONICAL_BASE}/sitemap.xml")),
            "{robots}"
        );
    }

    #[cfg(feature = "ssr")]
    #[test]
    fn the_sitemap_lists_the_home_and_every_page_once() {
        let xml = sitemap();
        assert!(
            xml.starts_with(r#"<?xml version="1.0" encoding="UTF-8"?>"#),
            "{xml}"
        );
        let mut expected = vec![format!("{CANONICAL_BASE}/")];
        expected.extend(PageKind::ALL.map(|kind| format!("{CANONICAL_BASE}{}", kind.path())));
        for url in &expected {
            let loc = format!("<loc>{url}</loc>");
            assert_eq!(xml.matches(&loc).count(), 1, "{loc} in {xml}");
        }
        assert_eq!(xml.matches("<loc>").count(), expected.len(), "{xml}");
    }
}
