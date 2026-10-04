//! Checks on what the page passes in: a wrong length or an index past the last point would
//! otherwise panic, and a panic aborts the whole module (`panic = "abort"`).

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
}
