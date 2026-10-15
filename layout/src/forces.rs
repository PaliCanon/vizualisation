//! The springs and the drag.

use crate::{body::Body, prng::SplitMix64};

/// A link with its rest length settled.
#[derive(Clone, Copy, Debug)]
pub struct Spring {
    /// First body.
    pub a: usize,
    /// Second body.
    pub b: usize,
    /// Rest length.
    pub length: f64,
}

/// Add every spring's pull (or push) to the bodies' forces. A spring of length zero between
/// two bodies on the same spot is split by a little random offset, as in ngraph.
pub fn add_spring_forces(
    bodies: &mut [Body],
    springs: &[Spring],
    coefficient: f64,
    rng: &mut SplitMix64,
) {
    // ngraph walks its list from the end; the order only matters to the last digits.
    for s in springs.iter().rev() {
        let mut d: [f64; 3] = std::array::from_fn(|k| bodies[s.b].pos[k] - bodies[s.a].pos[k]);
        let mut r = d.iter().map(|x| x * x).sum::<f64>().sqrt();
        if r == 0.0 {
            d = std::array::from_fn(|_| (rng.next_f64() - 0.5) / 50.0);
            r = d.iter().map(|x| x * x).sum::<f64>().sqrt();
        }
        let pull = coefficient * (r - s.length) / r;
        for (k, delta) in d.iter().enumerate() {
            bodies[s.a].force[k] += pull * delta;
            bodies[s.b].force[k] -= pull * delta;
        }
    }
}

/// Add the drag, against the velocity, to a body's force.
pub fn add_drag(body: &mut Body, coefficient: f64) {
    for k in 0..3 {
        body.force[k] -= coefficient * body.vel[k];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pair(distance: f64) -> Vec<Body> {
        vec![Body::at([0.0; 3], 1.0), Body::at([distance, 0.0, 0.0], 1.0)]
    }

    const LINK: [Spring; 1] = [Spring {
        a: 0,
        b: 1,
        length: 30.0,
    }];

    #[test]
    fn a_stretched_spring_pulls_the_bodies_together() {
        let mut b = pair(40.0);
        add_spring_forces(&mut b, &LINK, 0.5, &mut SplitMix64::new(1));
        // k * (r - length) = 0.5 * 10
        assert!((b[0].force[0] - 5.0).abs() < 1e-12, "{:?}", b[0].force);
        assert!((b[1].force[0] + 5.0).abs() < 1e-12, "{:?}", b[1].force);
    }

    #[test]
    fn a_squeezed_spring_pushes_them_apart() {
        let mut b = pair(20.0);
        add_spring_forces(&mut b, &LINK, 0.5, &mut SplitMix64::new(1));
        assert!((b[0].force[0] + 5.0).abs() < 1e-12 && (b[1].force[0] - 5.0).abs() < 1e-12);
    }

    #[test]
    fn a_spring_at_rest_length_does_nothing() {
        let mut b = pair(30.0);
        add_spring_forces(&mut b, &LINK, 0.5, &mut SplitMix64::new(1));
        assert!(b[0].force[0].abs() < 1e-12 && b[1].force[0].abs() < 1e-12);
    }

    #[test]
    fn springs_add_to_the_force_already_there() {
        let mut b = pair(40.0);
        b[0].force = [1.0, 2.0, 3.0];
        add_spring_forces(&mut b, &LINK, 0.5, &mut SplitMix64::new(1));
        assert_eq!(b[0].force[1], 2.0);
        assert!((b[0].force[0] - 6.0).abs() < 1e-12);
    }

    #[test]
    fn coincident_spring_ends_give_no_nan() {
        let mut b = pair(0.0);
        add_spring_forces(&mut b, &LINK, 0.5, &mut SplitMix64::new(1));
        assert!(b.iter().all(|x| x.force.iter().all(|f| f.is_finite())));
        assert!(b[0].force.iter().any(|f| *f != 0.0));
    }

    #[test]
    fn drag_opposes_velocity() {
        let mut b = Body::at([0.0; 3], 1.0);
        b.vel = [2.0, -4.0, 0.0];
        add_drag(&mut b, 0.5);
        assert_eq!(b.force, [-1.0, 2.0, 0.0]);
    }
}
