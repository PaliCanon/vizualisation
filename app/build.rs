//! This service's build script (copied from `ui-lab`'s gallery `build.rs`,
//! task 032's pattern): [`generate_catalogue`] generates this
//! service's typed catalogue (`leptos_i18n`) from `locales/<code>.json`,
//! one file per [`palicanon_ui_lib::Language`] code. A key missing from
//! (or surplus in) any locale fails the build: `leptos_i18n` only warns
//! and falls back to English, which would ship a mixed-language page.
//! So does a `{{ name }}` placeholder dropped or renamed in a translation
//! (ui-lib's `placeholders.rs`, through `vizualisation-i18n-check`):
//! `leptos_i18n` would render it without its value. Without ui-lib beside
//! this repository (a cargo git checkout) that one check is skipped, with a
//! build warning.
//!
//! The build reads no git: the footer badge is the deploy's (task 016).

use std::path::PathBuf;

use leptos_i18n_parser::parse_locales::options::{Config, ParseOptions};
use leptos_i18n_parser::parse_locales::parse_locales;

/// The default locale, then the others: every `Language` code.
const LOCALES: [&str; 3] = ["en", "fr", "vi"];

fn main() {
    generate_catalogue();
}

/// Generates the typed string catalogue into `OUT_DIR/i18n/mod.rs`,
/// failing the build when the locales disagree on their keys.
fn generate_catalogue() {
    // `interpolate_display` (as learn-pali's build.rs): `td_string!` with arguments renders
    // to a string, for a sentence with a count or names in it (`sim_stats`,
    // `texts_translated`, `credits_translated`).
    let config = Config::new(LOCALES[0])
        .and_then(|config| config.add_locales(&LOCALES[1..]))
        .expect("the locale list is valid")
        .parse_options(ParseOptions::new().interpolate_display(true));
    let parsed = parse_locales(None, config).expect("locales/*.json parse");
    for file in parsed.tracked_files.iter().flatten() {
        println!("cargo::rerun-if-changed={file}");
    }
    let (errors, warnings) = parsed.diag.borrow();
    let problems: Vec<String> = errors
        .iter()
        .map(ToString::to_string)
        .chain(warnings.iter().map(ToString::to_string))
        .collect();
    assert!(
        problems.is_empty(),
        "the catalogues disagree:\n{}",
        problems.join("\n")
    );
    drop((errors, warnings));
    assert_placeholder_parity();
    let code = leptos_i18n_codegen::gen_code(&parsed, None, false, None, true)
        .expect("the catalogue generates");
    let dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR")).join("i18n");
    std::fs::create_dir_all(&dir).expect("create OUT_DIR/i18n");
    std::fs::write(dir.join("mod.rs"), code.to_string()).expect("write the catalogue");
}

/// Every locale's `{{ name }}` placeholders match the default locale's,
/// key by key: ui-lib's check (`build/placeholders.rs`, through
/// `vizualisation-i18n-check`) when ui-lib sits beside this repository, as
/// today in development, the site's CI and its Docker build; skipped with a
/// warning in a cargo git checkout (the PaliCanon app pins this repository
/// by tag), where that file is absent.
fn assert_placeholder_parity() {
    let texts: Vec<(&str, String)> = LOCALES
        .iter()
        .map(|code| {
            let path = format!("locales/{code}.json");
            (
                *code,
                std::fs::read_to_string(&path).expect("read a locale file"),
            )
        })
        .collect();
    let locales: Vec<(&str, &str)> = texts.iter().map(|(c, t)| (*c, t.as_str())).collect();
    let Some(mismatches) = vizualisation_i18n_check::placeholder_mismatches(&locales) else {
        println!(
            "cargo:warning=the placeholder-parity check is skipped: ui-lib's \
             build/placeholders.rs is not beside this repository (a cargo git checkout)"
        );
        return;
    };
    assert!(
        mismatches.is_empty(),
        "the catalogues' placeholders disagree:\n{}",
        mismatches.join("\n")
    );
}
