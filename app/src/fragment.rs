//! The Map fragment: what a host mounts to get the map without the site's shell, footer or
//! footer pages (task 012). The PaliCanon app mounts it under its own `/map`; the site mounts
//! it at the root, after its footer pages.
//!
//! A host gives it [`MapProvider`] (its [`MapData`] and its head renderer, both required)
//! and mounts [`MapRoutes`] where the map lives:
//!
//! ```ignore
//! <MapProvider data=my_data head=|_head: MapHead| ()>
//!     <Routes fallback=NotFound>
//!         <ParentRoute path=path!("/map") view=Outlet>
//!             <MapRoutes />
//!         </ParentRoute>
//!     </Routes>
//! </MapProvider>
//! ```
//!
//! Its addresses are its public contract, relative to where it is mounted: the empty path
//! (SuttaCentral's parallels), `/<uid>` (a text there), `/sim` (the vectorial space), `/sim/<uid>`, and
//! `?th=` the vectorial space's threshold. A `<uid>` that is no SuttaCentral uid (`MN10`),
//! or that no view has, is the fragment's own 404 page.
//!
//! What the fragment does not hold: the `AppShell` and its rail, the footer, the footer pages
//! (Acknowledgements and its credits stay the site's), anything of its own in the document
//! head (the host's head renderer is given a [`MapHead`]). What it needs from its host:
//!
//! - **the one language signal**: the host calls `palicanon_ui_lib::provide_language` once;
//! - **root-served assets**: `/lotus.png` (the 404's brand) answers at the host's root,
//!   whatever the fragment's base;
//! - **one [`MapProvider`] per page**: the map's controls have fixed ids (`viz-search`,
//!   `viz-views`), and `<html data-engine>` names the one engine drawing;
//! - **the same ui-lib tag as the host's**: the host's build and this crate's resolve one
//!   `palicanon-ui-lib` (a second copy would carry its own context types and `Store`);
//! - **WebGPU or WebGL2** in the host's webview, else the map says there is no GPU;
//! - **Tailwind over these sources**: the fragment ships no CSS; its classes are in
//!   `app/src` (this crate's sources) and need ui-lib's theme (`style/theme.css`) at the
//!   host's tag, compiled by the host with its own (one scanner, one theme, one preflight);
//! - **external links outside the window**: "Read on SuttaCentral" is a plain `https` link
//!   with `target="_blank"`; the host opens it in the system browser.
//!
//! The fragment's base is never a prop: every link and history entry is built from
//! `use_matched()` of its route, so the route and the links cannot drift. Under `csr` (the
//! app, `palicanon_ui_lib::csr::mount`) the map starts from no server decision, keeps its
//! preferences in `localStorage` (ui-lib's `Store::Local`), reads its data through
//! [`MapData`], and on unmount stops every load, listener and frame, and releases the GPU.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use leptos::prelude::*;
use leptos_router::components::{ParentRoute, Route};
use leptos_router::path;

use crate::map::live::LiveMap;

/// The bytes of one data file, or `None` (see [`MapData::file`]).
pub type MapFile = Pin<Box<dyn Future<Output = Option<Vec<u8>>> + Send>>;

/// Where the map reads its data files from: the site fetches them at the manifest's
/// content-hashed URLs; the app reads its bundled copies, never the network.
///
/// The files asked: `graph.json`, `graph-sim.json`, `graph-sim.bin` (the name
/// `graph-sim.json` gives in `sim.url`), `translations.json`, `blurbs-en.json`,
/// `blurbs-fr.json` and `blurbs-vi.json`. Only the map's browser part awaits the future (a
/// fetch's or a file read's is not `Send` in a browser: an implementation wraps it, as
/// `send_wrapper` does, inside its own code). Each file is asked again on each mount: the
/// map keeps nothing while it is not shown.
pub trait MapData: Send + Sync + 'static {
    /// The bytes of the data file `name`; `None` when it is absent or cannot be read (the
    /// implementation logs why). `blurbs-vi.json` absent gives the texts no summary; any
    /// other file absent says so over the map (`MapError`).
    fn file(&self, name: &'static str) -> MapFile;
}

/// The map's document head, as data: what the host's head renderer is given, where the map
/// would set its head. The site renders its `<title>`, description, canonical link and
/// their Open Graph mirrors from it; the app, whose window title is its own, nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MapHead {
    /// A map address: its title, its description, and its canonical path relative to the
    /// fragment (`/mn10`, `/sim`; a text's first view, whatever view the address names).
    Page {
        /// The document title (`MN 10 Satipaṭṭhānasutta: Mindfulness Meditation — Suttas
        /// Map`, or the map's own).
        title: String,
        /// The text's summary, or the map's description.
        description: String,
        /// The canonical path, relative to the fragment's base.
        path: String,
    },
    /// The 404: the address names no text (the site's head is its title and `noindex`).
    NotFound,
}

/// The head renderer a host gives [`MapProvider`].
pub(crate) type HeadFn = Arc<dyn Fn(MapHead) -> AnyView + Send + Sync>;

/// What [`MapProvider`] puts in context for [`MapRoutes`].
#[derive(Clone)]
pub(crate) struct Provided {
    /// The data files' source.
    #[cfg_attr(
        not(all(feature = "browser", target_arch = "wasm32")),
        allow(dead_code, reason = "read by the browser part only")
    )]
    pub data: Arc<dyn MapData>,
    /// The head renderer.
    pub head: HeadFn,
}

impl Provided {
    /// The provider above; panics without one, which is a host's mistake.
    pub(crate) fn expect() -> Provided {
        use_context::<Provided>().expect("MapRoutes needs a MapProvider above it")
    }
}

/// Puts the host's [`MapData`] and head renderer in the context of everything below it: the
/// map reads its data files through `data`, and renders `head` with its [`MapHead`] where
/// the map's head belongs. Both are required, with no default: a default data source in the
/// app would be a fetch to an origin that is not a server. One provider per page.
#[component]
pub fn MapProvider<H, V>(
    /// Where the data files come from.
    data: Arc<dyn MapData>,
    /// Renders the map's head (`|_| ()` for none).
    head: H,
    /// The host's routes, [`MapRoutes`] among them.
    children: Children,
) -> impl IntoView
where
    H: Fn(MapHead) -> V + Send + Sync + 'static,
    V: IntoView + 'static,
{
    provide_context(Provided {
        data,
        head: Arc::new(move |h| head(h).into_any()),
    });
    children()
}

/// The map as relative routes the host mounts where it wants them: at the root (the site,
/// after its footer pages: routes match in the order given) or inside its own parent route
/// (the app's `/map`). One parent route whose view is the live map, kept while the reader
/// moves between its four addresses (the engine and the data with it); its children, with
/// empty views, are the addresses: the empty path and `/<uid>` (the parallels), `/sim` and
/// `/sim/<uid>` (the vectorial space). Needs a [`MapProvider`] above it.
#[component(transparent)]
pub fn MapRoutes() -> impl leptos_router::MatchNestedRoutes + Clone {
    view! {
        <ParentRoute path=path!("") view=LiveMap>
            <Route path=path!("") view=|| () />
            <Route path=path!("/sim") view=|| () />
            <Route path=path!("/sim/:uid") view=|| () />
            <Route path=path!("/:uid") view=|| () />
        </ParentRoute>
    }
    .into_inner()
}

#[cfg(all(test, feature = "ssr"))]
mod tests;
