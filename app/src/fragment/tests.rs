//! The fragment where a host mounts it (task 012): under the app's `ParentRoute /map`, and at
//! the root as the site does, beside a route of the host's. Rendered by the server with the
//! decisions it makes for the address (`FirstRender`), a head renderer that records what it
//! is given, and a data source the server never asks (the browser part loads the data).

use std::sync::{Arc, Mutex};

use leptos::prelude::*;
use leptos_meta::provide_meta_context;
use leptos_router::components::{Outlet, ParentRoute, Route, Router, Routes};
use leptos_router::location::RequestUrl;
use leptos_router::path;
use palicanon_ui_lib::Language;

use super::{MapData, MapFile, MapHead, MapProvider, MapRoutes};
use crate::map::first::{FirstRequest, first_render};
use crate::map::texts_lang::TextsLang;
use crate::map::ui::test_support::plain;
use crate::route::map_address;

/// A data source the server never asks: it would answer nothing.
struct NoData;

impl MapData for NoData {
    fn file(&self, _name: &'static str) -> MapFile {
        Box::pin(async { None })
    }
}

/// The committed data as the server serves it, read once.
fn served() -> crate::data::Served {
    use std::sync::OnceLock;
    static SERVED: OnceLock<crate::data::Served> = OnceLock::new();
    SERVED
        .get_or_init(|| {
            let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../public");
            let data = crate::data::Data::load(&dir).expect("the committed data");
            crate::data::Served::new(Arc::new(data))
        })
        .clone()
}

/// Where the host mounts the map.
#[derive(Clone, Copy, Debug)]
enum Mount {
    /// Inside `ParentRoute /map`, as the app does.
    App,
    /// At the root, after the host's own route, as the site does.
    Root,
}

impl Mount {
    /// The base the map is under.
    fn base(self) -> &'static str {
        match self {
            Mount::App => "/map",
            Mount::Root => "",
        }
    }
}

/// The page at `path` with the map mounted at `mount`: its HTML (comments stripped) and the
/// heads the map gave its host's renderer.
fn render(mount: Mount, path: &str) -> (String, Vec<MapHead>) {
    let heads = Arc::new(Mutex::new(Vec::new()));
    let html = Owner::new().with(|| {
        palicanon_ui_lib::provide_language().set(Language::En);
        provide_meta_context();
        provide_context(RequestUrl::new(path));
        let request = axum::http::Request::builder().uri(path);
        provide_context(request.body(()).expect("a request").into_parts().0);
        provide_context(served());
        // The server's decisions for the map's own address (the path past its base).
        let (view, uid) = path
            .strip_prefix(mount.base())
            .and_then(map_address)
            .unwrap_or((crate::route::View::Parallels, None));
        let first = first_render(
            &served(),
            &FirstRequest {
                path,
                view,
                uid: uid.as_deref(),
                th: None,
                language: Language::En,
                texts: TextsLang::Pali,
                light: false,
            },
        );
        provide_context(Arc::new(first));
        let data: Arc<dyn MapData> = Arc::new(NoData);
        let record = Arc::clone(&heads);
        let head = move |head: MapHead| record.lock().expect("the heads").push(head);
        view! {
            <Router>
                <MapProvider data=data head=head>
                    <HostRoutes mount=mount />
                </MapProvider>
            </Router>
        }
        .to_html()
    });
    let heads = heads.lock().expect("the heads").clone();
    (plain(&html), heads)
}

/// The host's routes: a route of its own, and the map where `mount` puts it.
#[component]
fn HostRoutes(mount: Mount) -> impl IntoView {
    match mount {
        Mount::App => view! {
            <Routes fallback=|| view! { <p id="host-404">"host 404"</p> }>
                <Route path=path!("/other") view=|| view! { <p id="other">"other"</p> } />
                <ParentRoute path=path!("/map") view=Outlet>
                    <MapRoutes />
                </ParentRoute>
            </Routes>
        }
        .into_any(),
        Mount::Root => view! {
            <Routes fallback=|| view! { <p id="host-404">"host 404"</p> }>
                <Route path=path!("/other") view=|| view! { <p id="other">"other"</p> } />
                <MapRoutes />
            </Routes>
        }
        .into_any(),
    }
}

/// Every `href` of `html` that is not an external `https` one (the read link, the
/// dictionary's).
fn internal_hrefs(html: &str) -> Vec<&str> {
    html.match_indices("href=\"")
        .map(|(at, found)| {
            let start = at + found.len();
            &html[start..start + html[start..].find('"').expect(html)]
        })
        .filter(|href| !href.starts_with("https://"))
        .collect()
}

/// Under `/map`, the map's four addresses render the map screen in their view (its `<h1>`),
/// a text's address its panel, and no host 404; the host's own route is the host's.
#[test]
fn the_maps_addresses_render_under_map() {
    for (path, view, text) in [
        ("/map", "SuttaCentral", None),
        ("/map/sim", "Vectorial space", None),
        ("/map/mn10", "SuttaCentral", Some("mn10")),
        ("/map/sim/mn10", "Vectorial space", Some("mn10")),
    ] {
        let (html, _) = render(Mount::App, path);
        assert!(
            html.contains(&format!(r#"<h1 class="sr-only">{view}</h1>"#)),
            "{path}: {html}"
        );
        assert!(!html.contains("host 404"), "{path}: {html}");
        assert!(!html.contains(">Anattā</h1>"), "{path}: {html}");
        if let Some(uid) = text {
            let read = format!(r#"href="https://suttacentral.net/{uid}""#);
            assert!(html.contains(&read), "{path}: the panel is open: {html}");
        }
    }
    let (html, heads) = render(Mount::App, "/other");
    assert!(html.contains(r#"<p id="other">other</p>"#), "{html}");
    assert!(heads.is_empty(), "the map is not there: {heads:?}");
}

/// A `:uid` that is no uid (`/map/MN10`), or one no view has (`/map/zz999`), is the map's
/// 404, its links to the map's own address; a path past the map's patterns is the host's.
#[test]
fn the_maps_404_links_to_the_map_under_map() {
    for path in ["/map/MN10", "/map/zz999", "/map/sim/zz999"] {
        let (html, heads) = render(Mount::App, path);
        assert!(html.contains(">Anattā</h1>"), "{path}: {html}");
        assert_eq!(internal_hrefs(&html), ["/map", "/map"], "{path}: {html}");
        assert_eq!(heads, [MapHead::NotFound], "{path}");
    }
    let (html, _) = render(Mount::App, "/map/sim/a/b");
    assert!(html.contains("host 404"), "{html}");
}

/// Every link the map renders under `/map` is under it (the 404's, the panel's are external);
/// mounted at the root, the same fragment keeps `/`.
#[test]
fn every_link_is_under_the_maps_base() {
    for path in [
        "/map",
        "/map/sim",
        "/map/mn10",
        "/map/sim/mn10",
        "/map/MN10",
    ] {
        let (html, _) = render(Mount::App, path);
        for href in internal_hrefs(&html) {
            assert!(href.starts_with("/map"), "{path}: {href}");
        }
    }
    for path in ["/", "/sim", "/mn10", "/sim/mn10"] {
        let (html, _) = render(Mount::Root, path);
        assert!(html.contains(r#"<h1 class="sr-only">"#), "{path}: {html}");
        for href in internal_hrefs(&html) {
            assert!(!href.starts_with("/map"), "{path}: {href}");
        }
    }
    let (html, _) = render(Mount::Root, "/MN10");
    assert_eq!(internal_hrefs(&html), ["/", "/"], "{html}");
    let (html, _) = render(Mount::Root, "/other");
    assert!(html.contains(r#"<p id="other">other</p>"#), "{html}");
}

/// The host's head renderer is given the map's head: a text's title and its canonical path
/// relative to the map (`/mn10`, its first view, though the address is the vectorial
/// space's), the map's own on a view, the 404 on an address that names no text.
#[test]
fn the_head_renderer_gets_the_maps_head() {
    let (_, heads) = render(Mount::App, "/map/sim/mn10");
    assert_eq!(heads.len(), 1, "{heads:?}");
    let MapHead::Page { title, path, .. } = &heads[0] else {
        panic!("a page: {heads:?}");
    };
    assert_eq!(
        title,
        "MN 10 Satipaṭṭhānasutta: Mindfulness Meditation — Suttas Map"
    );
    assert_eq!(path, "/mn10");
    let (_, heads) = render(Mount::App, "/map/sim");
    let MapHead::Page { title, path, .. } = &heads[0] else {
        panic!("a page: {heads:?}");
    };
    assert_eq!(
        title,
        "Suttas Map — the links between the suttas of the Pāli Canon"
    );
    assert_eq!(path, "/sim");
    let (_, heads) = render(Mount::App, "/map/MN10");
    assert_eq!(heads, [MapHead::NotFound]);
}
