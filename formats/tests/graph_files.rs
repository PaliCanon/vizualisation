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
    let counts = [
        ("graph.json", 3305),
        ("graph-sim.json", 7286),
        ("graph-sim-pca.json", 7286),
        ("graph-sim-umap.json", 7286),
        ("graph-sim-tsne.json", 7286),
    ];
    for (name, nodes) in counts {
        let g = graph(name);
        assert_eq!(g.nodes.len(), nodes, "{name}");
        assert_eq!(g.edges.len() % 3, 0, "{name}");
    }
    assert_eq!(graph("graph.json").edges.len(), 19839);
    assert_eq!(graph("graph-sim.json").edges.len(), 0);
}

#[test]
fn the_vectorial_views_name_the_pair_list_and_the_reductions_are_fixed() {
    for name in [
        "graph-sim.json",
        "graph-sim-pca.json",
        "graph-sim-umap.json",
        "graph-sim-tsne.json",
    ] {
        let g = graph(name);
        assert_eq!(g.sim.as_ref().unwrap().url, "graph-sim.bin", "{name}");
        assert_eq!(g.fixed.is_some(), name != "graph-sim.json", "{name}");
    }
    assert!(graph("graph.json").sim.is_none());
}

#[test]
fn sim_family_shares_node_set() {
    let base = graph("graph-sim.json");
    let uids: Vec<&str> = base.nodes.iter().map(|n| n.uid.as_str()).collect();
    for name in [
        "graph-sim-pca.json",
        "graph-sim-umap.json",
        "graph-sim-tsne.json",
    ] {
        let g = graph(name);
        let other: Vec<&str> = g.nodes.iter().map(|n| n.uid.as_str()).collect();
        assert_eq!(uids, other, "{name}");
        assert_eq!(base.sim, g.sim, "{name}");
    }
}

#[test]
fn rewriting_a_committed_file_keeps_its_content() {
    for name in ["graph.json", "graph-sim.json", "graph-sim-pca.json"] {
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
