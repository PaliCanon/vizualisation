//! Why a layout could not be set up.

use std::fmt;

/// An input the layout cannot start from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutError {
    /// A link names a body that does not exist.
    LinkOutOfRange {
        /// Index of the link in the list given.
        link: usize,
        /// The body it names.
        body: usize,
        /// How many bodies there are.
        bodies: usize,
    },
    /// A starting position or a rest length is NaN or infinite.
    NotFinite {
        /// What was not finite.
        what: &'static str,
        /// Its index.
        index: usize,
    },
}

impl fmt::Display for LayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LinkOutOfRange { link, body, bodies } => {
                write!(f, "link {link} names body {body}, only {bodies} bodies")
            }
            Self::NotFinite { what, index } => write!(f, "{what} {index} is not a finite number"),
        }
    }
}

impl std::error::Error for LayoutError {}
