//! The map's logic, without a DOM or an engine: what the React page (`src/App.tsx`,
//! `src/map/{data,theme,rust-engine}.ts`) decided before drawing, ported as pure functions
//! shared by the server (the panel, the head, the sitemap) and the browser (`EngineView`).
//!
//! - [`model`]: the data files' types (the `map_formats` crate) and what the page derived
//!   from them: a uid's index, a text's reference, its place, the trail.
//! - [`theme`]: the map's colours per theme.
//! - [`legend`]: the `viz_legend` cookie (coloured and hidden collections, panel open).
//! - [`search`]: `normalize` and the ranking of the search box.
//! - [`texts_lang`]: the texts' language (`viz_textlang`) and its availability masks.
//! - [`open_text`]: what a `/:view/:uid` address opens.
//! - [`draw`]: the buffers the engine is given (points, segments, highlight, camera).

pub mod draw;
pub mod engine_view;
pub mod legend;
pub mod model;
pub mod open_text;
pub mod search;
pub mod texts_lang;
pub mod theme;

/// A small graph for the unit tests: three collections, five texts, three links of two
/// types; `snp1.1` has no link.
///
/// ```text
/// dn1 —full— mn1 —resembling— mn10
///             └——full—— mn2          snp1.1
/// ```
#[cfg(test)]
pub(crate) fn fixture() -> model::Graph {
    let json = r##"{
      "types": ["full", "resembling"],
      "langs": [{"iso": "dn", "name": "Dīgha Nikāya"}, {"iso": "mn", "name": "Majjhima Nikāya"},
                {"iso": "kn", "name": "Khuddaka Nikāya"}],
      "colors": ["#ffffff", "#ff6f8e", "#c792ff"],
      "pitakas": ["sutta"],
      "collections": [{"uid": "dn", "name": "Dīgha Nikāya"}, {"uid": "mn", "name": "Majjhima Nikāya"},
                      {"uid": "snp", "name": "Sutta Nipāta"}],
      "nodes": [
        ["dn1", "DN 1", "The All-embracing Net of Views", "Brahmajāla", 0, 0, 0, 0, 0, 0],
        ["mn1", "MN 1", "The Root of All Things", "Mūlapariyāya", 1, 1, 0, 10, 0, 0],
        ["mn10", "MN 10", "Mindfulness Meditation", "Satipaṭṭhāna", 1, 1, 0, 10, 30, 0],
        ["mn2", "MN 2", "All the Defilements", "Sabbāsava", 1, 1, 0, 10, 0, 40],
        ["snp1.1", "Snp 1.1", "The Snake", "Uraga", 2, 2, 0, -200, 0, 0]
      ],
      "edges": [0, 1, 0, 1, 2, 1, 1, 3, 0]
    }"##;
    model::Graph::from_json(json.as_bytes()).expect("the fixture is a graph")
}

/// A committed data file (`public/`), for the properties on the real data.
#[cfg(test)]
pub(crate) fn public_file(name: &str) -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../public")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}
