//! The force layout, run live in the page.
//!
//! The page gives the starting positions and the links (pairs of point indices with a rest
//! length each), asks for a few steps per animation frame, and reads the positions back to
//! hand them to the view. The physics is `map_layout`'s, the port of ngraph.forcelayout that the
//! data tools use too; this module checks what the page passes in and converts to the page's
//! `f32`.

use crate::check;
use map_layout::{Layout, Link, Params};

/// The layout without the JavaScript shell: errors are plain strings, so it runs in tests.
#[derive(Debug)]
pub struct Core {
    layout: Layout,
    steps: u32,
}

impl Core {
    /// Start from `positions` (x, y, z per point), with the links `ends` (two point indices per
    /// link) and their rest `lengths`.
    ///
    /// # Errors
    /// When `positions` is not a whole number of (x, y, z) triples, the lengths are not one per
    /// link, a link names a point that does not exist, or a number is not finite.
    pub fn new(positions: &[f32], ends: &[u32], lengths: &[f32]) -> Result<Core, String> {
        let n = positions.len() / 3;
        check::len("positions", positions.len(), n, 3)?;
        check::links(ends, lengths, n)?;
        if positions.iter().any(|x| !x.is_finite()) {
            return Err("positions must be finite numbers".into());
        }
        let start: Vec<[f64; 3]> = positions
            .as_chunks::<3>()
            .0
            .iter()
            .map(|p| p.map(f64::from))
            .collect();
        let links: Vec<Link> = ends
            .as_chunks::<2>()
            .0
            .iter()
            .zip(lengths)
            .map(|(&[a, b], &length)| Link::new(a as usize, b as usize, f64::from(length)))
            .collect();
        let layout = Layout::new(Params::map(), &start, &links).map_err(|e| e.to_string())?;
        Ok(Core { layout, steps: 0 })
    }

    /// Run `count` steps; returns the movement of the last step run so far (so `step(0)` leaves
    /// it, and the positions, as they were; 0 before any step).
    pub fn step(&mut self, count: u32) -> f64 {
        for _ in 0..count {
            self.layout.step();
        }
        self.steps = self.steps.saturating_add(count);
        self.layout.movement()
    }

    /// The positions, x, y, z per point.
    #[allow(clippy::cast_possible_truncation, reason = "the page draws in f32")]
    pub fn positions(&self) -> Vec<f32> {
        self.layout
            .positions()
            .iter()
            .flatten()
            .map(|x| *x as f32)
            .collect()
    }

    /// Steps run so far.
    pub fn steps(&self) -> u32 {
        self.steps
    }
}

/// The JavaScript side, in the wasm build only (like the view and the GPU code); `Core` above
/// stays native and tested.
#[cfg(target_arch = "wasm32")]
mod shell {
    use super::Core;
    use wasm_bindgen::prelude::*;

    /// A live layout for the page.
    #[wasm_bindgen]
    pub struct LiveLayout {
        core: Core,
    }

    #[wasm_bindgen]
    impl LiveLayout {
        /// Start a layout from `positions` (x, y, z per point), with the links `ends` (two point
        /// indices per link) and their rest `lengths`, and the map's parameters.
        ///
        /// # Errors
        /// When `positions` is not a whole number of (x, y, z) triples, the lengths are not one per
        /// link, a link names a point that does not exist, or a number is not finite.
        #[wasm_bindgen(constructor)]
        pub fn new(
            positions: &[f32],
            ends: &[u32],
            lengths: &[f32],
        ) -> Result<LiveLayout, JsError> {
            Core::new(positions, ends, lengths)
                .map(|core| LiveLayout { core })
                .map_err(|e| JsError::new(&e))
        }

        /// Run `count` steps; returns the movement of the last one, which falls as the layout
        /// settles.
        pub fn step(&mut self, count: u32) -> f64 {
            self.core.step(count)
        }

        /// The positions, x, y, z per point, in the order given.
        pub fn positions(&self) -> Vec<f32> {
            self.core.positions()
        }

        /// Steps run so far.
        pub fn steps(&self) -> u32 {
            self.core.steps()
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub use shell::LiveLayout;

#[cfg(test)]
mod tests {
    use super::*;

    const START: [f32; 9] = [0.0, 0.0, 0.0, 10.0, 0.0, 0.0, 0.0, 10.0, 0.0];

    #[test]
    fn steps_move_the_positions_and_are_counted() {
        let mut core = Core::new(&START, &[0, 1, 1, 2], &[30.0, 30.0]).unwrap();
        assert_eq!(core.positions(), START);
        let movement = core.step(5);
        assert!(movement > 0.0);
        assert_eq!(core.steps(), 5);
        assert_ne!(core.positions(), START);
        core.step(3);
        assert_eq!(core.steps(), 8);
    }

    #[test]
    fn zero_steps_change_nothing() {
        let mut core = Core::new(&START, &[0, 1], &[30.0]).unwrap();
        assert_eq!(core.step(0), 0.0);
        assert_eq!(core.positions(), START);
        assert_eq!(core.steps(), 0);
    }

    #[test]
    fn positions_keep_the_order_and_three_values_per_point() {
        let mut core = Core::new(&START, &[0, 1], &[30.0]).unwrap();
        core.step(10);
        let p = core.positions();
        assert_eq!(p.len(), 9);
        assert!(p.iter().all(|x| x.is_finite()));
    }

    #[test]
    fn it_is_the_layout_crates_physics() {
        // The same run through map_layout directly, converted to f32.
        let mut core = Core::new(&START, &[0, 1, 1, 2], &[30.0, 30.0]).unwrap();
        core.step(20);
        let start: Vec<[f64; 3]> = START
            .chunks(3)
            .map(|c| [f64::from(c[0]), f64::from(c[1]), f64::from(c[2])])
            .collect();
        let links = [Link::new(0, 1, 30.0), Link::new(1, 2, 30.0)];
        let mut direct = Layout::new(Params::map(), &start, &links).unwrap();
        for _ in 0..20 {
            direct.step();
        }
        #[allow(
            clippy::cast_possible_truncation,
            reason = "the page's positions are f32"
        )]
        let want: Vec<f32> = direct
            .positions()
            .iter()
            .flatten()
            .map(|x| *x as f32)
            .collect();
        assert_eq!(core.positions(), want);
    }

    #[test]
    fn bad_input_is_an_error_not_a_panic() {
        assert!(
            Core::new(&START[..8], &[], &[])
                .unwrap_err()
                .contains("positions")
        );
        assert!(
            Core::new(&START, &[0, 5], &[30.0])
                .unwrap_err()
                .contains("point 5")
        );
        assert!(
            Core::new(&START, &[0, 1], &[])
                .unwrap_err()
                .contains("rest lengths")
        );
        assert!(
            Core::new(&[f32::NAN, 0.0, 0.0], &[], &[])
                .unwrap_err()
                .contains("finite")
        );
    }

    #[test]
    fn no_points_no_error() {
        let mut core = Core::new(&[], &[], &[]).unwrap();
        assert_eq!(core.step(3), 0.0);
        assert_eq!(core.positions().len(), 0);
    }

    #[test]
    fn a_position_or_a_length_that_is_not_usable_is_an_error() {
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let pos = [0.0, bad, 0.0, 1.0, 1.0, 1.0];
            assert!(Core::new(&pos, &[], &[]).unwrap_err().contains("finite"));
            assert!(
                Core::new(&START, &[0, 1], &[bad])
                    .unwrap_err()
                    .contains("finite")
            );
        }
        assert!(
            Core::new(&START, &[0, 1], &[-0.5])
                .unwrap_err()
                .contains("negative")
        );
    }

    #[test]
    fn step_zero_twice_changes_nothing() {
        let mut core = Core::new(&START, &[0, 1, 1, 2], &[30.0, 30.0]).unwrap();
        let moved = core.step(4);
        let positions = core.positions();
        assert!(moved > 0.0);
        for _ in 0..2 {
            assert_eq!(core.step(0), moved, "the movement is the last step's");
            assert_eq!(core.positions(), positions);
            assert_eq!(core.steps(), 4);
        }
    }

    #[test]
    fn the_wasm_class_is_gated_to_wasm32_like_the_view() {
        // The JavaScript shell exists only in the wasm build; `Core` stays native and tested.
        let source = include_str!("live_layout.rs");
        let production = source.split("#[cfg(test)]").next().unwrap();
        let gate = "#[cfg(target_arch = \"wasm32\")]\nmod shell";
        let (native, shell) = production
            .split_once(gate)
            .expect("the shell module is gated");
        assert!(
            !native.contains("wasm_bindgen"),
            "wasm_bindgen outside the gated module"
        );
        assert!(shell.contains("#[wasm_bindgen]") && shell.contains("pub struct LiveLayout"));
    }
}
