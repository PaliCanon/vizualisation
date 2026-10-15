//! The stamp: what a data file was built from.

use serde::{Deserialize, Serialize};

/// A source repository at the commit the data was built from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceCommit {
    /// The repository, for example `sc-data` or `bilara-data`.
    pub name: String,
    /// The commit hash the clone was at.
    pub commit: String,
}

/// What a data file was built from: the source commits, and the vectors' digest for the files
/// that come from the vectorial space.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stamp {
    /// The source repositories and their commits.
    #[serde(default)]
    pub sources: Vec<SourceCommit>,
    /// The digest (sha256, hex) of the vectors the similarity files were computed from; every
    /// file of the similarity family carries the same one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vectors_sha: Option<String>,
}
