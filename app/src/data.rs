//! The data files, loaded once at boot from the image (design v4: `MEM`):
//! every view's graph, the similarity pairs, the translations and the blurbs,
//! each read through the `formats` crate, the only reader of these files. A
//! file missing or unreadable stops the service before it listens, naming the
//! file. The similarity family's shared-vectors check waits for the stamps
//! (task 007: today's files carry none).
//!
//! Each file's bytes are kept as read ([`RawFiles`]): the browser fetches
//! them as committed, at the hashed URLs of [`crate::manifest`]. The
//! `Served` bundle (the data, the manifest, the `TextIndex`) is what every
//! route finds in its context (task 006 M1).

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::body::Bytes;
use map_formats::FormatError;
use map_formats::blurbs::Blurbs;
use map_formats::graph::Graph;
use map_formats::pairs::Pairs;
use map_formats::stamp::Stamp;
use map_formats::translations::Translations;

use crate::manifest::Manifest;
use crate::route::View;

/// The variable naming the data directory; `public` (the repository's) when unset.
pub const DATA_VAR: &str = "VIZ_DATA_DIR";

/// The nine data files, in the order they are read.
pub const FILES: [&str; 9] = [
    "graph-sim.json",
    "graph-sim.bin",
    "graph.json",
    "graph-sim-pca.json",
    "graph-sim-umap.json",
    "graph-sim-tsne.json",
    "translations.json",
    "blurbs-en.json",
    "blurbs-fr.json",
];

/// Each data file's bytes as read at boot, by name: what `/data/…` serves.
#[derive(Clone, Default)]
pub struct RawFiles(BTreeMap<&'static str, Bytes>);

impl RawFiles {
    /// The bytes of the file `name`, `None` for a name not read.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&Bytes> {
        self.0.get(name)
    }

    /// Every file's name and bytes, ordered by name.
    pub fn iter(&self) -> impl Iterator<Item = (&'static str, &Bytes)> {
        self.0.iter().map(|(name, bytes)| (*name, bytes))
    }
}

/// Names and sizes only: the bytes are megabytes.
impl fmt::Debug for RawFiles {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map()
            .entries(self.0.iter().map(|(name, bytes)| (name, bytes.len())))
            .finish()
    }
}

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
    /// Every file's bytes, as read.
    pub raw: RawFiles,
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
        let mut raw = BTreeMap::new();
        for name in FILES {
            let file = dir.join(name);
            let bytes = std::fs::read(&file).map_err(|e| DataError {
                file,
                why: e.to_string(),
            })?;
            raw.insert(name, Bytes::from(bytes));
        }
        let raw = RawFiles(raw);
        let parse = |name: &'static str| Parse {
            dir,
            raw: &raw,
            name,
        };
        let graph = |name| parse(name).with(Graph::from_json);
        let sim = graph("graph-sim.json")?;
        let pairs = parse("graph-sim.bin").with(|bytes| {
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
            translations: parse("translations.json").with(|bytes| {
                let translations = Translations::from_json(bytes)?;
                translations.check()?;
                Ok(translations)
            })?,
            blurbs: [
                parse("blurbs-en.json").with(Blurbs::from_json)?,
                parse("blurbs-fr.json").with(Blurbs::from_json)?,
            ],
            raw,
        })
    }

    /// The graph `view` draws.
    pub(crate) fn graph(&self, view: View) -> &Graph {
        match view {
            View::Parallels => &self.parallels,
            View::Sim => &self.sim,
            View::Pca => &self.reductions[0],
            View::Umap => &self.reductions[1],
            View::Tsne => &self.reductions[2],
        }
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

/// One file read at boot, about to be parsed; a failure names `dir/name`.
struct Parse<'a> {
    dir: &'a Path,
    raw: &'a RawFiles,
    name: &'static str,
}

impl Parse<'_> {
    /// Parses the file's kept bytes with `parse`.
    fn with<T>(self, parse: impl FnOnce(&[u8]) -> Result<T, FormatError>) -> Result<T, DataError> {
        // Every name of `FILES` was read before any is parsed.
        let bytes = self.raw.get(self.name).map_or(&[][..], AsRef::as_ref);
        parse(bytes).map_err(|e| DataError {
            file: self.dir.join(self.name),
            why: e.to_string(),
        })
    }
}

/// What a text is and where it is: its names (as its graph rows give them)
/// and, for each view that has it, its row's index there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TextEntry {
    /// The short name, `MN 1`.
    pub acronym: String,
    /// The English title, `""` when there is none.
    pub en: String,
    /// The Pāli title, `""` when there is none.
    pub root: String,
    /// Each view that has the text, in [`View::ALL`]'s order, with the
    /// text's index in that view's nodes.
    pub views: Vec<(View, usize)>,
}

/// Every text of every view by uid, built once at boot for the server's
/// render and for search (task 006 M1).
#[derive(Clone, Debug, Default)]
pub(crate) struct TextIndex(BTreeMap<String, TextEntry>);

impl TextIndex {
    /// The index of every view's nodes.
    pub(crate) fn build(data: &Data) -> TextIndex {
        let mut texts = BTreeMap::<String, TextEntry>::new();
        for view in View::ALL {
            for (at, node) in data.graph(view).nodes.iter().enumerate() {
                texts
                    .entry(node.uid.clone())
                    .or_insert_with(|| TextEntry {
                        acronym: node.acronym.clone(),
                        en: node.en.clone(),
                        root: node.root.clone(),
                        views: Vec::new(),
                    })
                    .views
                    .push((view, at));
            }
        }
        TextIndex(texts)
    }

    /// The text `uid`, `None` when no view has it.
    pub(crate) fn get(&self, uid: &str) -> Option<&TextEntry> {
        self.0.get(uid)
    }

    /// The index of `uid` in `view`'s nodes, `None` when the view lacks it.
    pub(crate) fn position(&self, uid: &str, view: View) -> Option<usize> {
        self.get(uid)?
            .views
            .iter()
            .find(|(v, _)| *v == view)
            .map(|(_, at)| *at)
    }

    /// The canonical view of `uid` (`open_text::canonical_view` over [`View::ALL`]): the
    /// sitemap's, and the first render's through `open_text`.
    pub(crate) fn canonical(&self, uid: &str) -> Option<View> {
        crate::map::open_text::canonical_view(uid, &View::ALL, &|uid: &str, view: View| {
            self.position(uid, view)
        })
    }

    /// Every text, ordered by uid.
    pub(crate) fn iter(&self) -> impl Iterator<Item = (&str, &TextEntry)> {
        self.0.iter().map(|(uid, entry)| (uid.as_str(), entry))
    }
}

/// What every route finds in its context: the data, its files' manifest
/// and the text index, each built once at boot (cheap to clone).
#[derive(Clone, Debug)]
pub(crate) struct Served {
    /// Every data file, parsed and as read.
    pub data: Arc<Data>,
    /// Each data file's hashed URL.
    pub manifest: Arc<Manifest>,
    /// Every text by uid.
    pub texts: Arc<TextIndex>,
}

impl Served {
    /// The manifest and the index of `data`.
    pub(crate) fn new(data: Arc<Data>) -> Served {
        let manifest =
            Manifest::from_files(data.raw.iter().map(|(name, bytes)| (name, bytes.as_ref())));
        let texts = TextIndex::build(&data);
        Served {
            data,
            manifest: Arc::new(manifest),
            texts: Arc::new(texts),
        }
    }
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
    fn the_text_index_knows_every_uid_of_every_view() {
        let data = Data::load(&committed()).expect("the committed data");
        let index = TextIndex::build(&data);
        for view in View::ALL {
            for (at, node) in data.graph(view).nodes.iter().enumerate() {
                let entry = index.get(&node.uid);
                assert!(entry.is_some(), "{view:?}: {} not in the index", node.uid);
                let entry = entry.unwrap();
                assert!(
                    entry.views.contains(&(view, at)),
                    "{view:?}: {} at {at}",
                    node.uid
                );
                assert_eq!(
                    (&entry.acronym, &entry.en, &entry.root),
                    (&node.acronym, &node.en, &node.root),
                    "{}",
                    node.uid
                );
            }
        }
        let in_view = |view| {
            index
                .iter()
                .filter(|(_, e)| e.views.iter().any(|(v, _)| *v == view))
                .count()
        };
        assert_eq!(in_view(View::Sim), 7286);
        assert_eq!(in_view(View::Parallels), data.parallels.nodes.len());
        let mn1 = index.get("mn1").expect("mn1");
        let order: Vec<View> = mn1.views.iter().map(|(view, _)| *view).collect();
        assert_eq!(
            order,
            View::ALL,
            "every view has mn1, in the views menu's order"
        );
        assert!(index.get("not-a-text").is_none());
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
