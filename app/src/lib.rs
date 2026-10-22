//! vizualisation: the 3D map of the Pāli canon, Leptos SSR + hydrate on
//! `palicanon-ui-lib` (task 005; the map screen is task 006), and the Map
//! fragment the PaliCanon app mounts client side ([`fragment`], `csr`, task
//! 012).

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
pub mod data;
pub mod fragment;
#[cfg(feature = "ssr")]
pub mod gate;
mod head;
pub mod manifest;
pub mod map;
#[cfg(feature = "ssr")]
pub mod origin;
mod pages;
pub mod release;
mod route;
#[cfg(feature = "ssr")]
pub mod router;

/// Browser entry point: hydrate the server-rendered page from its root
/// (`<div id="pc-app">`, written by [`app::shell`]), not from `<body>`,
/// whose first child behind Cloudflare is an injected decoy link
/// (search-front 017, ui-lib 063); `<html>` then carries `data-hydrated`,
/// which `scripts/rollout-check.sh` reads. The release the server wrote in
/// `<head>` is read first and provided to the app (task 016).
#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use leptos::prelude::*;

    console_error_panic_hook::set_once();
    // The release the server wrote in `<head>`, read before the app
    // mounts: the badge is the server's, never recomputed here (task 016).
    palicanon_ui_lib::hydrate(|| {
        provide_context(release::ReleaseInfo::from_document());
        view! { <app::App /> }
    });
}
