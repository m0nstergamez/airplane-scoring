//! Player input, reduced to one number: `steer` in [-1, 1].
//! -1 = climb (up the screen), +1 = dive (down), 0 = level.
//! Every input device (keys, touch, later tilt) maps to this in pure,
//! tested functions; the game crate only reads raw device state.

/// How far (px) from the finger the plane starts easing off. Inside this
/// distance steer scales down linearly, so the plane settles on the finger
/// instead of jittering past it.
pub const TOUCH_EASE_DISTANCE: f32 = 48.0;

/// One tick's worth of player intent. Always finite and in [-1, 1].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Input {
    steer: f32,
}

impl Input {
    /// No steering: fly level.
    pub const NONE: Input = Input { steer: 0.0 };

    /// Clamps to [-1, 1]; NaN or infinite (e.g. a sensor glitch) means level.
    pub fn new(steer: f32) -> Self {
        if steer.is_finite() {
            Self {
                steer: steer.clamp(-1.0, 1.0),
            }
        } else {
            Self::NONE
        }
    }

    pub fn steer(&self) -> f32 {
        self.steer
    }
}

/// Keyboard: up climbs, down dives, both or neither cancel out.
pub fn steer_from_keys(up: bool, down: bool) -> f32 {
    f32::from(u8::from(down)) - f32::from(u8::from(up))
}

/// Touch / mouse: steer the plane toward the finger's height.
/// Full strength when far away, easing linearly to 0 within
/// [`TOUCH_EASE_DISTANCE`] so it settles without overshooting.
pub fn steer_toward(target_y: f32, plane_y: f32) -> f32 {
    ((target_y - plane_y) / TOUCH_EASE_DISTANCE).clamp(-1.0, 1.0)
}

/// Combine devices: a finger (or held mouse) on screen wins over the keys.
pub fn pick_steer(pointer: Option<f32>, keys: f32) -> f32 {
    pointer.unwrap_or(keys)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_keeps_steer_in_range() {
        assert_eq!(Input::new(0.25).steer(), 0.25);
        assert_eq!(Input::new(2.0).steer(), 1.0);
        assert_eq!(Input::new(-3.0).steer(), -1.0);
    }

    #[test]
    fn input_treats_non_finite_steer_as_level() {
        assert_eq!(Input::new(f32::NAN).steer(), 0.0);
        assert_eq!(Input::new(f32::INFINITY).steer(), 0.0);
    }

    #[test]
    fn up_key_climbs_and_down_key_dives() {
        assert_eq!(steer_from_keys(true, false), -1.0);
        assert_eq!(steer_from_keys(false, true), 1.0);
    }

    #[test]
    fn both_or_no_keys_fly_level() {
        assert_eq!(steer_from_keys(true, true), 0.0);
        assert_eq!(steer_from_keys(false, false), 0.0);
    }

    #[test]
    fn finger_far_away_steers_at_full_strength() {
        assert_eq!(steer_toward(500.0, 100.0), 1.0); // finger below: dive
        assert_eq!(steer_toward(100.0, 500.0), -1.0); // finger above: climb
    }

    #[test]
    fn finger_on_plane_flies_level() {
        assert_eq!(steer_toward(200.0, 200.0), 0.0);
    }

    #[test]
    fn finger_close_to_plane_eases_in() {
        let half = TOUCH_EASE_DISTANCE / 2.0;
        assert!((steer_toward(200.0 + half, 200.0) - 0.5).abs() < 1e-6);
        assert!((steer_toward(200.0 - half, 200.0) + 0.5).abs() < 1e-6);
    }

    #[test]
    fn pointer_on_screen_wins_over_keys() {
        assert_eq!(pick_steer(Some(0.3), -1.0), 0.3);
    }

    #[test]
    fn keys_steer_when_no_pointer_is_down() {
        assert_eq!(pick_steer(None, -1.0), -1.0);
    }
}
