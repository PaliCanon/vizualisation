//! The data files, fetched by the browser at the manifest's hashed URLs (`src/map/data.ts`'s
//! `loadGraph`, `loadPairs`, `loadTranslations`, `loadBlurbs`) and read through the
//! `formats` crate, as the server reads them. The browser only (hydrate or csr), wasm32 only.

use std::sync::Arc;

use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

use crate::manifest::Manifest;
use crate::map::model::{Blurbs, Graph, Pairs, Translations};

/// The bytes of the data file `name`, at its URL in `manifest`; `None` when the manifest
/// lacks it or the request fails (the map then stays on "Loading…", as the React page did).
async fn bytes(manifest: &Manifest, name: &str) -> Option<Vec<u8>> {
    let url = manifest.url(name)?;
    let window = web_sys::window()?;
    let response: web_sys::Response = JsFuture::from(window.fetch_with_str(url))
        .await
        .ok()?
        .dyn_into()
        .ok()?;
    if !response.ok() {
        log(&format!("{name}: HTTP {}", response.status()));
        return None;
    }
    let buffer = JsFuture::from(response.array_buffer().ok()?).await.ok()?;
    Some(js_sys::Uint8Array::new(&buffer).to_vec())
}

/// An error on the console: the map shows nothing for it.
fn log(message: &str) {
    web_sys::console::error_1(&format!("map data: {message}").into());
}

/// A view's graph file.
pub(crate) async fn graph(manifest: &Manifest, file: &str) -> Option<Arc<Graph>> {
    let bytes = bytes(manifest, file).await?;
    Graph::from_json(&bytes)
        .map_err(|e| log(&format!("{file}: {e}")))
        .ok()
        .map(Arc::new)
}

/// The similarity pairs of the vectorial views (`graph-sim.bin`).
pub(crate) async fn pairs(manifest: &Manifest, file: &str) -> Option<Arc<Pairs>> {
    let bytes = bytes(manifest, file).await?;
    Pairs::from_bytes(&bytes)
        .map_err(|e| log(&format!("{file}: {e}")))
        .ok()
        .map(Arc::new)
}

/// The translations (`translations.json`).
pub(crate) async fn translations(manifest: &Manifest) -> Option<Arc<Translations>> {
    let bytes = bytes(manifest, "translations.json").await?;
    Translations::from_json(&bytes)
        .map_err(|e| log(&format!("translations.json: {e}")))
        .ok()
        .map(Arc::new)
}

/// The summaries in `code` (`blurbs-<code>.json`); empty when there is no such file
/// (data.ts's `loadBlurbs` answers `{}`).
pub(crate) async fn blurbs(manifest: &Manifest, code: &str) -> Arc<Blurbs> {
    let file = format!("blurbs-{code}.json");
    let Some(bytes) = bytes(manifest, &file).await else {
        return Arc::new(Blurbs::default());
    };
    Arc::new(
        Blurbs::from_json(&bytes)
            .map_err(|e| log(&format!("{file}: {e}")))
            .unwrap_or_default(),
    )
}
