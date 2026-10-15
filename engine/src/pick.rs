//! Which point is under the pointer: project every shown point to the screen and keep the
//! nearest within its drawn radius (at least a few pixels, so small points stay clickable).

use glam::{Mat4, Vec2, Vec3};

/// Minimum pick radius, CSS pixels.
const MIN_RADIUS: f32 = 6.0;

/// The point under `at` (CSS pixels from the canvas' top left), or None.
/// `radius_px(i, depth)` gives point i's drawn radius in CSS pixels; 0 means hidden.
pub fn pick(
    at: Vec2,
    size: Vec2,
    view_proj: Mat4,
    positions: &[Vec3],
    radius_px: impl Fn(usize, f32) -> f32,
) -> Option<usize> {
    let mut best: Option<(usize, f32)> = None;
    for (i, p) in positions.iter().enumerate() {
        let clip = view_proj * p.extend(1.0);
        if clip.w <= 0.0 {
            continue; // behind the camera
        }
        let r = radius_px(i, clip.w);
        if r <= 0.0 {
            continue;
        }
        let ndc = clip.truncate() / clip.w;
        let screen = Vec2::new(
            f32::midpoint(ndc.x, 1.0) * size.x,
            (1.0 - ndc.y) * 0.5 * size.y,
        );
        let d = screen.distance(at);
        if d <= r.max(MIN_RADIUS) && best.is_none_or(|(_, bd)| d < bd) {
            best = Some((i, d));
        }
    }
    best.map(|(i, _)| i)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cam() -> Mat4 {
        glam::camera::rh::proj::directx::perspective(1.0, 1.0, 0.1, 100.0)
            * glam::camera::rh::view::look_at_mat4(Vec3::new(0.0, 0.0, 10.0), Vec3::ZERO, Vec3::Y)
    }

    #[test]
    fn picks_the_point_at_the_centre() {
        let pts = [Vec3::new(3.0, 0.0, 0.0), Vec3::ZERO];
        assert_eq!(
            pick(
                Vec2::splat(50.0),
                Vec2::splat(100.0),
                cam(),
                &pts,
                |_, _| 4.0
            ),
            Some(1)
        );
    }

    #[test]
    fn skips_hidden_points() {
        let pts = [Vec3::ZERO];
        assert_eq!(
            pick(
                Vec2::splat(50.0),
                Vec2::splat(100.0),
                cam(),
                &pts,
                |_, _| 0.0
            ),
            None
        );
    }
}
