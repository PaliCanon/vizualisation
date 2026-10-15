//! The layout itself: bodies, springs, and the step that moves them.

use crate::{
    LayoutError,
    body::Body,
    forces::{Spring, add_drag, add_spring_forces},
    integrate::integrate,
    octree::Octree,
    params::{Link, Params},
    prng::SplitMix64,
};

/// A running layout.
#[derive(Debug)]
pub struct Layout {
    params: Params,
    bodies: Vec<Body>,
    springs: Vec<Spring>,
    tree: Octree,
    rng: SplitMix64,
    positions: Vec<[f64; 3]>,
    movement: f64,
}

impl Layout {
    /// A layout of one body per starting position, linked by `links`. A body's mass is
    /// `1 + degree / 3`.
    ///
    /// # Errors
    /// When a link names a body that does not exist, or a position or length is not finite.
    pub fn new(params: Params, start: &[[f64; 3]], links: &[Link]) -> Result<Layout, LayoutError> {
        if let Some(index) = start.iter().position(|p| p.iter().any(|x| !x.is_finite())) {
            return Err(LayoutError::NotFinite {
                what: "position",
                index,
            });
        }
        let springs = springs_of(&params, start.len(), links)?;
        let mut degree = vec![0_u32; start.len()];
        for l in links {
            degree[l.a] += 1;
            if l.b != l.a {
                degree[l.b] += 1;
            }
        }
        let bodies = start
            .iter()
            .zip(&degree)
            .map(|(&p, &d)| Body::at(p, 1.0 + f64::from(d) / 3.0))
            .collect();
        Ok(Layout {
            params,
            bodies,
            springs,
            tree: Octree::new(params.gravity, params.theta),
            rng: SplitMix64::new(params.seed),
            positions: start.to_vec(),
            movement: 0.0,
        })
    }

    /// One step of the simulation; returns the movement it made (see [`Layout::movement`]).
    ///
    /// Every body is repelled by all the others, drags against its velocity, and is pulled by
    /// its springs; then the bodies move, and are drawn towards the origin by
    /// [`Params::centring`].
    pub fn step(&mut self) -> f64 {
        if self.bodies.is_empty() {
            return 0.0;
        }
        self.tree.build(&mut self.bodies, &mut self.rng);
        for i in (0..self.bodies.len()).rev() {
            let f = self.tree.force_on(i, &self.bodies, &mut self.rng);
            self.bodies[i].force = f;
            add_drag(&mut self.bodies[i], self.params.drag);
        }
        add_spring_forces(
            &mut self.bodies,
            &self.springs,
            self.params.spring_coefficient,
            &mut self.rng,
        );
        self.movement = integrate(&mut self.bodies, self.params.time_step);
        let pull = self.params.centring;
        for (body, out) in self.bodies.iter_mut().zip(&mut self.positions) {
            if pull.total_cmp(&1.0).is_ne() {
                body.pos = body.pos.map(|x| x * pull);
            }
            *out = body.pos;
        }
        self.movement
    }

    /// How much the last step moved: the squared per-axis totals of the displacements, summed
    /// and divided by the number of bodies. It falls as the layout settles.
    #[must_use]
    pub fn movement(&self) -> f64 {
        self.movement
    }

    /// Where the bodies are, in the order they were given.
    #[must_use]
    pub fn positions(&self) -> &[[f64; 3]] {
        &self.positions
    }
}

/// The links as springs, with every index and length checked.
fn springs_of(params: &Params, bodies: usize, links: &[Link]) -> Result<Vec<Spring>, LayoutError> {
    links
        .iter()
        .enumerate()
        .map(|(i, l)| {
            if let Some(&body) = [l.a, l.b].iter().find(|&&x| x >= bodies) {
                return Err(LayoutError::LinkOutOfRange {
                    link: i,
                    body,
                    bodies,
                });
            }
            let length = l.length.unwrap_or(params.spring_length);
            if !length.is_finite() {
                return Err(LayoutError::NotFinite {
                    what: "rest length of link",
                    index: i,
                });
            }
            Ok(Spring {
                a: l.a,
                b: l.b,
                length,
            })
        })
        .collect()
}

/// `n` seeded random positions in the cube of half-side `half`: a start for a layout that has
/// none.
#[must_use]
pub fn scatter(n: usize, half: f64, seed: u64) -> Vec<[f64; 3]> {
    let mut rng = SplitMix64::new(seed);
    (0..n)
        .map(|_| std::array::from_fn(|_| (rng.next_f64() * 2.0 - 1.0) * half))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ring(n: usize) -> Vec<Link> {
        (0..n).map(|i| Link::new(i, (i + 1) % n, 30.0)).collect()
    }

    fn run(params: Params, start: &[[f64; 3]], links: &[Link], steps: usize) -> Layout {
        let mut l = Layout::new(params, start, links).unwrap();
        for _ in 0..steps {
            l.step();
        }
        l
    }

    fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
        (0..3).map(|k| (a[k] - b[k]).powi(2)).sum::<f64>().sqrt()
    }

    #[test]
    fn scatter_is_seeded_and_inside_the_cube() {
        let a = scatter(50, 10.0, 3);
        assert_eq!(a, scatter(50, 10.0, 3));
        assert_ne!(a, scatter(50, 10.0, 4));
        assert_eq!(a.len(), 50);
        assert!(a.iter().flatten().all(|x| x.abs() <= 10.0));
    }

    #[test]
    fn same_seed_same_positions() {
        let start = scatter(60, 100.0, 7);
        let a = run(Params::map(), &start, &ring(60), 50);
        let b = run(Params::map(), &start, &ring(60), 50);
        assert_eq!(a.positions(), b.positions());
        assert_ne!(a.positions(), start.as_slice(), "the layout moved");
    }

    #[test]
    fn the_seed_matters_only_for_bodies_on_the_same_spot() {
        let start = scatter(30, 100.0, 7);
        let p = |seed| Params {
            seed,
            ..Params::map()
        };
        assert_eq!(
            run(p(1), &start, &ring(30), 20).positions(),
            run(p(2), &start, &ring(30), 20).positions()
        );
        let mut stacked = start.clone();
        stacked[1] = stacked[0];
        assert_ne!(
            run(p(1), &stacked, &ring(30), 5).positions(),
            run(p(2), &stacked, &ring(30), 5).positions()
        );
    }

    #[test]
    fn two_clusters_separate() {
        // Two cliques of 25 joined by one link, started interleaved in one cloud.
        let count = 50;
        let mut links = vec![Link::new(0, 1, 30.0)];
        for side in 0..2 {
            for first in (side..count).step_by(2) {
                for second in ((first + 2)..count).step_by(2) {
                    links.push(Link::new(first, second, 30.0));
                }
            }
        }
        let layout = run(Params::map(), &scatter(count, 50.0, 5), &links, 400);
        let pos = layout.positions();
        let centroid = |side: usize| {
            let members: Vec<[f64; 3]> = (side..count).step_by(2).map(|i| pos[i]).collect();
            let sum = members
                .iter()
                .fold([0.0; 3], |s, q| [s[0] + q[0], s[1] + q[1], s[2] + q[2]]);
            sum.map(|x| x / 25.0)
        };
        let (left, right) = (centroid(0), centroid(1));
        let own = |i: usize| if i.is_multiple_of(2) { left } else { right };
        let intra = (0..count).map(|i| dist(pos[i], own(i))).sum::<f64>() / 50.0;
        let between = dist(left, right);
        assert!(intra < 0.5 * between, "intra {intra}, between {between}");
    }

    #[test]
    fn energy_falls() {
        let start = scatter(60, 60.0, 11);
        let mut l = Layout::new(Params::map(), &start, &ring(60)).unwrap();
        let early = (0..20).map(|_| l.step()).last().unwrap();
        let late = (0..380).map(|_| l.step()).last().unwrap();
        assert!(late < early, "movement {early} then {late}");
        assert_eq!(l.movement(), late);
    }

    #[test]
    fn a_stretched_link_comes_back_towards_its_rest_length() {
        let start = [[0.0; 3], [200.0, 0.0, 0.0]];
        let p = Params {
            gravity: 0.0,
            ..Params::map()
        };
        let l = run(p, &start, &[Link::new(0, 1, 30.0)], 400);
        let d = dist(l.positions()[0], l.positions()[1]);
        assert!(d < 100.0, "{d}");
    }

    #[test]
    fn centring_scales_positions_after_each_step() {
        let p = Params {
            centring: 0.5,
            ..Params::map()
        };
        let mut l = Layout::new(p, &[[10.0, 20.0, 30.0]], &[]).unwrap();
        l.step();
        assert_eq!(l.positions()[0], [5.0, 10.0, 15.0]);
    }

    #[test]
    fn mass_is_one_plus_degree_over_three() {
        let start = scatter(4, 10.0, 1);
        let links = [Link::plain(0, 1), Link::plain(0, 2), Link::plain(0, 3)];
        let l = Layout::new(Params::map(), &start, &links).unwrap();
        let masses: Vec<f64> = l.bodies.iter().map(|b| b.mass).collect();
        assert_eq!(
            masses,
            [2.0, 1.0 + 1.0 / 3.0, 1.0 + 1.0 / 3.0, 1.0 + 1.0 / 3.0]
        );
    }

    #[test]
    fn a_link_without_a_length_takes_the_default() {
        let start = [[0.0; 3], [10.0, 0.0, 0.0]];
        let l = Layout::new(
            Params {
                spring_length: 7.0,
                ..Params::map()
            },
            &start,
            &[Link::plain(0, 1)],
        )
        .unwrap();
        assert_eq!(l.springs[0].length, 7.0);
    }

    #[test]
    fn empty_graph_and_isolated_nodes() {
        let mut empty = Layout::new(Params::map(), &[], &[]).unwrap();
        assert_eq!(empty.step(), 0.0);
        assert_eq!(empty.positions().len(), 0);
        let start = scatter(5, 1.0, 2);
        let l = run(Params::map(), &start, &[], 10);
        assert_eq!(l.positions().len(), 5);
        assert!(l.positions().iter().flatten().all(|x| x.is_finite()));
        assert_ne!(
            l.positions(),
            start.as_slice(),
            "isolated bodies still repel"
        );
    }

    #[test]
    fn rejects_link_to_missing_node() {
        let err =
            Layout::new(Params::map(), &scatter(3, 1.0, 1), &[Link::plain(0, 3)]).unwrap_err();
        assert_eq!(
            err,
            LayoutError::LinkOutOfRange {
                link: 0,
                body: 3,
                bodies: 3
            }
        );
        assert!(err.to_string().contains("body 3"));
    }

    #[test]
    fn rejects_what_is_not_a_number() {
        let err = Layout::new(Params::map(), &[[0.0, f64::NAN, 0.0]], &[]).unwrap_err();
        assert_eq!(
            err,
            LayoutError::NotFinite {
                what: "position",
                index: 0
            }
        );
        let err = Layout::new(
            Params::map(),
            &scatter(2, 1.0, 1),
            &[Link::new(0, 1, f64::INFINITY)],
        )
        .unwrap_err();
        assert_eq!(
            err,
            LayoutError::NotFinite {
                what: "rest length of link",
                index: 0
            }
        );
    }

    #[test]
    fn a_link_in_an_empty_layout_is_an_error() {
        let err = Layout::new(Params::map(), &[], &[Link::plain(0, 1)]).unwrap_err();
        assert_eq!(
            err,
            LayoutError::LinkOutOfRange {
                link: 0,
                body: 0,
                bodies: 0
            }
        );
    }
}
