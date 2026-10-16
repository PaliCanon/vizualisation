//! `stamp.json`: the stamp of the files that cannot carry one inline, and the check that ties
//! the files together.
//!
//! The graph files hold their [`Stamp`] under a `"stamp"` key. `graph-sim.bin` is a bare list
//! of records, `translations.json` is read by iterating its keys and `blurbs-*.json` is a uid
//! map: none can take an extra key or header without breaking the page's reader. Their
//! digests, and the sources they were built from, are listed here instead.

use crate::{FormatError, graph::Graph, stamp::Stamp};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// The file's name in `public/`.
pub const FILE_NAME: &str = "stamp.json";

/// What is known of one file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileEntry {
    /// Digest (sha256, hex) of the file's bytes, computed by the tools.
    pub sha256: String,
}

/// `stamp.json`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StampFile {
    /// Sources and vectors digest, as in a graph file's stamp.
    #[serde(flatten)]
    pub stamp: Stamp,
    /// File name to its entry.
    #[serde(default)]
    pub files: IndexMap<String, FileEntry>,
}

impl StampFile {
    /// Parse the file.
    ///
    /// # Errors
    /// When the bytes are not a stamp file.
    pub fn from_json(bytes: &[u8]) -> Result<StampFile, FormatError> {
        Ok(serde_json::from_slice(bytes)?)
    }

    /// The file's JSON, compact.
    ///
    /// # Errors
    /// When serialisation fails.
    pub fn to_json(&self) -> Result<String, FormatError> {
        Ok(serde_json::to_string(self)?)
    }

    /// Compare the listed digests with the files' actual ones (`(name, sha256)`, computed by
    /// the caller): every file has an entry, every entry has its file, and the digests agree.
    ///
    /// # Errors
    /// With the first file that disagrees.
    pub fn check_files(&self, actual: &[(&str, &str)]) -> Result<(), FormatError> {
        let bad = |why: String| Err(FormatError::Invalid(why));
        for &(name, sha) in actual {
            match self.files.get(name) {
                None => return bad(format!("{name} has no entry in {FILE_NAME}")),
                Some(e) if e.sha256 != sha => {
                    return bad(format!(
                        "{name} is {sha}, {FILE_NAME} lists {}: rebuilt without re-stamping?",
                        e.sha256
                    ));
                }
                Some(_) => {}
            }
        }
        for name in self.files.keys() {
            if !actual.iter().any(|(n, _)| n == name) {
                return bad(format!("{FILE_NAME} lists {name}, which is absent"));
            }
        }
        Ok(())
    }
}

/// The similarity family (`graph-sim.json` and its pair list `graph-sim.bin`) is one build:
/// the graph is stamped, names its pair list and lists that list's digest, which is the digest
/// of `pairs` (the pair list's bytes).
///
/// # Errors
/// With the first rule broken, naming the file.
pub fn check_family(sim: &Graph, pairs: &[u8]) -> Result<(), FormatError> {
    let bad = |why: &str| Err(FormatError::Invalid(why.to_owned()));
    if sim.stamp.is_none() {
        return bad("graph-sim.json has no stamp");
    }
    let Some(info) = &sim.sim else {
        return bad("graph-sim.json names no pair list");
    };
    if info.sha256.is_none() {
        return bad("graph-sim.json lists no digest for its pair list (graph-sim.bin)");
    }
    crate::pairs::check_sha256(info, pairs)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILE: &str = r#"{"sources":[{"name":"sc-data","commit":"abc"}],"vectors_sha":"ff","files":{"graph-sim.bin":{"sha256":"aa"},"translations.json":{"sha256":"bb"}}}"#;

    fn graph(vectors: Option<&str>, uids: &[&str]) -> Graph {
        let rows: Vec<String> = uids
            .iter()
            .map(|u| format!(r#"["{u}","","","",0,0,0,0,0,0]"#))
            .collect();
        let stamp = vectors.map_or(String::new(), |v| {
            format!(r#","stamp":{{"sources":[],"vectors_sha":"{v}"}}"#)
        });
        let text = format!(
            r#"{{"types":[],"langs":[{{"iso":"a","name":"A"}}],"pitakas":["s"],"collections":[{{"uid":"a","name":"A"}}],"nodes":[{}],"edges":[]{stamp}}}"#,
            rows.join(",")
        );
        Graph::from_json(text.as_bytes()).unwrap()
    }

    #[test]
    fn stamp_round_trips() {
        let f = StampFile::from_json(FILE.as_bytes()).unwrap();
        assert_eq!(f.stamp.sources[0].name, "sc-data");
        assert_eq!(f.stamp.vectors_sha.as_deref(), Some("ff"));
        assert_eq!(f.files["graph-sim.bin"].sha256, "aa");
        assert_eq!(f.to_json().unwrap(), FILE);
    }

    #[test]
    fn matching_digests_pass() {
        let f = StampFile::from_json(FILE.as_bytes()).unwrap();
        f.check_files(&[("graph-sim.bin", "aa"), ("translations.json", "bb")])
            .unwrap();
    }

    #[test]
    fn missing_file_entry_is_an_error() {
        let f = StampFile::from_json(FILE.as_bytes()).unwrap();
        let err = f
            .check_files(&[
                ("graph-sim.bin", "aa"),
                ("translations.json", "bb"),
                ("blurbs-en.json", "cc"),
            ])
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("blurbs-en.json") && err.contains("no entry"),
            "{err}"
        );
    }

    #[test]
    fn an_entry_without_its_file_is_an_error() {
        let f = StampFile::from_json(FILE.as_bytes()).unwrap();
        let err = f
            .check_files(&[("graph-sim.bin", "aa")])
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("translations.json") && err.contains("absent"),
            "{err}"
        );
    }

    #[test]
    fn a_changed_file_is_an_error() {
        let f = StampFile::from_json(FILE.as_bytes()).unwrap();
        let err = f
            .check_files(&[("graph-sim.bin", "aa"), ("translations.json", "XX")])
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("translations.json") && err.contains("bb") && err.contains("XX"),
            "{err}"
        );
    }

    /// `graph-sim.json` naming `graph-sim.bin` with `digest`, stamped when `stamped`.
    fn sim_graph(stamped: bool, digest: Option<&str>) -> Graph {
        let mut g = graph(stamped.then_some("ff"), &["x", "y"]);
        g.sim = Some(crate::graph::SimInfo {
            url: "graph-sim.bin".into(),
            min: 0.3,
            max: 0.9,
            default: 0.5,
            range: None,
            sha256: digest.map(str::to_owned),
        });
        g
    }

    const PAIRS: &[u8] = b"\x00\x00\x01\x00\xff\xff";

    #[test]
    fn family_with_one_build_passes() {
        let digest = crate::pairs::sha256_hex(PAIRS);
        check_family(&sim_graph(true, Some(&digest)), PAIRS).unwrap();
    }

    #[test]
    fn another_pair_list_is_an_error() {
        let digest = crate::pairs::sha256_hex(b"other");
        let err = check_family(&sim_graph(true, Some(&digest)), PAIRS)
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("graph-sim.bin") && err.contains("sha256"),
            "{err}"
        );
    }

    #[test]
    fn an_unstamped_graph_is_an_error() {
        let digest = crate::pairs::sha256_hex(PAIRS);
        let err = check_family(&sim_graph(false, Some(&digest)), PAIRS)
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("graph-sim.json") && err.contains("no stamp"),
            "{err}"
        );
    }

    #[test]
    fn a_pair_list_without_its_digest_is_an_error() {
        let err = check_family(&sim_graph(true, None), PAIRS)
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("graph-sim.bin") && err.contains("no digest"),
            "{err}"
        );
        let err = check_family(&graph(Some("ff"), &["x"]), PAIRS)
            .unwrap_err()
            .to_string();
        assert!(err.contains("no pair list"), "{err}");
    }
}
