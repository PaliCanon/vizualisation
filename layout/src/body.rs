//! A body: a text on the map.

/// A point mass with its velocity and the force on it this step.
#[derive(Clone, Copy, Debug)]
pub struct Body {
    /// Position.
    pub pos: [f64; 3],
    /// Velocity.
    pub vel: [f64; 3],
    /// The force on it, reset at the start of each step.
    pub force: [f64; 3],
    /// Mass: `1 + degree / 3`.
    pub mass: f64,
}

impl Body {
    /// A body at rest at `pos`.
    pub fn at(pos: [f64; 3], mass: f64) -> Self {
        Self {
            pos,
            vel: [0.0; 3],
            force: [0.0; 3],
            mass,
        }
    }
}
