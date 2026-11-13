//! Hot-air balloons: the obstacles of level 1, the easiest level. They are
//! big, slow and rare. Each comes in from the right and drifts left with
//! the landscape, in one of four ways ([`BalloonWay`]): taking off from a
//! launch pad, landing on one, coming in level at the plane's height, or
//! sinking slowly from the plane's height. So half of them come at the
//! plane wherever it flies (even right at the top), the other half take
//! off and land. The ways take turns: every four
//! balloons bring each way once, in a random order, never the same way
//! twice in a row. You see a balloon coming for seconds and can fly over or
//! under it. Fly into a balloon or a launch pad and you crash.
//!
//! Its burner roars once ([`Event::BalloonBurner`]) as it appears, or for
//! a balloon on a pad as it takes off, so you hear it as well as see it.
//! Positions are screen px, like the player's; speeds are for a phone and
//! grow with the screen ([`Bounds::scale`]).
//!
//! Balloons have depth: on a launch pad they stand in the foreground with
//! the huts and are big; the higher they fly, the further away they are
//! and the smaller they get, down to the plane's size
//! ([`Balloon::size`]). Launch pads belong to the foreground landscape
//! like the huts.
//! Seeded, so the same seed gives the same balloons.

use crate::rng::Rng;
use crate::shape::Poly;
use crate::{Bounds, Event, Vec2, SCROLL_SPEED, TICK_SECONDS};

/// The envelope's radius, in px. The basket hangs below it.
pub const BALLOON_RADIUS: f32 = 30.0;
/// The basket's centre, below the envelope's centre, in px.
pub const BASKET_BELOW: f32 = 50.0;
/// Half the basket's height, in px (as drawn, rim included).
pub const BASKET_RADIUS: f32 = 7.0;
/// How far the basket's bottom is below its centre, in px (as drawn).
pub const BASKET_HALF: f32 = 7.0;
/// Time between two balloons, shortest and longest, in seconds.
pub const BALLOON_GAP_SECONDS: (f32, f32) = (3.0, 5.0);
/// No balloon in the first seconds of level 1, while "Level 1" shows.
pub const FIRST_BALLOON_SECONDS: f32 = 2.5;
/// How fast a balloon rises or sinks past, slowest and fastest, in px per
/// second.
pub const BALLOON_RISE: (f32, f32) = (8.0, 25.0);
/// Where a landing balloon comes in (the upper sky), as a fraction of
/// the screen height from the top.
pub const BALLOON_HIGH: (f32, f32) = (0.1, 0.3);
/// How many colour patterns there are (the game crate draws them).
pub const BALLOON_COLOURS: u32 = 4;
/// A balloon the plane flies into explodes and crashes to the ground: how
/// fast it falls (px per second squared) and how much of its drift it
/// keeps.
pub const BALLOON_FALL: f32 = 500.0;
pub const BALLOON_BURST_KEEP: f32 = 0.5;
/// Where the launch pads lie: their bottom, as a fraction of the screen
/// height from the top (in the foreground grass, like the huts).
pub const PAD_GROUND: f32 = crate::huts::HUT_GROUND;
/// A launch pad's width and height, in px.
pub const PAD_WIDTH: f32 = 76.0;
pub const PAD_HEIGHT: f32 = 6.0;
/// Where on the screen a balloon on a pad takes off, as a fraction of the
/// screen width from the left: soon after its pad comes into view.
pub const TAKEOFF_AT: (f32, f32) = (0.6, 0.85);
/// Where on the screen a landing balloon touches down on its pad, as a
/// fraction of the screen width from the left.
pub const TOUCHDOWN_AT: (f32, f32) = (0.3, 0.55);

/// The wind pushes a balloon in the air left faster than the landscape,
/// slowest and fastest, in px per second.
pub const BALLOON_DRIFT: (f32, f32) = (15.0, 40.0);
/// Two balloons stay at least this many envelope radii apart.
const BALLOON_APART: f32 = 4.5;
/// Balloons start and are dropped this far outside the screen, in px.
const MARGIN: f32 = 2.0 * BALLOON_RADIUS; // at full size

/// How a balloon crosses the sky.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BalloonWay {
    /// Stands on a launch pad, takes off and rises.
    Rising,
    /// Level, at the height the plane flew when it appeared: head-on.
    AtPlane,
    /// From the upper sky down onto a launch pad, staying above it.
    Landing,
    /// From the plane's height when it appeared, sinking slowly as it
    /// drifts past.
    Sinking,
}

impl BalloonWay {
    const ALL: [BalloonWay; 4] = [
        BalloonWay::Rising,
        BalloonWay::AtPlane,
        BalloonWay::Landing,
        BalloonWay::Sinking,
    ];

    /// Does it need a launch pad?
    pub fn uses_pad(self) -> bool {
        matches!(self, BalloonWay::Rising | BalloonWay::Landing)
    }
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
    /// While it stands on its pad before take-off: where it takes off
    /// (screen x) and how it flies then. `None` otherwise.
    pub takeoff: Option<(f32, Vec2)>,
    /// While a landing balloon is still in the air: the height (screen y)
    /// of its centre once it stands on the pad. `None` otherwise.
    pub touchdown: Option<f32>,
    /// Ticks since the plane burst it; `None` while it is whole.
    pub burst: Option<u32>,
    /// Its fall, px per second squared (0 while it is whole).
    fall: f32,
}

/// A launch pad in the grass. It slides with the landscape. Flying into
/// it is a crash, like a hut.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pad {
    /// Centre x on screen, in px.
    pub x: f32,
}

impl Pad {
    /// The pad's top on this screen, in px.
    pub fn top(bounds: Bounds) -> f32 {
        PAD_GROUND * bounds.height - PAD_HEIGHT * bounds.scale()
    }

    /// The pad as drawn, for crashing: the deck, then its two legs.
    pub fn shape(&self, bounds: Bounds) -> Vec<Poly> {
        let s = bounds.scale();
        let (w, h) = (PAD_WIDTH * s, PAD_HEIGHT * s);
        let (l, top) = (self.x - w / 2.0, Pad::top(bounds));
        vec![
            Poly::rect(l, top, w, h),
            Poly::rect(l + 4.0 * s, top + h, 3.0 * s, 4.0 * s),
            Poly::rect(l + w - 7.0 * s, top + h, 3.0 * s, 4.0 * s),
        ]
    }
}

impl Balloon {
    /// How big this balloon is right now, as a factor for its sizes in px
    /// (like [`Bounds::size`]): full size ([`Bounds::scale`]) standing on
    /// a pad, shrinking all the way as it rises ([`Bounds::depth`]).
    pub fn size(&self, bounds: Bounds) -> f32 {
        bounds.depth(self.pos.y)
    }

    /// Centre of the basket, in px, at this `size` ([`Balloon::size`]).
    pub fn basket(&self, size: f32) -> Vec2 {
        Vec2::new(self.pos.x, self.pos.y + BASKET_BELOW * size)
    }

    /// The balloon as drawn, for crashing, at `size` ([`Balloon::size`]):
    /// envelope, its taper to the mouth, the mouth, two ropes, the basket
    /// and its rim. The game crate draws these same parts.
    pub fn shape(&self, size: f32) -> Vec<Poly> {
        let r = BALLOON_RADIUS;
        let c = self.pos;
        let at = |points: &[(f32, f32)]| Poly::at(c, size, points);
        let neck = r * 1.15;
        let b = BASKET_BELOW;
        let p = |x: f32, y: f32| Vec2::new(c.x + x * size, c.y + y * size);
        vec![
            Poly::ellipse(c, r * size, r * 1.05 * size),
            at(&[
                (-r * 0.93, r * 0.35),
                (r * 0.93, r * 0.35),
                (0.0, neck + 4.0),
            ]),
            at(&[
                (-8.0, neck),
                (8.0, neck),
                (8.0, neck + 4.0),
                (-8.0, neck + 4.0),
            ]),
            Poly::line(p(-7.0, neck), p(-7.0, b - 6.0), 1.5 * size),
            Poly::line(p(7.0, neck), p(7.0, b - 6.0), 1.5 * size),
            at(&[
                (-8.0, b - 6.0),
                (8.0, b - 6.0),
                (8.0, b + 7.0),
                (-8.0, b + 7.0),
            ]),
            at(&[
                (-9.0, b - 7.0),
                (9.0, b - 7.0),
                (9.0, b - 4.0),
                (-9.0, b - 4.0),
            ]),
        ]
    }

    /// A whole balloon in the air at `pos`, flying at `vel`.
    pub fn new(pos: Vec2, vel: Vec2, colour: u32, way: BalloonWay) -> Self {
        Self {
            pos,
            vel,
            colour,
            way,
            takeoff: None,
            touchdown: None,
            burst: None,
            fall: 0.0,
        }
    }

    /// Burst by the plane: it explodes and crashes to the ground, no
    /// longer in the way. `scale` is the screen's ([`Bounds::scale`]).
    pub fn burst(&mut self, scale: f32) {
        if self.is_burst() {
            return;
        }
        self.burst = Some(0);
        self.takeoff = None;
        self.touchdown = None;
        self.vel = Vec2::new(self.vel.x * BALLOON_BURST_KEEP, 0.0);
        self.fall = BALLOON_FALL * scale;
    }

    /// Has it burst? Then it falls and cannot be hit again.
    pub fn is_burst(&self) -> bool {
        self.burst.is_some()
    }

    /// Seconds since it burst (0 while whole), for drawing the explosion.
    pub fn burst_seconds(&self) -> f32 {
        self.burst.unwrap_or(0) as f32 * TICK_SECONDS
    }

    /// Standing on its launch pad, waiting to take off.
    pub fn on_pad(&self) -> bool {
        self.takeoff.is_some()
    }

    /// Where a balloon standing on a pad (at full size) has its centre.
    pub fn standing_y(bounds: Bounds) -> f32 {
        Pad::top(bounds) - (BASKET_BELOW + BASKET_HALF) * bounds.scale()
    }

    /// Where a balloon coming at the plane (flying at `plane_y`) has its
    /// centre: the plane's height, but the whole balloon on screen. High up
    /// it is small, so it can come as high as the plane can fly; low down
    /// it is kept whole at full size, to be safe.
    pub fn at_plane_y(bounds: Bounds, plane_y: f32) -> f32 {
        let s = bounds.scale();
        let top = BALLOON_RADIUS * bounds.depth(BALLOON_RADIUS * s);
        let lowest = (bounds.height - (BASKET_BELOW + BASKET_RADIUS) * s).max(top);
        plane_y.clamp(top, lowest)
    }

    fn gone(&self, bounds: Bounds) -> bool {
        let size = self.size(bounds);
        let margin = MARGIN * size;
        let below = self.pos.y - BALLOON_RADIUS * size > bounds.height + margin;
        self.pos.x < -margin || self.basket(size).y < -margin || below
    }
}

fn ticks(seconds: f32) -> u32 {
    (seconds / TICK_SECONDS).round().max(1.0) as u32
}

#[derive(Debug, Clone, PartialEq)]
pub struct Balloons {
    /// The balloons on screen right now, oldest first.
    pub list: Vec<Balloon>,
    /// The launch pads on screen, oldest first.
    pub pads: Vec<Pad>,
    /// The ways still to come in this round of four, next one last.
    bag: Vec<BalloonWay>,
    /// The way of the last balloon, so a new round does not repeat it.
    last: Option<BalloonWay>,
    next_ticks: u32,
    rng: Rng,
    /// The game's speed (1 = normal), so landing balloons still touch
    /// down in view when everything slides past faster.
    speed: f32,
}

impl Balloons {
    pub fn new(seed: u64) -> Self {
        Self {
            list: Vec::new(),
            pads: Vec::new(),
            bag: Vec::new(),
            last: None,
            next_ticks: ticks(FIRST_BALLOON_SECONDS),
            speed: 1.0,
            // Its own stream, so balloons do not change the weather or gulls.
            rng: Rng::new(seed ^ 0xBA11_0075_0000_0004),
        }
    }

    /// How fast the game goes (see `World::speed`). The world moves the
    /// balloons and pads along for the extra speed; this only plans new
    /// landings so they still touch down in view.
    pub fn set_speed(&mut self, speed: f32) {
        self.speed = speed.max(0.1);
    }

    /// The way of the next balloon: a new round of all four, shuffled,
    /// whenever the last round is used up.
    fn next_way(&mut self) -> BalloonWay {
        if self.bag.is_empty() {
            let mut round = BalloonWay::ALL.to_vec();
            for i in (1..round.len()).rev() {
                let j = ((self.rng.unit() * (i + 1) as f32) as usize).min(i);
                round.swap(i, j);
            }
            // The bag is used from the end: never start with the last way.
            let end = round.len() - 1;
            if Some(round[end]) == self.last {
                round.swap(0, end);
            }
            self.bag = round;
        }
        *self.bag.last().expect("just filled")
    }

    /// Advance one tick. `active`: is level 1 fully on screen? Balloons in
    /// the sky keep drifting off when it is not. `plane_y`: the player's
    /// height, for balloons that come at it. `huts`: the x of the huts on
    /// screen, so a launch pad never lands on one. Burners go to `events`.
    pub fn tick(
        &mut self,
        bounds: Bounds,
        active: bool,
        plane_y: f32,
        huts: &[f32],
        events: &mut Vec<Event>,
    ) {
        let s = bounds.scale();
        let ground_speed = SCROLL_SPEED * s;
        for b in &mut self.list {
            // Take off from the pad: the burner roars.
            if let Some((at, vel)) = b.takeoff {
                if b.pos.x <= at {
                    b.vel = vel;
                    b.takeoff = None;
                    events.push(Event::BalloonBurner);
                }
            }
            if let Some(t) = &mut b.burst {
                *t += 1;
            }
            b.vel.y += b.fall * TICK_SECONDS;
            b.pos.x += b.vel.x * TICK_SECONDS;
            b.pos.y += b.vel.y * TICK_SECONDS;
            // Touch down on the pad: stand on it and slide off with it.
            if let Some(y) = b.touchdown {
                if b.pos.y >= y {
                    b.pos.y = y;
                    b.vel = Vec2::new(-ground_speed, 0.0);
                    b.touchdown = None;
                }
            }
        }
        self.list.retain(|b| !b.gone(bounds));
        for pad in &mut self.pads {
            pad.x -= ground_speed * TICK_SECONDS;
        }
        self.pads.retain(|p| p.x > -PAD_WIDTH * s);

        if !active {
            self.next_ticks = ticks(FIRST_BALLOON_SECONDS);
            return;
        }
        self.next_ticks = self.next_ticks.saturating_sub(1);
        if self.next_ticks > 0 {
            return;
        }
        let way = self.next_way();
        let x = bounds.width + MARGIN * s;
        // A pad needs free ground: wait a moment if a hut is there.
        let clear = (PAD_WIDTH / 2.0 + crate::huts::HUT_WIDTH.1 / 2.0 + 20.0) * s;
        if way.uses_pad() && huts.iter().any(|&hx| (hx - x).abs() < clear) {
            self.next_ticks = ticks(0.5);
            return;
        }
        self.bag.pop();
        self.last = Some(way);

        let r = &mut self.rng;
        // Never so fast that it catches up with a balloon ahead before
        // that one is gone: they stay apart.
        let keep = BALLOON_APART * BALLOON_RADIUS * s;
        let catch_up = (self.list.iter().filter(|b| !b.is_burst()))
            .map(|b| {
                let ahead_speed = (-b.vel.x).max(ground_speed);
                let seconds = (b.pos.x + MARGIN * s).max(0.0) / ahead_speed;
                ahead_speed - ground_speed + (x - b.pos.x - keep).max(0.0) / seconds.max(0.1)
            })
            .fold(f32::MAX, f32::min);
        let drift = (r.range(BALLOON_DRIFT.0, BALLOON_DRIFT.1) * s).min(catch_up.max(0.0));
        let speed = r.range(BALLOON_RISE.0, BALLOON_RISE.1) * s;
        let colour = (r.unit() * BALLOON_COLOURS as f32) as u32 % BALLOON_COLOURS;
        let h = bounds.height;
        let flying = Vec2::new(-(ground_speed + drift), 0.0);
        let mut balloon = Balloon::new(Vec2::new(x, 0.0), flying, colour, way);
        match way {
            BalloonWay::Rising => {
                // Standing on the pad, sliding in with it until take-off.
                let at = r.range(TAKEOFF_AT.0, TAKEOFF_AT.1) * bounds.width;
                balloon.pos.y = Balloon::standing_y(bounds);
                balloon.vel = Vec2::new(-ground_speed, 0.0);
                balloon.takeoff = Some((at, Vec2::new(flying.x, -speed)));
                self.pads.push(Pad { x });
            }
            BalloonWay::AtPlane => {
                balloon.pos.y = Balloon::at_plane_y(bounds, plane_y);
                events.push(Event::BalloonBurner);
            }
            BalloonWay::Landing => {
                // Right above its pad (no drift), sinking so that it stands
                // on it by the time both reach the touchdown point.
                let start = r.range(BALLOON_HIGH.0, BALLOON_HIGH.1) * h;
                let end = Balloon::standing_y(bounds);
                let at = r.range(TOUCHDOWN_AT.0, TOUCHDOWN_AT.1) * bounds.width;
                let seconds = (x - at) / (ground_speed * self.speed);
                balloon.pos.y = start;
                balloon.vel = Vec2::new(-ground_speed, (end - start).max(0.0) / seconds);
                balloon.touchdown = Some(end);
                self.pads.push(Pad { x });
                events.push(Event::BalloonBurner);
            }
            BalloonWay::Sinking => {
                balloon.pos.y = Balloon::at_plane_y(bounds, plane_y);
                balloon.vel.y = speed;
                events.push(Event::BalloonBurner);
            }
        }
        self.list.push(balloon);
        self.next_ticks = ticks(r.range(BALLOON_GAP_SECONDS.0, BALLOON_GAP_SECONDS.1));
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
            b.tick(PHONE, active, PLANE_Y, &[], &mut all);
        }
        all
    }

    #[test]
    fn no_balloons_outside_level_1() {
        let mut b = Balloons::new(1);
        assert!(run(&mut b, 120.0, false).is_empty());
        assert!(b.list.is_empty() && b.pads.is_empty());
    }

    #[test]
    fn the_first_balloon_waits_for_the_level_banner_and_comes_from_the_right() {
        let mut b = Balloons::new(1);
        run(&mut b, FIRST_BALLOON_SECONDS - 0.1, true);
        assert!(b.list.is_empty());
        run(&mut b, 0.2, true);
        assert_eq!(b.list.len(), 1);
        let first = b.list[0];
        let radius = BALLOON_RADIUS * first.size(PHONE);
        assert!(first.pos.x - radius > PHONE.width, "starts off screen");
        assert!(first.vel.x < 0.0, "drifts left");
    }

    #[test]
    fn every_balloon_roars_once() {
        let mut b = Balloons::new(3);
        let mut balloons = 0;
        let mut burners = 0;
        for _ in 0..ticks(120.0) {
            let before = b.list.len();
            let mut events = Vec::new();
            b.tick(PHONE, true, PLANE_Y, &[], &mut events);
            burners += events.len();
            balloons += b.list.len().saturating_sub(before).min(1);
        }
        // Balloons still standing on a pad have not roared yet.
        let waiting = b.list.iter().filter(|n| n.on_pad()).count();
        assert!(balloons > 0);
        assert!(
            burners + waiting >= balloons && burners <= balloons,
            "{burners} {balloons}"
        );
    }

    #[test]
    fn balloons_are_rare_and_never_bunch_up() {
        let mut b = Balloons::new(3);
        let mut most = 0;
        let mut spawned = 0;
        let mut last_x = f32::MIN;
        for _ in 0..ticks(120.0) {
            b.tick(PHONE, true, PLANE_Y, &[], &mut Vec::new());
            most = most.max(b.list.len());
            if let Some(n) = b.list.last() {
                if n.pos.x > last_x + 1.0 {
                    spawned += 1;
                }
                last_x = n.pos.x;
            }
            for pair in b.list.windows(2) {
                let gap = pair[1].pos.x - pair[0].pos.x;
                assert!(
                    gap > 4.0 * BALLOON_RADIUS * PHONE.scale(),
                    "too close: {gap}"
                );
            }
        }
        // One every 3–5 s over two minutes (the first after 2.5 s).
        assert!((23..=40).contains(&spawned), "{spawned}");
        assert!(most <= 5, "{most} at once");
    }

    #[test]
    fn balloons_take_off_land_or_come_at_the_plane() {
        let mut b = Balloons::new(5);
        let mut seen = Vec::new();
        let size = PHONE.scale();
        for _ in 0..ticks(240.0) {
            b.tick(PHONE, true, PLANE_Y, &[], &mut Vec::new());
            let Some(new) = b
                .list
                .last()
                .filter(|n| n.pos.x == PHONE.width + MARGIN * size)
            else {
                continue;
            };
            seen.push(new.way);
            let y = new.pos.y / PHONE.height;
            match new.way {
                BalloonWay::Rising => {
                    // Standing on its pad, basket on the pad's top.
                    assert!(new.on_pad() && new.vel.y == 0.0);
                    let bottom = new.basket(new.size(PHONE)).y + BASKET_HALF * size;
                    assert!((bottom - Pad::top(PHONE)).abs() < 1e-3);
                    assert!(b.pads.iter().any(|p| p.x == new.pos.x));
                }
                BalloonWay::AtPlane => assert!(new.pos.y == PLANE_Y && new.vel.y == 0.0),
                BalloonWay::Landing => {
                    let (lo, hi) = BALLOON_HIGH;
                    assert!((lo..=hi).contains(&y) && new.vel.y > 0.0);
                    assert!(new.touchdown.is_some());
                    assert!(b.pads.iter().any(|p| p.x == new.pos.x), "its pad");
                }
                BalloonWay::Sinking => {
                    assert!(new.pos.y == PLANE_Y && new.vel.y > 0.0);
                    assert!(new.vel.y <= BALLOON_RISE.1, "slowly");
                }
            }
            // In the air faster than the landscape, but slower than a
            // gull: in view for many seconds.
            // (Slower only so as not to catch up with one ahead.)
            if !new.way.uses_pad() {
                assert!(-new.vel.x >= SCROLL_SPEED, "{:?}", new.vel);
            }
            assert!(
                -new.vel.x <= SCROLL_SPEED + BALLOON_DRIFT.1,
                "{:?}",
                new.vel
            );
            assert!(new.colour < BALLOON_COLOURS);
        }
        for way in BalloonWay::ALL {
            assert!(seen.contains(&way), "no {way:?} in {seen:?}");
        }
    }

    #[test]
    fn the_ways_take_turns() {
        for seed in 0..20 {
            let mut b = Balloons::new(seed);
            let mut ways = Vec::new();
            for _ in 0..ticks(300.0) {
                let before = b.list.last().map(|n| n.pos.x);
                b.tick(PHONE, true, PLANE_Y, &[], &mut Vec::new());
                if let Some(n) = b.list.last() {
                    if before.is_none_or(|x| n.pos.x > x + 1.0) {
                        ways.push(n.way);
                    }
                }
            }
            assert!(ways.len() >= 30, "{}", ways.len());
            // Each round of four has every way once …
            for round in ways.chunks(4).filter(|c| c.len() == 4) {
                for way in BalloonWay::ALL {
                    assert!(round.contains(&way), "seed {seed}: {round:?}");
                }
            }
            // … and no way comes twice in a row, not even across rounds.
            for pair in ways.windows(2) {
                assert_ne!(pair[0], pair[1], "seed {seed}: {ways:?}");
            }
        }
    }

    #[test]
    fn a_landing_balloon_comes_down_onto_its_pad_in_view() {
        let mut b = Balloons::new(5);
        while !b.list.last().is_some_and(|n| n.way == BalloonWay::Landing) {
            b.tick(PHONE, true, PLANE_Y, &[], &mut Vec::new());
        }
        let mine = *b.list.last().unwrap();
        b.list = vec![mine];
        let standing = Balloon::standing_y(PHONE);
        let mut landed_at = None;
        for _ in 0..ticks(20.0) {
            b.tick(PHONE, false, PLANE_Y, &[], &mut Vec::new());
            let Some(n) = b.list.first() else { break };
            // Always right above its pad.
            assert!(b.pads.iter().any(|p| (p.x - n.pos.x).abs() < 1e-3));
            assert!(n.pos.y <= standing + 1e-3, "never below the pad");
            if landed_at.is_none() && n.touchdown.is_none() {
                landed_at = Some(n.pos.x);
                assert_eq!(n.pos.y, standing);
            }
        }
        let x = landed_at.expect("landed");
        let (lo, hi) = TOUCHDOWN_AT;
        let w = PHONE.width;
        assert!(x >= lo * w - 2.0 && x <= hi * w + 2.0, "lands in view: {x}");
    }

    #[test]
    fn balloons_are_big_on_the_pad_and_small_up_high() {
        let at = |y: f32| {
            Balloon::new(
                Vec2::new(400.0, y),
                Vec2::new(0.0, 0.0),
                0,
                BalloonWay::Rising,
            )
        };
        let standing = at(Balloon::standing_y(PHONE)).size(PHONE);
        assert!(
            (standing - PHONE.scale()).abs() < 1e-6,
            "full size on the pad"
        );
        let high = at(0.0).size(PHONE);
        assert!((high - PHONE.size()).abs() < 1e-6, "plane size up high");
        // Shrinking steadily on the way up.
        let mut last = f32::MAX;
        for i in 0..=20 {
            let y = Balloon::standing_y(PHONE) - i as f32 * 10.0;
            let size = at(y).size(PHONE);
            assert!(size <= last + 1e-6);
            last = size;
        }
        // On a bigger screen everything is bigger, in proportion.
        let computer = Bounds {
            width: 1440.0,
            height: 900.0,
        };
        let big = at(Balloon::standing_y(computer)).size(computer);
        assert!((big - computer.scale()).abs() < 1e-6);
    }

    /// A tiny 2×2 px probe at (x, y).
    fn probe(x: f32, y: f32) -> Vec<Poly> {
        vec![Poly::rect(x - 1.0, y - 1.0, 2.0, 2.0)]
    }

    #[test]
    fn only_touching_the_pad_itself_is_a_hit() {
        let pad = Pad { x: 400.0 };
        let shape = pad.shape(PHONE);
        let top = Pad::top(PHONE);
        let hit = |x: f32, y: f32| crate::shape::any_overlap(&shape, &probe(x, y));
        assert!(hit(400.0, top + 2.0), "the deck");
        assert!(!hit(400.0, top - 2.0), "just above it");
        let half = PAD_WIDTH * PHONE.scale() / 2.0;
        assert!(!hit(400.0 + half + 2.0, top + 2.0), "just beside it");
        assert!(
            !hit(400.0, top + PAD_HEIGHT + 3.0),
            "under the deck, between the legs"
        );
    }

    #[test]
    fn a_balloon_on_a_pad_slides_in_with_it_then_takes_off_with_a_roar() {
        let mut b = Balloons::new(5);
        while !b.list.last().is_some_and(|n| n.on_pad()) {
            b.tick(PHONE, true, PLANE_Y, &[], &mut Vec::new());
        }
        // Only this balloon from now on (inactive: no new ones come).
        let mine = *b.list.last().unwrap();
        b.list = vec![mine];
        let (at, _) = mine.takeoff.unwrap();
        assert!(
            at > 0.5 * PHONE.width && at < PHONE.width,
            "takes off in view"
        );
        loop {
            let mut events = Vec::new();
            b.tick(PHONE, false, PLANE_Y, &[], &mut events);
            let n = b.list[0];
            if n.on_pad() {
                assert_eq!(n.pos.y, mine.pos.y, "waits on the pad");
                let pad = b.pads.iter().any(|p| (p.x - n.pos.x).abs() < 1e-3);
                assert!(pad, "moves with its pad");
            } else {
                assert!(events.contains(&Event::BalloonBurner), "roars at take-off");
                assert!(n.vel.y < 0.0, "rises");
                break;
            }
        }
    }

    #[test]
    fn a_pad_never_lands_on_a_hut() {
        let mut b = Balloons::new(5);
        // A hut right where every new balloon would come in.
        let hut = PHONE.width + MARGIN * PHONE.scale();
        for _ in 0..ticks(120.0) {
            b.tick(PHONE, true, PLANE_Y, &[hut], &mut Vec::new());
        }
        assert!(b.pads.is_empty(), "no pad on the hut");
        // Once the hut has slid on, the pad comes.
        run(&mut b, 1.0, true);
        assert!(!b.pads.is_empty());
    }

    #[test]
    fn flying_right_at_the_top_is_no_hiding_place() {
        // As high as the plane can fly (see `World::tick`).
        let plane_y = crate::PLAYER_HALF_HEIGHT * PHONE.plane_depth(0.0);
        let mut b = Balloons::new(5);
        let mut came = 0;
        for _ in 0..ticks(120.0) {
            b.tick(PHONE, true, plane_y, &[], &mut Vec::new());
            let Some(new) = b
                .list
                .last()
                .filter(|n| n.pos.x == PHONE.width + MARGIN * PHONE.scale())
            else {
                continue;
            };
            if !new.way.uses_pad() {
                came += 1;
                let size = new.size(PHONE);
                let (top, bottom) = (new.pos.y - BALLOON_RADIUS * size, new.basket(size).y);
                assert!(
                    (top..=bottom).contains(&plane_y),
                    "{plane_y} {top} {bottom}"
                );
            }
        }
        assert!(came >= 6, "half of them come at the plane: {came}");
    }

    #[test]
    fn a_balloon_at_the_plane_stays_whole_on_screen() {
        for plane_y in [0.0, PHONE.height] {
            let mut b = Balloons::new(5);
            for _ in 0..ticks(240.0) {
                b.tick(PHONE, true, plane_y, &[], &mut Vec::new());
                // Level ones all the way, sinking ones as they come in.
                let start = PHONE.width + MARGIN * PHONE.scale();
                let at_plane = |n: &&Balloon| {
                    n.way == BalloonWay::AtPlane
                        || (n.way == BalloonWay::Sinking && n.pos.x == start)
                };
                for n in b.list.iter().filter(at_plane) {
                    let size = n.size(PHONE);
                    assert!(n.pos.y - BALLOON_RADIUS * size >= 0.0);
                    assert!(n.basket(size).y + BASKET_RADIUS * size <= PHONE.height);
                }
            }
        }
    }

    #[test]
    fn a_burst_balloon_crashes_to_the_ground_and_is_dropped() {
        let mut b = Balloons::new(4);
        while !b.list.iter().any(|b| b.way == BalloonWay::Landing) {
            run(&mut b, 0.5, true);
        }
        let i = (b.list.iter())
            .position(|b| b.way == BalloonWay::Landing)
            .unwrap();
        assert!(!b.list[i].is_burst());
        b.list[i].burst(PHONE.scale());
        assert!(b.list[i].is_burst());
        assert_eq!(b.list[i].touchdown, None, "it no longer lands on its pad");
        let mut y = b.list[i].pos.y;
        let mut falling = 0;
        for _ in 0..ticks(0.8) {
            run(&mut b, TICK_SECONDS, false);
            if b.list[i].pos.y > y {
                falling += 1;
            }
            y = b.list[i].pos.y;
        }
        assert!(falling > ticks(0.6), "it falls");
        assert!(b.list[i].burst_seconds() > 0.7);
        run(&mut b, 4.0, false);
        assert!(b.list.iter().all(|b| !b.is_burst()), "down below: gone");
    }

    #[test]
    fn balloons_and_pads_drift_off_and_are_dropped() {
        let mut b = Balloons::new(4);
        run(&mut b, 60.0, true);
        run(&mut b, 20.0, false);
        assert!(b.list.is_empty() && b.pads.is_empty());
    }

    #[test]
    fn only_touching_the_balloon_itself_is_a_hit() {
        let b = Balloon::new(
            Vec2::new(400.0, 200.0),
            Vec2::new(0.0, 0.0),
            0,
            BalloonWay::AtPlane,
        );
        let shape = b.shape(1.0);
        let hit = |x: f32, y: f32| crate::shape::any_overlap(&shape, &probe(x, y));
        let r = BALLOON_RADIUS;
        // Envelope: inside at its sides and top, free just outside.
        assert!(hit(400.0 - r + 2.0, 200.0));
        assert!(!hit(400.0 - r - 2.0, 200.0));
        assert!(hit(400.0, 200.0 - r * 1.05 + 2.0));
        assert!(!hit(400.0, 200.0 - r * 1.05 - 2.0));
        // Basket: hit; just below it and beside it: free.
        let basket = b.basket(1.0);
        assert!(hit(basket.x, basket.y));
        assert!(!hit(basket.x, basket.y + BASKET_RADIUS + 2.0));
        assert!(!hit(basket.x + 12.0, basket.y));
        // Between the ropes, under the mouth: free air.
        // (The mouth ends at 1.15 r + 4 = 38.5 px, the rim starts at 43.)
        assert!(!hit(400.0 - 3.0, 200.0 + 40.75));
        // Beside the taper, next to the bottom of the envelope: free.
        assert!(!hit(400.0 - r + 2.0, 200.0 + r * 0.9));
    }

    #[test]
    fn same_seed_same_balloons() {
        let (mut a, mut b) = (Balloons::new(4), Balloons::new(4));
        run(&mut a, 60.0, true);
        run(&mut b, 60.0, true);
        assert_eq!(a, b);
    }
}
