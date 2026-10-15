//! The Barnes-Hut octree: the repulsion between all bodies in `O(n log n)`.
//!
//! A port of ngraph.forcelayout's generated quad tree for three dimensions. The tree is rebuilt
//! every step over the bodies' bounding cube; a cell far enough from a body (its width over the
//! distance below `theta`) acts as one body at its centre of mass.

use crate::{body::Body, prng::SplitMix64};

/// Marks "no node" and "no body".
const NONE: u32 = u32::MAX;

/// A cell: a leaf holds one body; an inner cell holds the sum of its bodies' mass.
#[derive(Clone, Copy, Debug)]
struct Cell {
    /// The body of a leaf; `NONE` for an inner cell.
    body: u32,
    /// Mass of an inner cell.
    mass: f64,
    /// Sum of mass times position of an inner cell.
    moment: [f64; 3],
    /// The cube.
    min: [f64; 3],
    max: [f64; 3],
    children: [u32; 8],
}

impl Cell {
    fn new(min: [f64; 3], max: [f64; 3], body: u32) -> Self {
        Self {
            body,
            mass: 0.0,
            moment: [0.0; 3],
            min,
            max,
            children: [NONE; 8],
        }
    }
}

/// The tree, and the scratch space its queries reuse.
#[derive(Debug)]
pub struct Octree {
    gravity: f64,
    theta: f64,
    cells: Vec<Cell>,
    stack: Vec<(u32, u32)>,
    queue: Vec<u32>,
}

fn norm(d: [f64; 3]) -> f64 {
    d.iter().map(|x| x * x).sum::<f64>().sqrt()
}

fn same_spot(a: [f64; 3], b: [f64; 3]) -> bool {
    (0..3).all(|k| (a[k] - b[k]).abs() < 1e-8)
}

fn jitter(rng: &mut SplitMix64) -> [f64; 3] {
    std::array::from_fn(|_| (rng.next_f64() - 0.5) / 50.0)
}

fn add(total: &mut [f64; 3], more: [f64; 3]) {
    for (t, m) in total.iter_mut().zip(more) {
        *t += m;
    }
}

fn index(i: usize) -> u32 {
    u32::try_from(i).expect("fewer than 2^32 bodies")
}

impl Octree {
    /// An empty tree for these settings.
    pub fn new(gravity: f64, theta: f64) -> Self {
        Self {
            gravity,
            theta,
            cells: Vec::new(),
            stack: Vec::new(),
            queue: Vec::new(),
        }
    }

    /// Rebuild over `bodies`. Bodies on the same spot are moved apart (at random, from `rng`).
    pub fn build(&mut self, bodies: &mut [Body], rng: &mut SplitMix64) {
        self.cells.clear();
        if bodies.is_empty() {
            return;
        }
        // The bounding box, as ngraph computes it: its upper bound starts at JavaScript's
        // `Number.MIN_VALUE` (the smallest positive number), not at minus infinity, so a cloud
        // wholly below zero gets a box that reaches up to zero. Kept, so that the tree has the
        // same cells as ngraph's.
        let (mut min, mut max) = ([f64::MAX; 3], [f64::from_bits(1); 3]);
        for body in bodies.iter().rev() {
            for ((lo, hi), x) in min.iter_mut().zip(&mut max).zip(body.pos) {
                *lo = lo.min(x);
                *hi = hi.max(x);
            }
        }
        let side = (0..3)
            .map(|k| max[k] - min[k])
            .fold(f64::NEG_INFINITY, f64::max);
        self.cells.push(Cell::new(
            min,
            std::array::from_fn(|k| min[k] + side),
            index(bodies.len() - 1),
        ));
        for i in (0..bodies.len() - 1).rev() {
            self.insert(i, bodies, rng);
        }
    }

    fn insert(&mut self, body: usize, bodies: &mut [Body], rng: &mut SplitMix64) {
        self.stack.clear();
        self.stack.push((0, index(body)));
        while let Some((cell, b)) = self.stack.pop() {
            if self.cells[cell as usize].body == NONE {
                self.descend(cell as usize, b as usize, bodies);
            } else if !self.split_leaf(cell as usize, b as usize, bodies, rng) {
                return;
            }
        }
    }

    /// Add body `b` to the inner cell `cell`, into the right child (a new leaf if it is free).
    fn descend(&mut self, cell: usize, b: usize, bodies: &[Body]) {
        let pos = bodies[b].pos;
        let c = &mut self.cells[cell];
        c.mass += bodies[b].mass;
        for (moment, x) in c.moment.iter_mut().zip(pos) {
            *moment += bodies[b].mass * x;
        }
        let (mut lo, mut hi, mut quad) = (c.min, [0.0; 3], 0);
        for k in 0..3 {
            hi[k] = f64::midpoint(lo[k], c.max[k]);
            if pos[k] > hi[k] {
                quad += 1 << k;
                lo[k] = hi[k];
                hi[k] = c.max[k];
            }
        }
        match c.children[quad] {
            NONE => {
                let child = index(self.cells.len());
                self.cells[cell].children[quad] = child;
                self.cells.push(Cell::new(lo, hi, index(b)));
            }
            child => self.stack.push((child, index(b))),
        }
    }

    /// A leaf takes a second body: it becomes an inner cell and both are inserted again.
    /// False when the two sit on the same spot and cannot be moved apart.
    fn split_leaf(
        &mut self,
        cell: usize,
        b: usize,
        bodies: &mut [Body],
        rng: &mut SplitMix64,
    ) -> bool {
        let old = self.cells[cell].body as usize;
        self.cells[cell].body = NONE;
        if same_spot(bodies[old].pos, bodies[b].pos) {
            let (min, max) = (self.cells[cell].min, self.cells[cell].max);
            let mut retries = 3;
            loop {
                let offset = rng.next_f64();
                bodies[old].pos = std::array::from_fn(|k| min[k] + (max[k] - min[k]) * offset);
                retries -= 1;
                if retries == 0 || !same_spot(bodies[old].pos, bodies[b].pos) {
                    break;
                }
            }
            if retries == 0 && same_spot(bodies[old].pos, bodies[b].pos) {
                return false;
            }
        }
        self.stack.push((index(cell), index(old)));
        self.stack.push((index(cell), index(b)));
        true
    }

    /// The repulsion on body `target` from all the others.
    pub fn force_on(&mut self, target: usize, bodies: &[Body], rng: &mut SplitMix64) -> [f64; 3] {
        let (me, mut total) = (&bodies[target], [0.0; 3]);
        self.queue.clear();
        self.queue.push(0);
        let mut head = 0;
        while head < self.queue.len() {
            let cell = self.cells[self.queue[head] as usize];
            head += 1;
            if cell.body != NONE {
                if cell.body as usize != target {
                    let other = &bodies[cell.body as usize];
                    add(&mut total, self.pull(other.pos, other.mass, me, rng));
                }
                continue;
            }
            let centre = cell.moment.map(|m| m / cell.mass);
            let (delta, dist) = Self::offset(centre, me.pos, rng);
            if (cell.max[0] - cell.min[0]) / dist < self.theta {
                let strength = self.gravity * cell.mass * me.mass / (dist * dist * dist);
                add(&mut total, delta.map(|x| strength * x));
            } else {
                self.queue
                    .extend(cell.children.iter().copied().filter(|&ch| ch != NONE));
            }
        }
        total
    }

    /// The vector from `from` to `to` and its length; two points on one spot get a small
    /// random vector.
    fn offset(to: [f64; 3], from: [f64; 3], rng: &mut SplitMix64) -> ([f64; 3], f64) {
        let mut delta = std::array::from_fn(|k| to[k] - from[k]);
        let mut dist = norm(delta);
        if dist == 0.0 {
            delta = jitter(rng);
            dist = norm(delta);
        }
        (delta, dist)
    }

    /// The force a single body exerts on `me`.
    fn pull(&self, pos: [f64; 3], mass: f64, me: &Body, rng: &mut SplitMix64) -> [f64; 3] {
        let (delta, dist) = Self::offset(pos, me.pos, rng);
        let strength = self.gravity * mass * me.mass / (dist * dist * dist);
        delta.map(|x| strength * x)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bodies(n: usize, seed: u64) -> Vec<Body> {
        let mut rng = SplitMix64::new(seed);
        (0..n)
            .map(|_| {
                let pos = [
                    rng.next_f64() * 200.0 - 100.0,
                    rng.next_f64() * 200.0 - 100.0,
                    rng.next_f64() * 200.0 - 100.0,
                ];
                Body::at(pos, 1.0 + (rng.next_f64() * 3.0).floor() / 3.0)
            })
            .collect()
    }

    /// The exact O(n²) repulsion and the scale of its terms.
    fn brute(bodies: &[Body], target: usize, gravity: f64) -> ([f64; 3], f64) {
        let (mut total, mut scale) = ([0.0; 3], 0.0);
        for other in bodies
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != target)
            .map(|(_, b)| b)
        {
            let delta: [f64; 3] = std::array::from_fn(|k| other.pos[k] - bodies[target].pos[k]);
            let dist = delta.iter().map(|x| x * x).sum::<f64>().sqrt();
            let strength = gravity * other.mass * bodies[target].mass / (dist * dist * dist);
            for (t, d) in total.iter_mut().zip(delta) {
                *t += strength * d;
            }
            scale += (strength * dist).abs();
        }
        (total, scale)
    }

    /// (largest error against the scale of the pair forces, mean error against the net force)
    fn errors(theta: f64) -> (f64, f64) {
        let mut set = bodies(200, 9);
        let mut tree = Octree::new(-2.5, theta);
        let mut rng = SplitMix64::new(1);
        tree.build(&mut set, &mut rng);
        let (mut worst, mut mean): (f64, f64) = (0.0, 0.0);
        for i in 0..set.len() {
            let got = tree.force_on(i, &set, &mut rng);
            let (want, scale) = brute(&set, i, -2.5);
            let err = (0..3)
                .map(|k| (got[k] - want[k]).powi(2))
                .sum::<f64>()
                .sqrt();
            let net = want.iter().map(|x| x * x).sum::<f64>().sqrt();
            worst = worst.max(err / scale);
            mean += err / net / 200.0;
        }
        (worst, mean)
    }

    #[test]
    fn theta_zero_is_the_exact_sum() {
        let (worst, mean) = errors(0.0);
        assert!(worst < 1e-9 && mean < 1e-9, "{worst} {mean}");
    }

    #[test]
    fn octree_matches_brute_force() {
        // The error shrinks with theta (measured: worst 0.0015 at 0.2, 0.010 at 0.5, 0.16 at
        // 0.9, against the scale of the pair forces). 0.9 is the map's setting, a coarse one;
        // the parity test checks that the tree is ngraph's, this one that it approximates.
        assert!(errors(0.2).0 < 0.005);
        assert!(errors(0.5).0 < 0.02);
        let (worst, mean) = errors(0.9);
        assert!(worst < 0.25 && mean < 0.10, "{worst} {mean}");
    }

    #[test]
    fn a_wider_theta_is_less_exact_but_not_wild() {
        assert!(errors(0.9).0 >= errors(0.3).0);
        assert!(errors(0.3).0 > 0.0, "an approximation, not the exact sum");
    }

    #[test]
    fn a_lone_body_feels_nothing() {
        let mut set = vec![Body::at([1.0, 2.0, 3.0], 1.0)];
        let mut tree = Octree::new(-2.5, 0.9);
        let mut rng = SplitMix64::new(1);
        tree.build(&mut set, &mut rng);
        assert_eq!(tree.force_on(0, &set, &mut rng), [0.0; 3]);
    }

    #[test]
    fn two_bodies_repel_along_their_line() {
        let mut set = vec![
            Body::at([0.0, 0.0, 0.0], 1.0),
            Body::at([10.0, 0.0, 0.0], 1.0),
        ];
        let mut tree = Octree::new(-2.5, 0.9);
        let mut rng = SplitMix64::new(1);
        tree.build(&mut set, &mut rng);
        // v = gravity * m * m / r^3 = -0.0025; force = v * d, away from the other body.
        let f = tree.force_on(0, &set, &mut rng);
        assert!(
            (f[0] + 0.025).abs() < 1e-12 && f[1] == 0.0 && f[2] == 0.0,
            "{f:?}"
        );
        let g = tree.force_on(1, &set, &mut rng);
        assert!((g[0] - 0.025).abs() < 1e-12, "{g:?}");
    }

    #[test]
    fn coincident_bodies_are_split_and_the_force_is_finite() {
        let mut set = vec![
            Body::at([5.0; 3], 1.0),
            Body::at([5.0; 3], 1.0),
            Body::at([50.0, 0.0, 0.0], 1.0),
        ];
        let mut tree = Octree::new(-2.5, 0.9);
        let mut rng = SplitMix64::new(1);
        tree.build(&mut set, &mut rng);
        assert_ne!(set[0].pos, set[1].pos);
        for i in 0..3 {
            assert!(
                tree.force_on(i, &set, &mut rng)
                    .iter()
                    .all(|x| x.is_finite())
            );
        }
    }

    #[test]
    fn no_bodies_no_tree() {
        let mut set: Vec<Body> = Vec::new();
        let mut tree = Octree::new(-2.5, 0.9);
        tree.build(&mut set, &mut SplitMix64::new(1));
    }
}
