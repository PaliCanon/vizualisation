//! The Rust layout against ngraph.forcelayout 3.3.1's own trajectory.
//!
//! `tests/fixtures/small.json` holds a 300-node, two-cluster graph with
//! 779 similarity springs, the starting positions, and ngraph's positions after 1, 5, 20 and
//! 400 steps with the map's parameters and the 0.99 pull. This is a comparison within a
//! tolerance, not bit equality: the generator of the jitter and the order of float sums
//! differ, and the layout is not chaotic (a 1e-9 change of the start stays about 1e-9 after
//! 400 steps), so a faithful port stays close to the end.
//!
//! The fixture was made once by `tests/fixtures/gen.cjs`, running ngraph.forcelayout 3.3.1
//! itself under Node; the generator went with Node in task 008 and is in git history
//! (`git show 7d2e46f:layout/tests/fixtures/gen.cjs`). The fixture's own `note` still names
//! it. The committed `small.json` is the reference: it is not regenerated.

use map_layout::{Layout, Link, Params};
use serde_json::Value;

type Point = [f64; 3];

fn points(v: &Value) -> Vec<Point> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|p| std::array::from_fn(|k| p[k].as_f64().unwrap()))
        .collect()
}

fn dist(a: Point, b: Point) -> f64 {
    (0..3).map(|k| (a[k] - b[k]).powi(2)).sum::<f64>().sqrt()
}

/// Radius of gyration around the centroid.
fn gyration(p: &[Point]) -> f64 {
    #[allow(clippy::cast_precision_loss, reason = "a few hundred points")]
    let n = p.len() as f64;
    let centre: Point = std::array::from_fn(|k| p.iter().map(|q| q[k]).sum::<f64>() / n);
    (p.iter().map(|q| dist(*q, centre).powi(2)).sum::<f64>() / n).sqrt()
}

struct Gap {
    step: usize,
    worst: f64,
    gyration: f64,
    relative_gyration: f64,
}

fn gaps() -> Vec<Gap> {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/small.json"
    ))
    .unwrap();
    let fixture: Value = serde_json::from_str(&text).unwrap();
    let start = points(&fixture["start"]);
    let links: Vec<Link> = fixture["links"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| {
            let at = |i: usize| usize::try_from(l[i].as_u64().unwrap()).unwrap();
            Link::new(at(0), at(1), l[2].as_f64().unwrap())
        })
        .collect();
    assert_eq!((start.len(), links.len()), (300, 779));
    let mut layout = Layout::new(Params::map(), &start, &links).unwrap();
    let mut out = Vec::new();
    for step in 1..=400 {
        layout.step();
        if let Some(want) = fixture["snapshots"].get(step.to_string()) {
            let want = points(want);
            let got = layout.positions();
            let worst = got
                .iter()
                .zip(&want)
                .map(|(a, b)| dist(*a, *b))
                .fold(0.0, f64::max);
            let (g_got, g_want) = (gyration(got), gyration(&want));
            out.push(Gap {
                step,
                worst,
                gyration: g_want,
                relative_gyration: (g_got - g_want).abs() / g_want,
            });
        }
    }
    out
}

#[test]
fn the_rust_layout_follows_ngraphs_trajectory() {
    let gaps = gaps();
    for g in &gaps {
        println!(
            "step {:>3}: worst node gap {:.3e}, gyration {:.3} (relative gap {:.3e}), worst gap / gyration {:.3e}",
            g.step,
            g.worst,
            g.gyration,
            g.relative_gyration,
            g.worst / g.gyration
        );
    }
    for g in gaps.iter().filter(|g| g.step <= 5) {
        assert!(
            g.worst <= 1e-3,
            "step {}: worst node gap {}",
            g.step,
            g.worst
        );
    }
    for g in &gaps {
        assert!(
            g.worst <= 0.01 * g.gyration,
            "step {}: worst gap {} of gyration {}",
            g.step,
            g.worst,
            g.gyration
        );
        assert!(
            g.relative_gyration <= 0.01,
            "step {}: gyration gap {}",
            g.step,
            g.relative_gyration
        );
    }
}

#[test]
fn the_comparison_is_not_vacuous() {
    // A layout with other settings must be told apart by the same measure.
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/small.json"
    ))
    .unwrap();
    let fixture: Value = serde_json::from_str(&text).unwrap();
    let start = points(&fixture["start"]);
    let want = points(&fixture["snapshots"]["20"]);
    let mut other = Layout::new(
        Params {
            theta: 0.3,
            gravity: -3.5,
            ..Params::map()
        },
        &start,
        &[],
    )
    .unwrap();
    for _ in 0..20 {
        other.step();
    }
    let worst = other
        .positions()
        .iter()
        .zip(&want)
        .map(|(a, b)| dist(*a, *b))
        .fold(0.0, f64::max);
    assert!(worst > 1.0, "{worst}");
}
