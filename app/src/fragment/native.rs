//! The fragment built for the PaliCanon app (`csr`) on the host, not in a browser (task 012,
//! the app's first mount, PaliCanon/app 003): every browser-only path is gated on wasm32 as
//! well as on the private `browser` feature, so a host's native tests build this crate with
//! `csr` and match its routes.
//!
//! A native render is out of reach under `csr`: `leptos_router` 0.8's `Router` reads
//! `window.location` whenever its `ssr` feature is off (`BrowserUrl::new`), and ui-lib's
//! `provide_language` reads the page under its `browser` feature; both panic off wasm32.

use leptos::prelude::*;
use leptos_router::MatchNestedRoutes;
use leptos_router::components::{Outlet, ParentRoute};
use leptos_router::path;

use super::MapRoutes;

/// Whether the host's `ParentRoute /map` around [`MapRoutes`] matches the whole of `path`.
fn matches(path: &str) -> bool {
    let routes = view! {
        <ParentRoute path=path!("/map") view=Outlet>
            <MapRoutes />
        </ParentRoute>
    }
    .into_inner();
    let (matched, rest) = routes.match_nested(path);
    matched.is_some() && rest.is_empty()
}

/// Under `csr`, natively, the map's routes under `/map` match its four addresses (and what
/// they take as a uid; the map itself decides its 404), and nothing past them or outside.
#[test]
fn the_maps_routes_match_natively_under_csr() {
    for path in [
        "/map",
        "/map/sim",
        "/map/mn10",
        "/map/sim/mn10",
        "/map/MN10",
    ] {
        assert!(matches(path), "{path}");
    }
    for path in ["/other", "/map/sim/a/b", "/mapx"] {
        assert!(!matches(path), "{path}");
    }
}
