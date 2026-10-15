//! The data files, loaded once at boot from the image (design v4: `MEM`):
//! every view's graph, the similarity pairs, the translations and the blurbs,
//! each read through the `formats` crate, the only reader of these files. A
//! file missing or unreadable stops the service before it listens, naming the
//! file. The similarity family's shared-vectors check waits for the stamps
//! (task 007: today's files carry none).

use std::fmt;
use std::path::{Path, PathBuf};

use map_formats::FormatError;
use map_formats::blurbs::Blurbs;
use map_formats::graph::Graph;
use map_formats::pairs::Pairs;
use map_formats::stamp::Stamp;
use map_formats::translations::Translations;

/// The variable naming the data directory; `public` (the repository's) when unset.
pub const DATA_VAR: &str = "VIZ_DATA_DIR";

/// Every data file the service reads.
#[derive(Debug)]
pub struct Data {
    /// SuttaCentral's parallels (`graph.json`).
    pub parallels: Graph,
    /// The vectorial space (`graph-sim.json`).
    pub sim: Graph,
    /// Its pairs by similarity (`graph-sim.bin`).
    pub pairs: Pairs,
    /// The test views: PCA, UMAP, t-SNE reductions of the same space.
    pub reductions: [Graph; 3],
    /// The texts translated on SuttaCentral, by language.
    pub translations: Translations,
    /// The blurbs in English and in French.
    pub blurbs: [Blurbs; 2],
}

/// A data file the service could not use: which one, and why.
#[derive(Debug)]
pub struct DataError {
    /// The file's path.
    pub file: PathBuf,
    /// What is wrong with it.
    pub why: String,
}

impl fmt::Display for DataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.file.display(), self.why)
    }
}

impl std::error::Error for DataError {}

impl Data {
    /// Reads every file from `dir`.
    ///
    /// # Errors
    /// [`DataError`] with the first file that is missing or unreadable.
    pub fn load(dir: &Path) -> Result<Data, DataError> {
        let graph = |name: &str| read(dir, name, Graph::from_json);
        let sim = graph("graph-sim.json")?;
        let pairs = read(dir, "graph-sim.bin", |bytes| {
            if let Some(info) = &sim.sim {
                map_formats::pairs::check_sha256(info, bytes)?;
            }
            let pairs = Pairs::from_bytes(bytes)?;
            pairs.check(sim.nodes.len())?;
            Ok(pairs)
        })?;
        Ok(Data {
            parallels: graph("graph.json")?,
            reductions: [
                graph("graph-sim-pca.json")?,
                graph("graph-sim-umap.json")?,
                graph("graph-sim-tsne.json")?,
            ],
            sim,
            pairs,
            translations: read(dir, "translations.json", |bytes| {
                let translations = Translations::from_json(bytes)?;
                translations.check()?;
                Ok(translations)
            })?,
            blurbs: [
                read(dir, "blurbs-en.json", Blurbs::from_json)?,
                read(dir, "blurbs-fr.json", Blurbs::from_json)?,
            ],
        })
    }

    /// The lines `/healthz` shows: each graph file's stamp, `none` until the
    /// data tools write them (task 007).
    #[must_use]
    pub fn stamp_lines(&self) -> Vec<String> {
        let graphs = [
            ("graph.json", &self.parallels),
            ("graph-sim.json", &self.sim),
            ("graph-sim-pca.json", &self.reductions[0]),
            ("graph-sim-umap.json", &self.reductions[1]),
            ("graph-sim-tsne.json", &self.reductions[2]),
        ];
        graphs
            .iter()
            .map(|(name, graph)| format!("{name}: {}", describe(graph.stamp.as_ref())))
            .collect()
    }
}

/// Reads `dir/name` and parses it with `parse`; any failure names the file.
fn read<T>(
    dir: &Path,
    name: &str,
    parse: impl FnOnce(&[u8]) -> Result<T, FormatError>,
) -> Result<T, DataError> {
    let file = dir.join(name);
    let bytes = std::fs::read(&file).map_err(|e| DataError {
        file: file.clone(),
        why: e.to_string(),
    })?;
    parse(&bytes).map_err(|e| DataError {
        file,
        why: e.to_string(),
    })
}

/// One stamp as a line: its sources' short commits and the vectors' digest.
fn describe(stamp: Option<&Stamp>) -> String {
    let Some(stamp) = stamp else {
        return "none".into();
    };
    let short = |hash: &str, n: usize| hash.chars().take(n).collect::<String>();
    let mut parts: Vec<String> = stamp
        .sources
        .iter()
        .map(|source| format!("{} {}", source.name, short(&source.commit, 7)))
        .collect();
    if let Some(sha) = &stamp.vectors_sha {
        parts.push(format!("vectors {}", short(sha, 12)));
    }
    parts.join(" · ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn committed() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../public")
    }

    #[test]
    fn the_committed_files_load() {
        let data = Data::load(&committed()).expect("the committed data");
        assert_eq!(data.sim.nodes.len(), 7286);
        assert!(data.parallels.nodes.len() > 3000);
        for reduction in &data.reductions {
            assert_eq!(reduction.nodes.len(), data.sim.nodes.len());
        }
        assert!(!data.translations.0.is_empty());
        assert_eq!(
            data.stamp_lines()[0],
            "graph.json: none",
            "no stamps before 007"
        );
    }

    /// A copy of the committed files in a fresh directory, to break one.
    fn copy() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "viz-data-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        for entry in std::fs::read_dir(committed()).unwrap() {
            let path = entry.unwrap().path();
            if matches!(
                path.extension().and_then(|e| e.to_str()),
                Some("json" | "bin")
            ) {
                std::fs::copy(&path, dir.join(path.file_name().unwrap())).unwrap();
            }
        }
        dir
    }

    #[test]
    fn a_missing_or_broken_file_is_named() {
        // (case, the file, what is done to it)
        for (case, file, broken) in [
            ("missing", "translations.json", None),
            ("not json", "graph.json", Some(&b"{"[..])),
            (
                "pairs cut short",
                "graph-sim.bin",
                Some(&b"\x01\x00\x02"[..]),
            ),
            ("a reduction missing", "graph-sim-umap.json", None),
        ] {
            let dir = copy();
            let path = dir.join(file);
            match broken {
                None => std::fs::remove_file(&path).unwrap(),
                Some(bytes) => std::fs::write(&path, bytes).unwrap(),
            }
            let error = Data::load(&dir).expect_err(case);
            assert_eq!(error.file, path, "{case}: {error}");
            assert!(error.to_string().contains(file), "{case}: {error}");
        }
    }

    #[test]
    fn a_stamp_is_one_line() {
        assert_eq!(describe(None), "none");
        let stamp = Stamp {
            sources: vec![map_formats::stamp::SourceCommit {
                name: "sc-data".into(),
                commit: "0123456789abcdef".into(),
            }],
            vectors_sha: Some("fedcba9876543210".into()),
        };
        assert_eq!(
            describe(Some(&stamp)),
            "sc-data 0123456 · vectors fedcba987654"
        );
    }
}
