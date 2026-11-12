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
    throttle: f32,
}

impl Input {
    /// No steering: fly level.
    pub const NONE: Input = Input {
        steer: 0.0,
        throttle: 0.0,
    };

    /// Clamps to [-1, 1]; NaN or infinite (e.g. a sensor glitch) means level.
    pub fn new(steer: f32) -> Self {
        Self::with_throttle(steer, 0.0)
    }

    /// Steering plus throttle: -1 = brake, 0 = leave the speed be,
    /// 1 = full gas. Each clamped; NaN or infinite counts as 0.
    pub fn with_throttle(steer: f32, throttle: f32) -> Self {
        let clean = |v: f32| {
            if v.is_finite() {
                v.clamp(-1.0, 1.0)
            } else {
                0.0
            }
        };
        Self {
            steer: clean(steer),
            throttle: clean(throttle),
        }
    }

    pub fn steer(&self) -> f32 {
        self.steer
    }

    pub fn throttle(&self) -> f32 {
        self.throttle
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

/// Throttle: keys (left/A brake, right/D gas) win over tipping the phone.
pub fn pick_throttle(keys: f32, tilt: Option<f32>) -> f32 {
    if keys != 0.0 {
        keys
    } else {
        tilt.unwrap_or(0.0)
    }
}

/// Combine devices. A finger (or held mouse) on screen wins, then keys
/// that are pressed, then tilt; otherwise fly level.
pub fn pick_steer(pointer: Option<f32>, keys: f32, tilt: Option<f32>) -> f32 {
    match pointer {
        Some(p) => p,
        None if keys != 0.0 => keys,
        None => tilt.unwrap_or(0.0),
    }
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
    fn pointer_on_screen_wins_over_keys_and_tilt() {
        assert_eq!(pick_steer(Some(0.3), -1.0, Some(0.8)), 0.3);
    }

    #[test]
    fn pressed_keys_win_over_tilt() {
        assert_eq!(pick_steer(None, -1.0, Some(0.8)), -1.0);
    }

    #[test]
    fn tilt_steers_when_no_pointer_or_key_is_used() {
        assert_eq!(pick_steer(None, 0.0, Some(0.8)), 0.8);
    }

    #[test]
    fn nothing_used_flies_level() {
        assert_eq!(pick_steer(None, 0.0, None), 0.0);
    }
}
