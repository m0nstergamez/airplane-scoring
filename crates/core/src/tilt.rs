//! Tilt steering: the phone is the plane.
//!
//! Held in landscape, rotating the phone like a steering wheel steers:
//! right side (the plane's nose) up = climb, down = dive.
//!
//! Input is the gravity vector in the device's x/y plane (from the browser's
//! `devicemotion` `accelerationIncludingGravity`). We only use its *angle*,
//! relative to a neutral angle captured at the start. That makes it
//! independent of:
//! - which way the phone was turned into landscape (180° apart),
//! - iOS vs Android sign conventions (iOS negates the vector: also 180°),
//! - how far the phone is tipped toward the player.

/// Rotation (degrees) from neutral that is ignored, so a steady hand flies level.
pub const TILT_DEAD_ZONE_DEG: f32 = 2.0;
/// Rotation (degrees) from neutral that gives full steer.
pub const TILT_FULL_DEG: f32 = 25.0;
/// Below this much gravity in the screen plane (m/s²) the phone is lying
/// nearly flat and the angle is noise, so tilt is not used.
pub const TILT_MIN_GRAVITY: f32 = 3.0;
/// Per-sample low-pass factor (0..1). Lower = smoother but laggier.
pub const TILT_SMOOTHING: f32 = 0.25;

/// Tilt state: the neutral angle and the smoothed current angle (radians).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Tilt {
    neutral: Option<f32>,
    angle: f32,
}

impl Tilt {
    /// Forget the neutral angle; the next reliable sample becomes neutral.
    /// Call on start and whenever the screen orientation changes.
    pub fn recalibrate(&mut self) {
        self.neutral = None;
    }

    /// Feed one gravity sample (device x/y, any sign convention).
    /// Returns the steer in [-1, 1], or `None` when the sample is unusable
    /// (phone flat, or not a number).
    pub fn update(&mut self, gx: f32, gy: f32) -> Option<f32> {
        if !(gx.is_finite() && gy.is_finite()) || gx.hypot(gy) < TILT_MIN_GRAVITY {
            return None;
        }
        let raw = gy.atan2(gx);
        let Some(neutral) = self.neutral else {
            self.neutral = Some(raw);
            self.angle = raw;
            return Some(0.0);
        };
        // Low-pass along the shortest way round, so the ±180° seam is smooth.
        self.angle = wrap(self.angle + wrap(raw - self.angle) * TILT_SMOOTHING);
        let off = wrap(self.angle - neutral).to_degrees();
        let strength = ((off.abs() - TILT_DEAD_ZONE_DEG) / (TILT_FULL_DEG - TILT_DEAD_ZONE_DEG))
            .clamp(0.0, 1.0);
        Some(strength.copysign(off))
    }
}

/// Wrap an angle in radians to (-π, π].
fn wrap(a: f32) -> f32 {
    use std::f32::consts::{PI, TAU};
    let a = (a + PI).rem_euclid(TAU) - PI;
    if a <= -PI {
        a + TAU
    } else {
        a
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const G: f32 = 9.81;

    /// Gravity as the device sees it when the phone is rotated `deg` degrees
    /// clockwise (seen from the front) from a starting pose where gravity
    /// points along device angle `base_deg`.
    fn gravity(base_deg: f32, deg: f32) -> (f32, f32) {
        // Turning the phone clockwise turns world vectors counter-clockwise
        // in the device frame.
        let a = (base_deg + deg).to_radians();
        (G * a.cos(), G * a.sin())
    }

    /// Feed the same sample until the smoothing has settled.
    fn settle(t: &mut Tilt, (gx, gy): (f32, f32)) -> Option<f32> {
        let mut s = None;
        for _ in 0..200 {
            s = t.update(gx, gy);
        }
        s
    }

    fn calibrated(base_deg: f32) -> Tilt {
        let mut t = Tilt::default();
        let (gx, gy) = gravity(base_deg, 0.0);
        t.update(gx, gy);
        t
    }

    #[test]
    fn first_sample_becomes_neutral_and_flies_level() {
        let mut t = Tilt::default();
        let (gx, gy) = gravity(180.0, 0.0);
        assert_eq!(t.update(gx, gy), Some(0.0));
    }

    #[test]
    fn rotating_clockwise_dives() {
        let mut t = calibrated(180.0);
        assert_eq!(settle(&mut t, gravity(180.0, 30.0)), Some(1.0));
    }

    #[test]
    fn rotating_counter_clockwise_climbs() {
        let mut t = calibrated(180.0);
        assert_eq!(settle(&mut t, gravity(180.0, -30.0)), Some(-1.0));
    }

    #[test]
    fn small_wobble_inside_dead_zone_flies_level() {
        let mut t = calibrated(180.0);
        assert_eq!(
            settle(&mut t, gravity(180.0, TILT_DEAD_ZONE_DEG * 0.75)),
            Some(0.0)
        );
    }

    #[test]
    fn steer_grows_linearly_between_dead_zone_and_full() {
        let mut t = calibrated(180.0);
        let halfway = TILT_DEAD_ZONE_DEG + (TILT_FULL_DEG - TILT_DEAD_ZONE_DEG) / 2.0;
        let s = settle(&mut t, gravity(180.0, halfway)).unwrap();
        assert!((s - 0.5).abs() < 1e-3, "steer = {s}");
    }

    #[test]
    fn either_landscape_direction_steers_the_same_way() {
        // Landscape left vs right: gravity starts 180° apart in the device frame.
        for base in [0.0, 180.0] {
            let mut t = calibrated(base);
            assert_eq!(
                settle(&mut t, gravity(base, 30.0)),
                Some(1.0),
                "base {base}"
            );
        }
    }

    #[test]
    fn negated_vectors_like_ios_steer_the_same_way() {
        let mut t = Tilt::default();
        let (gx, gy) = gravity(180.0, 0.0);
        t.update(-gx, -gy);
        let (gx, gy) = gravity(180.0, 30.0);
        assert_eq!(settle_raw(&mut t, -gx, -gy), Some(1.0));
    }

    fn settle_raw(t: &mut Tilt, gx: f32, gy: f32) -> Option<f32> {
        settle(t, (gx, gy))
    }

    #[test]
    fn crossing_the_plus_minus_180_degree_seam_is_smooth() {
        // Neutral right next to the atan2 seam; a small clockwise turn must
        // read as a small dive, not a huge climb.
        let mut t = calibrated(177.0);
        let s = settle(&mut t, gravity(177.0, 8.0)).unwrap();
        assert!(s > 0.0 && s < 0.5, "steer = {s}");
    }

    #[test]
    fn smoothing_does_not_jump_to_full_on_one_sample() {
        let mut t = calibrated(180.0);
        let (gx, gy) = gravity(180.0, 30.0);
        let s = t.update(gx, gy).unwrap();
        assert!(s > 0.0 && s < 1.0, "steer = {s}");
    }

    #[test]
    fn phone_lying_flat_is_unusable() {
        let mut t = calibrated(180.0);
        assert_eq!(t.update(0.5, -0.5), None);
    }

    #[test]
    fn non_finite_samples_are_unusable() {
        let mut t = calibrated(180.0);
        assert_eq!(t.update(f32::NAN, 1.0), None);
        assert_eq!(t.update(1.0, f32::INFINITY), None);
    }

    #[test]
    fn recalibrate_makes_the_current_pose_neutral() {
        let mut t = calibrated(180.0);
        settle(&mut t, gravity(180.0, 30.0));
        t.recalibrate();
        let (gx, gy) = gravity(180.0, 30.0);
        assert_eq!(t.update(gx, gy), Some(0.0));
    }
}
