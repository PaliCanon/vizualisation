//! `public/stamp.json` and the family check against the committed files.

use map_formats::{
    graph::Graph,
    manifest::{FILE_NAME, StampFile, check_family},
};
use std::path::PathBuf;

fn public(name: &str) -> Option<Vec<u8>> {
    std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../public")
            .join(name),
    )
    .ok()
}

/// TRANSITIONAL (task 007 flips it): the tools do not stamp yet, so today's family fails the
/// strict check for want of a stamp. Once 007 writes the stamps this test must be replaced by
/// `check_family(..).unwrap()`.
#[test]
fn the_committed_family_is_not_stamped_yet() {
    let sim = Graph::from_json(&public("graph-sim.json").unwrap()).unwrap();
    let pairs = public("graph-sim.bin").unwrap();
    let err = check_family(&sim, &pairs).unwrap_err().to_string();
    assert!(err.contains("no stamp"), "{err}");
}

/// TRANSITIONAL: there is no `public/stamp.json` yet; once there is, it must parse.
#[test]
fn public_stamp_file_parses_when_it_exists() {
    match public(FILE_NAME) {
        None => {}
        Some(bytes) => {
            let f = StampFile::from_json(&bytes).unwrap();
            assert!(f.files.contains_key("graph-sim.bin"));
        }
    }
}
