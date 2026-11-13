//! The town at night (level 3): high-rises and tower cranes. They push
//! the plane two ways, so neither the top nor the bottom of the screen is
//! safe:
//!
//! * A **high-rise** stands at the front, on the ground line of the huts.
//!   Fly over it (its roof, the stair house and the antenna count).
//! * A **tower crane** stands far back in the town: its mast is part of
//!   the scenery and cannot be hit. Its long jib runs along the top of the
//!   screen, and from it a load (a steel beam) hangs on a cable. Jib,
//!   counterweight, cable and load count: fly under the load. Flying
//!   right at the top hits the jib.
//!
//! * Some high-rises have a **helicopter** on the roof. It either takes
//!   off, rising and flying away while the high-rise is still far ahead,
//!   or comes down from the sky and lands there in good time. Near the
//!   plane it is always parked on the roof (part of the high-rise: fly
//!   over it) or already gone.
//!
//! They come one at a time with room between them, so you can change from
//! "over" to "under" ([`CITY_GAP_SECONDS`]); never two cranes in a row.
//! Positions are screen px; sizes are for a phone and grow with the
//! screen ([`Bounds::scale`]). Seeded, so the same seed gives the same
//! town.

use crate::rng::Rng;
use crate::shape::Poly;
use crate::{Bounds, Vec2, SCROLL_SPEED, TICK_SECONDS};

/// The level with the town.
pub const CITY_LEVEL: u32 = 3;
/// Time between two of them (high-rise or crane), shortest and longest,
/// in seconds.
pub const CITY_GAP_SECONDS: (f32, f32) = (3.0, 5.0);
/// None in the first seconds of the town.
pub const FIRST_CITY_SECONDS: f32 = 3.0;
/// The least free ground between two of them, in px: room to change
/// from flying over to flying under.
pub const CITY_ROOM: f32 = 140.0;
/// How likely the next one is a crane (never two in a row).
pub const CRANE_SHARE: f32 = 0.4;

/// Where high-rises stand: their bottom, as a fraction of the screen
/// height from the top (the huts' ground line).
pub const HIGH_RISE_GROUND: f32 = crate::huts::HUT_GROUND;
/// A high-rise's width, narrowest and widest, in px.
pub const HIGH_RISE_WIDTH: (f32, f32) = (64.0, 96.0);
/// How tall a high-rise's walls are, as a fraction of the screen height.
pub const HIGH_RISE_HEIGHT: (f32, f32) = (0.32, 0.58);
/// The stair house on the roof (fraction of the width, px high) and the
/// antenna on it (px high, px thick).
pub const STAIR_HOUSE: (f32, f32) = (0.3, 10.0);
pub const ANTENNA: (f32, f32) = (18.0, 2.0);
/// How many looks (window patterns) there are; the game draws them.
pub const HIGH_RISE_STYLES: u32 = 3;

/// How many high-rises have a helicopter.
pub const HELI_SHARE: f32 = 0.35;
/// A helicopter's body (half length, half height), tail boom and rotor
/// (half span), in px.
pub const HELI_BODY: (f32, f32) = (14.0, 7.0);
pub const HELI_TAIL: f32 = 22.0;
pub const HELI_ROTOR: f32 = 24.0;
/// A taking-off helicopter starts its rotor once its high-rise is this
/// far across the screen (fraction of the width from the left), and lifts
/// off after [`HELI_SPIN_UP_SECONDS`].
pub const HELI_TAKEOFF_AT: (f32, f32) = (0.8, 0.95);
pub const HELI_SPIN_UP_SECONDS: f32 = 0.8;
/// How fast it climbs away, up and back (right, away from the plane),
/// in px per second relative to its high-rise.
pub const HELI_CLIMB: (f32, f32) = (110.0, 120.0);
/// Where a landing helicopter touches down: its high-rise this far across
/// the screen (fraction of the width from the left).
pub const HELI_TOUCHDOWN_AT: (f32, f32) = (0.55, 0.7);

/// The jib's top, as a fraction of the screen height, and its height in px.
pub const JIB_TOP: f32 = 0.03;
pub const JIB_HEIGHT: f32 = 10.0;
/// How far the jib reaches from the mast toward the plane (left), in px.
pub const JIB_REACH: (f32, f32) = (150.0, 220.0);
/// The counter-jib behind the mast (right), in px, and the counterweight
/// hanging under its end (width, height in px).
pub const COUNTER_JIB: f32 = 70.0;
pub const COUNTERWEIGHT: (f32, f32) = (26.0, 20.0);
/// The cable hangs this far in from the jib's tip, in px; its thickness.
pub const CABLE_IN: f32 = 16.0;
pub const CABLE_WIDTH: f32 = 2.5;
/// The load (a steel beam): width in px, its height in px, and where its
/// centre hangs, as a fraction of the screen height.
pub const LOAD_WIDTH: (f32, f32) = (70.0, 110.0);
pub const LOAD_HEIGHT: f32 = 12.0;
pub const LOAD_AT: (f32, f32) = (0.32, 0.55);
/// Where the mast's foot is drawn: far back in the town, as a fraction of
/// the screen height (only drawn, never hit).
pub const MAST_FOOT: f32 = 0.66;

/// What a helicopter on a high-rise does.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HeliState {
    /// Parked on the roof, rotor still; takes off once its high-rise is
    /// at `takeoff_x` (screen px).
    Waiting { takeoff_x: f32 },
    /// Rotor spinning up on the roof for this many more ticks.
    SpinningUp(u32),
    /// Climbing away; gone once off the screen.
    Climbing,
    /// Coming down onto the roof at this many px per second.
    Landing { sink: f32 },
    /// Landed: parked for good, its rotor running down for this many
    /// more ticks.
    Landed(u32),
}

/// A helicopter on (or above) a high-rise. Its place is kept relative to
/// the roof, so it slides along with its high-rise.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Heli {
    /// The body's centre, relative to the roof's middle-left pad point
    /// ([`HighRise::pad`]), in px.
    pub off: Vec2,
    pub state: HeliState,
    /// Ticks it has existed, for the rotor's turning.
    pub age: u32,
}

impl Heli {
    /// Is its rotor turning?
    pub fn rotor_on(&self) -> bool {
        !matches!(self.state, HeliState::Waiting { .. } | HeliState::Landed(0))
    }

    /// Off the roof, in the air?
    pub fn flying(&self) -> bool {
        matches!(self.state, HeliState::Climbing | HeliState::Landing { .. })
    }
}

/// A high-rise at the front.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HighRise {
    /// Centre x on screen, in px.
    pub x: f32,
    /// Width in px at [`Bounds::scale`] 1.
    pub width: f32,
    /// Wall height as a fraction of the screen height.
    pub height: f32,
    /// Which window pattern, 0 .. [`HIGH_RISE_STYLES`].
    pub style: u32,
    /// Its helicopter, if it has one (and it has not flown away yet).
    pub heli: Option<Heli>,
}

/// Where a high-rise's parts are on screen, in px: what you see is what
/// you hit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HighRiseParts {
    /// The walls: left, top, width, height.
    pub body: (f32, f32, f32, f32),
    /// The stair house on the roof: left, top, width, height.
    pub stair_house: (f32, f32, f32, f32),
    /// The antenna, from the stair house's roof up, and its thickness;
    /// the game puts a red light on its tip.
    pub antenna: (Vec2, Vec2),
    pub antenna_width: f32,
}

impl HighRise {
    /// Where each part is on this screen.
    pub fn parts(&self, bounds: Bounds) -> HighRiseParts {
        let s = bounds.scale();
        let ground = HIGH_RISE_GROUND * bounds.height;
        let (w, h) = (self.width * s, self.height * bounds.height);
        let top = ground - h;
        let (sw, sh) = (STAIR_HOUSE.0 * w, STAIR_HOUSE.1 * s);
        // The stair house sits off centre, toward the back (right).
        let sl = self.x + 0.1 * w;
        let foot = Vec2::new(sl + sw / 2.0, top - sh);
        HighRiseParts {
            body: (self.x - w / 2.0, top, w, h),
            stair_house: (sl, top - sh, sw, sh),
            antenna: (foot, Vec2::new(foot.x, foot.y - ANTENNA.0 * s)),
            antenna_width: ANTENNA.1 * s,
        }
    }

    /// Its highest point (the antenna's tip, or a parked helicopter's
    /// rotor), in px from the top.
    pub fn top(&self, bounds: Bounds) -> f32 {
        let antenna = self.parts(bounds).antenna.1.y;
        match self.heli_parts(bounds) {
            Some(h) if !self.heli.is_some_and(|h| h.flying()) => antenna.min(h.rotor.0.y),
            _ => antenna,
        }
    }

    /// The helipad's middle on the roof (the left half; the stair house
    /// is on the right), in px.
    pub fn pad(&self, bounds: Bounds) -> Vec2 {
        let (l, t, w, _) = self.parts(bounds).body;
        Vec2::new(l + 0.3 * w, t)
    }

    /// Where its helicopter's parts are on screen, if it has one.
    pub fn heli_parts(&self, bounds: Bounds) -> Option<HeliParts> {
        let heli = self.heli?;
        let s = bounds.scale();
        let pad = self.pad(bounds);
        let c = Vec2::new(pad.x + heli.off.x, pad.y + heli.off.y);
        let (hl, hh) = (HELI_BODY.0 * s, HELI_BODY.1 * s);
        let mast = c.y - hh - 3.0 * s;
        Some(HeliParts {
            body: (c, hl, hh),
            // The tail points back (right), away from the plane.
            tail: (
                Vec2::new(c.x + hl * 0.8, c.y - hh * 0.2),
                Vec2::new(c.x + hl + HELI_TAIL * s, c.y - hh * 0.6),
            ),
            tail_width: 3.0 * s,
            rotor: (
                Vec2::new(c.x - HELI_ROTOR * s, mast),
                Vec2::new(c.x + HELI_ROTOR * s, mast),
            ),
            skids: (
                Vec2::new(c.x - hl, c.y + hh + 2.0 * s),
                Vec2::new(c.x + hl, c.y + hh + 2.0 * s),
            ),
        })
    }

    /// The high-rise as drawn, for crashing, with its helicopter.
    pub fn shape(&self, bounds: Bounds) -> Vec<Poly> {
        let p = self.parts(bounds);
        let rect = |(l, t, w, h): (f32, f32, f32, f32)| Poly::rect(l, t, w, h);
        let mut shape = vec![
            rect(p.body),
            rect(p.stair_house),
            Poly::line(p.antenna.0, p.antenna.1, p.antenna_width),
        ];
        if let Some(h) = self.heli_parts(bounds) {
            let (c, hl, hh) = h.body;
            shape.push(Poly::ellipse(c, hl, hh));
            shape.push(Poly::line(h.tail.0, h.tail.1, h.tail_width));
            shape.push(Poly::line(h.rotor.0, h.rotor.1, 2.0 * bounds.scale()));
            shape.push(Poly::line(h.skids.0, h.skids.1, 2.0 * bounds.scale()));
        }
        shape
    }

    /// Move its helicopter on by one tick: take off, climb away, come
    /// down, land. `speed` is the game's (see `World::speed`).
    fn tick_heli(&mut self, bounds: Bounds) {
        let s = bounds.scale();
        let parked_y = -(HELI_BODY.1 + 2.0 + 1.0) * s; // skids on the roof
        let Some(h) = &mut self.heli else { return };
        h.age += 1;
        match h.state {
            HeliState::Waiting { takeoff_x } => {
                if self.x <= takeoff_x {
                    h.state = HeliState::SpinningUp(ticks(HELI_SPIN_UP_SECONDS));
                }
            }
            HeliState::SpinningUp(0) => h.state = HeliState::Climbing,
            HeliState::SpinningUp(n) => h.state = HeliState::SpinningUp(n - 1),
            HeliState::Climbing => {
                h.off.x += HELI_CLIMB.0 * s * TICK_SECONDS;
                h.off.y -= HELI_CLIMB.1 * s * TICK_SECONDS;
            }
            HeliState::Landing { sink } => {
                h.off.y = (h.off.y + sink * TICK_SECONDS).min(parked_y);
                if h.off.y >= parked_y {
                    h.state = HeliState::Landed(ticks(HELI_SPIN_UP_SECONDS));
                }
            }
            HeliState::Landed(n) => h.state = HeliState::Landed(n.saturating_sub(1)),
        }
        // Climbed off the screen: gone.
        let pad = self.pad(bounds);
        if self.heli.is_some_and(|h| {
            h.state == HeliState::Climbing && pad.y + h.off.y < -(HELI_ROTOR + 10.0) * s
        }) {
            self.heli = None;
        }
    }

    fn right(&self, bounds: Bounds) -> f32 {
        self.x + self.width * bounds.scale() / 2.0
    }
}

/// Where a helicopter's parts are on screen, in px.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeliParts {
    /// The body: centre, half length, half height (an ellipse).
    pub body: (Vec2, f32, f32),
    /// The tail boom, back to the right, and its thickness.
    pub tail: (Vec2, Vec2),
    pub tail_width: f32,
    /// The main rotor (as a line at full span) above the body.
    pub rotor: (Vec2, Vec2),
    /// The skids under the body.
    pub skids: (Vec2, Vec2),
}

/// A tower crane far back in the town, its jib and load up front.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Crane {
    /// The mast's x on screen, in px.
    pub x: f32,
    /// How far the jib reaches left of the mast, in px at scale 1.
    pub reach: f32,
    /// The load's width in px at scale 1.
    pub load_width: f32,
    /// Where the load's centre hangs, as a fraction of the screen height.
    pub load_at: f32,
}

/// Where a crane's parts are on screen, in px.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CraneParts {
    /// The jib with the counter-jib: left, top, width, height.
    pub jib: (f32, f32, f32, f32),
    /// The counterweight under the counter-jib's end.
    pub counterweight: (f32, f32, f32, f32),
    /// The cable from the jib down to the load, and its thickness.
    pub cable: (Vec2, Vec2),
    pub cable_width: f32,
    /// The load: left, top, width, height.
    pub load: (f32, f32, f32, f32),
    /// The mast, from the jib down to its foot far back (drawn only), and
    /// its width.
    pub mast: (Vec2, Vec2),
    pub mast_width: f32,
}

impl Crane {
    /// Where each part is on this screen.
    pub fn parts(&self, bounds: Bounds) -> CraneParts {
        let s = bounds.scale();
        let h = bounds.height;
        let jib_top = JIB_TOP * h;
        let jib_h = JIB_HEIGHT * s;
        let tip = self.x - self.reach * s;
        let back = self.x + COUNTER_JIB * s;
        let (cw, ch) = (COUNTERWEIGHT.0 * s, COUNTERWEIGHT.1 * s);
        let cable_x = tip + CABLE_IN * s;
        let (lw, lh) = (self.load_width * s, LOAD_HEIGHT * s);
        let load_top = self.load_at * h - lh / 2.0;
        CraneParts {
            jib: (tip, jib_top, back - tip, jib_h),
            counterweight: (back - cw, jib_top + jib_h, cw, ch),
            cable: (
                Vec2::new(cable_x, jib_top + jib_h),
                Vec2::new(cable_x, load_top),
            ),
            cable_width: CABLE_WIDTH * s,
            load: (cable_x - lw / 2.0, load_top, lw, lh),
            mast: (
                Vec2::new(self.x, jib_top + jib_h),
                Vec2::new(self.x, MAST_FOOT * h),
            ),
            mast_width: 8.0 * s,
        }
    }

    /// The bottom of the load, in px from the top: fly below it.
    pub fn load_bottom(&self, bounds: Bounds) -> f32 {
        let (_, top, _, height) = self.parts(bounds).load;
        top + height
    }

    /// The crane as drawn, for crashing: jib, counterweight, cable and
    /// load. The mast stands far back in the town and is not hit.
    pub fn shape(&self, bounds: Bounds) -> Vec<Poly> {
        let p = self.parts(bounds);
        let rect = |(l, t, w, h): (f32, f32, f32, f32)| Poly::rect(l, t, w, h);
        vec![
            rect(p.jib),
            rect(p.counterweight),
            Poly::line(p.cable.0, p.cable.1, p.cable_width),
            rect(p.load),
        ]
    }

    /// Its leftmost and rightmost x on screen.
    fn span(&self, bounds: Bounds) -> (f32, f32) {
        let p = self.parts(bounds);
        let (jl, _, jw, _) = p.jib;
        let (ll, _, _, _) = p.load;
        (jl.min(ll), jl + jw)
    }
}

fn ticks(seconds: f32) -> u32 {
    (seconds / TICK_SECONDS).round().max(1.0) as u32
}

#[derive(Debug, Clone, PartialEq)]
pub struct City {
    /// The high-rises on screen, oldest (leftmost) first.
    pub high_rises: Vec<HighRise>,
    /// The cranes on screen, oldest first.
    pub cranes: Vec<Crane>,
    /// Was the last one a crane? (Never two in a row.)
    last_crane: bool,
    next_ticks: u32,
    rng: Rng,
    /// The game's speed (1 = normal), see [`City::set_speed`].
    speed: f32,
}

impl City {
    pub fn new(seed: u64) -> Self {
        Self {
            high_rises: Vec::new(),
            cranes: Vec::new(),
            last_crane: false,
            next_ticks: ticks(FIRST_CITY_SECONDS),
            speed: 1.0,
            // Its own stream, so the town does not change other obstacles.
            rng: Rng::new(seed ^ 0xC17E_0000_0000_0003),
        }
    }

    /// Clear the town away (a new level).
    pub fn clear(&mut self) {
        self.high_rises.clear();
        self.cranes.clear();
    }

    /// Everything slides further left by `dx` px (the game going faster).
    pub fn shift(&mut self, dx: f32) {
        self.high_rises.iter_mut().for_each(|b| b.x += dx);
        self.cranes.iter_mut().for_each(|c| c.x += dx);
    }

    /// How fast the game goes (see `World::speed`), so landing
    /// helicopters still touch down in good time when it is faster.
    pub fn set_speed(&mut self, speed: f32) {
        self.speed = speed.max(0.1);
    }

    /// Advance one tick. `active`: is the town fully on screen? Whatever
    /// is there keeps sliding off when it is not.
    pub fn tick(&mut self, bounds: Bounds, active: bool) {
        let s = bounds.scale();
        let dx = -SCROLL_SPEED * s * TICK_SECONDS;
        self.shift(dx);
        for b in &mut self.high_rises {
            b.tick_heli(bounds);
        }
        let margin = 20.0 * s;
        self.high_rises.retain(|b| b.right(bounds) > -margin);
        self.cranes.retain(|c| c.span(bounds).1 > -margin);

        if !active {
            self.next_ticks = ticks(FIRST_CITY_SECONDS);
            return;
        }
        self.next_ticks = self.next_ticks.saturating_sub(1);
        if self.next_ticks > 0 {
            return;
        }
        // Comes in from just off the right edge, once the last one (a
        // crane's counter-jib reaches far right) has slid far enough in.
        let enter = bounds.width + margin;
        let last_right = (self.high_rises.iter().map(|b| b.right(bounds)))
            .chain(self.cranes.iter().map(|c| c.span(bounds).1))
            .fold(f32::MIN, f32::max);
        if last_right > enter - CITY_ROOM * s {
            self.next_ticks = ticks(0.2);
            return;
        }
        let r = &mut self.rng;
        let crane = !self.last_crane && r.unit() < CRANE_SHARE;
        if crane {
            let reach = r.range(JIB_REACH.0, JIB_REACH.1);
            let load_width = r.range(LOAD_WIDTH.0, LOAD_WIDTH.1);
            let left = (reach - CABLE_IN).max(0.0) + load_width / 2.0;
            self.cranes.push(Crane {
                x: enter + left * s,
                reach,
                load_width,
                load_at: r.range(LOAD_AT.0, LOAD_AT.1),
            });
        } else {
            let width = r.range(HIGH_RISE_WIDTH.0, HIGH_RISE_WIDTH.1);
            let mut b = HighRise {
                x: enter + width * s / 2.0,
                width,
                height: r.range(HIGH_RISE_HEIGHT.0, HIGH_RISE_HEIGHT.1),
                style: (r.unit() * HIGH_RISE_STYLES as f32) as u32 % HIGH_RISE_STYLES,
                heli: None,
            };
            if r.unit() < HELI_SHARE {
                let parked = Vec2::new(0.0, -(HELI_BODY.1 + 3.0) * s);
                let w = bounds.width;
                b.heli = Some(if r.unit() < 0.5 {
                    let at = r.range(HELI_TAKEOFF_AT.0, HELI_TAKEOFF_AT.1) * w;
                    Heli {
                        off: parked,
                        state: HeliState::Waiting { takeoff_x: at },
                        age: 0,
                    }
                } else {
                    // From above the screen down onto the roof, landing by
                    // the time its high-rise is at the touchdown point.
                    let at = r.range(HELI_TOUCHDOWN_AT.0, HELI_TOUCHDOWN_AT.1) * w;
                    let seconds = (b.x - at) / (SCROLL_SPEED * s * self.speed);
                    let roof = b.pad(bounds).y;
                    let start = -(roof + (HELI_ROTOR + 4.0) * s);
                    Heli {
                        off: Vec2::new(0.0, start),
                        state: HeliState::Landing {
                            sink: (parked.y - start) / seconds.max(0.5),
                        },
                        age: 0,
                    }
                });
            }
            self.high_rises.push(b);
        }
        self.last_crane = crane;
        self.next_ticks = ticks(r.range(CITY_GAP_SECONDS.0, CITY_GAP_SECONDS.1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shape::any_overlap;

    const PHONE: Bounds = Bounds {
        width: 844.0,
        height: 390.0,
    };

    fn run(c: &mut City, seconds: f32, active: bool) {
        for _ in 0..ticks(seconds) {
            c.tick(PHONE, active);
        }
    }

    fn probe(x: f32, y: f32) -> Vec<Poly> {
        vec![Poly::rect(x - 1.0, y - 1.0, 2.0, 2.0)]
    }

    #[test]
    fn nothing_outside_the_town() {
        let mut c = City::new(1);
        run(&mut c, 120.0, false);
        assert!(c.high_rises.is_empty() && c.cranes.is_empty());
    }

    #[test]
    fn the_first_comes_after_a_few_seconds_from_the_right() {
        let mut c = City::new(1);
        run(&mut c, FIRST_CITY_SECONDS - 0.1, true);
        assert!(c.high_rises.is_empty() && c.cranes.is_empty());
        run(&mut c, 0.2, true);
        let lefts: Vec<f32> = (c.high_rises.iter())
            .map(|b| b.parts(PHONE).body.0)
            .chain(c.cranes.iter().map(|k| k.span(PHONE).0))
            .collect();
        assert_eq!(lefts.len(), 1);
        assert!(lefts[0] > PHONE.width, "starts off screen: {lefts:?}");
    }

    /// Every new one, in order: true = crane.
    fn arrivals(seed: u64, seconds: f32) -> Vec<bool> {
        let mut c = City::new(seed);
        let mut kinds = Vec::new();
        let (mut b, mut k) = (0, 0);
        for _ in 0..ticks(seconds) {
            c.tick(PHONE, true);
            let fresh_b = c.high_rises.iter().filter(|h| h.x > PHONE.width).count();
            let fresh_k = c.cranes.iter().filter(|h| h.x > PHONE.width).count();
            if fresh_b > b {
                kinds.push(false);
            }
            if fresh_k > k {
                kinds.push(true);
            }
            (b, k) = (fresh_b, fresh_k);
        }
        kinds
    }

    #[test]
    fn they_come_one_at_a_time_and_never_two_cranes_in_a_row() {
        for seed in 0..10 {
            let kinds = arrivals(seed, 300.0);
            // One every 3–5 s over five minutes.
            assert!((55..=100).contains(&kinds.len()), "{}", kinds.len());
            let cranes = kinds.iter().filter(|&&k| k).count();
            assert!(cranes * 5 > kinds.len(), "enough cranes: {cranes}");
            for pair in kinds.windows(2) {
                assert!(!(pair[0] && pair[1]), "seed {seed}: two cranes");
            }
        }
    }

    #[test]
    fn they_never_overlap_and_leave_room_to_change_course() {
        let mut c = City::new(7);
        for _ in 0..ticks(300.0) {
            c.tick(PHONE, true);
            let mut spans: Vec<(f32, f32)> = (c.high_rises.iter())
                .map(|b| {
                    let (l, _, w, _) = b.parts(PHONE).body;
                    (l, l + w)
                })
                .chain(c.cranes.iter().map(|k| k.span(PHONE)))
                .collect();
            spans.sort_by(|a, b| a.0.total_cmp(&b.0));
            for pair in spans.windows(2) {
                let gap = pair[1].0 - pair[0].1;
                assert!(gap >= CITY_ROOM - 1.0, "too close: {gap}");
            }
        }
    }

    #[test]
    fn high_rises_leave_the_sky_free_and_cranes_the_ground() {
        let mut c = City::new(3);
        for _ in 0..ticks(300.0) {
            c.tick(PHONE, true);
            for b in &c.high_rises {
                assert!(b.top(PHONE) > 0.25 * PHONE.height, "room above");
            }
            for k in &c.cranes {
                let below = HIGH_RISE_GROUND * PHONE.height - k.load_bottom(PHONE);
                assert!(below > 0.35 * PHONE.height, "room below: {below}");
            }
        }
    }

    #[test]
    fn only_touching_a_high_rise_itself_is_a_hit() {
        let b = HighRise {
            x: 400.0,
            width: 80.0,
            height: 0.4,
            style: 0,
            heli: None,
        };
        let shape = b.shape(PHONE);
        let hit = |x: f32, y: f32| any_overlap(&shape, &probe(x, y));
        let p = b.parts(PHONE);
        let (l, top, w, _) = p.body;
        assert!(hit(l + 3.0, top + 3.0), "wall");
        assert!(!hit(l + 3.0, top - 3.0), "above the roof");
        assert!(!hit(l - 3.0, top + 20.0), "beside it");
        let tip = p.antenna.1;
        assert!(hit(tip.x, tip.y + 1.5), "the antenna");
        assert!(!hit(tip.x, tip.y - 3.0), "above the antenna");
        assert!(!hit(l + w - 2.0 + 6.0, top - 2.0), "beside the stair house");
    }

    #[test]
    fn only_jib_cable_and_load_of_a_crane_are_hit_not_its_mast() {
        let k = Crane {
            x: 600.0,
            reach: 180.0,
            load_width: 90.0,
            load_at: 0.45,
        };
        let shape = k.shape(PHONE);
        let hit = |x: f32, y: f32| any_overlap(&shape, &probe(x, y));
        let p = k.parts(PHONE);
        let (jl, jt, jw, jh) = p.jib;
        assert!(hit(jl + jw / 2.0, jt + jh / 2.0), "the jib");
        assert!(hit(p.cable.0.x, (p.cable.0.y + p.cable.1.y) / 2.0), "cable");
        let (ll, lt, lw, lh) = p.load;
        assert!(hit(ll + lw / 2.0, lt + lh / 2.0), "load");
        assert!(!hit(ll + lw / 2.0, lt + lh + 3.0), "just under the load");
        assert!(!hit(p.cable.0.x + 8.0, lt - 20.0), "beside the cable");
        let mid = (p.mast.0.y + p.mast.1.y) / 2.0;
        assert!(!hit(p.mast.0.x, mid), "the mast is far back");
    }

    #[test]
    fn helicopters_take_off_and_land_far_ahead_of_the_plane() {
        let plane_x = crate::PLAYER_X_FRACTION * PHONE.width;
        let (mut took_off, mut landed) = (0, 0);
        for seed in 0..6 {
            let mut c = City::new(seed);
            for _ in 0..ticks(240.0) {
                c.tick(PHONE, true);
                for b in &c.high_rises {
                    let (Some(h), Some(p)) = (b.heli, b.heli_parts(PHONE)) else {
                        continue;
                    };
                    let rotor_left = p.rotor.0.x;
                    if h.flying() || matches!(h.state, HeliState::SpinningUp(_)) {
                        assert!(
                            rotor_left > plane_x + 150.0,
                            "seed {seed}: {h:?} at {rotor_left}"
                        );
                    }
                    took_off += (h.state == HeliState::Climbing && h.age % 30 == 0) as u32;
                    landed += (h.state == HeliState::Landed(0) && h.age % 30 == 0) as u32;
                }
            }
        }
        assert!(took_off > 0 && landed > 0, "{took_off} {landed}");
    }

    #[test]
    fn a_parked_helicopter_is_part_of_its_high_rise() {
        let mut b = HighRise {
            x: 400.0,
            width: 80.0,
            height: 0.4,
            style: 0,
            heli: None,
        };
        let bare = b.top(PHONE);
        b.heli = Some(Heli {
            off: Vec2::new(0.0, -10.0),
            state: HeliState::Landed(0),
            age: 0,
        });
        let p = b.heli_parts(PHONE).unwrap();
        let shape = b.shape(PHONE);
        let (c, _, _) = p.body;
        assert!(any_overlap(&shape, &probe(c.x, c.y)), "its body");
        assert!(b.top(PHONE) < bare + 1.0 || p.rotor.0.y > bare);
        assert!(!b.heli.unwrap().rotor_on(), "parked: rotor still");
    }

    #[test]
    fn same_seed_same_town() {
        let (mut a, mut b) = (City::new(4), City::new(4));
        run(&mut a, 60.0, true);
        run(&mut b, 60.0, true);
        assert_eq!(a, b);
    }
}
