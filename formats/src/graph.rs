//! `graph.json` and `graph-sim.json`: the nodes of a view with their positions, its links (or,
//! for the vectorial view, where to find them).

use crate::{FormatError, stamp::Stamp};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// One text, as a row of ten values: `[uid, acronym, en, root, coll, group, pitaka, x, y, z]`.
#[derive(Clone, Debug, PartialEq)]
pub struct NodeRow {
    /// SuttaCentral's id, for example `mn1`.
    pub uid: String,
    /// The short name, for example `MN 1`.
    pub acronym: String,
    /// The English title, `""` when there is none.
    pub en: String,
    /// The Pāli title, `""` when there is none.
    pub root: String,
    /// Index into [`Graph::collections`].
    pub coll: u32,
    /// Index into [`Graph::langs`]: the colour group.
    pub group: u32,
    /// Index into [`Graph::pitakas`].
    pub pitaka: u32,
    /// Position, x.
    pub x: f64,
    /// Position, y.
    pub y: f64,
    /// Position, z.
    pub z: f64,
}

/// A coordinate, written as an integer when it is one (the committed files have integer
/// positions, and a rewrite should not turn `337` into `337.0`).
#[derive(Clone, Copy)]
struct Coord(f64);

impl Serialize for Coord {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let v = self.0;
        if !v.is_finite() {
            // serde_json would write `null`, which the page reads back as 0.
            return Err(serde::ser::Error::custom(
                "a coordinate is not a finite number",
            ));
        }
        if v.fract() == 0.0 && v.abs() < 1e15 {
            #[allow(
                clippy::cast_possible_truncation,
                reason = "integral and below 1e15, checked"
            )]
            s.serialize_i64(v as i64)
        } else {
            s.serialize_f64(v)
        }
    }
}

impl<'de> Deserialize<'de> for Coord {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        f64::deserialize(d).map(Coord)
    }
}

/// The row as JSON has it: a ten-element array.
type Row = (
    String,
    String,
    String,
    String,
    u32,
    u32,
    u32,
    Coord,
    Coord,
    Coord,
);

impl Serialize for NodeRow {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let row: Row = (
            self.uid.clone(),
            self.acronym.clone(),
            self.en.clone(),
            self.root.clone(),
            self.coll,
            self.group,
            self.pitaka,
            Coord(self.x),
            Coord(self.y),
            Coord(self.z),
        );
        row.serialize(s)
    }
}

impl<'de> Deserialize<'de> for NodeRow {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let (uid, acronym, en, root, coll, group, pitaka, x, y, z) = Row::deserialize(d)?;
        Ok(NodeRow {
            uid,
            acronym,
            en,
            root,
            coll,
            group,
            pitaka,
            x: x.0,
            y: y.0,
            z: z.0,
        })
    }
}

/// A colour group (the legend's entries).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Group {
    /// Short id (`dn`, `mn`, ...).
    pub iso: String,
    /// Display name.
    pub name: String,
}

/// A collection of texts (a nikāya, a book).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Collection {
    /// SuttaCentral's id of the collection.
    pub uid: String,
    /// Display name.
    pub name: String,
}

/// Where a vectorial view's links are and how to read them: the pair list is `graph-sim.bin`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SimInfo {
    /// File name of the pair list, next to the graph file.
    pub url: String,
    /// Lowest similarity in the pair list.
    pub min: f64,
    /// Highest similarity in the pair list.
    pub max: f64,
    /// The threshold the view opens with.
    pub default: f64,
    /// `[lo, hi]` the pair list's 16-bit values span; `[0, 1]` when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range: Option<[f64; 2]>,
    /// Digest (sha256, hex) of the pair list's bytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
}

/// A view's data file.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Graph {
    /// Title of the legend.
    #[serde(
        rename = "legendTitle",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub legend_title: Option<String>,
    /// Link types, in the order the links' type index refers to.
    pub types: Vec<String>,
    /// The colour groups.
    pub langs: Vec<Group>,
    /// One colour per group, `#rrggbb`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub colors: Option<Vec<String>>,
    /// The three piṭakas (or the ones present).
    pub pitakas: Vec<String>,
    /// The collections.
    pub collections: Vec<Collection>,
    /// The texts.
    pub nodes: Vec<NodeRow>,
    /// Links, flat: `[a, b, type, a, b, type, ...]`; empty for a vectorial view.
    pub edges: Vec<u32>,
    /// A vectorial view: where its pairs are.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sim: Option<SimInfo>,
    /// The positions are the data (a reduction): no live layout.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixed: Option<bool>,
    /// What this file was built from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stamp: Option<Stamp>,
    /// Keys this version does not know, kept so a rewrite does not lose them.
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

impl Graph {
    /// Parse a graph file.
    ///
    /// # Errors
    /// When the bytes are not a graph file, or the file contradicts itself ([`Graph::check`]).
    pub fn from_json(bytes: &[u8]) -> Result<Graph, FormatError> {
        let graph: Graph = serde_json::from_slice(bytes)?;
        graph.check()?;
        Ok(graph)
    }

    /// The file's JSON, compact.
    ///
    /// # Errors
    /// When a position or a similarity threshold is not a finite number (JSON would turn it
    /// into `null`).
    pub fn to_json(&self) -> Result<String, FormatError> {
        if let Some(sim) = &self.sim {
            let range = sim.range.iter().flatten();
            let numbers = [sim.min, sim.max, sim.default]
                .into_iter()
                .chain(range.copied());
            if numbers.into_iter().any(|x| !x.is_finite()) {
                return Err(FormatError::Invalid(
                    "sim: a threshold or range is not a finite number".into(),
                ));
            }
        }
        Ok(serde_json::to_string(self)?)
    }

    /// Check the file against itself: every index points at something, links come in triples.
    ///
    /// # Errors
    /// With the first inconsistency found.
    pub fn check(&self) -> Result<(), FormatError> {
        self.check_edges()?;
        self.check_nodes()
    }

    fn check_edges(&self) -> Result<(), FormatError> {
        let bad = |why: String| Err(FormatError::Invalid(why));
        if !self.edges.len().is_multiple_of(3) {
            return bad(format!(
                "{} edge values, expected triples",
                self.edges.len()
            ));
        }
        let n = self.nodes.len();
        for &[a, b, t] in self.edges.as_chunks::<3>().0 {
            if a as usize >= n || b as usize >= n {
                return bad(format!(
                    "edge {a}-{b} names node {}, only {n} nodes",
                    a.max(b)
                ));
            }
            if t as usize >= self.types.len() {
                return bad(format!(
                    "edge {a}-{b} has type {t}, only {} types",
                    self.types.len()
                ));
            }
        }
        Ok(())
    }

    fn check_nodes(&self) -> Result<(), FormatError> {
        let bad = |why: String| Err(FormatError::Invalid(why));
        for node in &self.nodes {
            let uid = &node.uid;
            if node.coll as usize >= self.collections.len() {
                return bad(format!(
                    "{uid}: collection {} of {}",
                    node.coll,
                    self.collections.len()
                ));
            }
            if node.group as usize >= self.langs.len() {
                return bad(format!(
                    "{uid}: group {} of {}",
                    node.group,
                    self.langs.len()
                ));
            }
            if node.pitaka as usize >= self.pitakas.len() {
                return bad(format!(
                    "{uid}: pitaka {} of {}",
                    node.pitaka,
                    self.pitakas.len()
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROW: &str =
        r#"["mn1","MN 1","The Root of All Things","Mūlapariyāya",1,2,0,-11,337.5,187]"#;

    fn graph(extra: &str) -> String {
        format!(
            r#"{{"types":["full"],"langs":[{{"iso":"dn","name":"Dīgha"}},{{"iso":"mn","name":"Majjhima"}},{{"iso":"sn","name":"S"}}],"pitakas":["sutta"],"collections":[{{"uid":"dn","name":"D"}},{{"uid":"mn","name":"M"}}],"nodes":[{ROW},{ROW}],"edges":[0,1,0]{extra}}}"#
        )
    }

    #[test]
    fn node_row_round_trips() {
        let row: NodeRow = serde_json::from_str(ROW).unwrap();
        assert_eq!(row.uid, "mn1");
        assert_eq!((row.coll, row.group, row.pitaka), (1, 2, 0));
        assert_eq!((row.x, row.y, row.z), (-11.0, 337.5, 187.0));
        // Integral positions are written as integers, as the committed files have them.
        assert_eq!(serde_json::to_string(&row).unwrap(), ROW);
    }

    #[test]
    fn node_row_wrong_length_errors() {
        assert!(serde_json::from_str::<NodeRow>(r#"["a","b","c","d",0,0,0,1,2]"#).is_err());
        assert!(serde_json::from_str::<NodeRow>(r#"["a","b","c","d",0,0,0,1,2,3,4]"#).is_err());
        assert!(serde_json::from_str::<NodeRow>(r#"["a","b","c","d","x",0,0,1,2,3]"#).is_err());
    }

    #[test]
    fn graph_without_stamp_parses() {
        let g = Graph::from_json(graph("").as_bytes()).unwrap();
        assert_eq!(g.nodes.len(), 2);
        assert_eq!(g.edges, [0, 1, 0]);
        assert!(g.stamp.is_none() && g.sim.is_none() && g.fixed.is_none());
    }

    #[test]
    fn graph_with_stamp_parses() {
        let g = Graph::from_json(
            graph(r#","stamp":{"sources":[{"name":"sc-data","commit":"abc"}],"vectors_sha":"ff"}"#)
                .as_bytes(),
        )
        .unwrap();
        let stamp = g.stamp.unwrap();
        assert_eq!(stamp.sources[0].commit, "abc");
        assert_eq!(stamp.vectors_sha.as_deref(), Some("ff"));
    }

    #[test]
    fn unknown_key_is_kept_not_rejected() {
        let g = Graph::from_json(graph(r#","future":{"a":1}"#).as_bytes()).unwrap();
        assert!(g.extra.contains_key("future"));
        assert!(g.to_json().unwrap().contains(r#""future":{"a":1}"#));
    }

    #[test]
    fn sim_info_reads_the_optional_keys() {
        let g = Graph::from_json(
            graph(r#","sim":{"url":"graph-sim.bin","min":0.29,"max":0.95,"default":0.56,"range":[0.29,0.95],"sha256":"ab"},"fixed":true"#)
                .as_bytes(),
        )
        .unwrap();
        let sim = g.sim.unwrap();
        assert_eq!(sim.range, Some([0.29, 0.95]));
        assert_eq!(sim.sha256.as_deref(), Some("ab"));
        assert_eq!(g.fixed, Some(true));
    }

    #[test]
    fn graph_round_trips_through_its_own_json() {
        let g = Graph::from_json(graph(r#","fixed":true"#).as_bytes()).unwrap();
        let again = Graph::from_json(g.to_json().unwrap().as_bytes()).unwrap();
        assert_eq!(g, again);
    }

    #[test]
    fn check_rejects_what_points_nowhere() {
        let bad = |what: &str, extra: &str| {
            let text = graph(extra).replace(
                what.split('|').next().unwrap(),
                what.split('|').nth(1).unwrap(),
            );
            Graph::from_json(text.as_bytes()).unwrap_err().to_string()
        };
        assert!(bad("[0,1,0]|[0,1]", "").contains("triples"));
        assert!(bad("[0,1,0]|[0,2,0]", "").contains("node 2"));
        assert!(bad("[0,1,0]|[0,1,3]", "").contains("type 3"));
        assert!(bad("1,2,0,-11|9,2,0,-11", "").contains("collection 9"));
        assert!(bad("1,2,0,-11|1,7,0,-11", "").contains("group 7"));
        assert!(bad("1,2,0,-11|1,2,5,-11", "").contains("pitaka 5"));
    }

    #[test]
    fn to_json_refuses_a_coordinate_that_is_not_finite() {
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let mut g = Graph::from_json(graph("").as_bytes()).unwrap();
            g.nodes[1].y = bad;
            let err = g
                .to_json()
                .expect_err(&format!("{bad} must not be written"));
            assert!(err.to_string().contains("finite"), "{err}");
        }
    }

    #[test]
    fn to_json_refuses_a_similarity_range_that_is_not_finite() {
        let mut g = Graph::from_json(
            graph(r#","sim":{"url":"b.bin","min":0.1,"max":0.9,"default":0.5}"#).as_bytes(),
        )
        .unwrap();
        g.sim.as_mut().unwrap().max = f64::NAN;
        assert!(g.to_json().is_err());
        g.sim.as_mut().unwrap().max = 0.9;
        g.sim.as_mut().unwrap().range = Some([0.1, f64::INFINITY]);
        assert!(g.to_json().is_err());
    }
}
