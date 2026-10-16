//! The committed graph files in `public/` parse and agree with themselves.

use map_formats::graph::Graph;
use std::path::PathBuf;

fn public(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../public")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn graph(name: &str) -> Graph {
    Graph::from_json(&public(name)).unwrap_or_else(|e| panic!("{name}: {e}"))
}

#[test]
fn public_graph_files_parse() {
    let counts = [("graph.json", 3305), ("graph-sim.json", 7286)];
    for (name, nodes) in counts {
        let g = graph(name);
        assert_eq!(g.nodes.len(), nodes, "{name}");
        assert_eq!(g.edges.len() % 3, 0, "{name}");
    }
    assert_eq!(graph("graph.json").edges.len(), 19839);
    assert_eq!(graph("graph-sim.json").edges.len(), 0);
}

#[test]
fn the_vectorial_view_names_the_pair_list_and_is_laid_out() {
    let g = graph("graph-sim.json");
    assert_eq!(g.sim.as_ref().unwrap().url, "graph-sim.bin");
    assert!(g.fixed.is_none(), "the force layout places it");
    assert!(graph("graph.json").sim.is_none());
}

/// The similarity family: `graph-sim.json` and its pair list `graph-sim.bin`, whose pairs
/// index its nodes.
#[test]
fn the_pair_list_fits_the_vectorial_space() {
    let base = graph("graph-sim.json");
    let bytes = public("graph-sim.bin");
    map_formats::pairs::check_sha256(base.sim.as_ref().unwrap(), &bytes).unwrap();
    let pairs = map_formats::pairs::Pairs::from_bytes(&bytes).unwrap();
    pairs.check(base.nodes.len()).unwrap();
}

#[test]
fn rewriting_a_committed_file_keeps_its_content() {
    for name in ["graph.json", "graph-sim.json"] {
        let g = graph(name);
        let again = Graph::from_json(g.to_json().unwrap().as_bytes()).unwrap();
        assert_eq!(g, again, "{name}");
    }
}

#[test]
fn graph_json_is_rewritten_byte_for_byte() {
    // The tools (007) become the writer: a rewrite of the committed file changes nothing.
    let bytes = public("graph.json");
    let text = graph("graph.json").to_json().unwrap();
    assert_eq!(text.len(), bytes.len());
    assert!(
        text.as_bytes() == bytes.as_slice(),
        "graph.json differs after a rewrite"
    );
}
