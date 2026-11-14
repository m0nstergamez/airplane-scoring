//! The town at night (level 3): high-rises, tower cranes and
//! helicopters. Buildings and cranes stand on the ground (fly over them);
//! helicopters fly at the plane's height, so the top of the screen is not
//! safe either:
//!
//! * A **high-rise** stands at the front, on the ground line of the huts.
//!   Fly over it (its roof, the stair house and the antenna count).
//! * A **tower crane** stands on the ground too: mast, jib, counterweight,
//!   cable and load all count. Fly over it. At the latest after
//!   [`HIGH_RISES_IN_A_ROW`] high-rises comes a crane; never two cranes in
//!   a row.
//! * Some high-rises have a **helicopter** on the roof. It either takes
//!   off, rising and flying away while the high-rise is still far ahead,
//!   or comes down from the sky and lands there in good time. Near the
//!   plane it is parked on the roof (fly over it) or already gone.
//! * Every few seconds a **flying helicopter** comes at the plane from
//!   the right, at the plane's height (but never lower than
//!   [`FLYING_HELI_LOWEST`]: always above the town). Dodge it.
//!
//! A helicopter the plane flies into (parked or flying) crashes: it
//! explodes, falls as a burning [`Wreck`] and is no obstacle any more.
//!
//! High-rises and cranes come one at a time with room between them
//! ([`CITY_GAP_SECONDS`], [`CITY_ROOM`]). Positions are screen px; sizes
//! are for a phone and grow with the screen ([`Bounds::scale`]). Seeded,
//! so the same seed gives the same town.

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
/// The least free ground between two of them, in px: room to come down
/// between them.
pub const CITY_ROOM: f32 = 140.0;
/// How likely the next one is a crane (never two in a row).
pub const CRANE_SHARE: f32 = 0.4;
/// At most this many high-rises in a row: then a crane comes for sure.
pub const HIGH_RISES_IN_A_ROW: u32 = 2;

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

/// The jib's top, as a fraction of the screen height (the crane stands on
/// the ground, so this is its highest point), and the jib's height in px.
pub const JIB_TOP: (f32, f32) = (0.36, 0.5);
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
/// The load (a steel beam): width in px, its height in px, and how far
/// under the jib its top hangs, in px.
pub const LOAD_WIDTH: (f32, f32) = (70.0, 110.0);
pub const LOAD_HEIGHT: f32 = 12.0;
pub const LOAD_DROP: (f32, f32) = (24.0, 70.0);
/// The mast's width in px; it stands on the ground ([`HIGH_RISE_GROUND`]).
pub const MAST_WIDTH: f32 = 8.0;

/// A flying helicopter comes every so many seconds (shortest, longest);
/// the first this long after the town begins.
pub const FLYING_HELI_GAP_SECONDS: (f32, f32) = (5.0, 8.0);
pub const FIRST_FLYING_HELI_SECONDS: f32 = 6.0;
/// How fast it flies toward the plane, on top of the ground sliding by,
/// in px per second (slowest, fastest).
pub const FLYING_HELI_SPEED: (f32, f32) = (60.0, 110.0);
/// The height it flies at: the plane's, right up to the top of the
/// screen, but never lower than this (fraction of the screen height from
/// the top), so it always flies above the town.
pub const FLYING_HELI_LOWEST: f32 = 0.26;
/// A crashed helicopter: how long it burns and falls, in seconds, and how
/// fast it falls (px per second, each second).
pub const WRECK_SECONDS: f32 = 1.2;
pub const WRECK_FALL: f32 = 420.0;

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
        let pad = self.pad(bounds);
        let c = Vec2::new(pad.x + heli.off.x, pad.y + heli.off.y);
        Some(HeliParts::at(c, bounds.scale()))
    }

    /// The high-rise as drawn, for crashing (its helicopter apart: see
    /// [`HighRise::heli_shape`]).
    pub fn shape(&self, bounds: Bounds) -> Vec<Poly> {
        let p = self.parts(bounds);
        let rect = |(l, t, w, h): (f32, f32, f32, f32)| Poly::rect(l, t, w, h);
        vec![
            rect(p.body),
            rect(p.stair_house),
            Poly::line(p.antenna.0, p.antenna.1, p.antenna_width),
        ]
    }

    /// Its helicopter as drawn, for crashing (empty without one).
    pub fn heli_shape(&self, bounds: Bounds) -> Vec<Poly> {
        (self.heli_parts(bounds))
            .map(|h| h.shape(bounds.scale()))
            .unwrap_or_default()
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

impl HeliParts {
    /// A helicopter with its body's centre at `c`, nose to the left, at
    /// the screen's `scale`.
    pub fn at(c: Vec2, scale: f32) -> Self {
        let s = scale;
        let (hl, hh) = (HELI_BODY.0 * s, HELI_BODY.1 * s);
        let mast = c.y - hh - 3.0 * s;
        HeliParts {
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
        }
    }

    /// As drawn, for crashing.
    pub fn shape(&self, scale: f32) -> Vec<Poly> {
        let (c, hl, hh) = self.body;
        vec![
            Poly::ellipse(c, hl, hh),
            Poly::line(self.tail.0, self.tail.1, self.tail_width),
            Poly::line(self.rotor.0, self.rotor.1, 2.0 * scale),
            Poly::line(self.skids.0, self.skids.1, 2.0 * scale),
        ]
    }
}

/// A helicopter flying at the plane, from the right.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlyingHeli {
    /// Its body's centre on screen, in px.
    pub pos: Vec2,
    /// How fast it flies toward the plane, in px per second at scale 1,
    /// on top of the ground sliding by.
    pub speed: f32,
    /// Ticks it has existed, for the rotor's turning.
    pub age: u32,
}

impl FlyingHeli {
    /// Where its parts are on this screen.
    pub fn parts(&self, bounds: Bounds) -> HeliParts {
        HeliParts::at(self.pos, bounds.scale())
    }

    /// As drawn, for crashing.
    pub fn shape(&self, bounds: Bounds) -> Vec<Poly> {
        self.parts(bounds).shape(bounds.scale())
    }
}

/// A helicopter the plane flew into: it explodes and falls, burning. No
/// obstacle any more.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Wreck {
    /// Where its body's centre is, in px.
    pub pos: Vec2,
    /// Its speed, px per second (it falls faster and faster).
    pub vel: Vec2,
    /// Ticks since it was hit.
    pub ticks: u32,
}

impl Wreck {
    /// From 0 (the moment it was hit) to 1 (gone).
    pub fn progress(&self) -> f32 {
        (self.ticks as f32 / ticks(WRECK_SECONDS) as f32).min(1.0)
    }
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
    /// The jib's top, as a fraction of the screen height.
    pub jib_at: f32,
    /// How far under the jib the load's top hangs, in px at scale 1.
    pub load_drop: f32,
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
    /// The mast, from the jib down to its foot on the ground, and its
    /// width.
    pub mast: (Vec2, Vec2),
    pub mast_width: f32,
}

impl Crane {
    /// Where each part is on this screen.
    pub fn parts(&self, bounds: Bounds) -> CraneParts {
        let s = bounds.scale();
        let h = bounds.height;
        let jib_top = self.jib_at * h;
        let jib_h = JIB_HEIGHT * s;
        let tip = self.x - self.reach * s;
        let back = self.x + COUNTER_JIB * s;
        let (cw, ch) = (COUNTERWEIGHT.0 * s, COUNTERWEIGHT.1 * s);
        let cable_x = tip + CABLE_IN * s;
        let (lw, lh) = (self.load_width * s, LOAD_HEIGHT * s);
        let load_top = jib_top + jib_h + self.load_drop * s;
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
                Vec2::new(self.x, HIGH_RISE_GROUND * h),
            ),
            mast_width: MAST_WIDTH * s,
        }
    }

    /// Its highest point (the jib's top), in px from the top: fly over it.
    pub fn top(&self, bounds: Bounds) -> f32 {
        self.parts(bounds).jib.1
    }

    /// The crane as drawn, for crashing: all of it, from the jib down the
    /// mast to the ground.
    pub fn shape(&self, bounds: Bounds) -> Vec<Poly> {
        let p = self.parts(bounds);
        let rect = |(l, t, w, h): (f32, f32, f32, f32)| Poly::rect(l, t, w, h);
        vec![
            rect(p.jib),
            rect(p.counterweight),
            Poly::line(p.cable.0, p.cable.1, p.cable_width),
            rect(p.load),
            Poly::line(p.mast.0, p.mast.1, p.mast_width),
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
    /// Helicopters flying at the plane, oldest first.
    pub flyers: Vec<FlyingHeli>,
    /// Crashed helicopters, falling and burning.
    pub wrecks: Vec<Wreck>,
    /// How many high-rises came since the last crane (0: the last one was
    /// a crane, so no crane next).
    since_crane: u32,
    next_ticks: u32,
    next_flyer_ticks: u32,
    rng: Rng,
    flyer_rng: Rng,
    /// The game's speed (1 = normal), see [`City::set_speed`].
    speed: f32,
}

impl City {
    pub fn new(seed: u64) -> Self {
        Self {
            high_rises: Vec::new(),
            cranes: Vec::new(),
            flyers: Vec::new(),
            wrecks: Vec::new(),
            since_crane: 1,
            next_ticks: ticks(FIRST_CITY_SECONDS),
            next_flyer_ticks: ticks(FIRST_FLYING_HELI_SECONDS),
            speed: 1.0,
            // Its own stream, so the town does not change other obstacles.
            rng: Rng::new(seed ^ 0xC17E_0000_0000_0003),
            flyer_rng: Rng::new(seed ^ 0xC17E_0000_0000_0004),
        }
    }

    /// Clear the town away (a new level).
    pub fn clear(&mut self) {
        self.high_rises.clear();
        self.cranes.clear();
        self.flyers.clear();
        self.wrecks.clear();
    }

    /// Everything slides further left by `dx` px (the game going faster).
    pub fn shift(&mut self, dx: f32) {
        self.high_rises.iter_mut().for_each(|b| b.x += dx);
        self.cranes.iter_mut().for_each(|c| c.x += dx);
        self.flyers.iter_mut().for_each(|f| f.pos.x += dx);
        self.wrecks.iter_mut().for_each(|w| w.pos.x += dx);
    }

    /// The plane flew into flying helicopter `i`: it crashes.
    pub fn down_flyer(&mut self, i: usize, bounds: Bounds) {
        let f = self.flyers.remove(i);
        let s = bounds.scale();
        self.wrecks.push(Wreck {
            pos: f.pos,
            vel: Vec2::new(-0.4 * f.speed * s, -40.0 * s),
            ticks: 0,
        });
    }

    /// The plane flew into the helicopter on high-rise `i`: it crashes.
    pub fn down_roof_heli(&mut self, i: usize, bounds: Bounds) {
        let b = &mut self.high_rises[i];
        let Some(h) = b.heli_parts(bounds) else {
            return;
        };
        b.heli = None;
        self.wrecks.push(Wreck {
            pos: h.body.0,
            vel: Vec2::new(0.0, -60.0 * bounds.scale()),
            ticks: 0,
        });
    }

    /// How fast the game goes (see `World::speed`), so landing
    /// helicopters still touch down in good time when it is faster.
    pub fn set_speed(&mut self, speed: f32) {
        self.speed = speed.max(0.1);
    }

    /// Advance one tick. `active`: is the town fully on screen? Whatever
    /// is there keeps sliding off when it is not. `plane_y`: the plane's
    /// height, where flying helicopters come.
    pub fn tick(&mut self, bounds: Bounds, active: bool, plane_y: f32) {
        let s = bounds.scale();
        let dx = -SCROLL_SPEED * s * TICK_SECONDS;
        self.shift(dx);
        for b in &mut self.high_rises {
            b.tick_heli(bounds);
        }
        for f in &mut self.flyers {
            f.pos.x -= f.speed * s * TICK_SECONDS;
            f.age += 1;
        }
        let ground = HIGH_RISE_GROUND * bounds.height;
        for w in &mut self.wrecks {
            w.vel.y += WRECK_FALL * s * TICK_SECONDS;
            w.pos.x += w.vel.x * TICK_SECONDS;
            w.pos.y = (w.pos.y + w.vel.y * TICK_SECONDS).min(ground);
            w.ticks += 1;
        }
        let margin = 20.0 * s;
        self.high_rises.retain(|b| b.right(bounds) > -margin);
        self.cranes.retain(|c| c.span(bounds).1 > -margin);
        let tail = (HELI_BODY.0 + HELI_TAIL + HELI_ROTOR) * s;
        self.flyers.retain(|f| f.pos.x + tail > -margin);
        self.wrecks.retain(|w| w.progress() < 1.0);

        if !active {
            self.next_ticks = ticks(FIRST_CITY_SECONDS);
            self.next_flyer_ticks = ticks(FIRST_FLYING_HELI_SECONDS);
            return;
        }
        self.tick_flyers(bounds, plane_y);
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
        let crane = self.since_crane > 0
            && (self.since_crane >= HIGH_RISES_IN_A_ROW || r.unit() < CRANE_SHARE);
        if crane {
            let reach = r.range(JIB_REACH.0, JIB_REACH.1);
            let load_width = r.range(LOAD_WIDTH.0, LOAD_WIDTH.1);
            let left = (reach - CABLE_IN).max(0.0) + load_width / 2.0;
            self.cranes.push(Crane {
                x: enter + left * s,
                reach,
                load_width,
                jib_at: r.range(JIB_TOP.0, JIB_TOP.1),
                load_drop: r.range(LOAD_DROP.0, LOAD_DROP.1),
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
        self.since_crane = if crane { 0 } else { self.since_crane + 1 };
        self.next_ticks = ticks(r.range(CITY_GAP_SECONDS.0, CITY_GAP_SECONDS.1));
    }

    /// Send a helicopter at the plane every few seconds, from just off
    /// the right edge, at the plane's height (no lower than
    /// [`FLYING_HELI_LOWEST`]).
    fn tick_flyers(&mut self, bounds: Bounds, plane_y: f32) {
        self.next_flyer_ticks = self.next_flyer_ticks.saturating_sub(1);
        if self.next_flyer_ticks > 0 {
            return;
        }
        let s = bounds.scale();
        let r = &mut self.flyer_rng;
        self.flyers.push(FlyingHeli {
            pos: Vec2::new(
                bounds.width + (HELI_ROTOR + 20.0) * s,
                plane_y.min(FLYING_HELI_LOWEST * bounds.height),
            ),
            speed: r.range(FLYING_HELI_SPEED.0, FLYING_HELI_SPEED.1),
            age: 0,
        });
        let gap = r.range(FLYING_HELI_GAP_SECONDS.0, FLYING_HELI_GAP_SECONDS.1);
        self.next_flyer_ticks = ticks(gap);
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

    /// Where the plane flies in these tests (low: the flying helicopters
    /// stay in their band above the town).
    const PLANE_Y: f32 = 0.5 * PHONE.height;

    fn run(c: &mut City, seconds: f32, active: bool) {
        for _ in 0..ticks(seconds) {
            c.tick(PHONE, active, PLANE_Y);
        }
    }

    fn probe(x: f32, y: f32) -> Vec<Poly> {
        vec![Poly::rect(x - 1.0, y - 1.0, 2.0, 2.0)]
    }

    #[test]
    fn nothing_outside_the_town() {
        let mut c = City::new(1);
        run(&mut c, 120.0, false);
        assert!(c.high_rises.is_empty() && c.cranes.is_empty() && c.flyers.is_empty());
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
            c.tick(PHONE, true, PLANE_Y);
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
            for pair in kinds.windows(2) {
                assert!(!(pair[0] && pair[1]), "seed {seed}: two cranes");
            }
        }
    }

    #[test]
    fn a_crane_comes_at_the_latest_after_two_high_rises() {
        for seed in 0..20 {
            let kinds = arrivals(seed, 300.0);
            let n = HIGH_RISES_IN_A_ROW as usize + 1;
            for window in kinds.windows(n) {
                assert!(window.contains(&true), "seed {seed}: {kinds:?}");
            }
            assert!(kinds[..n].contains(&true), "seed {seed}: one early on");
        }
    }

    #[test]
    fn they_never_overlap_and_leave_room_to_change_course() {
        let mut c = City::new(7);
        for _ in 0..ticks(300.0) {
            c.tick(PHONE, true, PLANE_Y);
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
    fn high_rises_and_cranes_leave_the_sky_free() {
        let mut c = City::new(3);
        for _ in 0..ticks(300.0) {
            c.tick(PHONE, true, PLANE_Y);
            for b in &c.high_rises {
                assert!(b.top(PHONE) > 0.25 * PHONE.height, "room above");
            }
            for k in &c.cranes {
                assert!(k.top(PHONE) > 0.33 * PHONE.height, "room above");
                let (_, lt, _, lh) = k.parts(PHONE).load;
                assert!(lt + lh < HIGH_RISE_GROUND * PHONE.height, "load in the air");
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
    fn all_of_a_crane_is_a_hit_down_to_the_foot_of_its_mast() {
        let k = Crane {
            x: 600.0,
            reach: 180.0,
            load_width: 90.0,
            jib_at: 0.42,
            load_drop: 40.0,
        };
        let shape = k.shape(PHONE);
        let hit = |x: f32, y: f32| any_overlap(&shape, &probe(x, y));
        let p = k.parts(PHONE);
        let (jl, jt, jw, jh) = p.jib;
        assert!(hit(jl + jw / 2.0, jt + jh / 2.0), "the jib");
        assert!(hit(p.cable.0.x, (p.cable.0.y + p.cable.1.y) / 2.0), "cable");
        let (ll, lt, lw, lh) = p.load;
        assert!(hit(ll + lw / 2.0, lt + lh / 2.0), "load");
        let (top, foot) = p.mast;
        assert!(hit(top.x, (top.y + foot.y) / 2.0), "the mast");
        assert!(hit(foot.x, foot.y - 3.0), "the mast's foot");
        assert_eq!(foot.y, HIGH_RISE_GROUND * PHONE.height, "on the ground");
        assert!(!hit(top.x, jt - 4.0), "over the crane is free");
        assert!(!hit(top.x + 12.0, foot.y - 30.0), "beside the mast");
    }

    #[test]
    fn helicopters_take_off_and_land_far_ahead_of_the_plane() {
        let plane_x = crate::PLAYER_X_FRACTION * PHONE.width;
        let (mut took_off, mut landed) = (0, 0);
        for seed in 0..6 {
            let mut c = City::new(seed);
            for _ in 0..ticks(240.0) {
                c.tick(PHONE, true, PLANE_Y);
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
        let (c, _, _) = p.body;
        assert!(
            any_overlap(&b.heli_shape(PHONE), &probe(c.x, c.y)),
            "its body"
        );
        assert!(
            !any_overlap(&b.shape(PHONE), &probe(c.x, c.y)),
            "not the walls"
        );
        assert!(b.top(PHONE) < bare + 1.0 || p.rotor.0.y > bare);
        assert!(!b.heli.unwrap().rotor_on(), "parked: rotor still");
    }

    #[test]
    fn flying_helicopters_come_at_the_planes_height_above_the_town() {
        let lowest = FLYING_HELI_LOWEST * PHONE.height;
        for (plane_y, want) in [
            (0.15 * PHONE.height, 0.15 * PHONE.height),
            (5.0, 5.0),
            (0.8 * PHONE.height, lowest),
        ] {
            let mut c = City::new(2);
            let mut seen = 0;
            for _ in 0..ticks(60.0) {
                c.tick(PHONE, true, plane_y);
                for f in c.flyers.iter().filter(|f| f.age == 0) {
                    assert!(f.pos.x > PHONE.width, "from off the right");
                    assert!((f.pos.y - want).abs() < 0.5, "{} not {want}", f.pos.y);
                    seen += 1;
                }
            }
            // One every 5–8 s after the first 6 s.
            assert!((6..=11).contains(&seen), "{seen}");
        }
    }

    #[test]
    fn flying_helicopters_stay_clear_of_the_buildings_and_cranes() {
        let mut c = City::new(9);
        for _ in 0..ticks(300.0) {
            c.tick(PHONE, true, PHONE.height); // the plane low: they fly lowest
            let tops = (c.high_rises.iter().map(|b| b.top(PHONE)))
                .chain(c.cranes.iter().map(|k| k.top(PHONE)));
            let lowest_heli = (c.flyers.iter())
                .map(|f| f.parts(PHONE).skids.0.y)
                .fold(f32::MIN, f32::max);
            for top in tops {
                assert!(top - lowest_heli > 15.0, "{top} {lowest_heli}");
            }
        }
    }

    #[test]
    fn a_crashed_helicopter_falls_burns_out_and_is_gone() {
        let mut c = City::new(2);
        c.flyers.push(FlyingHeli {
            pos: Vec2::new(400.0, 60.0),
            speed: 80.0,
            age: 0,
        });
        c.down_flyer(0, PHONE);
        assert!(c.flyers.is_empty(), "no obstacle any more");
        assert_eq!(c.wrecks.len(), 1);
        run(&mut c, WRECK_SECONDS / 2.0, false);
        assert!(c.wrecks[0].pos.y > 60.0, "falling");
        run(&mut c, WRECK_SECONDS, false);
        assert!(c.wrecks.is_empty(), "burnt out");

        let mut b = HighRise {
            x: 400.0,
            width: 80.0,
            height: 0.4,
            style: 0,
            heli: Some(Heli {
                off: Vec2::new(0.0, -10.0),
                state: HeliState::Landed(0),
                age: 0,
            }),
        };
        b.tick_heli(PHONE);
        c.high_rises.push(b);
        c.down_roof_heli(0, PHONE);
        assert!(c.high_rises[0].heli.is_none(), "off the roof");
        assert_eq!(c.wrecks.len(), 1);
    }

    #[test]
    fn same_seed_same_town() {
        let (mut a, mut b) = (City::new(4), City::new(4));
        run(&mut a, 60.0, true);
        run(&mut b, 60.0, true);
        assert_eq!(a, b);
    }
}
