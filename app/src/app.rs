//! The service's shell: the HTML document ([`shell`], server-rendered), the
//! app root ([`App`], what hydration mounts), and `AppShell`'s rail with the
//! services Search, Learn and Map (this one), its routes (the map's views,
//! the footer pages, the 404: `crate::route`), the footer under each
//! (`SiteFooter`) carrying the build badge. From search-front/app/src/app.rs
//! (commit 6d370b4) and learn-pali/app/src/app.rs; what differs: three rail
//! entries, and one view for every route, rendering what the typed table
//! says (`RoutedPage`).

use leptos::prelude::*;
use leptos_i18n::td_string;
use leptos_meta::{HashedStylesheet, MetaTags, provide_meta_context};
use leptos_router::components::{Route, Router, Routes};
use leptos_router::hooks::use_location;
use leptos_router::path;
use palicanon_ui_lib::{
    AppShell, EnvBadgeInfo, HydrationRoot, IconName, NavEntry, NavList, NavSection,
    language_attribute, provide_language, theme_attribute,
};

use crate::catalogue::strings;
use crate::components::map_placeholder::MapPlaceholder;
use crate::components::not_found::NotFoundPage;
use crate::components::site_footer::SiteFooter;
use crate::head::SiteHead;
use crate::pages::FooterPage;
use crate::route;

/// The branch this build came from (`build.rs`,
/// `VIZUALISATION_GIT_BRANCH`, from git, or from the `BUILD_BRANCH` the
/// image build passes when it has no `.git`): `main`, `feat/…` — this
/// service's own `env` for [`EnvBadgeInfo`] (replaced by `dev` / `prod`
/// once this deploys, like every other service).
pub(crate) const BUILD_BRANCH: &str = env!("VIZUALISATION_GIT_BRANCH");
/// The short commit this build came from (`build.rs`,
/// `VIZUALISATION_GIT_SHA`, from git, or from `BUILD_SHA`), a trailing `*`
/// when the worktree had uncommitted changes at build time (git only).
pub(crate) const BUILD_SHA: &str = env!("VIZUALISATION_GIT_SHA");

/// This build, as the footer's badge shows it (at the footer's left —
/// `SiteFooter`; nothing on the shell's panel).
pub(crate) const BADGE: EnvBadgeInfo = EnvBadgeInfo {
    env: BUILD_BRANCH,
    sha: BUILD_SHA,
};

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
/// (learn-pali/app/src/app.rs).
pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang=language_attribute() data-theme=theme_attribute()>
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
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
/// `<html lang>` from the very same resolution).
#[component]
pub fn App() -> impl IntoView {
    provide_language();
    provide_meta_context();
    view! {
        <SiteHead />
        <Router>
            <Layout />
        </Router>
    }
}

/// The rail's entries: the services, Search (the main site), Learn, then Map
/// (this one, the current entry on `/`). `IconName::Grid` stands in for Learn
/// as in learn-pali, `IconName::Motion` for Map, until ui-lib has their icons.
pub(crate) fn nav_entries(locale: crate::catalogue::Locale) -> Vec<NavEntry> {
    vec![
        NavEntry {
            label: td_string!(locale, nav_search),
            icon: IconName::Search,
            href: "https://palicanon.app",
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
/// routes, the footer under each. The router knows every path pattern of
/// `crate::route` (so the server answers them, and the rest falls to the
/// 404 with its status); what each one shows is [`RoutedPage`]'s. The
/// pages come before `/:uid`, the views before `/:uid` too, so a page or a
/// view is never read as a text.
#[component]
fn Layout() -> impl IntoView {
    let location = use_location();
    view! {
        <AppShell nav=nav(location.pathname.into()) footer=|| view! { <SiteFooter /> }>
            <Routes fallback=|| view! { <NotFoundPage /> }>
                <Route path=path!("/acknowledgements") view=RoutedPage />
                <Route path=path!("/terms") view=RoutedPage />
                <Route path=path!("/privacy") view=RoutedPage />
                <Route path=path!("/legal") view=RoutedPage />
                <Route path=path!("/support") view=RoutedPage />
                <Route path=path!("/") view=RoutedPage />
                <Route path=path!("/sim") view=RoutedPage />
                <Route path=path!("/pca") view=RoutedPage />
                <Route path=path!("/umap") view=RoutedPage />
                <Route path=path!("/tsne") view=RoutedPage />
                <Route path=path!("/sim/:uid") view=RoutedPage />
                <Route path=path!("/pca/:uid") view=RoutedPage />
                <Route path=path!("/umap/:uid") view=RoutedPage />
                <Route path=path!("/tsne/:uid") view=RoutedPage />
                <Route path=path!("/:uid") view=RoutedPage />
            </Routes>
        </AppShell>
    }
}

/// What the current path shows, as [`route::Route::parse`] reads it: a
/// footer page, the map (its placeholder until task 006), or the 404 for a
/// path the router matched but the table refuses (`/MN10`).
#[component]
fn RoutedPage() -> impl IntoView {
    let location = use_location();
    move || match route::Route::parse(&location.pathname.get()) {
        Some(route::Route::Page(kind)) => view! { <FooterPage kind=kind /> }.into_any(),
        Some(route::Route::Map { view, uid }) => {
            view! { <MapPlaceholder view=view uid=uid /> }.into_any()
        }
        None => view! { <NotFoundPage /> }.into_any(),
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
            ["https://palicanon.app", "https://learn.palicanon.app", "/"]
        );
    }

    /// [`Layout`] as the server renders it for `path`, the language set
    /// (search-front's `render_layout`).
    #[cfg(feature = "ssr")]
    fn render_layout(language: Language, path: &str) -> String {
        use leptos_router::location::RequestUrl;
        Owner::new().with(|| {
            palicanon_ui_lib::provide_language().set(language);
            provide_meta_context();
            provide_context(RequestUrl::new(path));
            view! {
                <Router>
                    <Layout />
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
    const VIEW_TITLES: [(&str, [&str; 3]); 5] = [
        ("", ["SuttaCentral", "SuttaCentral", "SuttaCentral"]),
        (
            "/sim",
            ["Vectorial space", "Espace vectoriel", "Không gian vectơ"],
        ),
        ("/pca", ["Test · PCA 3D", "Test · ACP 3D", "Thử · PCA 3D"]),
        (
            "/umap",
            ["Test · UMAP 3D", "Test · UMAP 3D", "Thử · UMAP 3D"],
        ),
        (
            "/tsne",
            ["Test · t-SNE 3D", "Test · t-SNE 3D", "Thử · t-SNE 3D"],
        ),
    ];

    /// Every route of the table renders, inside the shell, its title in
    /// each language (a page's `<h1>`, the view's name on a map route),
    /// before the footer, and is no 404.
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
        assert_eq!(routes.len(), 15, "5 pages, 5 views with and without a uid");
        for (path, titles, uid) in routes {
            for (language, title) in Language::ALL.into_iter().zip(titles) {
                let html = render_layout(language, &path);
                let h1 = html.find(&format!(">{title}</h1>")).expect(&html);
                let footer = html.find("<footer").expect(&html);
                assert!(h1 < footer, "{path} {language:?}: {html}");
                assert!(!html.contains("Anattā"), "{path} {language:?}: {html}");
                if let Some(uid) = uid {
                    assert!(html.contains(&format!(">{uid}</p>")), "{path}: {html}");
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

    /// The whole document of [`shell`] for `path`, with the head the meta
    /// context collected injected the way `leptos_axum` does
    /// (learn-pali/app/src/app.rs's `render_document`).
    #[cfg(feature = "ssr")]
    fn render_document(path: &str) -> String {
        let options = LeptosOptions::builder()
            .output_name("vizualisation")
            .build();
        render_document_with(options, path)
    }

    /// [`render_document`] with the given options.
    #[cfg(feature = "ssr")]
    fn render_document_with(options: LeptosOptions, path: &str) -> String {
        use futures::StreamExt;
        use leptos_meta::ServerMetaContext;
        use leptos_router::location::RequestUrl;

        let (server, output) = ServerMetaContext::new();
        let body = Owner::new().with(|| {
            let request = axum::http::Request::builder().uri(path);
            provide_context(request.body(()).expect("a request").into_parts().0);
            provide_context(server);
            provide_context(RequestUrl::new(path));
            shell(options).to_html()
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
        for (path, title) in [
            ("/", home),
            ("/sim", home),
            ("/sim/mn10", home),
            ("/terms", "Terms of use — Suttas Map"),
        ] {
            let html = render_document(path);
            let head = &html[..html.find("</head>").expect(&html)];
            let url = format!("{}{path}", crate::head::CANONICAL_BASE);
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
        let html = render_document_with(options, "/");
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
        for path in ["/nope/x/y", "/sim/a/b"] {
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

    /// The build badge sits in the footer only, once, after the page.
    #[cfg(feature = "ssr")]
    #[test]
    fn the_badge_sits_in_the_footer_after_the_page() {
        let html = render_layout(Language::En, "/terms");
        let badge = format!(r#"title="{} · {}""#, BADGE.env, BADGE.sha);
        let page = html.find("</main>").expect(&html);
        let footer = html.find("<footer").expect(&html);
        let copies: Vec<usize> = html.match_indices(&badge).map(|(i, _)| i).collect();
        assert_eq!(copies.len(), 1, "one badge, in the footer: {html}");
        assert!(copies[0] > page && copies[0] > footer, "{html}");
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
}
