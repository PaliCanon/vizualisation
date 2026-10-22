//! The settings of a layout and the links it springs.

/// The physics, as ngraph.forcelayout names them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Params {
    /// Rest length of a link that has none of its own.
    pub spring_length: f64,
    /// Hooke's coefficient of every spring.
    pub spring_coefficient: f64,
    /// Coulomb's coefficient; negative, bodies repel.
    pub gravity: f64,
    /// Barnes-Hut opening angle: a cell whose width over distance is below it counts as one
    /// body. 0 is exact (and slow).
    pub theta: f64,
    /// Drag: the force against the velocity is `drag * v`.
    pub drag: f64,
    /// Time step of the integrator.
    pub time_step: f64,
    /// After each step positions are multiplied by this: 1 leaves them, less than 1 draws the
    /// isolated bodies gently towards the origin.
    pub centring: f64,
    /// Seed of the generator that splits bodies sitting on the same spot.
    pub seed: u64,
}

impl Default for Params {
    /// ngraph.forcelayout's own defaults.
    fn default() -> Self {
        Self {
            spring_length: 10.0,
            spring_coefficient: 0.8,
            gravity: -12.0,
            theta: 0.8,
            drag: 0.9,
            time_step: 0.5,
            centring: 1.0,
            seed: 42,
        }
    }
}

impl Params {
    /// The values the map's layouts all use: the engine's live layout
    /// (`engine/src/live_layout.rs`) and the offline data tools (`build-similarity.mjs`
    /// and `build-graph.mjs`, removed in task 008, in git history until task 007 ports
    /// them), with the pull to the origin the live layout and `build-similarity` apply.
    #[must_use]
    pub fn map() -> Self {
        Self {
            spring_length: 30.0,
            spring_coefficient: 0.0008,
            gravity: -2.5,
            theta: 0.9,
            drag: 0.04,
            time_step: 20.0,
            centring: 0.99,
            seed: 42,
        }
    }
}

/// A link between two bodies, a spring.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Link {
    /// Index of the first body.
    pub a: usize,
    /// Index of the second body.
    pub b: usize,
    /// Rest length; `None` takes [`Params::spring_length`].
    pub length: Option<f64>,
}

impl Link {
    /// A link with its own rest length.
    #[must_use]
    pub fn new(a: usize, b: usize, length: f64) -> Self {
        Self {
            a,
            b,
            length: Some(length),
        }
    }

    /// A link at the default rest length.
    #[must_use]
    pub fn plain(a: usize, b: usize) -> Self {
        Self { a, b, length: None }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_values_are_the_ones_the_scripts_pass() {
        let p = Params::map();
        assert_eq!(
            (
                p.spring_length,
                p.spring_coefficient,
                p.gravity,
                p.theta,
                p.drag,
                p.time_step
            ),
            (30.0, 0.0008, -2.5, 0.9, 0.04, 20.0)
        );
        assert_eq!(p.centring, 0.99);
    }

    #[test]
    fn defaults_are_ngraphs() {
        let p = Params::default();
        assert_eq!(
            (p.spring_length, p.gravity, p.theta, p.drag, p.time_step),
            (10.0, -12.0, 0.8, 0.9, 0.5)
        );
        assert_eq!(p.centring, 1.0);
    }

    #[test]
    fn links_with_and_without_a_length() {
        assert_eq!(Link::new(1, 2, 5.0).length, Some(5.0));
        assert_eq!(Link::plain(1, 2).length, None);
    }
}
