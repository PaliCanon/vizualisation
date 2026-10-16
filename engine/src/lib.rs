//! A 3D graph renderer for the web: points, line segments between them, an orbit camera and
//! picking, on wgpu (WebGPU, else WebGL2). It knows nothing of what the points are: the page
//! gives positions, colours, sizes and segments, forwards pointer events, and calls `frame`
//! on every animation frame.

pub mod camera;
// Input checks for the page-facing API: only the web view (wasm32) calls them.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
mod check;
#[cfg(target_arch = "wasm32")]
mod gpu;
pub mod live_layout;
pub mod pick;
#[cfg(target_arch = "wasm32")]
mod timer;
#[cfg(target_arch = "wasm32")]
mod view;

#[cfg(target_arch = "wasm32")]
pub use view::{GraphView, create_graph_view};

use std::sync::atomic::{AtomicUsize, Ordering};

/// How many graph views hold a GPU device now (made, not yet dropped).
static LIVE_VIEWS: AtomicUsize = AtomicUsize::new(0);

/// How many graph views hold a GPU device now: one per view made by `create_graph_view` and
/// not yet dropped (dropping a view releases its GPU: `device.destroy()` under WebGPU, the
/// WebGL context lost under WebGL2). A page that unmounted its map reads 0.
#[must_use]
pub fn live_views() -> usize {
    LIVE_VIEWS.load(Ordering::Relaxed)
}

/// A view's place in [`live_views`], counted from its making to its drop.
#[cfg_attr(
    not(target_arch = "wasm32"),
    allow(dead_code, reason = "made by the web view")
)]
pub(crate) struct Counted(());

impl Counted {
    /// One more live view.
    #[cfg_attr(
        not(target_arch = "wasm32"),
        allow(dead_code, reason = "made by the web view")
    )]
    pub(crate) fn new() -> Counted {
        LIVE_VIEWS.fetch_add(1, Ordering::Relaxed);
        Counted(())
    }
}

impl Drop for Counted {
    fn drop(&mut self) {
        LIVE_VIEWS.fetch_sub(1, Ordering::Relaxed);
    }
}
