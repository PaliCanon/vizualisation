//! The 3D force-directed layout of the map, a port of ngraph.forcelayout 3.3: generic (it
//! knows no corpus), deterministic (seeded) and buildable natively and for wasm32. The engine
//! runs it live in the page and the data tools run it offline, so both place the texts with
//! the same physics.
//!
//! **The model**, as ngraph has it in three dimensions: every body repels every other
//! (Coulomb, `gravity` < 0, approximated by a Barnes-Hut octree at `theta`), each link is a
//! spring with its own rest length, a drag slows the bodies, and an integrator moves them
//! (velocity capped to one, so a step never moves a body further than `time_step`). A body's
//! mass is `1 + degree / 3`.
//!
//! **Deterministic**: the only chance in the model is the little jitter that splits two bodies
//! on the same spot; it comes from a seeded generator, so a run is reproducible.
//!
//! ```
//! use map_layout::{Layout, Link, Params};
//!
//! let start = [[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [0.0, 10.0, 0.0]];
//! let links = [Link::new(0, 1, 30.0), Link::new(1, 2, 30.0)];
//! let mut layout = Layout::new(Params::map(), &start, &links).unwrap();
//! for _ in 0..50 {
//!     layout.step();
//! }
//! assert_eq!(layout.positions().len(), 3);
//! ```

mod body;
mod error;
mod forces;
mod integrate;
mod layout;
mod octree;
mod params;
mod prng;

pub use error::LayoutError;
pub use layout::{Layout, scatter};
pub use params::{Link, Params};
