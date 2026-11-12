//! Hot-air balloons: the obstacles of level 1, the easiest level. They are
//! big, slow and rare. Each comes in from the right and drifts left with
//! the landscape (and a bit of wind), in one of three ways
//! ([`BalloonWay`]): rising from the lower sky, coming in level at the
//! plane's height, or sinking from the upper sky to land. You see it
//! coming for seconds and can fly over or under it. Fly into one and you
//! crash.
//!
//! As it appears, its burner roars once ([`Event::BalloonBurner`]), so you
//! hear it as well as see it. Positions are screen px, like the player's.
//! Seeded, so the same seed gives the same balloons.

use crate::rng::Rng;
use crate::{Bounds, Event, Vec2, SCROLL_SPEED, TICK_SECONDS};

/// The envelope's radius, in px. The basket hangs below it.
pub const BALLOON_RADIUS: f32 = 30.0;
/// The basket's centre, below the envelope's centre, in px.
pub const BASKET_BELOW: f32 = 50.0;
/// Half the basket's size for crashing, in px (envelope lines included).
pub const BASKET_RADIUS: f32 = 10.0;
/// Time between two balloons, shortest and longest, in seconds.
pub const BALLOON_GAP_SECONDS: (f32, f32) = (5.0, 8.0);
/// No balloon in the first seconds of level 1, while "Level 1" shows.
pub const FIRST_BALLOON_SECONDS: f32 = 2.5;
/// How fast a balloon rises or sinks, slowest and fastest, in px per
/// second.
pub const BALLOON_RISE: (f32, f32) = (8.0, 25.0);
/// Where a rising balloon comes in: its envelope's centre, as a fraction
/// of the screen height from the top (the lower half of the sky).
pub const BALLOON_START_HEIGHT: (f32, f32) = (0.45, 0.8);
/// Where a landing balloon comes in (the upper sky), like above.
pub const BALLOON_LANDING_HEIGHT: (f32, f32) = (0.1, 0.3);
/// Share of balloons that rise and that come at the plane's height; the
/// rest come down to land.
pub const BALLOON_RISING: f32 = 0.4;
pub const BALLOON_AT_PLANE: f32 = 0.3;
/// How many colour patterns there are (the game crate draws them).
pub const BALLOON_COLOURS: u32 = 4;

/// The wind pushes a balloon left a little faster than the landscape.
const DRIFT: (f32, f32) = (0.0, 20.0);
/// Balloons start and are dropped this far outside the screen, in px.
const MARGIN: f32 = 2.0 * BALLOON_RADIUS;

/// How a balloon crosses the sky.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BalloonWay {
    /// From the lower sky, going up.
    Rising,
    /// Level, at the height the plane flew when it appeared: head-on.
    AtPlane,
    /// From the upper sky, going down to land.
    Landing,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Balloon {
    /// Centre of the envelope, in px.
    pub pos: Vec2,
    /// px per second (x < 0: drifting left, y < 0: rising).
    pub vel: Vec2,
    /// Which colour pattern, 0 .. [`BALLOON_COLOURS`].
    pub colour: u32,
    pub way: BalloonWay,
}

impl Balloon {
    /// Centre of the basket, in px.
    pub fn basket(&self) -> Vec2 {
        Vec2::new(self.pos.x, self.pos.y + BASKET_BELOW)
    }

    /// Does a circle (the player's plane) touch the envelope or the basket?
    pub fn touches(&self, centre: Vec2, radius: f32) -> bool {
        let near = |at: Vec2, r: f32| {
            let reach = r + radius;
            let (dx, dy) = (at.x - centre.x, at.y - centre.y);
            dx * dx + dy * dy < reach * reach
        };
        near(self.pos, BALLOON_RADIUS) || near(self.basket(), BASKET_RADIUS)
    }

    fn gone(&self, bounds: Bounds) -> bool {
        let below = self.pos.y - BALLOON_RADIUS > bounds.height + MARGIN;
        self.pos.x < -MARGIN || self.basket().y < -MARGIN || below
    }
}

fn ticks(seconds: f32) -> u32 {
    (seconds / TICK_SECONDS).round().max(1.0) as u32
}

#[derive(Debug, Clone, PartialEq)]
pub struct Balloons {
    /// The balloons in the sky right now, oldest first.
    pub list: Vec<Balloon>,
    next_ticks: u32,
    rng: Rng,
}

impl Balloons {
    pub fn new(seed: u64) -> Self {
        Self {
            list: Vec::new(),
            next_ticks: ticks(FIRST_BALLOON_SECONDS),
            // Its own stream, so balloons do not change the weather or gulls.
            rng: Rng::new(seed ^ 0xBA11_0075_0000_0004),
        }
    }

    /// Advance one tick. `active`: is level 1 fully on screen? Balloons in
    /// the sky keep drifting off when it is not. `plane_y`: the player's
    /// height, for balloons that come at it. A new balloon's burner goes
    /// to `events`.
    pub fn tick(&mut self, bounds: Bounds, active: bool, plane_y: f32, events: &mut Vec<Event>) {
        for b in &mut self.list {
            b.pos.x += b.vel.x * TICK_SECONDS;
            b.pos.y += b.vel.y * TICK_SECONDS;
        }
        self.list.retain(|b| !b.gone(bounds));

        if !active {
            self.next_ticks = ticks(FIRST_BALLOON_SECONDS);
            return;
        }
        self.next_ticks -= 1;
        if self.next_ticks == 0 {
            let r = &mut self.rng;
            let kind = r.unit();
            let drift = r.range(DRIFT.0, DRIFT.1);
            let speed = r.range(BALLOON_RISE.0, BALLOON_RISE.1);
            let colour = (r.unit() * BALLOON_COLOURS as f32) as u32 % BALLOON_COLOURS;
            let h = bounds.height;
            let (way, y, climb) = if kind < BALLOON_RISING {
                let y = r.range(BALLOON_START_HEIGHT.0, BALLOON_START_HEIGHT.1) * h;
                (BalloonWay::Rising, y, -speed)
            } else if kind < BALLOON_RISING + BALLOON_AT_PLANE {
                // The plane's height, but the whole balloon on screen.
                let lowest = (h - BASKET_BELOW - BASKET_RADIUS).max(BALLOON_RADIUS);
                (
                    BalloonWay::AtPlane,
                    plane_y.clamp(BALLOON_RADIUS, lowest),
                    0.0,
                )
            } else {
                let y = r.range(BALLOON_LANDING_HEIGHT.0, BALLOON_LANDING_HEIGHT.1) * h;
                (BalloonWay::Landing, y, speed)
            };
            self.list.push(Balloon {
                pos: Vec2::new(bounds.width + MARGIN, y),
                vel: Vec2::new(-(SCROLL_SPEED + drift), climb),
                colour,
                way,
            });
            events.push(Event::BalloonBurner);
            self.next_ticks = ticks(r.range(BALLOON_GAP_SECONDS.0, BALLOON_GAP_SECONDS.1));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PHONE: Bounds = Bounds {
        width: 844.0,
        height: 390.0,
    };
    const PLANE_Y: f32 = 150.0;

    fn run(b: &mut Balloons, seconds: f32, active: bool) -> Vec<Event> {
        let mut all = Vec::new();
        for _ in 0..ticks(seconds) {
            b.tick(PHONE, active, PLANE_Y, &mut all);
        }
        all
    }

    #[test]
    fn no_balloons_outside_level_1() {
        let mut b = Balloons::new(1);
        assert!(run(&mut b, 120.0, false).is_empty());
        assert!(b.list.is_empty());
    }

    #[test]
    fn the_first_balloon_waits_for_the_level_banner_and_roars_once() {
        let mut b = Balloons::new(1);
        assert!(run(&mut b, FIRST_BALLOON_SECONDS - 0.1, true).is_empty());
        assert_eq!(run(&mut b, 0.2, true), [Event::BalloonBurner]);
        assert_eq!(b.list.len(), 1);
        let first = b.list[0];
        assert!(
            first.pos.x - BALLOON_RADIUS > PHONE.width,
            "starts off screen"
        );
        assert!(first.vel.x < 0.0, "drifts left");
    }

    #[test]
    fn balloons_are_rare_and_never_bunch_up() {
        let mut b = Balloons::new(3);
        let mut most = 0;
        let mut burners = 0;
        for _ in 0..ticks(120.0) {
            let mut events = Vec::new();
            b.tick(PHONE, true, PLANE_Y, &mut events);
            burners += events.len();
            most = most.max(b.list.len());
            for pair in b.list.windows(2) {
                let gap = pair[1].pos.x - pair[0].pos.x;
                assert!(gap > 4.0 * BALLOON_RADIUS, "too close: {gap}");
            }
        }
        // One every 5–8 s over two minutes (the first after 2.5 s).
        assert!((14..=24).contains(&burners), "{burners}");
        assert!(most <= 3, "{most} at once");
    }

    #[test]
    fn balloons_rise_come_at_the_plane_or_land_slowly() {
        let mut b = Balloons::new(5);
        let mut seen = Vec::new();
        for _ in 0..ticks(240.0) {
            b.tick(PHONE, true, PLANE_Y, &mut Vec::new());
            let Some(new) = b.list.last().filter(|n| n.pos.x == PHONE.width + MARGIN) else {
                continue;
            };
            seen.push(new.way);
            let y = new.pos.y / PHONE.height;
            let within = |(lo, hi): (f32, f32)| (lo..=hi).contains(&y);
            match new.way {
                BalloonWay::Rising => assert!(within(BALLOON_START_HEIGHT) && new.vel.y < 0.0),
                BalloonWay::AtPlane => assert!(new.pos.y == PLANE_Y && new.vel.y == 0.0),
                BalloonWay::Landing => assert!(within(BALLOON_LANDING_HEIGHT) && new.vel.y > 0.0),
            }
            // Slower than a gull: in view for many seconds.
            assert!(-new.vel.x <= SCROLL_SPEED + DRIFT.1, "{:?}", new.vel);
            assert!(new.vel.y.abs() <= BALLOON_RISE.1);
            assert!(new.colour < BALLOON_COLOURS);
        }
        for way in [BalloonWay::Rising, BalloonWay::AtPlane, BalloonWay::Landing] {
            assert!(seen.contains(&way), "no {way:?} in {seen:?}");
        }
    }

    #[test]
    fn a_balloon_at_the_plane_stays_whole_on_screen() {
        for plane_y in [0.0, PHONE.height] {
            let mut b = Balloons::new(5);
            for _ in 0..ticks(240.0) {
                b.tick(PHONE, true, plane_y, &mut Vec::new());
                for n in b.list.iter().filter(|n| n.way == BalloonWay::AtPlane) {
                    assert!(n.pos.y - BALLOON_RADIUS >= 0.0);
                    assert!(n.basket().y + BASKET_RADIUS <= PHONE.height);
                }
            }
        }
    }

    #[test]
    fn balloons_drift_off_and_are_dropped() {
        let mut b = Balloons::new(4);
        run(&mut b, 60.0, true);
        run(&mut b, 20.0, false);
        assert!(b.list.is_empty());
    }

    #[test]
    fn touching_the_envelope_or_the_basket_is_a_hit() {
        let b = Balloon {
            pos: Vec2::new(400.0, 200.0),
            vel: Vec2::new(0.0, 0.0),
            colour: 0,
            way: BalloonWay::AtPlane,
        };
        let r = 16.0;
        let reach = BALLOON_RADIUS + r;
        assert!(b.touches(Vec2::new(400.0 - reach + 1.0, 200.0), r));
        assert!(!b.touches(Vec2::new(400.0 - reach - 1.0, 200.0), r));
        assert!(
            !b.touches(Vec2::new(400.0, 200.0 - reach - 1.0), r),
            "above"
        );
        let basket = b.basket();
        assert!(b.touches(Vec2::new(basket.x, basket.y + BASKET_RADIUS + r - 1.0), r));
        assert!(!b.touches(Vec2::new(basket.x, basket.y + BASKET_RADIUS + r + 1.0), r));
    }

    #[test]
    fn same_seed_same_balloons() {
        let (mut a, mut b) = (Balloons::new(4), Balloons::new(4));
        run(&mut a, 60.0, true);
        run(&mut b, 60.0, true);
        assert_eq!(a, b);
    }
}
