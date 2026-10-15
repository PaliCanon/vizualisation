//! The committed `graph-sim.bin` against the graph that names it.

use map_formats::{
    graph::Graph,
    pairs::{Pairs, RECORD, range_of},
};
use std::path::PathBuf;

fn public(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../public")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

#[test]
fn public_pairs_file_round_trips_byte_for_byte() {
    let bytes = public("graph-sim.bin");
    assert_eq!(bytes.len(), 556_668);
    let pairs = Pairs::from_bytes(&bytes).unwrap();
    assert_eq!(pairs.records.len(), bytes.len() / RECORD);
    assert_eq!(pairs.records.len(), 92_778);
    assert!(
        pairs.to_bytes() == bytes,
        "graph-sim.bin differs after a rewrite"
    );
}

#[test]
fn pairs_indices_are_ordered_and_below_node_count() {
    let graph = Graph::from_json(&public("graph-sim.json")).unwrap();
    let pairs = Pairs::from_bytes(&public("graph-sim.bin")).unwrap();
    pairs.check(graph.nodes.len()).unwrap();
}

#[test]
fn the_list_is_sorted_by_similarity_highest_first() {
    let pairs = Pairs::from_bytes(&public("graph-sim.bin")).unwrap();
    assert!(pairs.records.windows(2).all(|w| w[0].t >= w[1].t));
    assert_eq!(pairs.records[0].t, 65_535);
    assert_eq!(pairs.records.last().unwrap().t, 0);
}

#[test]
fn links_at_the_default_threshold_match_the_pages_count() {
    // Counted with the page's arithmetic (single precision) on the committed file.
    let graph = Graph::from_json(&public("graph-sim.json")).unwrap();
    let sim = graph.sim.as_ref().unwrap();
    let range = range_of(sim);
    let pairs = Pairs::from_bytes(&public("graph-sim.bin")).unwrap();
    assert_eq!(pairs.links_at(range, 0.56).len(), 22_154);
    assert_eq!(pairs.links_at(range, 0.7).len(), 11_934);
    assert_eq!(pairs.links_at(range, 0.9).len(), 3_365);
}

#[test]
fn the_graph_carries_the_bin_digest_once_the_tools_stamp_it() {
    // Transitional (007 writes it): today's file has no digest; when it has one, it is the
    // file's. The digest itself is computed by the tools, not by this crate.
    let graph = Graph::from_json(&public("graph-sim.json")).unwrap();
    let sha = graph.sim.unwrap().sha256;
    assert!(
        sha.is_none_or(|s| s.len() == 64),
        "a sha256 is 64 hex digits"
    );
}
