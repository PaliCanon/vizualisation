//! The types of every data file of the map (`graph*.json`, `graph-sim.bin`, `translations.json`,
//! `blurbs-*.json`) and their stamp: the only reader (the app) and writer (the tools).
//!
//! The files are the contract between the tools that build them and the app that serves
//! them, so each has one type here and no other crate parses them. Nothing reads a clock or
//! the file system: callers hand over bytes, so the crate builds natively and for wasm32.

pub mod blurbs;
pub mod error;
pub mod graph;
pub mod pairs;
pub mod stamp;
pub mod translations;

pub use error::FormatError;
