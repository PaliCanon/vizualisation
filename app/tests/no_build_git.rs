//! The badge is the deploy's, not the image's (task 016): the image is identical across refs, so nothing
//! in the build reads git. Fails when `build.rs`, the Dockerfile or a source file mentions the capture again,
//! or when `src/git_info.rs` comes back. Not caught: the same capture under another name.
#![cfg(not(target_arch = "wasm32"))]

use std::fs;
use std::path::{Path, PathBuf};

/// Words only the build-time git capture used.
const FORBIDDEN: [&str; 9] = [
    "VIZUALISATION_GIT",
    "git_info",
    "capture_git_info",
    "rev-parse",
    "git status",
    "commondir",
    "\"cargo:rustc-env",
    "BUILD_BRANCH",
    "BUILD_SHA",
];

/// The `app/` package directory.
fn app_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

/// Every `*.rs` file under `dir`, recursively.
fn rust_files(dir: &Path, into: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("a readable directory") {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            rust_files(&path, into);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            into.push(path);
        }
    }
}

/// The forbidden words found in a file, one line each.
fn offences(path: &Path) -> Vec<String> {
    let text = fs::read_to_string(path).expect("a readable file");
    FORBIDDEN
        .iter()
        .filter(|word| text.contains(**word))
        .map(|word| format!("{} mentions `{word}`", path.display()))
        .collect()
}

#[test]
fn the_build_script_reads_no_git() {
    let found = offences(&app_dir().join("build.rs"));
    assert_eq!(found.len(), 0, "{found:?}");
}

#[test]
fn the_dockerfile_reads_no_git() {
    let found = offences(&app_dir().join("../Dockerfile"));
    assert_eq!(found.len(), 0, "{found:?}");
}

#[test]
fn no_source_or_test_mentions_the_capture() {
    let mut files = Vec::new();
    rust_files(&app_dir().join("src"), &mut files);
    rust_files(&app_dir().join("tests"), &mut files);
    let this = Path::new(file!()).file_name().expect("this file's name");
    let found: Vec<String> = files
        .iter()
        .filter(|path| path.file_name() != Some(this))
        .flat_map(|path| offences(path))
        .collect();
    assert_eq!(found.len(), 0, "{found:?}");
}

#[test]
fn the_git_info_module_is_gone() {
    let path = app_dir().join("src/git_info.rs");
    assert!(!path.exists(), "{} came back", path.display());
}
