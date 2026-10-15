//! vizualisation: the 3D map of the Pāli canon, Leptos SSR + hydrate on
//! `palicanon-ui-lib` (task 005; the map screen is task 006).

// `AppShell`'s nested `Option<ViewFn>` markup builds a type deep enough that
// rustc's default query-depth limit can overflow (search-front/app/src/lib.rs,
// ui-lab tasks 010/027): raised the same way.
#![recursion_limit = "512"]

pub mod app;
#[cfg(feature = "ssr")]
pub mod boot;
mod catalogue;
mod components;
#[cfg(feature = "ssr")]
pub mod gate;
#[cfg(feature = "ssr")]
pub mod origin;
mod pages;
mod route;
#[cfg(feature = "ssr")]
pub mod router;

// `build.rs` includes this same file (`#[path]`) for the env badge's pure
// decisions; declared here too so `cargo test -p vizualisation` covers it.
#[allow(dead_code)]
mod git_info;

/// Browser entry point: hydrate the server-rendered page.
#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(app::App);
}
