//! `graph-sim.bin`: the links of the vectorial views, as a list of pairs.
//!
//! No header and no footer: little-endian records of three `u16`, six bytes each, `a`, `b` (the
//! nodes' indices, `a < b`) and `t`, the similarity quantised over the range the view's
//! [`SimInfo`] gives: `sim = lo + t / 65535 * (hi - lo)`. The list is sorted by similarity,
//! highest first. The page reads it with exactly this arithmetic (`src/map/data.ts`).

use crate::{FormatError, graph::SimInfo};

/// Bytes per record.
pub const RECORD: usize = 6;

/// One link between two nodes, with its quantised similarity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pair {
    /// Index of the first node.
    pub a: u16,
    /// Index of the second node (greater than `a`).
    pub b: u16,
    /// Similarity, quantised: 0 is the range's low end, 65535 its high end.
    pub t: u16,
}

/// A link with its similarity as a number, above some threshold.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Link {
    /// Index of the first node.
    pub a: u16,
    /// Index of the second node.
    pub b: u16,
    /// The similarity.
    pub sim: f32,
}

/// The pair list of a vectorial view.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Pairs {
    /// The records, in file order.
    pub records: Vec<Pair>,
}

/// The `[lo, hi]` a view's pair list spans: its `range`, else `[0, 1]`.
#[must_use]
pub fn range_of(sim: &SimInfo) -> [f64; 2] {
    sim.range.unwrap_or([0.0, 1.0])
}

/// `sim` as the 16-bit value written to the file (clamped to the range).
#[must_use]
pub fn quantise(sim: f64, range: [f64; 2]) -> u16 {
    let [lo, hi] = range;
    let unit = ((sim - lo) / (hi - lo)).clamp(0.0, 1.0);
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to 0..=1 then scaled to 0..=65535: it fits u16"
    )]
    let t = (unit * 65535.0).round() as u16;
    t
}

/// The similarity a 16-bit value stands for, as the page computes it (single precision).
#[must_use]
pub fn dequantise(t: u16, range: [f64; 2]) -> f32 {
    let [lo, hi] = range;
    #[allow(
        clippy::cast_possible_truncation,
        reason = "the page stores similarities as f32"
    )]
    let sim = (lo + (f64::from(t) / 65535.0) * (hi - lo)) as f32;
    sim
}

impl Pairs {
    /// Read a pair list.
    ///
    /// # Errors
    /// When the length is not a whole number of records.
    pub fn from_bytes(bytes: &[u8]) -> Result<Pairs, FormatError> {
        if !bytes.len().is_multiple_of(RECORD) {
            return Err(FormatError::Invalid(format!(
                "{} bytes is not a whole number of {RECORD}-byte records",
                bytes.len()
            )));
        }
        let records = bytes
            .as_chunks::<RECORD>()
            .0
            .iter()
            .map(|r| Pair {
                a: u16::from_le_bytes([r[0], r[1]]),
                b: u16::from_le_bytes([r[2], r[3]]),
                t: u16::from_le_bytes([r[4], r[5]]),
            })
            .collect();
        Ok(Pairs { records })
    }

    /// The file's bytes.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.records.len() * RECORD);
        for p in &self.records {
            out.extend_from_slice(&p.a.to_le_bytes());
            out.extend_from_slice(&p.b.to_le_bytes());
            out.extend_from_slice(&p.t.to_le_bytes());
        }
        out
    }

    /// A pair list from similarities, quantised over `range`, in the order given.
    #[must_use]
    pub fn from_similarities(items: &[(u16, u16, f64)], range: [f64; 2]) -> Pairs {
        let records = items
            .iter()
            .map(|&(a, b, sim)| Pair {
                a,
                b,
                t: quantise(sim, range),
            })
            .collect();
        Pairs { records }
    }

    /// The links whose similarity is at least `threshold` (the page's `linksAt`).
    #[must_use]
    pub fn links_at(&self, range: [f64; 2], threshold: f32) -> Vec<Link> {
        self.records
            .iter()
            .map(|p| Link {
                a: p.a,
                b: p.b,
                sim: dequantise(p.t, range),
            })
            .filter(|l| l.sim >= threshold)
            .collect()
    }

    /// Check the list against a graph of `node_count` nodes: every index exists, `a < b`.
    ///
    /// # Errors
    /// With the first record that breaks it.
    pub fn check(&self, node_count: usize) -> Result<(), FormatError> {
        for (k, p) in self.records.iter().enumerate() {
            if p.a >= p.b {
                return Err(FormatError::Invalid(format!(
                    "pair {k}: {} and {} are not in order (a < b)",
                    p.a, p.b
                )));
            }
            if usize::from(p.b) >= node_count {
                return Err(FormatError::Invalid(format!(
                    "pair {k} names node {}, only {node_count} nodes",
                    p.b
                )));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RANGE: [f64; 2] = [0.29, 0.95];

    #[test]
    fn record_layout_is_little_endian_6_bytes() {
        let bytes = [
            0x22, 0x19, 0x24, 0x19, 0xff, 0xff, 0x01, 0x00, 0x02, 0x00, 0x00, 0x80,
        ];
        let pairs = Pairs::from_bytes(&bytes).unwrap();
        assert_eq!(
            pairs.records,
            [
                Pair {
                    a: 0x1922,
                    b: 0x1924,
                    t: 0xffff
                },
                Pair {
                    a: 1,
                    b: 2,
                    t: 0x8000
                }
            ]
        );
        assert_eq!(pairs.to_bytes(), bytes);
    }

    #[test]
    fn rejects_truncated_file() {
        for len in [1, 5, 7, 11] {
            let err = Pairs::from_bytes(&vec![0; len]).unwrap_err().to_string();
            assert!(err.contains("records"), "{len}: {err}");
        }
        assert_eq!(Pairs::from_bytes(&[]).unwrap().records.len(), 0);
    }

    #[test]
    fn write_then_read_is_identity() {
        let items = [(1, 2, 0.95), (3, 9, 0.6), (4, 5, 0.29)];
        let pairs = Pairs::from_similarities(&items, RANGE);
        assert_eq!(Pairs::from_bytes(&pairs.to_bytes()).unwrap(), pairs);
    }

    #[test]
    fn quantisation_matches_js() {
        // Math.round(clamp((s - lo) / (hi - lo), 0, 1) * 65535), values from node.
        assert_eq!(quantise(0.56, RANGE), 26810);
        assert_eq!(quantise(0.6, RANGE), 30782);
        assert_eq!(quantise(0.3, RANGE), 993);
        assert_eq!(quantise(0.95, RANGE), 65535);
        assert_eq!(quantise(0.29, RANGE), 0);
        assert_eq!(quantise(0.1, RANGE), 0, "below the range clamps");
        assert_eq!(quantise(2.0, RANGE), 65535, "above the range clamps");
        assert_eq!(
            quantise(0.5, [0.0, 1.0]),
            32768,
            "half rounds up, as Math.round"
        );
    }

    #[test]
    fn dequantisation_matches_the_page() {
        assert_eq!(dequantise(0, RANGE), 0.29);
        assert_eq!(dequantise(65535, RANGE), 0.95);
        let mid = dequantise(26810, RANGE);
        assert!((mid - 0.560_002).abs() < 1e-6, "{mid}");
    }

    #[test]
    fn range_defaults_to_unit_interval() {
        let mut sim = SimInfo {
            url: "x.bin".into(),
            min: 0.1,
            max: 0.9,
            default: 0.5,
            range: None,
            sha256: None,
        };
        assert_eq!(range_of(&sim), [0.0, 1.0]);
        sim.range = Some([0.29, 0.95]);
        assert_eq!(range_of(&sim), [0.29, 0.95]);
    }

    #[test]
    fn links_at_keeps_the_pairs_at_or_above_the_threshold() {
        let items = [(1, 2, 0.95), (3, 9, 0.6), (4, 5, 0.3)];
        let pairs = Pairs::from_similarities(&items, RANGE);
        let links = pairs.links_at(RANGE, 0.56);
        assert_eq!(
            links.iter().map(|l| (l.a, l.b)).collect::<Vec<_>>(),
            [(1, 2), (3, 9)]
        );
        assert_eq!(pairs.links_at(RANGE, 0.0).len(), 3);
        assert_eq!(pairs.links_at(RANGE, 1.0).len(), 0);
    }

    #[test]
    fn check_rejects_missing_nodes_and_unordered_pairs() {
        let ok = Pairs {
            records: vec![Pair { a: 0, b: 1, t: 5 }],
        };
        assert!(ok.check(2).is_ok());
        assert!(ok.check(1).unwrap_err().to_string().contains("node 1"));
        let bad = Pairs {
            records: vec![Pair { a: 2, b: 2, t: 5 }],
        };
        assert!(bad.check(9).unwrap_err().to_string().contains("a < b"));
    }
}
