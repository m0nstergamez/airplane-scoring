//! What the player should hear, as numbers. Core decides *what* plays and
//! *how loud*; the game crate only loads the files and sets the volumes.
//!
//! Looping sounds (engine, wind, rain) are described by a [`Mix`]: one
//! volume per loop, recomputed every tick. One-shot sounds (thunder, crash)
//! are [`crate::Event`]s.

use crate::level::LevelView;
use crate::weather::{approach, Weather};
use crate::Player;

/// Loudest the two engine loops get (the plane near the ground).
pub const ENGINE_VOLUME: f32 = 0.5;
/// Share of [`ENGINE_VOLUME`] left when the plane is at the very top,
/// far away; in between it fades with its nearness, like its size.
pub const ENGINE_FAR_VOLUME: f32 = 0.3;
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
/// Volume of a hot-air balloon's burner as it appears: clear, not loud.
pub const BALLOON_BURNER_VOLUME: f32 = 0.35;
/// Loudest the runway rumble gets (just before lift-off).
pub const RUMBLE_VOLUME: f32 = 0.8;
/// Volume of the landing gear folding up.
pub const GEAR_VOLUME: f32 = 0.8;
/// Volume of the takeoff spool-up.
pub const TAKEOFF_VOLUME: f32 = 0.9;
/// Volume of the overspeed and stall warning tones.
pub const WARNING_VOLUME: f32 = 0.45;
/// Volume of the warning voices ("Overspeed", "Pull up").
pub const VOICE_VOLUME: f32 = 1.0;
/// Volume of the explosion when the last life goes.
pub const EXPLOSION_VOLUME: f32 = 1.0;
/// Volume of the fanfare for a new highscore.
pub const HIGHSCORE_VOLUME: f32 = 0.8;
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
    /// The overspeed warning tone (a fast clacker) while too fast.
    pub overspeed: f32,
    /// The stall warning tone (a whooping horn) while too slow.
    pub stall: f32,
    /// The wheels rumbling on the runway while the plane rolls.
    pub rumble: f32,
}

impl Mix {
    pub const SILENT: Mix = Mix {
        engine_idle: 0.0,
        engine_power: 0.0,
        wind: 0.0,
        rain: 0.0,
        beach: 0.0,
        overspeed: 0.0,
        stall: 0.0,
        rumble: 0.0,
    };

    /// The volumes that fit the current state of the plane, the weather
    /// and the level on screen. `near`: how near the plane is, 0 (top of
    /// the screen, far away) to 1 (down in the foreground), see
    /// [`Bounds::nearness`](crate::Bounds::nearness).
    pub fn target(player: &Player, weather: &Weather, levels: &LevelView, near: f32) -> Mix {
        // Throttle: 1 in a full climb, 0.5 flying level, 0 in a full dive.
        // The two engine loops cross-fade with it at constant loudness
        // (macroquad 0.4.16 cannot change a sound's pitch).
        // More airspeed sounds like more power too.
        let gas = (player.airspeed - 1.0) * 0.8;
        let throttle = (0.5 - 0.5 * player.steer + gas).clamp(0.0, 1.0);
        let angle = throttle * std::f32::consts::FRAC_PI_2;
        // A crashed plane's engine has cut out, and it falls like a dive.
        let (engine, dive) = if player.is_crashed() {
            (0.0, 1.0)
        } else {
            let far = ENGINE_FAR_VOLUME + (1.0 - ENGINE_FAR_VOLUME) * near.clamp(0.0, 1.0);
            (ENGINE_VOLUME * far, player.steer.max(0.0))
        };
        Mix {
            engine_idle: engine * angle.cos(),
            engine_power: engine * angle.sin(),
            wind: WIND_VOLUME * (weather.wind + DIVE_WIND * dive).clamp(0.0, 1.0),
            rain: RAIN_VOLUME * weather.rain.clamp(0.0, 1.0),
            beach: BEACH_VOLUME * levels.presence(BEACH_LEVEL).clamp(0.0, 1.0),
            overspeed: if player.is_overspeed() {
                WARNING_VOLUME
            } else {
                0.0
            },
            rumble: 0.0, // only on the runway, see `Mix::takeoff`
            stall: if player.is_stalling() {
                WARNING_VOLUME
            } else {
                0.0
            },
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
            overspeed: approach(self.overspeed, target.overspeed, step),
            stall: approach(self.stall, target.stall, step),
            rumble: approach(self.rumble, target.rumble, step),
        }
    }

    /// The takeoff roll's sound, `roll` 0 (standing) .. 1 (lifting off).
    /// The engine loops step aside: the spool-up is its own sound
    /// (`Event::Takeoff`, a whine rising in pitch, which the loops cannot
    /// do). The wheels rumble louder the faster they roll, and the wind
    /// comes with the speed.
    pub fn takeoff(self, roll: f32) -> Mix {
        let roll = roll.clamp(0.0, 1.0);
        Mix {
            engine_idle: 0.0,
            engine_power: 0.0,
            wind: self.wind * roll,
            rumble: RUMBLE_VOLUME * (0.3 + 0.7 * roll),
            ..self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{level_view, Vec2, CRASH_TICKS, LEVEL_FADE_SECONDS, LEVEL_SECONDS, TICK_SECONDS};

    /// The mix in level 1, before any fade: no beach.
    fn target(player: &Player, weather: &Weather) -> Mix {
        Mix::target(player, weather, &level_view(0, 1), 1.0)
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
    fn the_engine_is_quieter_the_further_away_the_plane_flies() {
        let (p, w) = (player(0.0), weather(0.0, 0.0));
        let loud = |near: f32| {
            let m = Mix::target(&p, &w, &level_view(0, 1), near);
            // Both loops together, at constant loudness.
            (m.engine_idle.powi(2) + m.engine_power.powi(2)).sqrt()
        };
        assert!(
            (loud(1.0) - ENGINE_VOLUME).abs() < 1e-6,
            "full near the ground"
        );
        assert!(
            (loud(0.0) - ENGINE_VOLUME * ENGINE_FAR_VOLUME).abs() < 1e-6,
            "quiet at the top"
        );
        let mut last = 0.0;
        for i in 0..=10 {
            let v = loud(i as f32 / 10.0);
            assert!(v > last, "louder the nearer");
            last = v;
        }
        // The wind does not change with it.
        let wind = |near: f32| Mix::target(&p, &w, &level_view(0, 1), near).wind;
        assert_eq!(wind(0.0), wind(1.0));
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
            overspeed: 0.0,
            stall: 0.0,
            rumble: 0.0,
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
                overspeed: 0.0,
                stall: 0.0,
                rumble: 0.0,
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
        let at = |s: f32| Mix::target(&p, &w, &level_view(ticks(s), 1), 1.0).beach;
        assert_eq!(at(0.0), 0.0);
        let half = at(LEVEL_SECONDS - LEVEL_FADE_SECONDS / 2.0);
        assert!((half - BEACH_VOLUME / 2.0).abs() < 0.01, "{half}");
        assert_eq!(at(LEVEL_SECONDS + 1.0), BEACH_VOLUME);
        assert_eq!(at(2.0 * LEVEL_SECONDS + 1.0), 0.0);
        // Starting at the beach (?level=2) means waves from the start.
        assert_eq!(
            Mix::target(&p, &w, &level_view(0, BEACH_LEVEL), 1.0).beach,
            BEACH_VOLUME
        );
    }
}
