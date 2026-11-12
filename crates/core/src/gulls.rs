//! Gulls: the obstacles at the beach (level 2). Fly into one and you crash.
//!
//! While the beach is on screen, a new gull shows up every
//! [`GULL_GAP_SECONDS`]: from the right (flying towards you), from the left
//! (catching up from behind) or taking off from the sand. Some call when
//! they appear; gulls from behind always do, so you hear them coming.
//! Gulls that have left the screen are dropped.
//!
//! Positions are screen px, like the player's. Seeded, so the same seed
//! gives the same gulls.

use crate::rng::Rng;
use crate::{Bounds, Event, Vec2, TICK_SECONDS};

/// A gull's size for crashing, in px (its drawn wings are a bit wider).
pub const GULL_RADIUS: f32 = 13.0;
/// Time between two new gulls, shortest and longest, in seconds.
pub const GULL_GAP_SECONDS: (f32, f32) = (1.2, 2.8);
/// No gull in the first seconds of the beach, while "Level 2" shows.
pub const FIRST_GULL_SECONDS: f32 = 2.5;
/// How fast a gull flies across the screen, in px per second.
pub const GULL_SPEED: (f32, f32) = (150.0, 240.0);
/// Share of gulls from the right and from the left; the rest take off
/// from the ground.
pub const GULL_FROM_RIGHT: f32 = 0.45;
pub const GULL_FROM_LEFT: f32 = 0.25;
/// Chance that a gull from the right or the ground calls when it appears.
pub const GULL_CALL_CHANCE: f32 = 0.5;
/// How far a gull bobs up and down while it flaps, in px.
pub const GULL_BOB: f32 = 6.0;

const BOB_PER_SECOND: f32 = 1.3;
/// Gulls start and are dropped this far outside the screen.
const MARGIN: f32 = 40.0;

/// Where a gull came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GullStart {
    Right,
    Left,
    Ground,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Gull {
    /// Centre, for drawing and crashing (includes the bob).
    pub pos: Vec2,
    /// px per second.
    pub vel: Vec2,
    pub from: GullStart,
    /// Ticks since it appeared (for the bob and the wing beat).
    pub age: u32,
    /// Where it would be without the bob.
    line: Vec2,
}

impl Gull {
    fn step(&mut self) {
        self.line.x += self.vel.x * TICK_SECONDS;
        self.line.y += self.vel.y * TICK_SECONDS;
        self.age += 1;
        let t = self.age as f32 * TICK_SECONDS;
        let bob = GULL_BOB * (std::f32::consts::TAU * BOB_PER_SECOND * t).sin();
        self.pos = Vec2::new(self.line.x, self.line.y + bob);
    }

    fn gone(&self, bounds: Bounds) -> bool {
        let p = self.line;
        p.x < -MARGIN * 2.0 || p.x > bounds.width + MARGIN * 2.0 || p.y < -MARGIN * 2.0
    }
}

fn ticks(seconds: f32) -> u32 {
    (seconds / TICK_SECONDS).round().max(1.0) as u32
}

#[derive(Debug, Clone, PartialEq)]
pub struct Gulls {
    /// The gulls in the air right now.
    pub list: Vec<Gull>,
    next_ticks: u32,
    rng: Rng,
}

impl Gulls {
    pub fn new(seed: u64) -> Self {
        Self {
            list: Vec::new(),
            next_ticks: ticks(FIRST_GULL_SECONDS),
            // Its own stream, so gulls do not change the weather.
            rng: Rng::new(seed ^ 0x6A11_5EED_0000_0002),
        }
    }

    /// Advance one tick. `active`: is the beach on screen (no gulls
    /// arrive while it fades in or out)? New calls go to `events`.
    pub fn tick(&mut self, bounds: Bounds, active: bool, events: &mut Vec<Event>) {
        for g in &mut self.list {
            g.step();
        }
        self.list.retain(|g| !g.gone(bounds));

        if !active {
            self.next_ticks = ticks(FIRST_GULL_SECONDS);
            return;
        }
        self.next_ticks -= 1;
        if self.next_ticks == 0 {
            self.spawn(bounds, events);
            self.next_ticks = ticks(self.rng.range(GULL_GAP_SECONDS.0, GULL_GAP_SECONDS.1));
        }
    }

    fn spawn(&mut self, Bounds { width, height }: Bounds, events: &mut Vec<Event>) {
        let r = &mut self.rng;
        let kind = r.unit();
        let speed = r.range(GULL_SPEED.0, GULL_SPEED.1);
        let sky_y = r.range(0.12, 0.72) * height;
        let drift = r.range(-25.0, 25.0);
        let (from, line, vel) = if kind < GULL_FROM_RIGHT {
            let at = Vec2::new(width + MARGIN, sky_y);
            (GullStart::Right, at, Vec2::new(-speed, drift))
        } else if kind < GULL_FROM_RIGHT + GULL_FROM_LEFT {
            // From behind: a bit slower, so it does not come as a surprise.
            let at = Vec2::new(-MARGIN, sky_y);
            (GullStart::Left, at, Vec2::new(speed * 0.6, drift))
        } else {
            let at = Vec2::new(r.range(0.4, 0.95) * width, height + MARGIN);
            let vel = Vec2::new(-r.range(30.0, 90.0), -speed * 0.75);
            (GullStart::Ground, at, vel)
        };
        let calls = from == GullStart::Left || r.unit() < GULL_CALL_CHANCE;
        self.list.push(Gull {
            pos: line,
            vel,
            from,
            age: 0,
            line,
        });
        if calls {
            events.push(Event::GullCall);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN: Bounds = Bounds {
        width: 844.0,
        height: 390.0,
    };

    fn run(g: &mut Gulls, seconds: f32, active: bool) -> Vec<Event> {
        let mut all = Vec::new();
        for _ in 0..ticks(seconds) {
            let mut events = Vec::new();
            g.tick(SCREEN, active, &mut events);
            all.extend(events);
        }
        all
    }

    #[test]
    fn no_gulls_away_from_the_beach() {
        let mut g = Gulls::new(1);
        assert!(run(&mut g, 120.0, false).is_empty());
        assert!(g.list.is_empty());
    }

    #[test]
    fn the_first_gull_waits_for_the_level_banner() {
        let mut g = Gulls::new(1);
        run(&mut g, FIRST_GULL_SECONDS - 0.1, true);
        assert!(g.list.is_empty());
        run(&mut g, 0.2, true);
        assert_eq!(g.list.len(), 1);
    }

    #[test]
    fn gulls_come_from_all_three_sides_and_fly_into_the_screen() {
        let mut g = Gulls::new(3);
        let mut seen = Vec::new();
        for _ in 0..ticks(60.0) {
            g.tick(SCREEN, true, &mut Vec::new());
            for gull in g.list.iter().filter(|gull| gull.age == 1) {
                seen.push(gull.from);
                match gull.from {
                    GullStart::Right => assert!(gull.pos.x > SCREEN.width && gull.vel.x < 0.0),
                    GullStart::Left => assert!(gull.pos.x < 0.0 && gull.vel.x > 0.0),
                    GullStart::Ground => assert!(gull.pos.y > SCREEN.height && gull.vel.y < 0.0),
                }
            }
        }
        for side in [GullStart::Right, GullStart::Left, GullStart::Ground] {
            assert!(seen.contains(&side), "no gull from {side:?} in {seen:?}");
        }
        // One every 1.2–2.8 s over a minute (the first after 2.5 s).
        assert!((20..=48).contains(&seen.len()), "{}", seen.len());
    }

    #[test]
    fn gulls_cross_the_screen_and_are_dropped() {
        let mut g = Gulls::new(5);
        run(&mut g, 60.0, true);
        assert!(g.list.len() <= 12, "{} gulls pile up", g.list.len());
        run(&mut g, 20.0, false); // beach gone: the rest fly off
        assert!(g.list.is_empty());
    }

    #[test]
    fn gulls_from_behind_always_call_and_not_every_gull_calls() {
        let mut g = Gulls::new(9);
        let (mut calls, mut gulls, mut left) = (0, 0, 0);
        for _ in 0..ticks(120.0) {
            let mut events = Vec::new();
            g.tick(SCREEN, true, &mut events);
            calls += events.iter().filter(|e| **e == Event::GullCall).count();
            if let Some(new) = g.list.last().filter(|n| n.age == 0) {
                gulls += 1;
                if new.from == GullStart::Left {
                    left += 1;
                    assert_eq!(events, [Event::GullCall]);
                }
            }
        }
        assert!(
            left > 0 && calls >= left && calls < gulls,
            "{left} {calls} {gulls}"
        );
    }

    #[test]
    fn gulls_bob_but_stay_near_their_line() {
        let mut g = Gulls::new(1);
        run(&mut g, FIRST_GULL_SECONDS, true);
        let gull = &mut g.list[0];
        let mut lowest: f32 = f32::MAX;
        let mut highest: f32 = f32::MIN;
        for _ in 0..60 {
            gull.step();
            let off = gull.pos.y - gull.line.y;
            (lowest, highest) = (lowest.min(off), highest.max(off));
        }
        assert!(highest > GULL_BOB * 0.9 && lowest < -GULL_BOB * 0.9);
        assert!(highest <= GULL_BOB + 1e-3 && lowest >= -GULL_BOB - 1e-3);
    }

    #[test]
    fn same_seed_same_gulls() {
        let (mut a, mut b) = (Gulls::new(4), Gulls::new(4));
        run(&mut a, 30.0, true);
        run(&mut b, 30.0, true);
        assert_eq!(a, b);
    }
}
