//! The data files, read by the browser through the host's [`MapData`] (`src/map/data.ts`'s
//! `loadGraph`, `loadPairs`, `loadTranslations`, `loadBlurbs`; the site fetches them at the
//! manifest's hashed URLs, the app reads its bundle) and parsed through the `formats` crate,
//! as the server reads them. The browser only (hydrate or csr), wasm32 only.

use std::sync::Arc;

use palicanon_ui_lib::Language;

use crate::fragment::MapData;
use crate::map::model::{Blurbs, Graph, Pairs, Translations};

/// An error on the console: the map shows nothing for it.
pub(crate) fn log(message: &str) {
    web_sys::console::error_1(&format!("map data: {message}").into());
}

/// The name of the similarity pairs' file a graph names (`sim.url`): one of the names
/// [`MapData`] is asked for, `None` (logged) for any other.
pub(crate) fn pairs_name(url: &str) -> Option<&'static str> {
    const PAIRS: [&str; 1] = ["graph-sim.bin"];
    let name = PAIRS.into_iter().find(|name| *name == url);
    if name.is_none() {
        log(&format!("{url}: not a data file the map reads"));
    }
    name
}

/// A view's graph file.
pub(crate) async fn graph(data: Arc<dyn MapData>, file: &'static str) -> Option<Arc<Graph>> {
    let bytes = data.file(file).await?;
    Graph::from_json(&bytes)
        .map_err(|e| log(&format!("{file}: {e}")))
        .ok()
        .map(Arc::new)
}

/// The similarity pairs of the vectorial views (`graph-sim.bin`).
pub(crate) async fn pairs(data: Arc<dyn MapData>, file: &'static str) -> Option<Arc<Pairs>> {
    let bytes = data.file(file).await?;
    Pairs::from_bytes(&bytes)
        .map_err(|e| log(&format!("{file}: {e}")))
        .ok()
        .map(Arc::new)
}

/// The translations (`translations.json`).
pub(crate) async fn translations(data: Arc<dyn MapData>) -> Option<Arc<Translations>> {
    let bytes = data.file("translations.json").await?;
    Translations::from_json(&bytes)
        .map_err(|e| log(&format!("translations.json: {e}")))
        .ok()
        .map(Arc::new)
}

/// The summaries' file of `language`.
const fn blurbs_file(language: Language) -> &'static str {
    match language {
        Language::En => "blurbs-en.json",
        Language::Fr => "blurbs-fr.json",
        Language::Vi => "blurbs-vi.json",
    }
}

/// The summaries in `language` (`blurbs-<code>.json`); empty when there is no such file
/// (data.ts's `loadBlurbs` answers `{}`).
pub(crate) async fn blurbs(data: Arc<dyn MapData>, language: Language) -> Arc<Blurbs> {
    let file = blurbs_file(language);
    let Some(bytes) = data.file(file).await else {
        return Arc::new(Blurbs::default());
    };
    Arc::new(
        Blurbs::from_json(&bytes)
            .map_err(|e| log(&format!("{file}: {e}")))
            .unwrap_or_default(),
    )
}
