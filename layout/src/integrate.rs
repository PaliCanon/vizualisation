//! The integrator: forces to velocities to positions.

use crate::body::Body;

/// Advance every body by one `time_step`; returns the movement: the squared per-axis totals of
/// the displacements' absolute values, summed over the axes and divided by the number of
/// bodies (ngraph's `totalMovement`).
///
/// A body accelerates by `time_step / mass * force`; its speed is then capped at one, so a
/// step moves a body at most `time_step`; a slower body keeps its velocity (it fades with the
/// drag, not here).
pub fn integrate(bodies: &mut [Body], time_step: f64) -> f64 {
    if bodies.is_empty() {
        return 0.0;
    }
    let mut total = [0.0; 3];
    for b in bodies.iter_mut() {
        let coeff = time_step / b.mass;
        for k in 0..3 {
            b.vel[k] += coeff * b.force[k];
        }
        let speed = b.vel.iter().map(|v| v * v).sum::<f64>().sqrt();
        if speed > 1.0 {
            for v in &mut b.vel {
                *v /= speed;
            }
        }
        for ((pos, vel), sum) in b.pos.iter_mut().zip(b.vel).zip(&mut total) {
            let step = time_step * vel;
            *pos += step;
            *sum += step.abs();
        }
    }
    #[allow(clippy::cast_precision_loss, reason = "a body count is far below 2^52")]
    let n = bodies.len() as f64;
    total.iter().map(|t| t * t).sum::<f64>() / n
}

#[cfg(test)]
mod tests {
    use super::*;

    fn norm(v: [f64; 3]) -> f64 {
        v.iter().map(|x| x * x).sum::<f64>().sqrt()
    }

    #[test]
    fn velocity_is_capped_at_one() {
        let mut b = [Body::at([0.0; 3], 1.0)];
        b[0].force = [300.0, 400.0, 0.0];
        integrate(&mut b, 2.0);
        assert!((norm(b[0].vel) - 1.0).abs() < 1e-12, "{:?}", b[0].vel);
        // moved time_step along the direction (0.6, 0.8, 0)
        assert!((b[0].pos[0] - 1.2).abs() < 1e-12 && (b[0].pos[1] - 1.6).abs() < 1e-12);
    }

    #[test]
    fn a_slow_body_keeps_its_velocity() {
        let mut b = [Body::at([0.0; 3], 1.0)];
        b[0].force = [0.1, 0.0, 0.0];
        integrate(&mut b, 2.0);
        assert!((b[0].vel[0] - 0.2).abs() < 1e-12);
        assert!((b[0].pos[0] - 0.4).abs() < 1e-12);
    }

    #[test]
    fn a_heavier_body_accelerates_less() {
        let mut b = [Body::at([0.0; 3], 1.0), Body::at([0.0; 3], 4.0)];
        b[0].force = [0.01, 0.0, 0.0];
        b[1].force = [0.01, 0.0, 0.0];
        integrate(&mut b, 1.0);
        assert!((b[0].vel[0] - 0.01).abs() < 1e-12 && (b[1].vel[0] - 0.0025).abs() < 1e-12);
    }

    #[test]
    fn velocity_carries_over_between_steps() {
        let mut b = [Body::at([0.0; 3], 1.0)];
        b[0].force = [0.1, 0.0, 0.0];
        integrate(&mut b, 1.0);
        b[0].force = [0.0; 3];
        integrate(&mut b, 1.0);
        assert!((b[0].pos[0] - 0.2).abs() < 1e-12);
    }

    #[test]
    fn movement_is_the_mean_squared_axis_displacement() {
        let mut b = [Body::at([0.0; 3], 1.0), Body::at([0.0; 3], 1.0)];
        b[0].force = [0.3, 0.0, 0.0];
        b[1].force = [0.0, -0.4, 0.0];
        // displacements 0.3 and 0.4: per-axis totals (0.3, 0.4, 0) -> 0.09 + 0.16, over 2 bodies
        let m = integrate(&mut b, 1.0);
        assert!((m - 0.125).abs() < 1e-12, "{m}");
    }

    #[test]
    fn no_bodies_no_movement() {
        assert_eq!(integrate(&mut [], 1.0), 0.0);
    }
}
