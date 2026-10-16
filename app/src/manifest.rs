//! The data files' addresses (design v4, task 006 M1): each file the map
//! fetches is served at a URL naming its content, `/data/<stem>.<hash16>.<ext>`
//! (`graph-sim.bin` → `/data/graph-sim.0123456789abcdef.bin`), so a browser
//! may keep it for good and a new build is never read from the old one's
//! cache. The [`Manifest`] maps each file's name to that URL; the server
//! builds it at boot from the bytes it serves, and the page carries it to the
//! browser (it is serializable), which shares this type.

use std::collections::BTreeMap;

// serde through leptos's server functions, the path their own macro takes:
// `app/Cargo.toml` has no `serde` of its own (a follow-up may add one).
use leptos::server_fn::serde::{Deserialize, Deserializer, Serialize, Serializer};

/// The path every data file is served under.
pub const DATA_PREFIX: &str = "/data/";

/// How many hex digits of the content's sha256 a URL carries.
pub const HASH_LEN: usize = 16;

/// Each data file's name (`graph.json`) and the URL it is served at
/// (`/data/graph.<hash16>.json`), ordered by name.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Manifest(BTreeMap<String, String>);

impl Manifest {
    /// The URL the file `name` is served at, `None` for a name not in it.
    #[must_use]
    pub fn url(&self, name: &str) -> Option<&str> {
        self.0.get(name).map(String::as_str)
    }

    /// Every file name with its URL, ordered by name.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.0
            .iter()
            .map(|(name, url)| (name.as_str(), url.as_str()))
    }

    /// The name of the file served at `/data/<segment>`: `None` for a name
    /// not in the manifest, or one whose hash is not the file's today.
    #[must_use]
    pub fn file_at(&self, segment: &str) -> Option<&str> {
        self.iter()
            .find(|(_, url)| url.strip_prefix(DATA_PREFIX) == Some(segment))
            .map(|(name, _)| name)
    }
}

#[cfg(feature = "ssr")]
impl Manifest {
    /// The manifest of these files (name, bytes as served).
    pub fn from_files<'a>(files: impl IntoIterator<Item = (&'a str, &'a [u8])>) -> Manifest {
        Manifest(
            files
                .into_iter()
                .map(|(name, bytes)| (name.to_owned(), hashed_url(name, bytes)))
                .collect(),
        )
    }
}

/// The URL `name` with these bytes is served at: `/data/<stem>.<hash16>.<ext>`,
/// the hash being the first [`HASH_LEN`] hex digits of the bytes' sha256.
#[cfg(feature = "ssr")]
#[must_use]
pub fn hashed_url(name: &str, bytes: &[u8]) -> String {
    let digest = map_formats::pairs::sha256_hex(bytes);
    let hash = &digest[..HASH_LEN];
    match name.rsplit_once('.') {
        Some((stem, ext)) => format!("{DATA_PREFIX}{stem}.{hash}.{ext}"),
        None => format!("{DATA_PREFIX}{name}.{hash}"),
    }
}

/// The `Content-Type` a data file is served with: JSON, or raw bytes (the
/// similarity pairs).
#[must_use]
pub fn content_type(name: &str) -> &'static str {
    // The names are this service's own (`data::FILES`): exact, lower case.
    match name.rsplit_once('.') {
        Some((_, "json")) => "application/json",
        _ => "application/octet-stream",
    }
}

impl Serialize for Manifest {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Manifest {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        BTreeMap::deserialize(deserializer).map(Manifest)
    }
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use std::collections::BTreeSet;
    use std::path::Path;

    use super::*;
    use crate::data::{Data, FILES};

    /// The manifest of the committed data, as one boot builds it.
    fn boot() -> Manifest {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../public");
        let data = Data::load(&dir).expect("the committed data");
        Manifest::from_files(data.raw.iter().map(|(name, bytes)| (name, bytes.as_ref())))
    }

    #[test]
    fn the_manifest_names_every_file() {
        let manifest = boot();
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../public");
        let names: BTreeSet<&str> = manifest.iter().map(|(name, _)| name).collect();
        assert_eq!(names, BTreeSet::from(FILES), "the nine files");
        for name in FILES {
            let bytes = std::fs::read(dir.join(name)).unwrap();
            let hash = &map_formats::pairs::sha256_hex(&bytes)[..HASH_LEN];
            let (stem, ext) = name.rsplit_once('.').unwrap();
            let url = format!("/data/{stem}.{hash}.{ext}");
            assert_eq!(manifest.url(name), Some(url.as_str()), "{name}");
            let segment = url.strip_prefix(DATA_PREFIX).unwrap();
            assert_eq!(manifest.file_at(segment), Some(name), "{url}");
        }
        let urls: BTreeSet<&str> = manifest.iter().map(|(_, url)| url).collect();
        assert_eq!(urls.len(), FILES.len(), "one URL per file");
        assert_eq!(boot(), manifest, "a second boot names the same URLs");
        // The page carries it: its JSON reads back to the same manifest.
        let json = serde_json::to_string(&manifest).unwrap();
        assert_eq!(serde_json::from_str::<Manifest>(&json).unwrap(), manifest);
        assert!(
            json.starts_with(r#"{"blurbs-en.json":"/data/blurbs-en."#),
            "{json}"
        );
    }

    #[test]
    fn a_file_is_json_or_bytes() {
        assert_eq!(content_type("graph.json"), "application/json");
        assert_eq!(content_type("graph-sim.bin"), "application/octet-stream");
        assert_eq!(
            hashed_url("a.b.json", b""),
            "/data/a.b.e3b0c44298fc1c14.json"
        );
    }
}
