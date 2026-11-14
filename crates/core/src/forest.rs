//! The autumn forest (level 5): tall firs, a buzzard that dives at the
//! plane, gusts full of leaves and banks of fog.
//!
//! * **Firs** stand on the ground in groups of one to three
//!   ([`FIR_GROUP`]), each its own height. They are solid: fly over them.
//!   A group waits until the last one has moved in by [`FOREST_ROOM`], so
//!   there is always a gap to come down into.
//! * A **buzzard** comes in from ahead, from above or up out of the trees
//!   ([`BuzzardFrom`]), never from behind, at irregular times
//!   ([`BUZZARD_GAP_SECONDS`]). It circles ahead of the plane for a moment
//!   ([`BUZZARD_CIRCLE_SECONDS`]), so you see it coming, then cries and
//!   dives straight at where the plane is then. Fly into it and you crash;
//!   it is knocked down and tumbles to the ground.
//! * **Gusts** of autumn leaves sweep down from the sky in a band that
//!   slides past with the ground. Inside one the plane is pressed down
//!   ([`GUST_PUSH`]); it costs no life, but it can push you into a fir.
//!   Climbing gets you out of it.
//! * **Fog** now and then: a bank drifts over for a few seconds
//!   ([`FOG_SECONDS`]) and the game veils the screen.
//!
//! Positions are screen px; sizes are for a phone and grow with the
//! screen ([`Bounds::scale`]). Seeded, so the same seed gives the same
//! forest.

use crate::rng::Rng;
use crate::shape::Poly;
use crate::{Bounds, Event, Vec2, SCROLL_SPEED, TICK_SECONDS};

/// The level with the autumn forest.
pub const FOREST_LEVEL: u32 = 5;

/// Where the firs stand: the ground line, as a fraction of the screen
/// height from the top.
pub const FOREST_GROUND: f32 = crate::huts::HUT_GROUND;
/// Time between two groups of firs, shortest and longest, and the first
/// one, in seconds.
pub const FIR_GAP_SECONDS: (f32, f32) = (2.2, 3.6);
pub const FIRST_FIR_SECONDS: f32 = 3.0;
/// How many firs stand together, fewest and most.
pub const FIR_GROUP: (u32, u32) = (1, 3);
/// The step from one fir of a group to the next, in px.
pub const FIR_GROUP_STEP: (f32, f32) = (26.0, 40.0);
/// A fir's tip, as a fraction of the screen height from the top (tallest
/// and shortest), and its width at the bottom as a share of its height.
pub const FIR_TOP: (f32, f32) = (0.38, 0.66);
pub const FIR_WIDTH: f32 = 0.34;
/// The least free ground between two groups, in px.
pub const FOREST_ROOM: f32 = 120.0;

/// Time between two buzzards, shortest and longest, and the first, in
/// seconds; how long one circles before it dives; how fast it glides in
/// and dives, in px per second.
/// (A wide range, so it comes at irregular times: now and then two close
/// together, now and then a long quiet.)
pub const BUZZARD_GAP_SECONDS: (f32, f32) = (3.5, 12.0);
pub const FIRST_BUZZARD_SECONDS: f32 = 5.0;
pub const BUZZARD_CIRCLE_SECONDS: f32 = 1.6;
pub const BUZZARD_GLIDE: f32 = 220.0;
pub const BUZZARD_DIVE: f32 = 330.0;
/// Where it circles: across the screen (fraction of the width from the
/// left) and up high (fraction of the height from the top); the circle's
/// radius in px.
/// (Clear of the lives and the clock at the top.)
pub const BUZZARD_CIRCLE_AT: ((f32, f32), (f32, f32)) = ((0.55, 0.75), (0.26, 0.36));
pub const BUZZARD_CIRCLE_RADIUS: f32 = 22.0;
/// A buzzard's half wingspan and half body length, in px.
pub const BUZZARD_SIZE: (f32, f32) = (20.0, 9.0);

/// Time between two gusts, shortest and longest, and the first, in
/// seconds; a gust's width in px and how far down it reaches (fraction of
/// the screen height from the top).
pub const GUST_GAP_SECONDS: (f32, f32) = (5.0, 8.0);
pub const FIRST_GUST_SECONDS: f32 = 6.0;
pub const GUST_WIDTH: (f32, f32) = (80.0, 120.0);
pub const GUST_BOTTOM: (f32, f32) = (0.55, 0.8);
/// How hard a gust presses the plane down, in px per second, each second
/// (at scale 1). Climbing (`PLAYER_CLIMB_SPEED`) is stronger.
pub const GUST_PUSH: f32 = 700.0;

/// Time between two banks of fog, shortest and longest, and the first,
/// in seconds; how long one lasts and how long it takes to come and go.
pub const FOG_GAP_SECONDS: (f32, f32) = (11.0, 17.0);
pub const FIRST_FOG_SECONDS: f32 = 9.0;
pub const FOG_SECONDS: f32 = 3.5;
pub const FOG_FADE_SECONDS: f32 = 0.8;

/// Things start and are dropped this far outside the screen, in px.
const MARGIN: f32 = 40.0;

fn ticks(seconds: f32) -> u32 {
    (seconds / TICK_SECONDS).round().max(1.0) as u32
}

/// A tall fir on the ground.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fir {
    /// Its trunk's x on screen, in px.
    pub x: f32,
    /// Its tip, as a fraction of the screen height from the top.
    pub top: f32,
}

/// Where a fir's parts are on screen, in px.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FirParts {
    /// Three tiers of branches, top to bottom, each a triangle: its tip,
    /// bottom left and bottom right.
    pub tiers: [(Vec2, Vec2, Vec2); 3],
    /// The trunk below them: left, top, width, height.
    pub trunk: (f32, f32, f32, f32),
}

impl Fir {
    /// Where each part is on this screen.
    pub fn parts(&self, bounds: Bounds) -> FirParts {
        let ground = FOREST_GROUND * bounds.height;
        let tip = self.top * bounds.height;
        let h = ground - tip;
        let half = FIR_WIDTH * h / 2.0;
        let trunk_h = 0.1 * h;
        let crown = h - trunk_h;
        // Three tiers, each a bit wider and overlapping the one above.
        let tier = |i: usize| {
            let t = tip + crown * [0.0, 0.25, 0.5][i];
            let b = tip + crown * [0.45, 0.75, 1.0][i];
            let w = half * [0.55, 0.8, 1.0][i];
            (
                Vec2::new(self.x, t),
                Vec2::new(self.x - w, b),
                Vec2::new(self.x + w, b),
            )
        };
        let tw = (0.12 * half).max(3.0);
        FirParts {
            tiers: [tier(0), tier(1), tier(2)],
            trunk: (self.x - tw / 2.0, ground - trunk_h, tw, trunk_h),
        }
    }

    /// The fir as drawn, for crashing: its three tiers and the trunk.
    pub fn shape(&self, bounds: Bounds) -> Vec<Poly> {
        let p = self.parts(bounds);
        let tri = |(a, b, c): (Vec2, Vec2, Vec2)| {
            Poly::at(
                Vec2::new(0.0, 0.0),
                1.0,
                &[(a.x, a.y), (b.x, b.y), (c.x, c.y)],
            )
        };
        let (l, t, w, h) = p.trunk;
        vec![
            tri(p.tiers[0]),
            tri(p.tiers[1]),
            tri(p.tiers[2]),
            Poly::rect(l, t, w, h),
        ]
    }

    /// Its leftmost and rightmost x, in px.
    pub fn span(&self, bounds: Bounds) -> (f32, f32) {
        let p = self.parts(bounds);
        (p.tiers[2].1.x, p.tiers[2].2.x)
    }
}

/// Where a buzzard comes from. Never from behind: it always circles
/// ahead of the plane before it dives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuzzardFrom {
    /// Gliding in from the right.
    Ahead,
    /// Dropping in from the top of the screen.
    Above,
    /// Rising up out of the trees.
    Below,
}

/// What a buzzard is doing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BuzzardState {
    /// Flying in (see [`BuzzardFrom`]) to where it circles.
    Gliding,
    /// Circling round `centre` for this many ticks so far.
    Circling { centre: Vec2, ticks: u32 },
    /// Diving straight on, at `vel` px per second.
    Diving { vel: Vec2 },
    /// Struck by the plane: tumbling down, for this many ticks so far.
    Down { ticks: u32, vel: Vec2 },
}

/// A buzzard.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Buzzard {
    /// Its body's centre on screen, in px.
    pub pos: Vec2,
    pub state: BuzzardState,
    /// Where it will circle, on screen, in px.
    pub circle_at: Vec2,
    /// Where it came from.
    pub from: BuzzardFrom,
    /// Ticks since it appeared (for the wing beat).
    pub age: u32,
}

impl Buzzard {
    /// Was it struck? Then it tumbles down and cannot be hit again.
    pub fn is_down(&self) -> bool {
        matches!(self.state, BuzzardState::Down { .. })
    }

    /// Diving?
    pub fn is_diving(&self) -> bool {
        matches!(self.state, BuzzardState::Diving { .. })
    }

    /// Which way it heads, in radians (0 = right): while it dives, along
    /// its dive; otherwise level, to the left.
    pub fn heading(&self) -> f32 {
        match self.state {
            BuzzardState::Diving { vel } | BuzzardState::Down { vel, .. } => vel.y.atan2(vel.x),
            // Flying in: toward where it will circle.
            BuzzardState::Gliding => {
                let (dx, dy) = (self.circle_at.x - self.pos.x, self.circle_at.y - self.pos.y);
                if dx.abs() + dy.abs() < 1.0 {
                    std::f32::consts::PI
                } else {
                    dy.atan2(dx)
                }
            }
            BuzzardState::Circling { .. } => std::f32::consts::PI,
        }
    }

    /// How far a struck buzzard has turned over, in radians.
    pub fn tumble(&self) -> f32 {
        match self.state {
            BuzzardState::Down { ticks, .. } => ticks as f32 * TICK_SECONDS * 7.0,
            _ => 0.0,
        }
    }

    /// Struck by the plane: it tumbles down.
    pub fn knock_down(&mut self, scale: f32) {
        if self.is_down() {
            return;
        }
        let vel = match self.state {
            BuzzardState::Diving { vel } => Vec2::new(vel.x * 0.3, -60.0 * scale),
            _ => Vec2::new(-40.0 * scale, -60.0 * scale),
        };
        self.state = BuzzardState::Down { ticks: 0, vel };
    }

    /// The buzzard as drawn, for crashing: its body along its heading and
    /// its spread wings across it.
    pub fn shape(&self, scale: f32) -> Vec<Poly> {
        let (span, body) = (BUZZARD_SIZE.0 * scale, BUZZARD_SIZE.1 * scale);
        let a = self.heading() + self.tumble();
        let along = Vec2::new(a.cos(), a.sin());
        let across = Vec2::new(-along.y, along.x);
        let p = self.pos;
        let at = |u: f32, v: f32| {
            Vec2::new(
                p.x + along.x * u + across.x * v,
                p.y + along.y * u + across.y * v,
            )
        };
        vec![
            Poly::line(at(-body, 0.0), at(body, 0.0), 6.0 * scale),
            Poly::line(at(0.0, -span), at(0.0, span), 4.0 * scale),
        ]
    }
}

/// A gust full of leaves, from the top of the sky down.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gust {
    /// Its middle on screen, in px.
    pub x: f32,
    /// Its width in px at scale 1.
    pub width: f32,
    /// How far down it reaches, as a fraction of the screen height.
    pub bottom: f32,
    /// Ticks since it appeared (for the leaves).
    pub age: u32,
}

impl Gust {
    /// Where it is: left, top, width, height, in px (from the top).
    pub fn rect(&self, bounds: Bounds) -> (f32, f32, f32, f32) {
        let w = self.width * bounds.scale();
        (self.x - w / 2.0, 0.0, w, self.bottom * bounds.height)
    }

    /// The gust, for finding out if the plane is in it.
    pub fn shape(&self, bounds: Bounds) -> Poly {
        let (l, t, w, h) = self.rect(bounds);
        Poly::rect(l, t, w, h)
    }
}

/// The autumn forest's firs, buzzards, gusts and fog.
#[derive(Debug, Clone, PartialEq)]
pub struct Forest {
    /// Firs on the ground, oldest first.
    pub firs: Vec<Fir>,
    /// Buzzards in the air.
    pub buzzards: Vec<Buzzard>,
    /// Gusts of leaves.
    pub gusts: Vec<Gust>,
    /// Ticks into the fog now; `None` between banks.
    fog: Option<u32>,
    next_fir: u32,
    next_buzzard: u32,
    next_gust: u32,
    next_fog: u32,
    rng: Rng,
}

impl Forest {
    pub fn new(seed: u64) -> Self {
        Self {
            firs: Vec::new(),
            buzzards: Vec::new(),
            gusts: Vec::new(),
            fog: None,
            next_fir: ticks(FIRST_FIR_SECONDS),
            next_buzzard: ticks(FIRST_BUZZARD_SECONDS),
            next_gust: ticks(FIRST_GUST_SECONDS),
            next_fog: ticks(FIRST_FOG_SECONDS),
            // Its own stream, so the forest does not change other things.
            rng: Rng::new(seed ^ 0xF0AE_5700_0000_0005),
        }
    }

    /// Clear it all away (a new level).
    pub fn clear(&mut self) {
        self.firs.clear();
        self.buzzards.clear();
        self.gusts.clear();
        self.fog = None;
    }

    /// Ground things (firs, gusts) slide further left by `dx` px (the game
    /// going faster). Buzzards do not: they fly in the air, circle at a
    /// place on the screen ahead of the plane and dive straight at it, so
    /// at any speed they never end up behind it or miss their aim.
    pub fn shift(&mut self, dx: f32) {
        self.firs.iter_mut().for_each(|f| f.x += dx);
        self.gusts.iter_mut().for_each(|g| g.x += dx);
    }

    /// How thick the fog is now: 0 = none, 1 = full, coming and going
    /// over [`FOG_FADE_SECONDS`].
    pub fn fog(&self) -> f32 {
        let Some(t) = self.fog else { return 0.0 };
        let t = t as f32 * TICK_SECONDS;
        let fade_in = (t / FOG_FADE_SECONDS).min(1.0);
        let fade_out = ((FOG_SECONDS - t) / FOG_FADE_SECONDS).clamp(0.0, 1.0);
        fade_in.min(fade_out)
    }

    /// Is the plane, as `me`, in a gust?
    pub fn in_gust(&self, bounds: Bounds, me: &[Poly]) -> bool {
        (self.gusts.iter()).any(|g| crate::shape::any_overlap(me, &[g.shape(bounds)]))
    }

    /// Advance one tick. `active`: is the forest on screen (nothing new
    /// comes while it is not)? `plane`: where a diving buzzard aims.
    /// Cries go to `events`.
    pub fn tick(&mut self, bounds: Bounds, active: bool, plane: Vec2, events: &mut Vec<Event>) {
        let s = bounds.scale();
        let dx = -SCROLL_SPEED * s * TICK_SECONDS;
        for f in &mut self.firs {
            f.x += dx;
        }
        for g in &mut self.gusts {
            g.x += dx;
            g.age += 1;
        }
        self.firs.retain(|f| f.span(bounds).1 > -MARGIN * s);
        self.gusts.retain(|g| g.x + g.width * s / 2.0 > -MARGIN * s);
        for b in &mut self.buzzards {
            b.age += 1;
            if Self::step_buzzard(b, bounds, plane) {
                events.push(Event::BuzzardCry);
            }
        }
        let (w, h) = (bounds.width, bounds.height);
        let m = 2.0 * MARGIN * s;
        self.buzzards.retain(|b| {
            let p = b.pos;
            p.x > -m && p.y < h + m && p.y > -m && p.x < w + 2.0 * m
        });
        if let Some(t) = &mut self.fog {
            *t += 1;
            if *t >= ticks(FOG_SECONDS) {
                self.fog = None;
            }
        }

        if !active {
            self.next_fir = ticks(FIRST_FIR_SECONDS);
            self.next_buzzard = ticks(FIRST_BUZZARD_SECONDS);
            self.next_gust = ticks(FIRST_GUST_SECONDS);
            self.next_fog = ticks(FIRST_FOG_SECONDS);
            return; // (a fog going on still fades out)
        }
        self.next_fir -= 1;
        let last_fir = (self.firs.iter().map(|f| f.span(bounds).1)).fold(f32::MIN, f32::max);
        if self.next_fir == 0 && last_fir > w - FOREST_ROOM * s {
            self.next_fir = ticks(0.2);
        }
        if self.next_fir == 0 {
            self.spawn_firs(bounds);
            self.next_fir = ticks(self.rng.range(FIR_GAP_SECONDS.0, FIR_GAP_SECONDS.1));
        }
        self.next_buzzard -= 1;
        if self.next_buzzard == 0 {
            let r = &mut self.rng;
            let at = Vec2::new(
                r.range(BUZZARD_CIRCLE_AT.0 .0, BUZZARD_CIRCLE_AT.0 .1) * w,
                r.range(BUZZARD_CIRCLE_AT.1 .0, BUZZARD_CIRCLE_AT.1 .1) * h,
            );
            let from = match (r.unit() * 3.0) as u32 {
                0 => BuzzardFrom::Ahead,
                1 => BuzzardFrom::Above,
                _ => BuzzardFrom::Below,
            };
            let pos = match from {
                BuzzardFrom::Ahead => Vec2::new(w + MARGIN * s, at.y),
                BuzzardFrom::Above => Vec2::new(at.x + r.range(-0.1, 0.15) * w, -MARGIN * s),
                BuzzardFrom::Below => Vec2::new(at.x + r.range(-0.1, 0.15) * w, h + MARGIN * s),
            };
            self.buzzards.push(Buzzard {
                pos,
                state: BuzzardState::Gliding,
                circle_at: at,
                from,
                age: 0,
            });
            self.next_buzzard = ticks(r.range(BUZZARD_GAP_SECONDS.0, BUZZARD_GAP_SECONDS.1));
        }
        self.next_gust -= 1;
        if self.next_gust == 0 {
            let r = &mut self.rng;
            let width = r.range(GUST_WIDTH.0, GUST_WIDTH.1);
            self.gusts.push(Gust {
                // (x is its middle; its left edge starts just off the screen)
                x: w + (width / 2.0 + 10.0) * s,
                width,
                bottom: r.range(GUST_BOTTOM.0, GUST_BOTTOM.1),
                age: 0,
            });
            self.next_gust = ticks(r.range(GUST_GAP_SECONDS.0, GUST_GAP_SECONDS.1));
        }
        self.next_fog -= 1;
        if self.next_fog == 0 {
            self.fog = Some(0);
            self.next_fog = ticks(self.rng.range(FOG_GAP_SECONDS.0, FOG_GAP_SECONDS.1));
        }
    }

    /// Move a buzzard on by one tick. Returns true when it starts its
    /// dive (it cries then).
    fn step_buzzard(b: &mut Buzzard, bounds: Bounds, plane: Vec2) -> bool {
        let s = bounds.scale();
        match b.state {
            BuzzardState::Gliding => {
                // Straight to the circle's right-hand point, then round.
                let r = BUZZARD_CIRCLE_RADIUS * s;
                let target = Vec2::new(b.circle_at.x + r, b.circle_at.y);
                let (dx, dy) = (target.x - b.pos.x, target.y - b.pos.y);
                let len = (dx * dx + dy * dy).sqrt();
                let step = BUZZARD_GLIDE * s * TICK_SECONDS;
                if len <= step {
                    b.pos = target;
                    b.state = BuzzardState::Circling {
                        centre: b.circle_at,
                        ticks: 0,
                    };
                } else {
                    b.pos.x += dx / len * step;
                    b.pos.y += dy / len * step;
                }
                false
            }
            BuzzardState::Circling { centre, ticks: t } => {
                let t = t + 1;
                let a = t as f32 * TICK_SECONDS * std::f32::consts::TAU / BUZZARD_CIRCLE_SECONDS;
                let r = BUZZARD_CIRCLE_RADIUS * s;
                b.pos = Vec2::new(centre.x + r * a.cos(), centre.y - r * a.sin());
                if t >= ticks(BUZZARD_CIRCLE_SECONDS) {
                    let (dx, dy) = (plane.x - b.pos.x, plane.y - b.pos.y);
                    let len = (dx * dx + dy * dy).sqrt().max(1.0);
                    let speed = BUZZARD_DIVE * s;
                    b.state = BuzzardState::Diving {
                        vel: Vec2::new(dx / len * speed, dy / len * speed),
                    };
                    return true;
                }
                b.state = BuzzardState::Circling { centre, ticks: t };
                false
            }
            BuzzardState::Diving { vel } => {
                b.pos.x += vel.x * TICK_SECONDS;
                b.pos.y += vel.y * TICK_SECONDS;
                false
            }
            BuzzardState::Down { ticks: t, vel } => {
                let vel = Vec2::new(vel.x, vel.y + 650.0 * s * TICK_SECONDS);
                b.pos.x += vel.x * TICK_SECONDS;
                b.pos.y += vel.y * TICK_SECONDS;
                b.state = BuzzardState::Down { ticks: t + 1, vel };
                false
            }
        }
    }

    /// A group of one to three firs from just off the right edge, each
    /// its own height.
    fn spawn_firs(&mut self, bounds: Bounds) {
        let s = bounds.scale();
        let r = &mut self.rng;
        let n = (r.range(FIR_GROUP.0 as f32, FIR_GROUP.1 as f32 + 1.0) as u32).min(FIR_GROUP.1);
        let mut x = bounds.width + MARGIN * s;
        for i in 0..n {
            let top = r.range(FIR_TOP.0, FIR_TOP.1);
            let fir = Fir { x, top };
            // Its left edge just off the screen (the first) or a step on.
            let half = x - fir.span(bounds).0;
            let fir = Fir {
                x: if i == 0 { x + half } else { x },
                top,
            };
            x = fir.x + r.range(FIR_GROUP_STEP.0, FIR_GROUP_STEP.1) * s;
            self.firs.push(fir);
        }
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
    const PLANE: Vec2 = Vec2 {
        x: 0.2 * 844.0,
        y: 200.0,
    };

    fn run(f: &mut Forest, seconds: f32, active: bool) -> Vec<Event> {
        let mut events = Vec::new();
        for _ in 0..ticks(seconds) {
            f.tick(PHONE, active, PLANE, &mut events);
        }
        events
    }

    fn probe(x: f32, y: f32) -> Vec<Poly> {
        vec![Poly::rect(x - 1.0, y - 1.0, 2.0, 2.0)]
    }

    #[test]
    fn nothing_away_from_the_forest() {
        let mut f = Forest::new(1);
        let events = run(&mut f, 60.0, false);
        assert!(f.firs.is_empty() && f.buzzards.is_empty() && f.gusts.is_empty());
        assert!(events.is_empty());
        assert_eq!(f.fog(), 0.0);
    }

    #[test]
    fn firs_come_in_groups_of_different_heights_with_room_between() {
        let mut f = Forest::new(2);
        let mut max_group = 0;
        let mut heights = Vec::new();
        for _ in 0..ticks(120.0) {
            let before = f.firs.len();
            f.tick(PHONE, true, PLANE, &mut Vec::new());
            let new = f.firs.len().saturating_sub(before);
            max_group = max_group.max(new);
            heights.extend(f.firs[f.firs.len() - new..].iter().map(|x| x.top));
            for fir in &f.firs {
                let tip = fir.parts(PHONE).tiers[0].0.y;
                assert!(tip > 0.35 * PHONE.height, "room above: {tip}");
            }
        }
        assert!((2..=3).contains(&max_group), "{max_group}");
        heights.dedup();
        assert!(heights.len() > 10, "all sorts of heights");
    }

    #[test]
    fn groups_of_firs_leave_room_between_them() {
        let mut f = Forest::new(3);
        let mut last_group_right = f32::MIN;
        let mut gaps = Vec::new();
        for _ in 0..ticks(120.0) {
            let before = f.firs.len();
            f.tick(PHONE, true, PLANE, &mut Vec::new());
            if f.firs.len() > before {
                let left = f.firs[before].span(PHONE).0;
                if last_group_right > f32::MIN {
                    gaps.push(left - last_group_right);
                }
                last_group_right = f.firs.last().unwrap().span(PHONE).1;
            } else {
                last_group_right -= SCROLL_SPEED * TICK_SECONDS;
            }
        }
        assert!(!gaps.is_empty());
        for g in gaps {
            assert!(g >= FOREST_ROOM - 2.0, "{g}");
        }
    }

    #[test]
    fn a_fir_is_hit_on_its_branches_and_trunk_not_beside_or_above() {
        let fir = Fir { x: 400.0, top: 0.5 };
        let shape = fir.shape(PHONE);
        let hit = |x: f32, y: f32| any_overlap(&shape, &probe(x, y));
        let p = fir.parts(PHONE);
        assert!(hit(400.0, p.tiers[0].0.y + 4.0), "the tip");
        let (_, b, _) = p.tiers[2];
        assert!(hit(b.x + 3.0, b.y - 2.0), "the lowest branches");
        let (l, t, w, _) = p.trunk;
        assert!(hit(l + w / 2.0, t + 2.0), "the trunk");
        assert!(!hit(400.0, p.tiers[0].0.y - 4.0), "above the tip");
        assert!(!hit(b.x - 6.0, b.y - 2.0), "beside it");
    }

    #[test]
    fn a_buzzard_circles_first_then_cries_and_dives_at_the_plane() {
        let mut f = Forest::new(4);
        let mut events = run(&mut f, FIRST_BUZZARD_SECONDS + 0.05, true);
        assert_eq!(f.buzzards.len(), 1);
        let p = f.buzzards[0].pos;
        let off = p.x > PHONE.width || p.y < 0.0 || p.y > PHONE.height;
        assert!(off, "from off the screen");
        let mut circled = false;
        while !f.buzzards[0].is_diving() {
            circled |= matches!(f.buzzards[0].state, BuzzardState::Circling { .. });
            events.extend(run(&mut f, 1.0 / 60.0, true));
            if matches!(f.buzzards[0].state, BuzzardState::Circling { .. }) {
                let y = f.buzzards[0].pos.y;
                assert!(y < 0.45 * PHONE.height, "it circles up high: {y}");
            }
        }
        assert!(circled, "it circles before it dives");
        assert!(events.contains(&Event::BuzzardCry), "a cry as it dives");
        // It dives toward where the plane was.
        let b = f.buzzards[0];
        let BuzzardState::Diving { vel } = b.state else {
            unreachable!()
        };
        let to_plane = Vec2::new(PLANE.x - b.pos.x, PLANE.y - b.pos.y);
        let cos = (vel.x * to_plane.x + vel.y * to_plane.y)
            / ((vel.x.hypot(vel.y)) * to_plane.x.hypot(to_plane.y));
        assert!(cos > 0.99, "aimed at the plane: {cos}");
    }

    #[test]
    fn buzzards_come_from_ahead_above_and_below_never_from_behind() {
        let mut f = Forest::new(7);
        let mut froms = Vec::new();
        let mut starts = Vec::new();
        let mut last = 0u32;
        let mut gaps = Vec::new();
        for tick in 0..ticks(600.0) {
            let before = f.buzzards.len();
            f.tick(PHONE, true, PLANE, &mut Vec::new());
            if f.buzzards.len() > before {
                let b = *f.buzzards.last().unwrap();
                froms.push(b.from);
                starts.push(b.pos);
                if last > 0 {
                    gaps.push((tick - last) as f32 * TICK_SECONDS);
                }
                last = tick;
            }
            for b in &f.buzzards {
                if !b.is_down() && !b.is_diving() {
                    assert!(
                        b.pos.x > PLANE.x + 100.0,
                        "always ahead of the plane: {b:?}"
                    );
                }
            }
        }
        for from in [BuzzardFrom::Ahead, BuzzardFrom::Above, BuzzardFrom::Below] {
            assert!(froms.contains(&from), "{from:?}");
        }
        for (from, p) in froms.iter().zip(&starts) {
            match from {
                BuzzardFrom::Ahead => assert!(p.x > PHONE.width),
                BuzzardFrom::Above => assert!(p.y < 0.0),
                BuzzardFrom::Below => assert!(p.y > PHONE.height),
            }
        }
        // Irregular: sometimes soon after the last, sometimes long after.
        let (min, max) = gaps
            .iter()
            .fold((f32::MAX, 0.0f32), |(a, b), &g| (a.min(g), b.max(g)));
        assert!(min < 5.0 && max > 10.0, "{min} .. {max}");
    }

    #[test]
    fn a_struck_buzzard_tumbles_down_and_is_gone() {
        let mut f = Forest::new(4);
        run(&mut f, FIRST_BUZZARD_SECONDS + 2.0, true);
        f.buzzards[0].knock_down(PHONE.scale());
        assert!(f.buzzards[0].is_down());
        let y = f.buzzards[0].pos.y;
        run(&mut f, 0.5, false);
        assert!(f.buzzards[0].pos.y > y, "falling");
        run(&mut f, 3.0, false);
        assert!(
            f.buzzards.iter().all(|b| !b.is_down()),
            "fell off the screen"
        );
    }

    #[test]
    fn gusts_sweep_down_from_the_sky_and_slide_with_the_ground() {
        let mut f = Forest::new(5);
        run(&mut f, FIRST_GUST_SECONDS + 0.05, true);
        assert_eq!(f.gusts.len(), 1);
        let (l, t, _, h) = f.gusts[0].rect(PHONE);
        assert!(l > PHONE.width, "from off the right");
        assert_eq!(t, 0.0, "from the top");
        assert!(h < 0.85 * PHONE.height, "not down to the ground");
        let x = f.gusts[0].x;
        run(&mut f, 1.0, false);
        assert!((x - f.gusts[0].x - SCROLL_SPEED).abs() < 1.0);
    }

    #[test]
    fn fog_comes_now_and_then_and_fades() {
        let mut f = Forest::new(6);
        run(&mut f, FIRST_FOG_SECONDS - 0.1, true);
        assert_eq!(f.fog(), 0.0);
        run(&mut f, FOG_FADE_SECONDS + 0.2, true);
        assert_eq!(f.fog(), 1.0);
        run(&mut f, FOG_SECONDS, true);
        assert_eq!(f.fog(), 0.0);
        // A fog going on still fades out when the forest ends.
        let mut f = Forest::new(6);
        run(&mut f, FIRST_FOG_SECONDS + FOG_FADE_SECONDS + 0.2, true);
        run(&mut f, 0.1, false);
        assert!(f.fog() > 0.5);
    }

    #[test]
    fn same_seed_same_forest() {
        let (mut a, mut b) = (Forest::new(9), Forest::new(9));
        run(&mut a, 40.0, true);
        run(&mut b, 40.0, true);
        assert_eq!(a, b);
    }
}
