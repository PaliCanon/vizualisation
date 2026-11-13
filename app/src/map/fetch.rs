//! The data files, read by the browser through the host's [`MapData`] (`src/map/data.ts`'s
//! `loadGraph`, `loadPairs`, `loadTranslations`, `loadBlurbs`; the site fetches them at the
//! manifest's hashed URLs, the app reads its bundle) and parsed through the `formats` crate,
//! as the server reads them. The browser only (hydrate or csr), wasm32 only.

use std::sync::Arc;

use palicanon_ui_lib::Language;

use crate::fragment::MapData;
use crate::map::model::{Blurbs, Graph, Pairs, Translations};

/// A bounded data failure: never a raw URL, parser message or data value.
pub(crate) fn log(name: &str, reason: &'static str, status: Option<u16>) {
    crate::telemetry::data_failed(name, reason, status);
}

/// The name of the similarity pairs' file a graph names (`sim.url`): one of the names
/// [`MapData`] is asked for, `None` (logged) for any other.
pub(crate) fn pairs_name(url: &str) -> Option<&'static str> {
    const PAIRS: [&str; 1] = ["graph-sim.bin"];
    let name = PAIRS.into_iter().find(|name| *name == url);
    if name.is_none() {
        log("", "unknown_asset", None);
    }
    name
}

/// A view's graph file.
pub(crate) async fn graph(data: Arc<dyn MapData>, file: &'static str) -> Option<Arc<Graph>> {
    let bytes = data.file(file).await?;
    Graph::from_json(&bytes)
        .map_err(|_| log(file, "parse", None))
        .ok()
        .map(Arc::new)
}

/// The similarity pairs of the vectorial views (`graph-sim.bin`).
pub(crate) async fn pairs(data: Arc<dyn MapData>, file: &'static str) -> Option<Arc<Pairs>> {
    let bytes = data.file(file).await?;
    Pairs::from_bytes(&bytes)
        .map_err(|_| log(file, "parse", None))
        .ok()
        .map(Arc::new)
}

/// The translations (`translations.json`).
pub(crate) async fn translations(data: Arc<dyn MapData>) -> Option<Arc<Translations>> {
    let bytes = data.file("translations.json").await?;
    Translations::from_json(&bytes)
        .map_err(|_| log("translations.json", "parse", None))
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

/// The summaries in `language` (`blurbs-<code>.json`); `None` when the file is absent or
/// unreadable (logged). The caller decides: English's absent is the map's error, another
/// language's gives its texts no summary of their own (data.ts's `loadBlurbs` answered `{}`).
pub(crate) async fn blurbs(data: Arc<dyn MapData>, language: Language) -> Option<Arc<Blurbs>> {
    let file = blurbs_file(language);
    let bytes = data.file(file).await?;
    Blurbs::from_json(&bytes)
        .map_err(|_| log(file, "parse", None))
        .ok()
        .map(Arc::new)
}
