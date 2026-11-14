//! Which level is on screen. Every [`LEVEL_SECONDS`] the game moves on to
//! the next level; during the last [`LEVEL_FADE_SECONDS`] of a level the
//! next level's landscape fades in over it. After the last level the game
//! starts again with level 1.
//!
//! Everything follows from the tick count and the start level, so there
//! is no state in `World` and the simulation stays deterministic.

use crate::TICK_SECONDS;

/// How many levels exist (`web/levels/level1.png` … `levelN.png`).
pub const LEVEL_COUNT: u32 = 5;

/// About how long one level lasts at cruising speed, in seconds of
/// flying. The level itself is a distance ([`LEVEL_DISTANCE`]): more
/// throttle gets there sooner.
pub const LEVEL_SECONDS: f32 = 60.0;

/// How far one level is, in px of landscape at scale 1: what the plane
/// covers in [`LEVEL_SECONDS`] at cruising speed. Then it lands.
pub const LEVEL_DISTANCE: f32 = crate::SCROLL_SPEED * LEVEL_SECONDS;

/// How long the next landscape takes to fade in, at the end of a level.
pub const LEVEL_FADE_SECONDS: f32 = 3.0;

/// How long "Level N" shows at the start of a level (the last half
/// second it fades out).
pub const LEVEL_BANNER_SECONDS: f32 = 2.5;

/// Levels where it may rain (and storm). None so far: rain does not fit
/// the balloons over the coast (level 1) nor the sunset at the beach
/// (level 2). The weather keeps running underneath, so a later level only
/// has to be listed here to get rain and thunderstorms.
pub const RAINY_LEVELS: &[u32] = &[];

const BANNER_FADE_SECONDS: f32 = 0.5;

fn to_ticks(seconds: f32) -> u64 {
    (seconds / TICK_SECONDS).round() as u64
}

/// What to draw for the levels at one moment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LevelView {
    /// The level being flown, 1 ..= [`LEVEL_COUNT`].
    pub level: u32,
    /// The level that comes after it.
    pub next: u32,
    /// How far `next` has faded in over `level`: 0 = not at all, 1 = fully.
    pub fade: f32,
    /// How visible the "Level N" label for `level` is: 0 = hidden, 1 = fully.
    pub banner: f32,
}

impl LevelView {
    /// How heavy the rain may be now (0..1): how much of the rainy levels
    /// ([`RAINY_LEVELS`]) is on screen, so rain fades with the landscape.
    pub fn rain_room(&self) -> f32 {
        self.rain_room_in(RAINY_LEVELS)
    }

    fn rain_room_in(&self, rainy: &[u32]) -> f32 {
        let room: f32 = rainy.iter().map(|&l| self.presence(l)).sum();
        room.min(1.0)
    }

    /// How much of `level` is on screen: 1 = only it, 0 = not at all,
    /// in between while it fades in or out. Sounds that belong to a level
    /// (the beach) follow this, so they fade with the landscape.
    pub fn presence(&self, level: u32) -> f32 {
        let mut p = 0.0;
        if self.level == level {
            p += 1.0 - self.fade;
        }
        if self.next == level {
            p += self.fade;
        }
        p
    }
}

/// A start level as given from outside (for example `?level=2` in the
/// URL), made valid: anything outside 1 ..= [`LEVEL_COUNT`] means level 1.
pub fn start_level(requested: i32) -> u32 {
    if (1..=LEVEL_COUNT as i32).contains(&requested) {
        requested as u32
    } else {
        1
    }
}

/// Is there a level after `level`? Not after the last one: "Level
/// complete" then offers only "Menu" (more levels are still to come).
pub fn has_next_level(level: u32) -> bool {
    level < LEVEL_COUNT
}

/// The levels on screen after `ticks` ticks, when the flight started in
/// level `start` (1 ..= [`LEVEL_COUNT`]; anything else counts as 1).
pub fn level_view(ticks: u64, start: u32) -> LevelView {
    let start = start_level(start as i32);
    let level_ticks = to_ticks(LEVEL_SECONDS);
    let fade_ticks = to_ticks(LEVEL_FADE_SECONDS).min(level_ticks);
    let banner_ticks = to_ticks(LEVEL_BANNER_SECONDS);
    let banner_fade_ticks = to_ticks(BANNER_FADE_SECONDS).min(banner_ticks);

    let index = (start as u64 - 1 + ticks / level_ticks) % LEVEL_COUNT as u64;
    let level = index as u32 + 1;
    let next = level % LEVEL_COUNT + 1;
    let t = ticks % level_ticks; // ticks into this level

    let fade_start = level_ticks - fade_ticks;
    let fade = if t < fade_start || fade_ticks == 0 {
        0.0
    } else {
        (t - fade_start) as f32 / fade_ticks as f32
    };

    let banner = if t >= banner_ticks {
        0.0
    } else if t < banner_ticks - banner_fade_ticks {
        1.0
    } else {
        (banner_ticks - t) as f32 / banner_fade_ticks as f32
    };

    LevelView {
        level,
        next,
        fade,
        banner,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECOND: u64 = 60;
    fn at(seconds: f32) -> u64 {
        to_ticks(seconds)
    }

    #[test]
    fn the_flight_starts_in_level_1() {
        let v = level_view(0, 1);
        assert_eq!((v.level, v.next, v.fade), (1, 2, 0.0));
    }

    #[test]
    fn level_1_stays_alone_until_the_fade() {
        let v = level_view(at(LEVEL_SECONDS - LEVEL_FADE_SECONDS) - 1, 1);
        assert_eq!((v.level, v.fade), (1, 0.0));
    }

    #[test]
    fn the_next_landscape_fades_in_at_the_end_of_a_level() {
        let half = at(LEVEL_SECONDS - LEVEL_FADE_SECONDS / 2.0);
        let v = level_view(half, 1);
        assert_eq!((v.level, v.next), (1, 2));
        assert!((v.fade - 0.5).abs() < 0.01, "{}", v.fade);
    }

    #[test]
    fn level_2_starts_after_level_seconds() {
        let v = level_view(at(LEVEL_SECONDS), 1);
        assert_eq!((v.level, v.next, v.fade), (2, 3, 0.0));
    }

    #[test]
    fn the_switch_has_no_jump() {
        // Just before the switch almost only the next level shows; just
        // after, only that level. Nothing pops.
        let before = level_view(at(LEVEL_SECONDS) - 1, 1);
        let after = level_view(at(LEVEL_SECONDS), 1);
        assert_eq!(before.next, after.level);
        assert!(before.fade > 0.99, "{}", before.fade);
        assert_eq!(after.fade, 0.0);
    }

    #[test]
    fn after_the_last_level_comes_level_1_again() {
        let v = level_view(at(LEVEL_SECONDS) * LEVEL_COUNT as u64, 1);
        assert_eq!(v.level, 1);
    }

    #[test]
    fn a_flight_can_start_in_level_2() {
        let v = level_view(0, 2);
        assert_eq!((v.level, v.next), (2, 3));
        assert_eq!(level_view(at(LEVEL_SECONDS), 2).level, 3);
    }

    #[test]
    fn after_the_last_level_a_flight_goes_on_in_level_1() {
        let v = level_view(0, LEVEL_COUNT);
        assert_eq!((v.level, v.next), (LEVEL_COUNT, 1));
        assert_eq!(level_view(at(LEVEL_SECONDS), LEVEL_COUNT).level, 1);
    }

    #[test]
    fn the_banner_shows_at_the_start_of_each_level_and_then_fades() {
        for start in [0, at(LEVEL_SECONDS)] {
            assert_eq!(level_view(start, 1).banner, 1.0);
            assert_eq!(level_view(start + SECOND, 1).banner, 1.0);
            let fading = level_view(start + at(LEVEL_BANNER_SECONDS - 0.25), 1).banner;
            assert!(fading > 0.0 && fading < 1.0, "{fading}");
            assert_eq!(level_view(start + at(LEVEL_BANNER_SECONDS), 1).banner, 0.0);
        }
        assert_eq!(level_view(at(LEVEL_SECONDS / 2.0), 1).banner, 0.0);
    }

    #[test]
    fn no_level_has_rain_yet() {
        for t in 0..at(2.0 * LEVEL_SECONDS) {
            assert_eq!(level_view(t, 1).rain_room(), 0.0);
        }
    }

    #[test]
    fn rain_fades_with_a_rainy_landscape() {
        let rainy = [1];
        assert_eq!(level_view(0, 1).rain_room_in(&rainy), 1.0);
        let half = level_view(at(LEVEL_SECONDS - LEVEL_FADE_SECONDS / 2.0), 1);
        assert!((half.rain_room_in(&rainy) - 0.5).abs() < 0.01);
        assert_eq!(level_view(at(LEVEL_SECONDS), 1).rain_room_in(&rainy), 0.0);
    }

    #[test]
    fn presence_follows_the_fade() {
        let start = level_view(0, 1);
        assert_eq!((start.presence(1), start.presence(2)), (1.0, 0.0));
        let half = level_view(at(LEVEL_SECONDS - LEVEL_FADE_SECONDS / 2.0), 1);
        assert!((half.presence(2) - 0.5).abs() < 0.01);
        assert!((half.presence(1) + half.presence(2) - 1.0).abs() < 1e-6);
        let two = level_view(at(LEVEL_SECONDS), 1);
        assert_eq!((two.presence(1), two.presence(2)), (0.0, 1.0));
        // Fading back to level 1 at the end of level 2.
        let back = level_view(at(2.0 * LEVEL_SECONDS - LEVEL_FADE_SECONDS / 2.0), 1);
        assert!((back.presence(2) - 0.5).abs() < 0.01);
    }

    #[test]
    fn only_the_last_level_has_no_next_level() {
        for level in 1..LEVEL_COUNT {
            assert!(has_next_level(level), "{level}");
        }
        assert!(!has_next_level(LEVEL_COUNT));
    }

    #[test]
    fn invalid_start_levels_mean_level_1() {
        assert_eq!(start_level(2), 2);
        for bad in [0, -1, LEVEL_COUNT as i32 + 1, i32::MAX] {
            assert_eq!(start_level(bad), 1, "{bad}");
        }
        assert_eq!(level_view(0, 0).level, 1);
        assert_eq!(level_view(0, 99).level, 1);
    }

    #[test]
    fn values_stay_in_range_after_a_long_flight() {
        for ticks in (0..10 * 60 * 60 * SECOND).step_by(97) {
            let v = level_view(ticks, 1);
            assert!((1..=LEVEL_COUNT).contains(&v.level));
            assert!((1..=LEVEL_COUNT).contains(&v.next));
            assert!((0.0..1.0).contains(&v.fade), "{}", v.fade);
            assert!((0.0..=1.0).contains(&v.banner), "{}", v.banner);
        }
    }
}
