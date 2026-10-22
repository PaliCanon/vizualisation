//! The service's shell: the HTML document ([`shell`], server-rendered), the
//! app root ([`App`], what hydration mounts), and `AppShell`'s rail with the
//! services Search, Learn and Map (this one), its routes (the map's views,
//! the footer pages, the 404: `crate::route`), the footer under each
//! (`SiteFooter`) carrying the release badge. From search-front/app/src/app.rs
//! (commit 6d370b4) and learn-pali/app/src/app.rs; what differs: three rail
//! entries, and one view for every route, rendering what the typed table
//! says (`RoutedPage`).

use leptos::prelude::*;
use leptos_i18n::td_string;
use leptos_meta::provide_meta_context;
#[cfg(feature = "ssr")]
use leptos_meta::{HashedStylesheet, MetaTags};
use leptos_router::components::{Route, Router, Routes};
use leptos_router::hooks::use_location;
use leptos_router::path;
use palicanon_ui_lib::{AppShell, IconName, NavEntry, NavList, NavSection, provide_language};
#[cfg(feature = "ssr")]
use palicanon_ui_lib::{HydrationRoot, language_attribute, theme_attribute};

use crate::catalogue::strings;
use crate::components::not_found::NotFoundPage;
use crate::components::site_footer::SiteFooter;
use crate::fragment::{MapProvider, MapRoutes};
use crate::head::{SiteHead, site_map_head};
use crate::map::first::FirstRender;
use crate::map::site_data::SiteData;
use crate::pages::FooterPage;
#[cfg(feature = "ssr")]
use crate::release::{ENV_META, ReleaseInfo, SHA_META};
use crate::route;

/// The HTML document around the app, rendered on the server. `data-theme`
/// and `lang` follow the request's cookies ([`theme_attribute`],
/// [`language_attribute`]), so the first paint never flashes the wrong one.
/// The app sits in ui-lib's [`HydrationRoot`] (`<div id="pc-app">`), the
/// one element of `<body>` the server renders, which the browser's entry
/// (`palicanon_ui_lib::hydrate`, in `lib.rs`) hydrates from: what
/// Cloudflare adds to `<body>` (a decoy link first, scripts after) stays
/// outside it (search-front 017, ui-lib 063). The stylesheet is linked
/// here by [`HashedStylesheet`], under the name `cargo-leptos` gave it: a
/// release build hashes the files (`hash-files`), so the CSS is
/// `/pkg/vizualisation.<hash>.css`, which a fixed `href` would miss
/// (learn-pali/app/src/app.rs). The deploy's release
/// ([`ReleaseInfo::server`]) is provided to the app (the footer's badge)
/// and written in `<head>` for the browser.
#[cfg(feature = "ssr")]
pub fn shell(options: LeptosOptions) -> impl IntoView {
    document(options, ReleaseInfo::server().clone())
}

/// [`shell`] for a given release (a function of its argument, so a test
/// renders every case): `release` is provided to the app, written in
/// `<head>` as two plain metas ([`ENV_META`], [`SHA_META`]; the sha one
/// always, empty for a tag) and read back by the browser before it mounts.
#[cfg(feature = "ssr")]
fn document(options: LeptosOptions, release: ReleaseInfo) -> impl IntoView {
    provide_context(release.clone());
    view! {
        <!DOCTYPE html>
        <html lang=language_attribute() data-theme=theme_attribute()>
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <meta name=ENV_META content=release.env />
                <meta name=SHA_META content=release.sha />
                <AutoReload options=options.clone() />
                <HashedStylesheet id="leptos" options=options.clone() />
                <HydrationScripts options />
                <MetaTags />
            </head>
            <body>
                <HydrationRoot>
                    <App />
                </HydrationRoot>
            </body>
        </html>
    }
}

/// The app root: what hydration mounts, so it provides the page's one
/// language signal ([`provide_language`]; the server's shell already set
/// `<html lang>` from the very same resolution), and the server's decisions
/// for this address (`map::first::FirstRender`: the data files' URLs, the text it
/// opens or its 404, the translators), made on the server from the data and
/// read back by the browser from the page (a `SharedValue`, created first
/// so the browser finds it at the same place). The map is the Map fragment
/// (task 012), given the site's data (the manifest's URLs) and head.
#[component]
pub fn App() -> impl IntoView {
    provide_language();
    provide_meta_context();
    let first = SharedValue::new(first_render).into_inner();
    let data = site_data(&first);
    provide_context(std::sync::Arc::new(first));
    view! {
        <SiteHead />
        <Router>
            <MapProvider data=data head=site_map_head>
                <Layout />
            </MapProvider>
        </Router>
    }
}

/// The site's data source for the map: the files at the URLs of `first`'s manifest.
fn site_data(first: &FirstRender) -> std::sync::Arc<dyn crate::fragment::MapData> {
    std::sync::Arc::new(SiteData::new(first.manifest.clone()))
}

/// The server's decisions for the request being rendered; nothing in the
/// browser (it reads the server's from the page; the page lacking them is an
/// error). Under `csr` alone the site's App is not mounted: the map starts
/// from no decision (`FirstRender::default`) and makes its own from its data.
fn first_render() -> FirstRender {
    #[cfg(feature = "ssr")]
    {
        crate::map::first::of_request()
    }
    #[cfg(all(feature = "hydrate", not(feature = "ssr")))]
    {
        FirstRender::not_in_page()
    }
    #[cfg(not(any(feature = "ssr", feature = "hydrate")))]
    {
        FirstRender::default()
    }
}

/// The rail's entries: the services, Search (search.palicanon.app, as the
/// showcase's `SERVICE_LINKS`), Learn, then Map
/// (this one, the current entry on every map address: [`rail_current`]). `IconName::Grid` stands in for Learn
/// as in learn-pali, `IconName::Motion` for Map, until ui-lib has their icons.
pub(crate) fn nav_entries(locale: crate::catalogue::Locale) -> Vec<NavEntry> {
    vec![
        NavEntry {
            label: td_string!(locale, nav_search),
            icon: IconName::Search,
            href: "https://search.palicanon.app",
        },
        NavEntry {
            label: td_string!(locale, nav_learn),
            icon: IconName::Grid,
            href: "https://learn.palicanon.app",
        },
        NavEntry {
            label: td_string!(locale, nav_map),
            icon: IconName::Motion,
            href: "/",
        },
    ]
}

/// The path the rail marks current for `path`: the Map entry's (`/`) on every map
/// address — a view, a text in a view —, the path itself elsewhere (a footer page, the
/// 404: no entry).
pub(crate) fn rail_current(path: &str) -> String {
    match route::Route::parse(path) {
        Some(route::Route::Map { .. }) => "/".to_owned(),
        _ => path.to_owned(),
    }
}

/// The rail, following the language and the current path.
fn nav(current: Signal<String>) -> NavList {
    let t = strings();
    let sections = Signal::derive(move || {
        vec![NavSection {
            title: None,
            entries: nav_entries(t.locale()),
        }]
    });
    NavList::new(sections, current)
}

/// The shell around the pages: `AppShell`'s rail, theme and languages, the
/// routes, the footer under each. The footer pages come first, then the Map
/// fragment's routes at the root (`MapRoutes`: `/`, `/sim`, `/sim/:uid`,
/// `/:uid`; routes match in the order given, so a page is never read as a
/// text); the rest falls to the 404 with its status. The map is one parent
/// route, kept while the reader moves between views and texts (the engine
/// and the data with it), mounted on the first map address and dropped on a
/// page.
#[component]
fn Layout() -> impl IntoView {
    let location = use_location();
    let current = Signal::derive(move || location.pathname.with(|path| rail_current(path)));
    view! {
        <AppShell nav=nav(current) footer=|| view! { <SiteFooter /> }>
            <Routes fallback=|| view! { <NotFoundPage /> }>
                <Route path=path!("/acknowledgements") view=RoutedPage />
                <Route path=path!("/terms") view=RoutedPage />
                <Route path=path!("/privacy") view=RoutedPage />
                <Route path=path!("/legal") view=RoutedPage />
                <Route path=path!("/support") view=RoutedPage />
                <MapRoutes />
            </Routes>
        </AppShell>
    }
}

/// The footer page at the current path, as [`route::Route::parse`] reads it.
#[component]
fn RoutedPage() -> impl IntoView {
    let location = use_location();
    move || match route::Route::parse(&location.pathname.get()) {
        Some(route::Route::Page(kind)) => view! { <FooterPage kind=kind /> }.into_any(),
        _ => view! { <NotFoundPage /> }.into_any(),
    }
}

#[cfg(test)]
mod tests {
    use palicanon_ui_lib::Language;

    use super::*;
    use crate::catalogue::Locale;

    #[test]
    fn the_rail_lists_search_learn_then_map() {
        let labels: Vec<&str> = nav_entries(Locale::from(Language::En))
            .iter()
            .map(|entry| entry.label)
            .collect();
        assert_eq!(labels, ["Search", "Learn", "Map"]);
        let hrefs: Vec<&str> = nav_entries(Locale::from(Language::Fr))
            .iter()
            .map(|entry| entry.href)
            .collect();
        assert_eq!(
            hrefs,
            [
                "https://search.palicanon.app",
                "https://learn.palicanon.app",
                "/"
            ]
        );
    }

    /// The committed data as the server serves it, read once for every test.
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

    /// What the server's router provides a page: the data, the request
    /// (its path and, when given, its `Cookie` header).
    #[cfg(feature = "ssr")]
    fn provide_request(path: &str, cookie: Option<&str>) {
        let mut request = axum::http::Request::builder().uri(path);
        if let Some(cookie) = cookie {
            request = request.header(axum::http::header::COOKIE, cookie);
        }
        provide_context(request.body(()).expect("a request").into_parts().0);
        provide_context(served());
    }

    /// [`Layout`] as the server renders it for `path`, the language set
    /// (search-front's `render_layout`), with the data and the first render's
    /// decisions [`App`] provides.
    #[cfg(feature = "ssr")]
    fn render_layout(language: Language, path: &str) -> String {
        use leptos_router::location::RequestUrl;
        Owner::new().with(|| {
            palicanon_ui_lib::provide_language().set(language);
            provide_meta_context();
            provide_context(RequestUrl::new(path));
            provide_request(path, None);
            provide_context(ReleaseInfo::from_vars(
                Some("main".to_owned()),
                Some("34c489c".to_owned()),
            ));
            let first = first_render();
            let data = site_data(&first);
            provide_context(std::sync::Arc::new(first));
            view! {
                <Router>
                    <MapProvider data=data head=site_map_head>
                        <Layout />
                    </MapProvider>
                </Router>
            }
            .to_html()
        })
    }

    /// The footer pages' paths and their titles in en, fr, vi.
    #[cfg(feature = "ssr")]
    const PAGE_TITLES: [(&str, [&str; 3]); 5] = [
        (
            "/acknowledgements",
            ["Acknowledgements", "Remerciements", "Tri ân"],
        ),
        (
            "/terms",
            [
                "Terms of use",
                "Conditions d’utilisation",
                "Điều khoản sử dụng",
            ],
        ),
        ("/privacy", ["Privacy", "Confidentialité", "Quyền riêng tư"]),
        (
            "/legal",
            ["Legal notice", "Mentions légales", "Thông tin pháp lý"],
        ),
        ("/support", ["Support", "Soutenir", "Ủng hộ"]),
    ];

    /// The views' path prefixes and their names in en, fr, vi.
    #[cfg(feature = "ssr")]
    const VIEW_TITLES: [(&str, [&str; 3]); 2] = [
        ("", ["SuttaCentral", "SuttaCentral", "SuttaCentral"]),
        (
            "/sim",
            ["Vectorial space", "Espace vectoriel", "Không gian vectơ"],
        ),
    ];

    /// Every route of the table renders, inside the shell, its title in
    /// each language (a page's `<h1>`, the view's name on a map route),
    /// before the footer, and is no 404; a map route with a text opens its
    /// panel.
    #[cfg(feature = "ssr")]
    #[test]
    fn every_route_renders() {
        let mut routes: Vec<(String, [&str; 3], Option<&str>)> = PAGE_TITLES
            .iter()
            .map(|(path, titles)| ((*path).to_owned(), *titles, None))
            .collect();
        for (prefix, titles) in VIEW_TITLES {
            let bare = if prefix.is_empty() { "/" } else { prefix };
            routes.push((bare.to_owned(), titles, None));
            routes.push((format!("{prefix}/mn10"), titles, Some("mn10")));
        }
        assert_eq!(routes.len(), 9, "5 pages, 2 views with and without a uid");
        for (path, titles, uid) in routes {
            for (language, title) in Language::ALL.into_iter().zip(titles) {
                let html = render_layout(language, &path);
                let h1 = html.find(&format!(">{title}</h1>")).expect(&html);
                let footer = html.find("<footer").expect(&html);
                assert!(h1 < footer, "{path} {language:?}: {html}");
                assert!(!html.contains("Anattā"), "{path} {language:?}: {html}");
                if let Some(uid) = uid {
                    let read = format!(r#"href="https://suttacentral.net/{uid}""#);
                    assert!(html.contains(&read), "{path}: the panel is open: {html}");
                }
            }
        }
    }

    /// A path the table refuses renders the 404 page, in each language,
    /// whether the router matched a pattern (`/MN10` is `/:uid`) or not.
    #[cfg(feature = "ssr")]
    #[test]
    fn an_unknown_path_renders_the_404() {
        for path in ["/nope/x/y", "/sim/a/b", "/MN10"] {
            for language in Language::ALL {
                let html = render_layout(language, path);
                assert!(html.contains(">Anattā</h1>"), "{path}: {html}");
                assert!(html.contains(">404</p>"), "{path}: {html}");
                assert!(html.contains("<footer"), "{path}: {html}");
            }
        }
    }

    /// The whole document of [`document`] for `path`, with the head the meta
    /// context collected injected the way `leptos_axum` does
    /// (learn-pali/app/src/app.rs's `render_document`).
    #[cfg(feature = "ssr")]
    fn render_document(path: &str) -> String {
        render_document_with_cookie(path, None)
    }

    /// [`render_document`] for a request carrying `cookie`.
    #[cfg(feature = "ssr")]
    fn render_document_with_cookie(path: &str, cookie: Option<&str>) -> String {
        let options = LeptosOptions::builder()
            .output_name("vizualisation")
            .build();
        render_document_with(options, path, cookie, ReleaseInfo::from_vars(None, None))
    }

    /// [`render_document`] on `/`, for the given release.
    #[cfg(feature = "ssr")]
    fn render_release(env: &str, sha: &str) -> String {
        let options = LeptosOptions::builder()
            .output_name("vizualisation")
            .build();
        let release = ReleaseInfo::from_vars(Some(env.to_owned()), Some(sha.to_owned()));
        render_document_with(options, "/", None, release)
    }

    /// [`render_document`] with the given options, cookie and release.
    #[cfg(feature = "ssr")]
    fn render_document_with(
        options: LeptosOptions,
        path: &str,
        cookie: Option<&str>,
        release: ReleaseInfo,
    ) -> String {
        use futures::StreamExt;
        use leptos_meta::ServerMetaContext;
        use leptos_router::location::RequestUrl;

        let (server, output) = ServerMetaContext::new();
        let body = Owner::new().with(|| {
            provide_request(path, cookie);
            provide_context(server);
            provide_context(RequestUrl::new(path));
            document(options, release).to_html()
        });
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("a runtime");
        runtime.block_on(async {
            // The injection waits a tick of the global executor; it is set
            // once per process, a second call is the expected refusal.
            let _ = any_spawner::Executor::init_tokio();
            output
                .inject_meta_context(futures::stream::iter([body]))
                .await
                .collect::<String>()
                .await
        })
    }

    /// Each indexable route's head has its title, its canonical link at the
    /// production origin, the `og:url` mirroring it, and the site-wide share
    /// image (`head.rs`; React's `useHead` and `index.html`).
    #[cfg(feature = "ssr")]
    #[test]
    fn each_route_has_its_title_and_canonical() {
        let home = "Suttas Map — the links between the suttas of the Pāli Canon";
        let mn10 = "MN 10 Satipaṭṭhānasutta: Mindfulness Meditation — Suttas Map";
        for (path, title, canonical) in [
            ("/", home, "/"),
            ("/sim", home, "/sim"),
            ("/sim/mn10", mn10, "/mn10"),
            ("/terms", "Terms of use — Suttas Map", "/terms"),
        ] {
            let html = render_document(path);
            let head = &html[..html.find("</head>").expect(&html)];
            let url = format!("{}{canonical}", crate::head::CANONICAL_BASE);
            assert!(
                head.contains(&format!("<title>{title}</title>")),
                "{path}: {head}"
            );
            assert!(
                head.contains(&format!(r#"<link href="{url}" rel="canonical">"#)),
                "{path}: {head}"
            );
            assert!(
                head.contains(&format!(r#"<meta property="og:url" content="{url}">"#)),
                "{path}: {head}"
            );
            assert!(
                head.contains(r#"<meta name="description""#),
                "{path}: {head}"
            );
            assert!(
                head.contains(
                    r#"<meta property="og:image" content="https://map.palicanon.app/og.png">"#
                ),
                "{path}: {head}"
            );
            assert!(
                head.contains(r#""@type":"WebApplication""#),
                "{path}: {head}"
            );
        }
    }

    /// The shell's `<head>` links the stylesheet once, by the name the
    /// build gave it (`/pkg/vizualisation.css` when files are not hashed).
    #[cfg(feature = "ssr")]
    #[test]
    fn the_head_links_the_stylesheet() {
        let html = render_document("/");
        let head = &html[..html.find("</head>").expect(&html)];
        assert!(
            head.contains(r#"rel="stylesheet" href="/pkg/vizualisation.css""#),
            "{head}"
        );
        assert_eq!(html.matches("vizualisation.css").count(), 1, "{html}");
    }

    /// With `hash-files` on (the release build), the shell links the
    /// stylesheet by its hashed name, read from the hash file `cargo-leptos`
    /// writes: the unhashed name is a 404 there (006 M3, the page had no
    /// style). The hash file here is a temporary one, named by an absolute
    /// path (learn-pali's `the_stylesheet_is_linked_by_its_hashed_name`).
    #[cfg(feature = "ssr")]
    #[test]
    fn the_head_links_the_stylesheet_by_its_hashed_name() {
        let hash_file = std::env::temp_dir().join("vizualisation-test-hash.txt");
        std::fs::write(&hash_file, "js: aaa111\nwasm: bbb222\ncss: ccc333\n").expect("hash file");
        let options = LeptosOptions::builder()
            .output_name("vizualisation")
            .hash_files(true)
            .hash_file(hash_file.to_string_lossy().as_ref().into())
            .build();
        let html = render_document_with(options, "/", None, ReleaseInfo::from_vars(None, None));
        let head = &html[..html.find("</head>").expect(&html)];
        assert!(
            head.contains(r#"href="/pkg/vizualisation.ccc333.css""#),
            "{head}"
        );
        assert!(!html.contains(r#"href="/pkg/vizualisation.css""#), "{html}");
        assert_eq!(
            html.matches("vizualisation.ccc333.css").count(),
            1,
            "{html}"
        );
        let _ = std::fs::remove_file(hash_file);
    }

    /// The 404 asks not to be indexed (`<meta name="robots"
    /// content="noindex">` in the document's `<head>`, through
    /// `leptos_meta`); a page and a map route do not.
    #[cfg(feature = "ssr")]
    #[test]
    fn the_404_is_noindex() {
        let noindex = r#"<meta name="robots" content="noindex">"#;
        for path in [
            "/nope/x/y",
            "/sim/a/b",
            "/zz999",
            "/sim/zz999",
            "/pca",
            "/umap/mn10",
            "/tsne",
        ] {
            let html = render_document(path);
            let head = &html[..html.find("</head>").expect(&html)];
            assert!(head.contains(noindex), "{path}: {head}");
            assert!(
                head.contains("<title>Anattā — Suttas Map</title>"),
                "{head}"
            );
        }
        for path in ["/terms", "/sim/mn10"] {
            let html = render_document(path);
            assert!(!html.contains("noindex"), "{path}: {html}");
        }
    }

    /// The release badge sits in the footer only, once, after the page.
    #[cfg(feature = "ssr")]
    #[test]
    fn the_badge_sits_in_the_footer_after_the_page() {
        let html = render_layout(Language::En, "/terms");
        let badge = r#"title="main · 34c489c""#;
        let page = html.find("</main>").expect(&html);
        let footer = html.find("<footer").expect(&html);
        let copies: Vec<usize> = html.match_indices(badge).map(|(i, _)| i).collect();
        assert_eq!(copies.len(), 1, "one badge, in the footer: {html}");
        assert!(copies[0] > page && copies[0] > footer, "{html}");
    }

    /// The `<meta name content>` of the document's head, as the server
    /// writes it.
    #[cfg(feature = "ssr")]
    fn meta(name: &str, content: &str) -> String {
        format!(r#"<meta name="{name}" content="{content}">"#)
    }

    /// The document's `<head>` ... `</head>`.
    #[cfg(feature = "ssr")]
    fn head(html: &str) -> &str {
        let start = html.find("<head>").expect(html);
        &html[start..html.find("</head>").expect(html)]
    }

    /// The release's cases, the deploy's variables to the metas and the
    /// badge's title: a tag (no sha), `main`, a dispatched branch, a local
    /// run (a blank env drops a sha with it).
    #[cfg(feature = "ssr")]
    const CASES: [(&str, &str, &str, &str, &str); 5] = [
        ("v0.1.8", "", "v0.1.8", "", "v0.1.8"),
        ("main", "34c489c", "main", "34c489c", "main · 34c489c"),
        ("feat/x", "34c489c", "feat/x", "34c489c", "feat/x · 34c489c"),
        ("", "", "dev", "", "dev"),
        ("  ", "34c489c", "dev", "", "dev"),
    ];

    /// Pins: the document's `<head>` carries the release in two metas, the
    /// sha one always. Does not catch a wrong value that stays after the
    /// viewport and before the sha.
    #[cfg(feature = "ssr")]
    #[test]
    fn the_head_carries_the_release_in_two_metas_after_the_viewport() {
        for (env, sha, want_env, want_sha, _) in CASES {
            let html = render_release(env, sha);
            let head = head(&html);
            let viewport = head.find(r#"name="viewport""#).expect(head);
            let env_at = head.find(&meta(ENV_META, want_env)).expect(head);
            let sha_at = head.find(&meta(SHA_META, want_sha)).expect(head);
            assert!(viewport < env_at && env_at < sha_at, "{env}: {head}");
            assert_eq!(head.matches(r#"name="viz-release-"#).count(), 2, "{head}");
        }
    }

    /// A tag has no sha, and the sha meta is there all the same: the client
    /// tells "a tag" from "no metas" by its presence.
    #[cfg(feature = "ssr")]
    #[test]
    fn a_tag_writes_the_sha_meta_empty() {
        let html = render_release("v0.1.8", "");
        assert!(
            head(&html).contains(r#"<meta name="viz-release-sha" content="">"#),
            "{html}"
        );
    }

    /// The badge is rendered from the same value as the metas: on the whole
    /// document, a tag's has no sha and no separator, `main` and a branch
    /// have both, a local run is `dev`.
    #[cfg(feature = "ssr")]
    #[test]
    fn the_footer_badge_shows_the_same_release() {
        for (env, sha, _, _, title) in CASES {
            let html = render_release(env, sha);
            let footer = html.find("<footer").expect(&html);
            let at = footer + html[footer..].find("title=").expect(&html);
            assert!(
                html[at..].starts_with(&format!(r#"title="{title}""#)),
                "{title}: {html}"
            );
            let nav = footer + html[footer..].find("<nav").expect(&html);
            if !sha.is_empty() && !env.trim().is_empty() {
                assert!(html[at..nav].contains(sha), "{html}");
            } else {
                assert!(!html[at..nav].contains('·'), "no separator: {html}");
            }
        }
    }

    /// Where the `<div …>` opened at `start` closes: the index of its
    /// matching `</div>` (the app's markup nests divs, never self-closes one).
    #[cfg(feature = "ssr")]
    fn matching_div_close(html: &str, start: usize) -> usize {
        let mut depth = 0usize;
        let mut at = start;
        loop {
            let open = html[at..].find("<div").map(|i| at + i);
            let close = html[at..].find("</div>").map(|i| at + i).expect(html);
            match open {
                Some(open) if open < close => {
                    depth += 1;
                    at = open + 4;
                }
                _ => {
                    depth -= 1;
                    if depth == 0 {
                        return close;
                    }
                    at = close + 6;
                }
            }
        }
    }

    /// search-front 017 (ui-lib 063): the server's document wraps the app
    /// in ui-lib's hydration root, `<div id="pc-app">`, the one element of
    /// `<body>` it renders, which the browser's `palicanon_ui_lib::hydrate`
    /// starts from: the nodes Cloudflare adds to `<body>` (its decoy link
    /// first) then sit outside the app.
    #[cfg(feature = "ssr")]
    #[test]
    fn the_shell_body_is_the_hydration_root_around_the_app() {
        let html = render_document("/privacy");
        let body = html.find("<body>").expect(&html) + "<body>".len();
        let end = html.find("</body>").expect(&html);
        assert!(
            html[body..].starts_with(r#"<div id="pc-app">"#),
            "the root opens the body: {html}"
        );
        assert_eq!(
            matching_div_close(&html, body) + "</div>".len(),
            end,
            "the root closes the body: {html}"
        );
        let page = html.find(">Privacy</h1>").expect(&html);
        let footer = html.find("<footer").expect(&html);
        assert!(body < page && footer < end, "the app is inside: {html}");
    }

    /// The opening tag of the `<aside>` holding the legend.
    #[cfg(feature = "ssr")]
    fn legend_aside(html: &str) -> &str {
        let at = html.find("w-64 max-w-[85vw]").expect(html);
        let open = html[..at].rfind("<aside").expect(html);
        &html[open..open + html[open..].find('>').expect(html)]
    }

    /// `/:view/:uid` is the text's page, rendered by the server: its title
    /// (reference and name) and summary in the head, its canonical at its
    /// first view (`/mn10`, the parallels have it), and the panel — its
    /// reference, the SuttaCentral link, its neighbours in the view at the
    /// view's threshold (one similar text at 0.56 for MN 10).
    #[cfg(feature = "ssr")]
    #[test]
    fn the_panel_is_server_rendered_for_a_text() {
        let html = render_document("/sim/mn10");
        let head = &html[..html.find("</head>").expect(&html)];
        assert!(
            head.contains(
                "<title>MN 10 Satipaṭṭhānasutta: Mindfulness Meditation — Suttas Map</title>"
            ),
            "{head}"
        );
        assert!(
            head.contains(r#"<link href="https://map.palicanon.app/mn10" rel="canonical">"#),
            "{head}"
        );
        assert!(
            head.contains(r#"<meta name="description" content="Here the Buddha details"#),
            "{head}"
        );
        let body = &html[html.find("<body>").expect(&html)..];
        assert!(
            body.contains(r#"href="https://suttacentral.net/mn10""#),
            "{body}"
        );
        assert!(body.contains("Read on SuttaCentral"), "{body}");
        assert!(body.contains("Satipaṭṭhānasutta"), "{body}");
        assert!(
            body.contains(">Similarity<"),
            "a section of similar texts: {body}"
        );
        assert!(body.contains("Texts visited"), "the trail: {body}");
        assert!(!html.contains("noindex"), "{head}");
    }

    /// A uid the view lacks but another view has (`ds1.1`: the vectorial
    /// space's, not the parallels') opens its panel anyway — no 404 —,
    /// without a neighbour in this view and with nothing highlighted; its
    /// canonical is its first view.
    #[cfg(feature = "ssr")]
    #[test]
    fn a_uid_missing_from_the_view_opens_the_panel_with_nothing_highlighted() {
        use crate::map::first::{FirstOpen, FirstRequest, first_render};
        use crate::map::texts_lang::TextsLang;
        use crate::route::View;

        let html = render_document("/ds1.1");
        let head = &html[..html.find("</head>").expect(&html)];
        assert!(!head.contains("noindex"), "{head}");
        assert!(
            head.contains(r#"<link href="https://map.palicanon.app/sim/ds1.1" rel="canonical">"#),
            "{head}"
        );
        let body = &html[html.find("<body>").expect(&html)..];
        assert!(
            body.contains(r#"href="https://suttacentral.net/ds1.1""#),
            "the panel is open: {body}"
        );
        // The panel's sections, not the legend's link types (the legend is the server's too).
        let read = body.find("suttacentral.net/ds1.1").expect(body);
        let open = body[..read].rfind("<aside").expect(body);
        let panel = &body[open..read + body[read..].find("</aside>").expect(body)];
        for section in [">Full parallel<", ">Similarity<", ">Resembling<"] {
            assert!(!panel.contains(section), "no link in this view: {section}");
        }
        let first = first_render(
            &served(),
            &FirstRequest {
                path: "/ds1.1",
                view: View::Parallels,
                uid: Some("ds1.1"),
                th: None,
                language: Language::En,
                texts: TextsLang::Pali,
                light: false,
            },
        );
        let FirstOpen::Text(text) = first.open else {
            panic!("ds1.1 opens a panel: {:?}", first.open);
        };
        assert_eq!(text.index_in_view, None);
        assert_eq!(text.highlighted, None, "nothing highlighted");
        assert_eq!(text.canonical, View::Sim);
        assert_eq!(text.panel.item.uid, "ds1.1");
        assert!(text.panel.sections.is_empty(), "{:?}", text.panel.sections);
    }

    /// A uid in no view is the 404, in the document: its title, noindex,
    /// no panel.
    #[cfg(feature = "ssr")]
    #[test]
    fn a_uid_in_no_view_renders_the_404() {
        for path in ["/zz999", "/sim/zz999"] {
            let html = render_document(path);
            assert!(html.contains(">Anattā</h1>"), "{path}: {html}");
            assert!(
                html.contains("<title>Anattā — Suttas Map</title>"),
                "{path}"
            );
            assert!(!html.contains("suttacentral.net/zz999"), "{path}: {html}");
        }
    }

    /// The map's cookies, read by the server for the first render: the
    /// legend closed on a desktop (`viz_legend`'s last field 0), the texts in
    /// French (`viz_textlang` chosen while the interface was in English) —
    /// the chip says so and the panel's titles are French. Without them: the
    /// legend open, the texts in Pāli, the titles in the interface's English
    /// (`translations.json`'s, as App.tsx's `refOf`).
    #[cfg(feature = "ssr")]
    #[test]
    fn the_server_renders_the_maps_cookies() {
        let html =
            render_document_with_cookie("/mn10", Some("viz_legend=mn||0; viz_textlang=fr|en"));
        let aside = legend_aside(&html);
        assert!(aside.contains("md:hidden"), "{aside}");
        assert!(!aside.contains("md:flex"), "{aside}");
        assert!(html.contains("Français"), "the texts chip: {html}");
        assert!(
            html.contains("Le récit de l’attention vigilante"),
            "French titles: {html}"
        );
        let html = crate::map::ui::test_support::plain(&render_document("/mn10"));
        let aside = legend_aside(&html);
        assert!(aside.contains("md:flex"), "{aside}");
        assert!(html.contains(">Pāli<"), "the texts chip: {html}");
        assert!(
            html.contains(">Mindfulness Meditation<"),
            "SuttaCentral's English title (translations.json's): {html}"
        );
        assert!(
            !html.contains("Le récit de l’attention vigilante"),
            "{html}"
        );
    }

    /// The links marked `aria-current="page"` in `html` (the rail's entries; the panel's
    /// trail marks its current chip, a `<span>`, too).
    #[cfg(feature = "ssr")]
    fn current_tags(html: &str) -> Vec<&str> {
        html.match_indices(r#"aria-current="page""#)
            .map(|(at, _)| {
                let start = html[..at].rfind('<').expect(html);
                &html[start..at + html[at..].find('>').expect(html)]
            })
            .filter(|tag| tag.starts_with("<a"))
            .collect()
    }

    /// 006 review: the rail's Map entry (`/`) is current on every map route — a view, a
    /// text in a view —, never on a footer page.
    #[cfg(feature = "ssr")]
    #[test]
    fn the_rails_map_entry_is_current_on_every_map_route() {
        for path in ["/", "/sim", "/sim/mn10", "/mn10"] {
            let html = render_layout(Language::En, path);
            let current = current_tags(&html);
            assert!(!current.is_empty(), "{path}: no current entry");
            for tag in current {
                assert!(tag.contains(r#"href="/""#), "{path}: {tag}");
            }
        }
        for path in ["/terms", "/acknowledgements"] {
            let html = render_layout(Language::En, path);
            assert!(
                current_tags(&html)
                    .iter()
                    .all(|tag| !tag.contains(r#"href="/""#)),
                "{path}: the map is not current"
            );
        }
    }

    /// 006 review: the legend is the server's, its rows and counts in the aside before any
    /// script runs (it was empty until the browser's data came in): each collection with
    /// its texts, each link type with its links (the vectorial space's at its default
    /// threshold), as the browser counts them.
    #[cfg(feature = "ssr")]
    #[test]
    fn the_server_renders_the_legend_with_its_counts() {
        use crate::map::draw::{Scene, links_at};
        use crate::map::theme::MapTheme;
        use crate::map::ui::number;
        use crate::map::ui::test_support::plain;

        let data = &served().data;
        for (path, graph) in [("/", &data.parallels), ("/sim", &data.sim)] {
            let html = plain(&render_document(path));
            let open = html.find("w-64 max-w-[85vw]").expect(&html);
            let aside = &html[open..open + html[open..].find("</aside>").expect(&html)];
            let theme = MapTheme::new(false, graph);
            let rows = crate::map::ui::legend::collection_rows(graph, &theme, None);
            assert!(rows.len() > 3, "{path}");
            for row in rows {
                let line = format!(
                    r#"{}<span class="ml-auto tabular-nums text-subtle">{}</span>"#,
                    row.name,
                    number(row.count, Language::En)
                );
                assert!(aside.contains(&line), "{path}: {line}: {aside}");
            }
            let mut scene = Scene::new(graph);
            if let Some(sim) = &graph.sim {
                scene.set_links(links_at(&data.pairs, sim, sim.default));
            }
            let links = scene.link_counts(graph);
            for (name, count) in graph.types.iter().zip(&links) {
                let label = crate::map::ui::type_label(name, Language::En.into());
                let line = format!(
                    r#"{label}<span class="ml-auto tabular-nums text-subtle">{}</span>"#,
                    number(*count, Language::En)
                );
                assert!(aside.contains(&line), "{path}: {line}: {aside}");
            }
        }
    }

    /// The Acknowledgements page has the translators table, read from the
    /// data: the Pāli edition's 7,286 texts (the vectorial space's), then a
    /// row per language with its count (`translations.json`: 6,516 English,
    /// 1,191 French, 5,259 Vietnamese).
    #[cfg(feature = "ssr")]
    #[test]
    fn the_acknowledgements_list_the_translators_from_the_data() {
        use crate::map::ui::test_support::plain;
        let html = plain(&render_layout(Language::En, "/acknowledgements"));
        assert!(html.contains("Texts and translators"), "{html}");
        for count in ["7,286", "6,516", "1,191", "5,259"] {
            assert!(html.contains(&format!(">{count}<")), "{count}: {html}");
        }
        assert!(html.contains("Mahāsaṅgīti Tipiṭaka"), "{html}");
        let fr = plain(&render_layout(Language::Fr, "/acknowledgements"));
        assert!(fr.contains("7\u{202f}286"), "{fr}");
    }
}
