//! Turns real frame time into fixed ticks of [`TICK_SECONDS`], and keeps
//! the game standing still until the player starts ("Tap to fly").
//!
//! Before the start no time passes for the game: no ticks run and none
//! are saved up, so the flight (weather, levels, the other plane) begins
//! exactly at the start, however long the start screen was open.

use crate::TICK_SECONDS;

/// A frame longer than this (for example after the browser tab slept)
/// counts only this long, so the game does not race to catch up.
pub const MAX_FRAME_SECONDS: f32 = 0.25;

/// Float rounding must not swallow a tick that is due.
const EPSILON: f32 = 1e-6;

#[derive(Debug, Default, Clone)]
pub struct Clock {
    /// Frame time not yet used up by ticks, in seconds.
    saved: f32,
}

impl Clock {
    /// How many ticks to run for a frame that took `frame_seconds`.
    /// `started`: has the player started yet? Before that it is always 0.
    pub fn ticks_for(&mut self, frame_seconds: f32, started: bool) -> u32 {
        if !started {
            self.saved = 0.0;
            return 0;
        }
        self.saved += frame_seconds.clamp(0.0, MAX_FRAME_SECONDS);
        let mut ticks = 0;
        while self.saved + EPSILON >= TICK_SECONDS {
            self.saved -= TICK_SECONDS;
            ticks += 1;
        }
        self.saved = self.saved.max(0.0);
        ticks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_runs_before_the_start() {
        let mut c = Clock::default();
        for _ in 0..600 {
            assert_eq!(c.ticks_for(0.1, false), 0);
        }
    }

    #[test]
    fn waiting_on_the_start_screen_saves_up_no_time() {
        let mut c = Clock::default();
        for _ in 0..600 {
            c.ticks_for(0.1, false); // one minute on the start screen
        }
        // The first frame after the start runs only that frame's ticks.
        assert_eq!(c.ticks_for(TICK_SECONDS, true), 1);
    }

    #[test]
    fn one_second_of_frames_is_sixty_ticks() {
        let mut c = Clock::default();
        let total: u32 = (0..60).map(|_| c.ticks_for(TICK_SECONDS, true)).sum();
        assert_eq!(total, 60);
    }

    #[test]
    fn slow_frames_run_several_ticks_and_fast_ones_sometimes_none() {
        let mut c = Clock::default();
        assert_eq!(c.ticks_for(3.0 * TICK_SECONDS, true), 3);
        let mut c = Clock::default();
        let ticks: Vec<u32> = (0..4)
            .map(|_| c.ticks_for(TICK_SECONDS / 2.0, true))
            .collect();
        assert_eq!(ticks, [0, 1, 0, 1]);
    }

    #[test]
    fn a_very_long_frame_is_capped() {
        let mut c = Clock::default();
        let capped = (MAX_FRAME_SECONDS / TICK_SECONDS).round() as u32;
        assert_eq!(c.ticks_for(30.0, true), capped);
    }

    #[test]
    fn negative_frame_time_runs_nothing() {
        let mut c = Clock::default();
        assert_eq!(c.ticks_for(-1.0, true), 0);
    }
}
