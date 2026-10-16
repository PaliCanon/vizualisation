//! The site's [`MapData`] (task 012): each data file fetched by the browser at the URL the
//! manifest names (content-hashed, `/data/graph.<hash16>.json`), the manifest being the
//! server's, read from the page (`map::first::FirstRender`). The server never awaits it: it
//! renders the first page from the data it holds in memory.

use crate::fragment::{MapData, MapFile};
use crate::manifest::Manifest;

/// The data files at the manifest's URLs.
pub(crate) struct SiteData {
    /// The URL of each data file.
    #[cfg_attr(
        not(all(feature = "browser", target_arch = "wasm32")),
        allow(dead_code, reason = "fetched by the browser only")
    )]
    manifest: Manifest,
}

impl SiteData {
    /// The files at `manifest`'s URLs.
    pub(crate) fn new(manifest: Manifest) -> SiteData {
        SiteData { manifest }
    }
}

impl MapData for SiteData {
    /// The bytes at the manifest's URL for `name`; `None` when the manifest has no such file
    /// (`blurbs-vi.json`: Vietnamese has no summaries) or the request fails (logged). On the
    /// server: `None`, never awaited.
    fn file(&self, name: &'static str) -> MapFile {
        #[cfg(all(feature = "browser", target_arch = "wasm32"))]
        {
            let url = self.manifest.url(name).map(str::to_owned);
            Box::pin(send_wrapper::SendWrapper::new(async move {
                browser::bytes(name, &url?).await
            }))
        }
        #[cfg(not(all(feature = "browser", target_arch = "wasm32")))]
        {
            let _ = name;
            Box::pin(async { None })
        }
    }
}

/// The fetch, in the browser.
#[cfg(all(feature = "browser", target_arch = "wasm32"))]
mod browser {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    use crate::map::fetch::log;

    /// The bytes at `url`, the data file `name`; `None` when the request fails.
    pub(super) async fn bytes(name: &str, url: &str) -> Option<Vec<u8>> {
        let window = web_sys::window()?;
        let response: web_sys::Response = JsFuture::from(window.fetch_with_str(url))
            .await
            .map_err(|_| log(&format!("{name}: the request failed")))
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
}
