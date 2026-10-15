//! Orbit camera: rotates around a target, zooms with the wheel, keeps turning slowly until
//! the user touches it, eases towards a new target ("fly to"), travels with the arrow keys.
//! Mirrors three.js's `OrbitControls` (damping 0.05, auto-rotate 0.25) so the map feels the
//! same. Kept near the scene: zoom between a minimum and a maximum distance, target within
//! twice the scene's radius, so the points never all leave the view.

use glam::{Mat4, Vec3};
use std::f32::consts::{PI, TAU};

const DAMPING: f32 = 0.05;
const AUTO_ROTATE: f32 = 0.25; // three.js units: one turn in 60 s / speed
const FLIGHT_MS: f64 = 1100.0;
const FOV_Y: f32 = 55.0;
const KEY_SPEED: f32 = 0.8; // distances to the target per second, while an arrow key is held
const MIN_ZOOM: f32 = 0.02; // closest eye distance, in scene radii
const MAX_ZOOM: f32 = 5.0; // farthest eye distance, in scene radii
const ROAM: f32 = 2.0; // the target stays within this many scene radii of the centre

/// An eased move of the target and the eye.
struct Flight {
    from: Vec3,
    to: Vec3,
    eye_from: Vec3,
    eye_to: Vec3,
    t0: f64,
}

/// Where the camera is and how it moves.
pub struct Orbit {
    /// The point the camera looks at.
    pub target: Vec3,
    radius: f32,
    theta: f32, // around the vertical axis
    phi: f32,   // from the vertical axis
    d_theta: f32,
    d_phi: f32,
    zoom: f32, // pending radius factor
    /// Turn slowly around the target until the user takes over.
    pub auto_rotate: bool,
    flight: Option<Flight>,
    last: Option<f64>,
    keys: Vec3,        // held keys: x right - left, y up - down, z forward - back
    center: Vec3,      // the scene's centre
    scene_radius: f32, // and radius: the limits are relative to it
}

fn ease(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

impl Orbit {
    /// A camera at `eye` looking at `target`, for a scene of this `scene_radius` around `target`.
    pub fn new(target: Vec3, eye: Vec3, scene_radius: f32) -> Self {
        let mut o = Orbit {
            target,
            radius: 1.0,
            theta: 0.0,
            phi: 0.0,
            d_theta: 0.0,
            d_phi: 0.0,
            zoom: 1.0,
            auto_rotate: true,
            flight: None,
            last: None,
            keys: Vec3::ZERO,
            center: target,
            scene_radius: scene_radius.max(1e-3),
        };
        o.set_eye(eye);
        o
    }

    fn set_eye(&mut self, eye: Vec3) {
        let v = eye - self.target;
        self.radius = v.length().max(1e-3);
        self.theta = v.x.atan2(v.z);
        self.phi = (v.y / self.radius).clamp(-1.0, 1.0).acos();
    }

    /// A new scene (centre, radius): the limits follow it.
    pub fn set_scene(&mut self, center: Vec3, scene_radius: f32) {
        self.center = center;
        self.scene_radius = scene_radius.max(1e-3);
    }

    /// The camera position.
    pub fn eye(&self) -> Vec3 {
        let s = self.phi.sin();
        self.target
            + self.radius * Vec3::new(s * self.theta.sin(), self.phi.cos(), s * self.theta.cos())
    }

    /// Distance from the eye to the target.
    #[cfg(test)]
    pub fn distance(&self) -> f32 {
        self.radius
    }

    /// A drag of (dx, dy) CSS pixels on a canvas `height` pixels tall.
    pub fn rotate(&mut self, dx: f32, dy: f32, height: f32) {
        self.d_theta -= TAU * dx / height;
        self.d_phi -= TAU * dy / height;
        self.stop();
    }

    /// A wheel step: positive zooms out.
    pub fn wheel(&mut self, delta_y: f32) {
        let k = 0.95f32.powf(delta_y.abs() * 0.01);
        self.zoom *= if delta_y > 0.0 { 1.0 / k } else { k };
        self.stop();
    }

    /// Keys held, each -1, 0 or 1: `x` right - left, `y` up - down, `z` forward - back. The eye
    /// and the target move together: the camera travels, it does not turn.
    pub fn set_keys(&mut self, x: f32, y: f32, z: f32) {
        self.keys = Vec3::new(x, y, z);
        if self.keys != Vec3::ZERO {
            self.stop();
        }
    }

    /// Move eye and target by the held keys for `dt` seconds.
    fn travel(&mut self, dt: f32) {
        let forward = (self.target - self.eye()).normalize_or(Vec3::NEG_Z);
        let right = forward.cross(Vec3::Y).normalize_or(Vec3::X);
        let dir = right * self.keys.x + Vec3::Y * self.keys.y + forward * self.keys.z;
        let target = self.target + dir * KEY_SPEED * self.radius * dt;
        let roam = self.scene_radius * ROAM;
        let off = target - self.center;
        self.target = self.center + off.clamp_length_max(roam);
    }

    /// Keep the eye between the closest and the farthest distance; a zoom stops at the limit.
    fn clamp_radius(&mut self) {
        let r = self
            .radius
            .clamp(self.scene_radius * MIN_ZOOM, self.scene_radius * MAX_ZOOM);
        if r.total_cmp(&self.radius).is_ne() {
            self.radius = r;
            self.zoom = 1.0; // the rest of this zoom goes nowhere
        }
    }

    fn stop(&mut self) {
        self.auto_rotate = false;
        self.flight = None;
    }

    /// Ease the target to `to`, the eye `distance` away from it along the current direction.
    pub fn fly_to(&mut self, to: Vec3, distance: f32, now: f64) {
        let dir = (self.eye() - self.target).normalize_or(Vec3::Z);
        let (from, eye_from) = (self.target, self.eye());
        self.flight = Some(Flight {
            from,
            to,
            eye_from,
            eye_to: to + dir * distance,
            t0: now,
        });
        self.auto_rotate = false;
    }

    /// Advance to time `now` (ms); returns true while something moves.
    #[allow(
        clippy::cast_possible_truncation,
        reason = "a frame delta in seconds and a 0..1 flight fraction fit f32"
    )]
    pub fn update(&mut self, now: f64) -> bool {
        let dt = self
            .last
            .map_or(0.0, |l| ((now - l) / 1000.0) as f32)
            .min(0.1);
        self.last = Some(now);
        if let Some(f) = &self.flight {
            let t = ((now - f.t0) / FLIGHT_MS).min(1.0) as f32;
            let k = ease(t);
            let (target, eye) = (f.from.lerp(f.to, k), f.eye_from.lerp(f.eye_to, k));
            if t >= 1.0 {
                self.flight = None;
            }
            self.target = target;
            self.set_eye(eye);
            return true;
        }
        if self.auto_rotate {
            self.d_theta += TAU / 60.0 * AUTO_ROTATE * dt;
        }
        self.travel(dt);
        self.theta += self.d_theta * DAMPING * 4.0;
        self.phi = (self.phi + self.d_phi * DAMPING * 4.0).clamp(1e-3, PI - 1e-3);
        self.radius *= 1.0 + (self.zoom - 1.0) * DAMPING * 4.0;
        self.clamp_radius();
        self.d_theta *= 1.0 - DAMPING * 4.0;
        self.d_phi *= 1.0 - DAMPING * 4.0;
        self.zoom = 1.0 + (self.zoom - 1.0) * (1.0 - DAMPING * 4.0);
        self.auto_rotate
            || self.keys != Vec3::ZERO
            || self.d_theta.abs() + self.d_phi.abs() > 1e-5
            || (self.zoom - 1.0).abs() > 1e-4
    }

    /// View matrix.
    pub fn view(&self) -> Mat4 {
        glam::camera::rh::view::look_at_mat4(self.eye(), self.target, Vec3::Y)
    }

    /// Projection for a viewport of this aspect ratio; the far plane reaches past the farthest
    /// point the eye can have in front of it (target within ROAM radii, points within one).
    pub fn projection(&self, aspect: f32) -> Mat4 {
        let far = self.radius + self.scene_radius * (ROAM + 2.0);
        let near = (self.scene_radius * MIN_ZOOM * 0.05).max(far * 1e-6);
        glam::camera::rh::proj::directx::perspective(FOV_Y.to_radians(), aspect, near, far)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eye_round_trips() {
        let o = Orbit::new(Vec3::new(1.0, 2.0, 3.0), Vec3::new(4.0, 6.0, 15.0), 100.0);
        assert!((o.eye() - Vec3::new(4.0, 6.0, 15.0)).length() < 1e-3);
    }

    #[test]
    fn forward_moves_eye_and_target_together() {
        let mut o = Orbit::new(Vec3::ZERO, Vec3::new(0.0, 0.0, 100.0), 100.0);
        o.update(0.0);
        o.set_keys(0.0, 0.0, 1.0);
        o.update(100.0);
        assert!(o.target.z < -1.0, "target moved forward (towards -z)");
        assert!((o.distance() - 100.0).abs() < 1e-3, "no zoom, no turn");
    }

    #[test]
    fn arrows_stay_near_the_scene() {
        let mut o = Orbit::new(Vec3::ZERO, Vec3::new(0.0, 0.0, 100.0), 100.0);
        o.update(0.0);
        o.set_keys(1.0, 0.0, 0.0);
        for k in 1..=200 {
            o.update(f64::from(k) * 100.0);
        }
        assert!(o.target.length() <= 100.0 * ROAM + 1e-3);
    }

    #[test]
    fn zoom_out_stops_at_the_maximum() {
        let mut o = Orbit::new(Vec3::ZERO, Vec3::new(0.0, 0.0, 100.0), 100.0);
        for k in 0..200 {
            o.wheel(1000.0);
            o.update(f64::from(k) * 16.0);
        }
        assert!((o.distance() - 100.0 * MAX_ZOOM).abs() < 1e-3);
        assert!(!o.update(4000.0), "nothing left moving at the limit");
    }

    #[test]
    fn zoom_in_works_again_from_the_maximum() {
        let mut o = Orbit::new(Vec3::ZERO, Vec3::new(0.0, 0.0, 600.0), 100.0);
        o.update(0.0);
        o.update(16.0);
        assert!((o.distance() - 100.0 * MAX_ZOOM).abs() < 1e-3);
        o.wheel(-500.0);
        o.update(32.0);
        assert!(o.distance() < 100.0 * MAX_ZOOM);
    }

    #[test]
    fn flight_lands_on_target() {
        let mut o = Orbit::new(Vec3::ZERO, Vec3::new(0.0, 0.0, 100.0), 100.0);
        o.fly_to(Vec3::new(10.0, 0.0, 0.0), 50.0, 0.0);
        o.update(FLIGHT_MS + 1.0);
        assert!((o.target - Vec3::new(10.0, 0.0, 0.0)).length() < 1e-3);
        assert!((o.distance() - 50.0).abs() < 1e-2);
    }
}
