//! What the player should hear, as numbers. Core decides *what* plays and
//! *how loud*; the game crate only loads the files and sets the volumes.
//!
//! Looping sounds (engine, wind, rain) are described by a [`Mix`]: one
//! volume per loop, recomputed every tick. One-shot sounds (thunder, crash)
//! are [`crate::Event`]s.

use crate::level::LevelView;
use crate::weather::{approach, Weather};
use crate::Player;

/// Loudest the two engine loops get.
pub const ENGINE_VOLUME: f32 = 0.5;
/// Loudest the wind loop gets.
pub const WIND_VOLUME: f32 = 0.6;
/// Loudest the rain loop gets.
pub const RAIN_VOLUME: f32 = 0.7;
/// Crash volume.
pub const CRASH_VOLUME: f32 = 0.9;
/// The level whose landscape is the beach: its waves and gulls play there.
pub const BEACH_LEVEL: u32 = 2;
/// Loudest the beach loop (waves and gulls) gets.
pub const BEACH_VOLUME: f32 = 0.5;
/// Volume of the short tune when a new level begins.
pub const NEW_LEVEL_VOLUME: f32 = 0.8;
/// Volume of a gull's call as it appears (as loud as the calls in the
/// first beach loop, a bit more so you hear gulls from behind).
pub const GULL_CALL_VOLUME: f32 = 0.25;
/// A full dive adds this much wind: the plane is faster.
pub const DIVE_WIND: f32 = 0.25;
/// Volumes move at most this much per second (silent to full in 0.25 s),
/// so nothing clicks or pops when the steering jumps.
pub const MIX_CHANGE_PER_SECOND: f32 = 4.0;

/// Volume (0..1) of every looping sound.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mix {
    /// Low engine loop: loudest in a dive, when the engine idles.
    pub engine_idle: f32,
    /// High engine loop: loudest in a climb, at full power.
    pub engine_power: f32,
    pub wind: f32,
    pub rain: f32,
    /// Waves and gulls: only at the beach, fading with its landscape.
    pub beach: f32,
}

impl Mix {
    pub const SILENT: Mix = Mix {
        engine_idle: 0.0,
        engine_power: 0.0,
        wind: 0.0,
        rain: 0.0,
        beach: 0.0,
    };

    /// The volumes that fit the current state of the plane, the weather
    /// and the level on screen.
    pub fn target(player: &Player, weather: &Weather, levels: &LevelView) -> Mix {
        // Throttle: 1 in a full climb, 0.5 flying level, 0 in a full dive.
        // The two engine loops cross-fade with it at constant loudness
        // (macroquad 0.4.16 cannot change a sound's pitch).
        let throttle = 0.5 - 0.5 * player.steer;
        let angle = throttle * std::f32::consts::FRAC_PI_2;
        // A crashed plane's engine has cut out, and it falls like a dive.
        let (engine, dive) = if player.is_crashed() {
            (0.0, 1.0)
        } else {
            (ENGINE_VOLUME, player.steer.max(0.0))
        };
        Mix {
            engine_idle: engine * angle.cos(),
            engine_power: engine * angle.sin(),
            wind: WIND_VOLUME * (weather.wind + DIVE_WIND * dive).clamp(0.0, 1.0),
            rain: RAIN_VOLUME * weather.rain.clamp(0.0, 1.0),
            beach: BEACH_VOLUME * levels.presence(BEACH_LEVEL).clamp(0.0, 1.0),
        }
    }

    /// Move every volume toward `target`, by at most `step`.
    pub fn approach(self, target: Mix, step: f32) -> Mix {
        Mix {
            engine_idle: approach(self.engine_idle, target.engine_idle, step),
            engine_power: approach(self.engine_power, target.engine_power, step),
            wind: approach(self.wind, target.wind, step),
            rain: approach(self.rain, target.rain, step),
            beach: approach(self.beach, target.beach, step),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{level_view, Vec2, CRASH_TICKS, LEVEL_FADE_SECONDS, LEVEL_SECONDS, TICK_SECONDS};

    /// The mix in level 1, before any fade: no beach.
    fn target(player: &Player, weather: &Weather) -> Mix {
        Mix::target(player, weather, &level_view(0, 1))
    }

    fn player(steer: f32) -> Player {
        Player {
            steer,
            ..Player::at(Vec2::new(0.0, 0.0))
        }
    }

    fn weather(rain: f32, wind: f32) -> Weather {
        let mut w = Weather::new(0);
        (w.rain, w.wind) = (rain, wind);
        w
    }

    #[test]
    fn climbing_is_full_power_and_diving_is_idle() {
        let calm = weather(0.0, 0.0);
        let climb = target(&player(-1.0), &calm);
        assert!((climb.engine_power - ENGINE_VOLUME).abs() < 1e-6);
        assert!(climb.engine_idle.abs() < 1e-6);
        let dive = target(&player(1.0), &calm);
        assert!((dive.engine_idle - ENGINE_VOLUME).abs() < 1e-6);
        assert!(dive.engine_power.abs() < 1e-6);
    }

    #[test]
    fn level_flight_mixes_both_engine_loops_equally() {
        let m = target(&player(0.0), &weather(0.0, 0.0));
        assert!((m.engine_idle - m.engine_power).abs() < 1e-6);
        assert!(m.engine_idle > 0.0);
    }

    #[test]
    fn engine_loudness_is_the_same_at_every_steer() {
        for i in -10..=10 {
            let m = target(&player(i as f32 / 10.0), &weather(0.0, 0.0));
            let power = m.engine_idle.powi(2) + m.engine_power.powi(2);
            assert!((power - ENGINE_VOLUME.powi(2)).abs() < 1e-5, "steer {i}");
        }
    }

    #[test]
    fn wind_follows_the_weather_and_grows_in_a_dive() {
        let breeze = weather(0.0, 0.4);
        let level = target(&player(0.0), &breeze);
        assert!((level.wind - WIND_VOLUME * 0.4).abs() < 1e-6);
        let climb = target(&player(-1.0), &breeze);
        assert_eq!(climb.wind, level.wind);
        let dive = target(&player(1.0), &breeze);
        assert!((dive.wind - WIND_VOLUME * (0.4 + DIVE_WIND)).abs() < 1e-6);
        let gale = target(&player(1.0), &weather(0.0, 1.0));
        assert_eq!(gale.wind, WIND_VOLUME);
    }

    #[test]
    fn rain_is_silent_when_dry_and_loudest_in_a_downpour() {
        assert_eq!(target(&player(0.0), &weather(0.0, 0.0)).rain, 0.0);
        let half = target(&player(0.0), &weather(0.5, 0.0)).rain;
        assert!((half - RAIN_VOLUME * 0.5).abs() < 1e-6);
        assert_eq!(target(&player(0.0), &weather(1.0, 0.0)).rain, RAIN_VOLUME);
    }

    #[test]
    fn a_crashed_plane_has_no_engine_but_still_wind_and_rain() {
        let mut p = player(0.0);
        p.crash_ticks = CRASH_TICKS;
        let m = target(&p, &weather(0.5, 0.2));
        assert_eq!((m.engine_idle, m.engine_power), (0.0, 0.0));
        assert!(m.wind > 0.0 && m.rain > 0.0);
    }

    #[test]
    fn approach_moves_by_at_most_one_step_and_then_arrives() {
        let target = Mix {
            engine_idle: 0.5,
            engine_power: 0.0,
            wind: 0.05,
            rain: 1.0,
            beach: 0.3,
        };
        let m = Mix::SILENT.approach(target, 0.1);
        assert_eq!(
            m,
            Mix {
                engine_idle: 0.1,
                engine_power: 0.0,
                wind: 0.05,
                rain: 0.1,
                beach: 0.1,
            }
        );
        let mut m = m;
        for _ in 0..10 {
            m = m.approach(target, 0.1);
        }
        assert!((m.rain - 1.0).abs() < 1e-6 && (m.engine_idle - 0.5).abs() < 1e-6);
    }

    #[test]
    fn the_beach_is_heard_only_at_the_beach_and_fades_with_it() {
        let (p, w) = (player(0.0), weather(0.0, 0.0));
        let ticks = |s: f32| (s / TICK_SECONDS).round() as u64;
        let at = |s: f32| Mix::target(&p, &w, &level_view(ticks(s), 1)).beach;
        assert_eq!(at(0.0), 0.0);
        let half = at(LEVEL_SECONDS - LEVEL_FADE_SECONDS / 2.0);
        assert!((half - BEACH_VOLUME / 2.0).abs() < 0.01, "{half}");
        assert_eq!(at(LEVEL_SECONDS + 1.0), BEACH_VOLUME);
        assert_eq!(at(2.0 * LEVEL_SECONDS + 1.0), 0.0);
        // Starting at the beach (?level=2) means waves from the start.
        assert_eq!(
            Mix::target(&p, &w, &level_view(0, BEACH_LEVEL)).beach,
            BEACH_VOLUME
        );
    }
}
