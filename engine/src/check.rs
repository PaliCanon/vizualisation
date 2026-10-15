//! Checks on what the page passes in: a wrong length or an index past the last point would
//! otherwise panic, and a panic aborts the whole module (`panic = "abort"`). Also the few
//! numeric conversions between the page's `f32`/`usize` and the GPU's `u32`, each saturating
//! rather than wrapping, so no cast is left to chance.

/// `got` values where `n` items of `per` values each are expected.
pub fn len(what: &str, got: usize, n: usize, per: usize) -> Result<(), String> {
    if got == n * per {
        Ok(())
    } else {
        Err(format!(
            "{what}: {got} values, expected {} ({n} × {per})",
            n * per
        ))
    }
}

/// Segments: an even number of ends, every one a point index, and r, g, b, a per segment.
pub fn segments(ends: &[u32], colors: &[f32], n_points: usize) -> Result<(), String> {
    if !ends.len().is_multiple_of(2) {
        return Err(format!("segments: {} ends, expected pairs", ends.len()));
    }
    len("segment colours", colors.len(), ends.len() / 2, 4)?;
    match ends.iter().find(|&&k| k as usize >= n_points) {
        Some(k) => Err(format!(
            "segments: point {k} does not exist ({n_points} points)"
        )),
        None => Ok(()),
    }
}

/// Links for a layout: pairs of point indices, one finite non-negative rest length per pair,
/// every index a point.
pub fn links(ends: &[u32], lengths: &[f32], n_points: usize) -> Result<(), String> {
    if !ends.len().is_multiple_of(2) {
        return Err(format!("links: {} ends, expected pairs", ends.len()));
    }
    len("rest lengths", lengths.len(), ends.len() / 2, 1)?;
    if let Some(k) = ends.iter().find(|&&k| k as usize >= n_points) {
        return Err(format!(
            "links: point {k} does not exist ({n_points} points)"
        ));
    }
    if lengths.iter().any(|l| !l.is_finite()) {
        return Err("rest lengths must be finite numbers".into());
    }
    if lengths.iter().any(|&l| l < 0.0) {
        return Err("rest lengths must not be negative".into());
    }
    Ok(())
}

/// A count as the GPU's `u32`; saturates (a draw call cannot take more).
pub fn to_u32(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// A size in pixels as `u32`: negative and NaN give 0, too large saturates (what `as` does for
/// floats, named once here).
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "`as` from a float saturates and maps NaN to 0: the clamp wanted for a pixel size"
)]
pub fn px(v: f32) -> u32 {
    v as u32
}

/// Index of the 90th percentile in a sorted slice of `len` (> 0) items.
pub fn p90_index(len: usize) -> usize {
    (len * 9 / 10).min(len.saturating_sub(1))
}

/// A point index as the `i32` the page gets back (-1 means none); saturates.
pub fn point_id(i: usize) -> i32 {
    i32::try_from(i).unwrap_or(i32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lengths() {
        assert!(len("x", 6, 2, 3).is_ok());
        assert!(len("x", 5, 2, 3).is_err());
    }

    #[test]
    fn segments_need_pairs_colours_and_known_points() {
        assert!(segments(&[0, 1], &[0.0; 4], 2).is_ok());
        assert!(segments(&[0], &[], 2).is_err());
        assert!(segments(&[0, 1], &[0.0; 3], 2).is_err());
        assert!(segments(&[0, 2], &[0.0; 4], 2).is_err());
    }

    #[test]
    fn to_u32_saturates_instead_of_wrapping() {
        assert_eq!(to_u32(7), 7);
        assert_eq!(to_u32(u32::MAX as usize), u32::MAX);
        #[cfg(target_pointer_width = "64")]
        assert_eq!(to_u32(usize::MAX), u32::MAX);
    }

    #[test]
    fn px_clamps_negative_and_nan_to_zero_and_saturates() {
        assert_eq!(px(12.7), 12);
        assert_eq!(px(-3.0), 0);
        assert_eq!(px(f32::NAN), 0);
        assert_eq!(px(1e20), u32::MAX);
    }

    #[test]
    fn p90_index_stays_inside_the_slice() {
        assert_eq!(p90_index(1), 0);
        assert_eq!(p90_index(10), 9);
        assert_eq!(p90_index(7286), 6557);
        for n in 1..200 {
            assert!(p90_index(n) < n);
        }
    }

    #[test]
    fn point_id_saturates_for_the_page() {
        assert_eq!(point_id(5), 5);
        assert_eq!(point_id(usize::MAX), i32::MAX);
    }

    #[test]
    fn links_check_rejects_odd_length_and_missing_node() {
        assert!(links(&[0, 1, 1, 2], &[1.0, 2.0], 3).is_ok());
        assert!(links(&[], &[], 0).is_ok());
        assert!(
            links(&[0, 1, 1], &[1.0, 2.0], 3)
                .unwrap_err()
                .contains("pairs")
        );
        assert!(
            links(&[0, 1, 1, 2], &[1.0], 3)
                .unwrap_err()
                .contains("rest lengths")
        );
        assert!(links(&[0, 3], &[1.0], 3).unwrap_err().contains("point 3"));
        assert!(links(&[0, 1], &[-1.0], 3).unwrap_err().contains("negative"));
        assert!(
            links(&[0, 1], &[0.0], 3).is_ok(),
            "a zero rest length is allowed"
        );
        assert!(
            links(&[0, 1], &[f32::NEG_INFINITY], 3)
                .unwrap_err()
                .contains("finite")
        );
        assert!(
            links(&[0, 1], &[f32::NAN], 3)
                .unwrap_err()
                .contains("finite")
        );
        assert!(
            links(&[0, 1], &[f32::INFINITY], 3)
                .unwrap_err()
                .contains("finite")
        );
    }
}
