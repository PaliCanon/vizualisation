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
