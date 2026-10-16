//! Whether ui-lib sits beside this repository (`../ui-lib`, as in a
//! development checkout, the site's CI and its Docker build): then
//! `cfg(ui_lib_sibling)` and the crate includes ui-lib's
//! `build/placeholders.rs`. In a cargo git checkout (the PaliCanon app pins
//! this repository by tag) there is no sibling, and the module is left out
//! before it is ever read: a `#[path]` to a missing file only fails when the
//! module is compiled.

use std::path::Path;

fn main() {
    println!("cargo::rustc-check-cfg=cfg(ui_lib_sibling)");
    let dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let sibling = Path::new(&dir).join("../../ui-lib/build/placeholders.rs");
    if sibling.is_file() {
        println!("cargo::rustc-cfg=ui_lib_sibling");
        println!("cargo::rerun-if-changed={}", sibling.display());
    } else {
        // Nothing to watch: a git checkout never gains the sibling.
        println!("cargo::rerun-if-changed=build.rs");
    }
}
