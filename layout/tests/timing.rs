//! How long a step takes at the map's size. Run by hand, in release mode:
//! `cargo test -p map-layout --release --test timing -- --ignored --nocapture`.
//! Reported, never gated: the page runs the same code compiled to wasm32, slower than native.

use map_layout::{Layout, Link, Params, scatter};
use std::time::Instant;

#[test]
#[ignore = "a measurement, run by hand in release mode"]
fn step_time_at_the_size_of_the_similarity_view() {
    // 7286 texts, 22154 links (the default threshold of graph-sim.bin), each text linked to
    // near neighbours so that the graph has clusters, as the real one has.
    let n = 7286;
    let mut state = 12_345_u64;
    let mut next = move |bound: usize| {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        usize::try_from(state >> 33).unwrap() % bound
    };
    let links: Vec<Link> = (0..22_154)
        .map(|_| {
            let a = next(n);
            let b = (a + 1 + next(60)) % n;
            Link::new(a, b, 17.5 + 30.0 * 0.5)
        })
        .collect();
    let mut layout = Layout::new(Params::map(), &scatter(n, 800.0, 3), &links).unwrap();
    let t0 = Instant::now();
    for _ in 0..20 {
        layout.step();
    }
    let per_step = t0.elapsed().as_secs_f64() * 1000.0 / 20.0;
    println!(
        "{n} bodies, {} links: {per_step:.1} ms per step (400 steps = {:.1} s)",
        links.len(),
        per_step * 0.4
    );
}
