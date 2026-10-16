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

/// `sitemap.xml`: the map's home, the footer pages, then every text of
/// `texts` once, at its canonical address (`TextIndex::canonical`, the rule
/// of `open_text` the first render's head follows: `/mn10`, `/sim/ds1.1`).
#[cfg(feature = "ssr")]
#[must_use]
pub(crate) fn sitemap(texts: &crate::data::TextIndex) -> String {
    use crate::map::address::address;
    let pages = std::iter::once("/")
        .chain(PageKind::ALL.map(PageKind::path))
        .map(str::to_owned);
    let texts = texts.iter().filter_map(|(uid, _)| {
        let view = texts.canonical(uid)?;
        Some(address(view, Some(uid), None))
    });
    let urls = pages.chain(texts).fold(String::new(), |mut urls, path| {
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

    /// The committed data's text index, as the server builds it at boot.
    #[cfg(feature = "ssr")]
    fn texts() -> crate::data::TextIndex {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../public");
        let data = crate::data::Data::load(&dir).expect("the committed data");
        crate::data::TextIndex::build(&data)
    }

    /// A text's URL in the sitemap is its page's canonical: the address of the
    /// view the server's first render names canonical (the head's `<link
    /// rel="canonical">`), for texts of every kind (the parallels', the
    /// vectorial space's only), asked from any view. One uid in 40 of the
    /// index, plus `mn10` and `ds1.1`: a first render is a panel each.
    #[cfg(feature = "ssr")]
    #[test]
    fn a_texts_sitemap_url_is_its_pages_canonical() {
        use crate::map::address::address;
        use crate::map::first::{FirstRequest, first_render};
        use crate::map::texts_lang::TextsLang;
        use crate::route::View;
        use palicanon_ui_lib::Language;

        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../public");
        let served = crate::data::Served::new(std::sync::Arc::new(
            crate::data::Data::load(&dir).expect("the committed data"),
        ));
        let xml = sitemap(&served.texts);
        let uids: Vec<&str> = served
            .texts
            .iter()
            .map(|(uid, _)| uid)
            .step_by(40)
            .chain(["mn10", "ds1.1"])
            .collect();
        for (k, uid) in uids.iter().enumerate() {
            let view = View::ALL[k % View::ALL.len()];
            let path = address(view, Some(uid), None);
            let first = first_render(
                &served,
                &FirstRequest {
                    path: &path,
                    view,
                    uid: Some(uid),
                    th: None,
                    language: Language::En,
                    texts: TextsLang::Pali,
                    light: false,
                },
            );
            let canonical = first.opened().expect(uid).canonical;
            let loc = format!(
                "<loc>{CANONICAL_BASE}{}</loc>",
                address(canonical, Some(uid), None)
            );
            assert_eq!(xml.matches(&loc).count(), 1, "{uid} from {path}: {loc}");
        }
    }

    /// The home and every page once, then every text once at its canonical
    /// address — its first view (`/mn10` for a text of the parallels,
    /// `/sim/ds1.1` for one of the vectorial space only) —, an address the
    /// route table reads back as that text. Mutations it catches: a text
    /// left out, listed twice, or at another view than its first.
    #[cfg(feature = "ssr")]
    #[test]
    fn the_sitemap_lists_each_text_once_at_its_canonical() {
        use crate::route::{Route, View};
        let texts = texts();
        let xml = sitemap(&texts);
        assert!(
            xml.starts_with(r#"<?xml version="1.0" encoding="UTF-8"?>"#),
            "{xml}"
        );
        let mut expected = vec![format!("{CANONICAL_BASE}/")];
        expected.extend(PageKind::ALL.map(|kind| format!("{CANONICAL_BASE}{}", kind.path())));
        for (uid, entry) in texts.iter() {
            let view = entry.views[0].0;
            let path = Route::Map {
                view,
                uid: Some(uid.to_owned()),
            }
            .path();
            assert_eq!(
                Route::parse(&path),
                Some(Route::Map {
                    view,
                    uid: Some(uid.to_owned())
                }),
                "{path}"
            );
            expected.push(format!("{CANONICAL_BASE}{path}"));
        }
        for url in &expected {
            let loc = format!("<loc>{url}</loc>");
            assert_eq!(xml.matches(&loc).count(), 1, "{loc}");
        }
        assert_eq!(xml.matches("<loc>").count(), expected.len());
        assert!(xml.contains(&format!("<loc>{CANONICAL_BASE}/mn10</loc>")));
        assert!(xml.contains(&format!("<loc>{CANONICAL_BASE}/sim/ds1.1</loc>")));
        assert!(!xml.contains(&format!("<loc>{CANONICAL_BASE}/sim/mn10</loc>")));
        assert!(
            texts.iter().any(|(_, e)| e.views[0].0 == View::Sim),
            "some texts are sim-only"
        );
        assert_eq!(texts.iter().count(), 8268, "every uid of the five views");
    }
}
