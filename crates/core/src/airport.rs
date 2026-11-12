//! The airport the flight starts from (level 1): a runway in the
//! foreground grass, at the huts' ground line, with a control tower and a
//! windsock at its start. The plane stands on it, rolls while the
//! landscape speeds up, lifts off and climbs away on its own; then the
//! player takes over. The airport slides off with the landscape.

use crate::{Bounds, PLAYER_X_FRACTION};

/// Where the runway lies: its bottom, like the huts.
pub const RUNWAY_GROUND: f32 = crate::huts::HUT_GROUND;
/// The runway's length and thickness, in px at phone size.
pub const RUNWAY_LENGTH: f32 = 900.0;
pub const RUNWAY_HEIGHT: f32 = 12.0;
/// How long the plane rolls before it lifts off, in seconds; the
/// landscape speeds up from standing still over this time.
pub const ROLL_SECONDS: f32 = 6.0;
/// How long it climbs away on its own after lifting off, in seconds.
pub const CLIMB_OUT_SECONDS: f32 = 2.0;
/// How hard it climbs out (a steer value, -1 = full climb).
pub const CLIMB_OUT_STEER: f32 = -0.4;
/// A level ends with a landing: this long before it, no new obstacles
/// come, so the sky is clear for the approach.
pub const CLEAR_SECONDS: f32 = 8.0;
/// The landing runway comes in this far beyond the right edge, in px.
pub const APPROACH_DISTANCE: f32 = 60.0;
/// The plane touches down this far down the runway, in px.
pub const TOUCHDOWN_INTO: f32 = 80.0;
/// After touchdown it brakes to a stop over this long, in seconds.
pub const BRAKE_SECONDS: f32 = 3.5;
/// After "Level complete" (which waits for the player), the next level
/// counts down this long before its takeoff.
pub const COUNTDOWN_SECONDS: f32 = 3.0;

/// After lift-off the gear waits this long, then folds up over
/// [`GEAR_RETRACT_SECONDS`] (with its whir and clunk, `Event::GearUp`).
pub const GEAR_DELAY_SECONDS: f32 = 0.4;
pub const GEAR_RETRACT_SECONDS: f32 = 1.0;
/// How far below the plane's centre its wheels touch the runway, in px at
/// size 1 (the drawn landing gear).
pub const WHEELS_BELOW: f32 = 16.0;

/// The airport. `x` is where the runway starts, in screen px.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Airport {
    pub x: f32,
}

impl Airport {
    /// An airport whose runway starts a little behind the plane.
    pub fn at_start(bounds: Bounds) -> Self {
        Self {
            x: bounds.width * PLAYER_X_FRACTION - 70.0 * bounds.scale(),
        }
    }

    /// The ground line the airport stands on, in screen px.
    pub fn ground(bounds: Bounds) -> f32 {
        RUNWAY_GROUND * bounds.height
    }

    /// The runway's top, in screen px.
    pub fn runway_top(bounds: Bounds) -> f32 {
        RUNWAY_GROUND * bounds.height - RUNWAY_HEIGHT * bounds.scale()
    }

    /// Has it slid off the left edge completely (with its tower)?
    /// (The tower and windsock stand up to 120 px behind the runway.)
    pub fn gone(&self, bounds: Bounds) -> bool {
        self.x + RUNWAY_LENGTH * bounds.scale() < -20.0 * bounds.scale()
    }
}
