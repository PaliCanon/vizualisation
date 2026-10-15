//! The committed `translations.json` and `blurbs-*.json`.

use map_formats::{blurbs::Blurbs, graph::Graph, translations::Translations};
use std::{collections::HashSet, path::PathBuf};

fn public(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../public")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

#[test]
fn public_translations_parse_en_fr_vi() {
    let t = Translations::from_json(&public("translations.json")).unwrap();
    assert_eq!(t.0.keys().collect::<Vec<_>>(), ["en", "fr", "vi"]);
    let counts: Vec<(usize, usize)> =
        t.0.values()
            .map(|l| (l.titles.len(), l.translators.len()))
            .collect();
    assert_eq!(counts, [(6516, 25), (1191, 14), (5259, 3)]);
}

#[test]
fn by_index_is_in_range_in_the_committed_file() {
    let t = Translations::from_json(&public("translations.json")).unwrap();
    for (lang, l) in &t.0 {
        assert_eq!(l.by.len(), l.titles.len(), "{lang}");
        let top = l.by.values().max().copied().unwrap();
        assert!((top as usize) < l.translators.len(), "{lang}");
    }
}

#[test]
fn public_blurbs_parse() {
    assert_eq!(
        Blurbs::from_json(&public("blurbs-en.json"))
            .unwrap()
            .0
            .len(),
        5000
    );
    assert_eq!(
        Blurbs::from_json(&public("blurbs-fr.json"))
            .unwrap()
            .0
            .len(),
        616
    );
    assert_eq!(
        Blurbs::from_json(&public("blurbs-en.json"))
            .unwrap()
            .get("an3.1"),
        Some("Fools are dangerous, but the wise are safe.")
    );
}

#[test]
fn the_text_files_are_rewritten_byte_for_byte() {
    let bytes = public("translations.json");
    let text = Translations::from_json(&bytes).unwrap().to_json().unwrap();
    assert!(
        text.as_bytes() == bytes.as_slice(),
        "translations.json differs after a rewrite"
    );
    for name in ["blurbs-en.json", "blurbs-fr.json"] {
        let bytes = public(name);
        let text = Blurbs::from_json(&bytes).unwrap().to_json().unwrap();
        assert!(
            text.as_bytes() == bytes.as_slice(),
            "{name} differs after a rewrite"
        );
    }
}

/// Reported, not asserted: SuttaCentral has texts the vectorial views do not hold. The
/// numbers are printed with `cargo test -- --nocapture` (design question Q3).
#[test]
fn report_uids_the_views_do_not_know() {
    let nodes: HashSet<String> = ["graph.json", "graph-sim.json"]
        .iter()
        .flat_map(|f| Graph::from_json(&public(f)).unwrap().nodes)
        .map(|n| n.uid)
        .collect();
    for lang in ["en", "fr"] {
        let b = Blurbs::from_json(&public(&format!("blurbs-{lang}.json"))).unwrap();
        let unknown = b.0.keys().filter(|u| !nodes.contains(*u)).count();
        println!(
            "blurbs-{lang}: {unknown} of {} uids are in no view",
            b.0.len()
        );
    }
    let t = Translations::from_json(&public("translations.json")).unwrap();
    for (lang, l) in &t.0 {
        let unknown = l.titles.keys().filter(|u| !nodes.contains(*u)).count();
        println!(
            "translations {lang}: {unknown} of {} uids are in no view",
            l.titles.len()
        );
    }
}
