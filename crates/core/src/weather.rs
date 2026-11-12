//! Weather: rain that comes and goes, wind, lightning and thunder.
//!
//! Everything is driven by a seed, one tick at a time, so the same seed
//! gives the same weather. The game crate only reads the levels (to draw
//! rain and set sound volumes) and the events (to flash and to play thunder).

use crate::rng::Rng;
use crate::{Event, TICK_SECONDS};

/// The game always starts dry for this long, so nobody starts in a storm.
pub const FIRST_DRY_SECONDS: f32 = 10.0;
/// A dry spell lasts between these many seconds.
pub const DRY_SECONDS: (f32, f32) = (15.0, 30.0);
/// A rain spell lasts between these many seconds.
pub const RAIN_SECONDS: (f32, f32) = (20.0, 40.0);
/// How heavy a rain spell gets at its peak (0 = dry, 1 = downpour).
pub const RAIN_PEAK: (f32, f32) = (0.4, 1.0);
/// Rain grows and fades this much per second (dry to downpour in 8 s).
pub const RAIN_CHANGE_PER_SECOND: f32 = 0.125;
/// From this much rain on it is a thunderstorm: lightning can strike.
pub const STORM_RAIN: f32 = 0.6;
/// In a thunderstorm, lightning strikes this many seconds apart.
pub const STRIKE_GAP_SECONDS: (f32, f32) = (4.0, 10.0);
/// Thunder follows lightning after this long: (strike overhead, far away).
pub const THUNDER_DELAY_SECONDS: (f32, f32) = (0.3, 3.0);
/// Thunder volume: (strike overhead, far away).
pub const THUNDER_VOLUME: (f32, f32) = (1.0, 0.3);
/// How long the sky stays lit after a strike.
pub const FLASH_SECONDS: f32 = 0.35;
/// Wind when it is dry and calm.
pub const CALM_WIND: f32 = 0.15;
/// Extra wind at full rain.
pub const RAIN_WIND: f32 = 0.6;
/// Gusts add up to this much on top.
pub const GUST_WIND: f32 = 0.25;

/// Seconds between new gust strengths, and how fast a gust builds.
const GUST_SECONDS: (f32, f32) = (2.0, 6.0);
const GUST_CHANGE_PER_SECOND: f32 = 0.1;

fn ticks(seconds: f32) -> u32 {
    (seconds / TICK_SECONDS).round().max(1.0) as u32
}

fn lerp((a, b): (f32, f32), t: f32) -> f32 {
    a + (b - a) * t
}

/// Move `value` toward `target`, by at most `step`.
pub(crate) fn approach(value: f32, target: f32, step: f32) -> f32 {
    value + (target - value).clamp(-step, step)
}

#[derive(Debug, Clone, PartialEq)]
pub struct Weather {
    /// How hard it rains right now: 0 = dry, 1 = downpour.
    pub rain: f32,
    /// How hard the wind blows right now, 0..1.
    pub wind: f32,
    /// Brightness of the lightning flash: jumps up on a strike, fades to 0.
    pub flash: f32,
    rain_target: f32,
    spell_ticks: u32,
    strike_ticks: u32,
    gust: f32,
    gust_target: f32,
    gust_ticks: u32,
    /// Thunder on its way: (ticks until it is heard, volume).
    thunder: Vec<(u32, f32)>,
    rng: Rng,
}

impl Weather {
    pub fn new(seed: u64) -> Self {
        let mut rng = Rng::new(seed);
        let strike_ticks = ticks(rng.range(STRIKE_GAP_SECONDS.0, STRIKE_GAP_SECONDS.1));
        Self {
            rain: 0.0,
            wind: CALM_WIND,
            flash: 0.0,
            rain_target: 0.0,
            spell_ticks: ticks(FIRST_DRY_SECONDS),
            strike_ticks,
            gust: 0.0,
            gust_target: 0.0,
            gust_ticks: 1,
            thunder: Vec::new(),
            rng,
        }
    }

    /// True while lightning can strike.
    pub fn is_storm(&self) -> bool {
        self.rain >= STORM_RAIN
    }

    /// Advance by one fixed step. New lightning and thunder go to `events`.
    /// `max_rain` (0..1) caps the rain: the beach is always dry, so at
    /// the beach it is 0, and during a fade it follows the landscape. With
    /// it at 0 there is no thunderstorm, and thunder still on its way is
    /// dropped. Dry and rain spells keep their timing underneath.
    pub fn tick(&mut self, events: &mut Vec<Event>, max_rain: f32) {
        // Rain spells and dry spells take turns.
        self.spell_ticks -= 1;
        if self.spell_ticks == 0 {
            if self.rain_target > 0.0 {
                self.rain_target = 0.0;
                self.spell_ticks = ticks(self.rng.range(DRY_SECONDS.0, DRY_SECONDS.1));
            } else {
                self.rain_target = self.rng.range(RAIN_PEAK.0, RAIN_PEAK.1);
                self.spell_ticks = ticks(self.rng.range(RAIN_SECONDS.0, RAIN_SECONDS.1));
            }
        }
        self.rain = approach(
            self.rain,
            self.rain_target,
            RAIN_CHANGE_PER_SECOND * TICK_SECONDS,
        )
        .min(max_rain.max(0.0));
        if max_rain <= 0.0 {
            self.thunder.clear();
        }

        // Wind follows the rain, with gusts on top.
        self.gust_ticks -= 1;
        if self.gust_ticks == 0 {
            self.gust_target = self.rng.unit();
            self.gust_ticks = ticks(self.rng.range(GUST_SECONDS.0, GUST_SECONDS.1));
        }
        self.gust = approach(
            self.gust,
            self.gust_target,
            GUST_CHANGE_PER_SECOND * TICK_SECONDS,
        );
        self.wind = (CALM_WIND + RAIN_WIND * self.rain + GUST_WIND * self.gust).clamp(0.0, 1.0);

        for t in &mut self.thunder {
            t.0 -= 1;
            if t.0 == 0 {
                events.push(Event::Thunder { volume: t.1 });
            }
        }
        self.thunder.retain(|t| t.0 > 0);

        // Lightning: seen at once, heard later. The further away the strike,
        // the dimmer the flash and the later and quieter the thunder.
        self.flash = (self.flash - TICK_SECONDS / FLASH_SECONDS).max(0.0);
        if self.is_storm() {
            self.strike_ticks -= 1;
            if self.strike_ticks == 0 {
                let distance = self.rng.unit();
                events.push(Event::Lightning { distance });
                self.flash = 1.0 - 0.5 * distance;
                self.thunder.push((
                    ticks(lerp(THUNDER_DELAY_SECONDS, distance)),
                    lerp(THUNDER_VOLUME, distance),
                ));
                self.strike_ticks =
                    ticks(self.rng.range(STRIKE_GAP_SECONDS.0, STRIKE_GAP_SECONDS.1));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINUTE: usize = 3600; // ticks

    /// Run for `n` ticks; returns (tick, rain at that tick, event) per event.
    fn run(w: &mut Weather, n: usize) -> Vec<(usize, f32, Event)> {
        let mut log = Vec::new();
        let mut events = Vec::new();
        for i in 0..n {
            w.tick(&mut events, 1.0);
            log.extend(events.drain(..).map(|e| (i, w.rain, e)));
        }
        log
    }

    #[test]
    fn the_game_starts_dry_and_quiet() {
        let mut w = Weather::new(3);
        assert_eq!(w.rain, 0.0);
        let log = run(&mut w, ticks(FIRST_DRY_SECONDS) as usize - 1);
        assert_eq!(w.rain, 0.0);
        assert!(log.is_empty());
    }

    #[test]
    fn rain_comes_and_goes() {
        for seed in 0..20 {
            let mut w = Weather::new(seed);
            let mut events = Vec::new();
            let (mut wet, mut dry_again) = (false, false);
            for _ in 0..3 * MINUTE {
                w.tick(&mut events, 1.0);
                wet |= w.rain >= RAIN_PEAK.0;
                dry_again |= wet && w.rain == 0.0;
            }
            assert!(wet && dry_again, "seed {seed}: {wet} {dry_again}");
        }
    }

    #[test]
    fn rain_and_wind_change_smoothly_and_stay_in_range() {
        let mut w = Weather::new(11);
        let mut events = Vec::new();
        let (mut rain, mut wind) = (w.rain, w.wind);
        for _ in 0..5 * MINUTE {
            w.tick(&mut events, 1.0);
            assert!((0.0..=1.0).contains(&w.rain), "rain {}", w.rain);
            assert!((0.0..=1.0).contains(&w.wind), "wind {}", w.wind);
            assert!((0.0..=1.0).contains(&w.flash), "flash {}", w.flash);
            assert!((w.rain - rain).abs() <= RAIN_CHANGE_PER_SECOND * TICK_SECONDS + 1e-6);
            assert!((w.wind - wind).abs() < 0.01, "wind jumped");
            (rain, wind) = (w.rain, w.wind);
        }
    }

    #[test]
    fn more_rain_means_more_wind() {
        let mut w = Weather::new(5);
        let mut events = Vec::new();
        let (mut dry, mut storm) = (Vec::new(), Vec::new());
        for _ in 0..10 * MINUTE {
            w.tick(&mut events, 1.0);
            if w.rain == 0.0 {
                dry.push(w.wind);
            } else if w.is_storm() {
                storm.push(w.wind);
            }
        }
        let mean = |v: &[f32]| v.iter().sum::<f32>() / v.len() as f32;
        assert!(!dry.is_empty() && !storm.is_empty());
        assert!(mean(&storm) > mean(&dry) + 0.3);
    }

    #[test]
    fn lightning_only_strikes_in_a_thunderstorm() {
        let mut strikes = 0;
        for seed in 0..10 {
            for (_, rain, e) in run(&mut Weather::new(seed), 10 * MINUTE) {
                if matches!(e, Event::Lightning { .. }) {
                    assert!(rain >= STORM_RAIN, "seed {seed}: strike at rain {rain}");
                    strikes += 1;
                }
            }
        }
        assert!(strikes > 10, "only {strikes} strikes");
    }

    #[test]
    fn lightning_lights_up_the_sky_and_fades() {
        let mut w = Weather::new(1);
        let mut events = Vec::new();
        while events.is_empty() {
            w.tick(&mut events, 1.0);
        }
        let Event::Lightning { distance } = events[0] else {
            panic!("first event should be lightning: {events:?}");
        };
        assert!((w.flash - (1.0 - 0.5 * distance)).abs() < 1e-6);
        // Run past the flash, but not into the next strike.
        for _ in 0..ticks(FLASH_SECONDS) + 1 {
            w.tick(&mut events, 1.0);
        }
        assert_eq!(w.flash, 0.0);
    }

    #[test]
    fn thunder_follows_every_lightning_later_and_quieter_with_distance() {
        let log = run(&mut Weather::new(2), 10 * MINUTE);
        let mut waiting: Vec<(usize, f32)> = Vec::new(); // (due tick, volume)
        let mut heard = 0;
        for (i, _, e) in log {
            match e {
                Event::Lightning { distance } => {
                    assert!((0.0..1.0).contains(&distance));
                    let delay = ticks(lerp(THUNDER_DELAY_SECONDS, distance)) as usize;
                    waiting.push((i + delay, lerp(THUNDER_VOLUME, distance)));
                }
                Event::Thunder { volume } => {
                    let at = waiting.iter().position(|w| w.0 == i);
                    let (_, expected) = waiting.remove(at.expect("thunder without lightning"));
                    assert_eq!(volume, expected);
                    assert!((THUNDER_VOLUME.1..=THUNDER_VOLUME.0).contains(&volume));
                    heard += 1;
                }
                Event::Crash => panic!("weather cannot crash a plane"),
                Event::NewLevel { .. } | Event::GullCall | Event::GameOver => panic!("not weather"),
            }
        }
        assert!(heard > 3, "only {heard} thunders");
        assert!(waiting.len() <= 1, "thunder got lost: {waiting:?}");
    }

    #[test]
    fn same_seed_same_weather_other_seed_other_weather() {
        let (mut a, mut b, mut c) = (Weather::new(9), Weather::new(9), Weather::new(10));
        let (la, lb, lc) = (
            run(&mut a, 5 * MINUTE),
            run(&mut b, 5 * MINUTE),
            run(&mut c, 5 * MINUTE),
        );
        assert_eq!(a, b);
        assert_eq!(la, lb);
        assert!(a != c || la != lc);
    }

    #[test]
    fn no_rain_and_no_storm_when_rain_is_not_allowed() {
        let mut w = Weather::new(2);
        let mut events = Vec::new();
        for _ in 0..10 * MINUTE {
            w.tick(&mut events, 0.0);
            assert_eq!(w.rain, 0.0);
            assert!(!w.is_storm());
        }
        assert!(events.is_empty(), "{events:?}");
    }

    #[test]
    fn capped_rain_follows_the_cap_and_comes_back_without_a_jump() {
        let mut w = Weather::new(2);
        let mut events = Vec::new();
        // Find a heavy shower.
        while w.rain < 0.5 {
            w.tick(&mut events, 1.0);
        }
        w.tick(&mut events, 0.2);
        assert!(w.rain <= 0.2);
        let before = w.rain;
        w.tick(&mut events, 1.0);
        assert!(w.rain - before <= RAIN_CHANGE_PER_SECOND * TICK_SECONDS + 1e-6);
    }
}
