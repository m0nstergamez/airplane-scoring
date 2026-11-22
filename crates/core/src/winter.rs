//! Winter in the mountains (level 7): firs deep in snow, ski jumps whose
//! jumpers fly at the plane, and snowstorms.
//!
//! * **Snowy firs** stand on the ground in groups of one to three
//!   ([`SNOW_FIR_GROUP`]), like the firs in the autumn forest (level 5).
//!   They are solid: fly over them.
//! * **Ski jumps** come now and then ([`SKI_JUMP_GAP_SECONDS`]): a steel
//!   tower with an inrun down to the take-off on a post. Tower, inrun and
//!   post are solid; under the inrun is free. When it is near the plane ([`SKI_START_AHEAD`]), a jumper sets off at the
//!   top: you see them race down the inrun ([`SKI_INRUN_SECONDS`]), then
//!   they jump and fly at you in a long, falling arc ([`SKI_SPEED`],
//!   [`SKI_GRAVITY`]) and land in the snow. A jumper the plane flies into
//!   costs a life and tumbles down.
//! * **Snowstorms** now and then ([`STORM_GAP_SECONDS`]): for a few
//!   seconds ([`STORM_SECONDS`]) the wind presses the plane down
//!   ([`STORM_PUSH`]) and the game veils the screen in driving snow. It
//!   costs no life, but it can push you into a fir; climbing is stronger.
//!
//! Firs and ski jumps stand on the ground and keep [`WINTER_ROOM`]
//! between them, so there is always a gap to come down into. Positions
//! are screen px; sizes are for a phone and grow with the screen
//! ([`Bounds::scale`]). Seeded, so the same seed gives the same winter.

use crate::forest::Fir;
use crate::rng::Rng;
use crate::shape::Poly;
use crate::{Bounds, Event, Vec2, SCROLL_SPEED, TICK_SECONDS};

/// The level with the winter mountains.
pub const WINTER_LEVEL: u32 = 7;

/// Where the snow is: the ground line, as a fraction of the screen
/// height from the top.
pub const WINTER_GROUND: f32 = crate::huts::HUT_GROUND;
/// The least free ground between two things standing on it (groups of
/// firs, ski jumps), in px.
pub const WINTER_ROOM: f32 = 110.0;

/// Time between two groups of firs, shortest and longest, and the first
/// one, in seconds.
pub const SNOW_FIR_GAP_SECONDS: (f32, f32) = (2.4, 4.0);
pub const FIRST_SNOW_FIR_SECONDS: f32 = 3.0;
/// How many firs stand together, fewest and most.
pub const SNOW_FIR_GROUP: (u32, u32) = (1, 3);
/// A fir's tip, as a fraction of the screen height from the top (tallest
/// and shortest).
pub const SNOW_FIR_TOP: (f32, f32) = (0.42, 0.68);

/// Time between two ski jumps, shortest and longest, and the first, in
/// seconds. Rare: a few per level.
pub const SKI_JUMP_GAP_SECONDS: (f32, f32) = (6.0, 9.0);
pub const FIRST_SKI_JUMP_SECONDS: f32 = 5.0;
/// The top of a ski jump's inrun, as a fraction of the screen height
/// from the top (highest and lowest), and how much lower its take-off
/// is (fraction of the screen height).
pub const SKI_JUMP_TOP: (f32, f32) = (0.36, 0.46);
pub const SKI_JUMP_DROP: (f32, f32) = (0.12, 0.16);
/// A ski jump's sizes, in px: the inrun's length (across) and how thick
/// it is, the tower at its top (width) and the post under the take-off
/// (width).
pub const SKI_INRUN: f32 = 120.0;
pub const SKI_DECK: f32 = 7.0;
pub const SKI_TOWER: f32 = 26.0;
pub const SKI_POST: f32 = 6.0;
/// How far ahead of the plane a ski jump's take-off is when its jumper
/// sets off at the top, nearest and furthest, in px; how long the
/// jumper takes down the inrun, in seconds.
pub const SKI_START_AHEAD: (f32, f32) = (330.0, 560.0);
pub const SKI_INRUN_SECONDS: f32 = 0.9;
/// A jumper's flight: speed toward the plane and up off the take-off, in
/// px per second, and how fast they sink, in px per second, each second
/// (all at scale 1, on top of the landscape sliding past).
pub const SKI_SPEED: f32 = 200.0;
pub const SKI_LIFT: f32 = 40.0;
pub const SKI_GRAVITY: f32 = 150.0;
/// A jumper's skis, from behind their boots to the tips, in px.
pub const SKI_LENGTH: (f32, f32) = (14.0, 22.0);

/// Time between two snowstorms, shortest and longest, and the first, in
/// seconds; how long one lasts and how long it takes to come and go.
pub const STORM_GAP_SECONDS: (f32, f32) = (10.0, 15.0);
pub const FIRST_STORM_SECONDS: f32 = 8.0;
pub const STORM_SECONDS: f32 = 4.5;
pub const STORM_FADE_SECONDS: f32 = 0.7;
/// How hard a full storm presses the plane down, in px per second, each
/// second (at scale 1). Weaker than a gust (level 5), as it lasts longer;
/// climbing is stronger.
pub const STORM_PUSH: f32 = 500.0;

/// Things start and are dropped this far outside the screen, in px.
const MARGIN: f32 = 40.0;

fn ticks(seconds: f32) -> u32 {
    (seconds / TICK_SECONDS).round().max(1.0) as u32
}

/// An upright rectangle: left, top, width, height, in px.
pub type Rect = (f32, f32, f32, f32);

/// A ski jump: a steel tower with an inrun down to the take-off, which
/// faces the plane, on a post.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkiJump {
    /// The take-off's x on screen, in px.
    pub x: f32,
    /// The inrun's top, as a fraction of the screen height from the top.
    pub top: f32,
    /// The take-off, as a fraction of the screen height from the top.
    pub lip: f32,
    /// How far ahead of the plane the take-off is when the jumper sets
    /// off, in px at scale 1.
    pub start: f32,
    /// Ticks since the jumper set off down the inrun; `None` before.
    pub slide: Option<u32>,
    /// Has the jumper left the take-off?
    pub jumped: bool,
}

impl SkiJump {
    /// The take-off edge and the inrun's top, in px.
    pub fn inrun(&self, bounds: Bounds) -> (Vec2, Vec2) {
        let s = bounds.scale();
        (
            Vec2::new(self.x, self.lip * bounds.height),
            Vec2::new(self.x + SKI_INRUN * s, self.top * bounds.height),
        )
    }

    /// Where its parts are, in px: the tower (left, top, width, height,
    /// from the inrun's top down to the snow) and the post under the
    /// take-off (the same).
    pub fn parts(&self, bounds: Bounds) -> (Rect, Rect) {
        let s = bounds.scale();
        let ground = WINTER_GROUND * bounds.height;
        let (lip, top) = self.inrun(bounds);
        let post = SKI_POST * s;
        (
            (top.x - 2.0 * s, top.y, SKI_TOWER * s, ground - top.y),
            (lip.x, lip.y, post, ground - lip.y),
        )
    }

    /// The ski jump as drawn, for crashing: the inrun, the tower and the
    /// post. Under the inrun is free.
    pub fn shape(&self, bounds: Bounds) -> Vec<Poly> {
        let (lip, top) = self.inrun(bounds);
        let ((tl, tt, tw, th), (pl, pt, pw, ph)) = self.parts(bounds);
        let half = SKI_DECK * bounds.scale() / 2.0;
        // (the inrun's top side is where the jumper's skis run)
        let down = |p: Vec2| Vec2::new(p.x, p.y + half);
        vec![
            Poly::line(down(lip), down(top), 2.0 * half),
            Poly::rect(tl, tt, tw, th),
            Poly::rect(pl, pt, pw, ph),
        ]
    }

    /// Its leftmost and rightmost x, in px.
    pub fn span(&self, bounds: Bounds) -> (f32, f32) {
        let ((tl, _, tw, _), (pl, _, _, _)) = self.parts(bounds);
        (pl, tl + tw)
    }

    /// The jumper while on the inrun: where their boots are and which way
    /// they head (radians, 0 = right). `None` before they set off and
    /// after they jumped.
    pub fn rider(&self, bounds: Bounds) -> Option<(Vec2, f32)> {
        let t = self.slide?;
        if self.jumped {
            return None;
        }
        let (lip, top) = self.inrun(bounds);
        // Faster and faster down the inrun.
        let f = (t as f32 * TICK_SECONDS / SKI_INRUN_SECONDS).min(1.0);
        let f = f * f;
        let at = Vec2::new(top.x + (lip.x - top.x) * f, top.y + (lip.y - top.y) * f);
        Some((at, libm::atan2f(lip.y - top.y, lip.x - top.x)))
    }
}

/// What a ski jumper is doing after the take-off.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JumperState {
    /// In the air.
    Flying,
    /// Down in the snow, gliding off.
    Landed,
    /// Struck by the plane: tumbling down, for this many ticks so far.
    Down { ticks: u32 },
}

/// A ski jumper in the air.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Jumper {
    /// Where their boots are on screen, in px.
    pub pos: Vec2,
    /// How fast they move over the snow, in px per second (the landscape
    /// sliding past comes on top).
    pub vel: Vec2,
    pub state: JumperState,
}

impl Jumper {
    /// Was it struck? Then it tumbles down and cannot be hit again.
    pub fn is_down(&self) -> bool {
        matches!(self.state, JumperState::Down { .. })
    }

    /// Which way the skis point, in radians (0 = right): along the
    /// flight, turning over when struck.
    pub fn heading(&self) -> f32 {
        let along = libm::atan2f(self.vel.y, self.vel.x);
        match self.state {
            JumperState::Down { ticks } => along + ticks as f32 * TICK_SECONDS * 7.0,
            JumperState::Landed => std::f32::consts::PI,
            JumperState::Flying => along,
        }
    }

    /// Struck by the plane: it tumbles down.
    pub fn knock_down(&mut self, scale: f32) {
        if self.is_down() {
            return;
        }
        self.vel = Vec2::new(self.vel.x * 0.3, -60.0 * scale);
        self.state = JumperState::Down { ticks: 0 };
    }

    /// The jumper as drawn, for crashing: skis and body.
    pub fn shape(&self, scale: f32) -> Vec<Poly> {
        jumper_shape(self.pos, self.heading(), scale)
    }
}

/// Where a jumper's parts are, with their boots at `at` heading `angle`
/// (radians, 0 = right): skis, body (boots to shoulders) and head, in px.
pub fn jumper_parts(at: Vec2, angle: f32, scale: f32) -> ((Vec2, Vec2), (Vec2, Vec2), Vec2) {
    let along = Vec2::new(libm::cosf(angle), libm::sinf(angle));
    // "Up" from the skis: to the left of the heading when it points
    // left, so the jumper stands on their skis either way.
    let up = if along.x <= 0.0 {
        Vec2::new(along.y, -along.x)
    } else {
        Vec2::new(-along.y, along.x)
    };
    let up = if up.y > 0.0 {
        Vec2::new(-up.x, -up.y)
    } else {
        up
    };
    let p = |u: f32, v: f32| {
        Vec2::new(
            at.x + (along.x * u + up.x * v) * scale,
            at.y + (along.y * u + up.y * v) * scale,
        )
    };
    // Leaning far forward over the skis, as jumpers fly.
    (
        (p(-SKI_LENGTH.0, 0.0), p(SKI_LENGTH.1, 0.0)),
        (p(-3.0, 3.0), p(13.0, 14.0)),
        p(17.0, 17.0),
    )
}

fn jumper_shape(at: Vec2, angle: f32, scale: f32) -> Vec<Poly> {
    let (skis, body, head) = jumper_parts(at, angle, scale);
    vec![
        Poly::line(skis.0, skis.1, 3.0 * scale),
        Poly::line(body.0, head, 8.0 * scale),
    ]
}

/// The winter mountains' firs, ski jumps, jumpers and snowstorms.
#[derive(Debug, Clone, PartialEq)]
pub struct Winter {
    /// Snowy firs on the ground, oldest first.
    pub firs: Vec<Fir>,
    /// Ski jumps, oldest first.
    pub jumps: Vec<SkiJump>,
    /// Jumpers that left the take-off.
    pub jumpers: Vec<Jumper>,
    /// Ticks into the storm now; `None` between storms.
    storm: Option<u32>,
    next_fir: u32,
    next_jump: u32,
    next_storm: u32,
    rng: Rng,
}

impl Winter {
    pub fn new(seed: u64) -> Self {
        Self {
            firs: Vec::new(),
            jumps: Vec::new(),
            jumpers: Vec::new(),
            storm: None,
            next_fir: ticks(FIRST_SNOW_FIR_SECONDS),
            next_jump: ticks(FIRST_SKI_JUMP_SECONDS),
            next_storm: ticks(FIRST_STORM_SECONDS),
            // Its own stream, so the winter does not change other things.
            rng: Rng::new(seed ^ 0x5A0E_7000_0000_0007),
        }
    }

    /// Clear it all away (a new level).
    pub fn clear(&mut self) {
        self.firs.clear();
        self.jumps.clear();
        self.jumpers.clear();
        self.storm = None;
    }

    /// Everything slides further left by `dx` px (the game going faster):
    /// the jumpers fly over the snow, so they slide with it.
    pub fn shift(&mut self, dx: f32) {
        self.firs.iter_mut().for_each(|f| f.x += dx);
        self.jumps.iter_mut().for_each(|j| j.x += dx);
        self.jumpers.iter_mut().for_each(|j| j.pos.x += dx);
    }

    /// How strong the storm is now: 0 = none, 1 = full, coming and going
    /// over [`STORM_FADE_SECONDS`].
    pub fn storm(&self) -> f32 {
        let Some(t) = self.storm else { return 0.0 };
        let t = t as f32 * TICK_SECONDS;
        let fade_in = (t / STORM_FADE_SECONDS).min(1.0);
        let fade_out = ((STORM_SECONDS - t) / STORM_FADE_SECONDS).clamp(0.0, 1.0);
        fade_in.min(fade_out)
    }

    /// The jumpers that can be hit now, with their shapes: those on an
    /// inrun (by the ski jump's index, `Err`) and those in the air or in
    /// the snow (by their own index, `Ok`).
    pub fn jumper_shapes(&self, bounds: Bounds) -> Vec<(Result<usize, usize>, Vec<Poly>)> {
        let s = bounds.scale();
        let riding = (self.jumps.iter().enumerate()).filter_map(|(i, j)| {
            j.rider(bounds)
                .map(|(at, a)| (Err(i), jumper_shape(at, a, s)))
        });
        let flying = (self.jumpers.iter().enumerate())
            .filter(|(_, j)| !j.is_down())
            .map(|(i, j)| (Ok(i), j.shape(s)));
        riding.chain(flying).collect()
    }

    /// The plane flew into a jumper (an index from
    /// [`Winter::jumper_shapes`]): it tumbles down. One still on the
    /// inrun is thrown off it.
    pub fn knock_down(&mut self, which: Result<usize, usize>, bounds: Bounds) {
        let s = bounds.scale();
        match which {
            Ok(i) => self.jumpers[i].knock_down(s),
            Err(i) => {
                let Some((pos, a)) = self.jumps[i].rider(bounds) else {
                    return;
                };
                self.jumps[i].jumped = true;
                let speed = SKI_SPEED * s;
                let mut j = Jumper {
                    pos,
                    vel: Vec2::new(libm::cosf(a) * speed, libm::sinf(a) * speed),
                    state: JumperState::Flying,
                };
                j.knock_down(s);
                self.jumpers.push(j);
            }
        }
    }

    /// Advance one tick. `active`: is the winter on screen (nothing new
    /// comes while it is not)? `plane_x`: where the plane is, so jumpers
    /// set off as it nears. Jumps and storms go to `events`.
    pub fn tick(&mut self, bounds: Bounds, active: bool, plane_x: f32, events: &mut Vec<Event>) {
        let s = bounds.scale();
        let dt = TICK_SECONDS;
        let dx = -SCROLL_SPEED * s * dt;
        let ground = WINTER_GROUND * bounds.height;
        for f in &mut self.firs {
            f.x += dx;
        }
        let mut jumped = false;
        for jump in &mut self.jumps {
            jump.x += dx;
            match jump.slide {
                None if jump.x - plane_x <= jump.start * s => jump.slide = Some(0),
                Some(t) if !jump.jumped => {
                    if t >= ticks(SKI_INRUN_SECONDS) {
                        // Off the take-off, toward the plane.
                        jump.jumped = true;
                        jumped = true;
                        let (lip, _) = jump.inrun(bounds);
                        self.jumpers.push(Jumper {
                            pos: lip,
                            vel: Vec2::new(-SKI_SPEED * s, -SKI_LIFT * s),
                            state: JumperState::Flying,
                        });
                    } else {
                        jump.slide = Some(t + 1);
                    }
                }
                _ => {}
            }
        }
        if jumped {
            events.push(Event::SkiJump);
        }
        for j in &mut self.jumpers {
            match j.state {
                JumperState::Flying => {
                    j.vel.y += SKI_GRAVITY * s * dt;
                    j.pos.x += j.vel.x * dt + dx;
                    j.pos.y += j.vel.y * dt;
                    if j.pos.y >= ground {
                        j.pos.y = ground;
                        j.vel = Vec2::new(j.vel.x * 0.6, 0.0);
                        j.state = JumperState::Landed;
                    }
                }
                JumperState::Landed => j.pos.x += j.vel.x * dt + dx,
                JumperState::Down { ticks: t } => {
                    j.vel.y += 650.0 * s * dt;
                    j.pos.x += j.vel.x * dt + dx;
                    j.pos.y += j.vel.y * dt;
                    j.state = JumperState::Down { ticks: t + 1 };
                }
            }
        }
        let m = MARGIN * s;
        self.firs.retain(|f| f.span(bounds).1 > -m);
        self.jumps.retain(|j| j.span(bounds).1 > -m);
        let h = bounds.height;
        self.jumpers
            .retain(|j| j.pos.x > -2.0 * m && j.pos.y < h + 2.0 * m);
        if let Some(t) = &mut self.storm {
            *t += 1;
            if *t >= ticks(STORM_SECONDS) {
                self.storm = None;
            }
        }

        if !active {
            self.next_fir = ticks(FIRST_SNOW_FIR_SECONDS);
            self.next_jump = ticks(FIRST_SKI_JUMP_SECONDS);
            self.next_storm = ticks(FIRST_STORM_SECONDS);
            return; // (a storm going on still dies down)
        }
        let crowded = self.crowded(bounds);
        self.next_fir -= 1;
        if self.next_fir == 0 && crowded {
            self.next_fir = ticks(0.2);
        }
        if self.next_fir == 0 {
            self.spawn_firs(bounds);
            let gap = self
                .rng
                .range(SNOW_FIR_GAP_SECONDS.0, SNOW_FIR_GAP_SECONDS.1);
            self.next_fir = ticks(gap);
        }
        let crowded = self.crowded(bounds);
        self.next_jump -= 1;
        if self.next_jump == 0 && crowded {
            self.next_jump = ticks(0.2);
        }
        if self.next_jump == 0 {
            let r = &mut self.rng;
            let top = r.range(SKI_JUMP_TOP.0, SKI_JUMP_TOP.1);
            let lip = top + r.range(SKI_JUMP_DROP.0, SKI_JUMP_DROP.1);
            let start = r.range(SKI_START_AHEAD.0, SKI_START_AHEAD.1);
            self.jumps.push(SkiJump {
                // (its post starts just off the screen)
                x: bounds.width + MARGIN * s,
                top,
                lip,
                start,
                slide: None,
                jumped: false,
            });
            let gap = r.range(SKI_JUMP_GAP_SECONDS.0, SKI_JUMP_GAP_SECONDS.1);
            self.next_jump = ticks(gap);
        }
        self.next_storm -= 1;
        if self.next_storm == 0 {
            self.storm = Some(0);
            events.push(Event::Blizzard);
            let gap = self.rng.range(STORM_GAP_SECONDS.0, STORM_GAP_SECONDS.1);
            self.next_storm = ticks(gap);
        }
    }

    /// Is the last fir or ski jump still too close to the right edge for
    /// the next one?
    fn crowded(&self, bounds: Bounds) -> bool {
        let last = (self.firs.iter().map(|f| f.span(bounds).1))
            .chain(self.jumps.iter().map(|j| j.span(bounds).1))
            .fold(f32::MIN, f32::max);
        last > bounds.width - WINTER_ROOM * bounds.scale()
    }

    /// One to three firs of different heights, just off the right edge.
    fn spawn_firs(&mut self, bounds: Bounds) {
        let s = bounds.scale();
        let r = &mut self.rng;
        let n =
            SNOW_FIR_GROUP.0 + (r.unit() * (SNOW_FIR_GROUP.1 - SNOW_FIR_GROUP.0 + 1) as f32) as u32;
        let n = n.min(SNOW_FIR_GROUP.1);
        let mut x = bounds.width + MARGIN * s;
        for _ in 0..n {
            let fir = Fir {
                x,
                top: r.range(SNOW_FIR_TOP.0, SNOW_FIR_TOP.1),
            };
            // (x is the trunk; move it in by its half width first)
            let half = fir.x - fir.span(bounds).0;
            self.firs.push(Fir { x: x + half, ..fir });
            x += half + r.range(26.0, 40.0) * s;
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
    const PLANE_X: f32 = 0.2 * 844.0;

    fn run(w: &mut Winter, seconds: f32, active: bool) -> Vec<Event> {
        let mut events = Vec::new();
        for _ in 0..(seconds / TICK_SECONDS) as u32 {
            w.tick(PHONE, active, PLANE_X, &mut events);
        }
        events
    }

    fn probe(x: f32, y: f32) -> Vec<Poly> {
        vec![Poly::rect(x - 2.0, y - 2.0, 4.0, 4.0)]
    }

    fn a_jump() -> SkiJump {
        SkiJump {
            x: 600.0,
            top: 0.4,
            lip: 0.54,
            start: 450.0,
            slide: None,
            jumped: false,
        }
    }

    #[test]
    fn nothing_away_from_the_winter() {
        let mut w = Winter::new(7);
        let events = run(&mut w, 60.0, false);
        assert!(w.firs.is_empty() && w.jumps.is_empty() && w.jumpers.is_empty());
        assert!(events.is_empty());
        assert_eq!(w.storm(), 0.0);
    }

    #[test]
    fn firs_and_ski_jumps_come_with_room_between_them() {
        let mut w = Winter::new(3);
        let mut seen = (0, 0);
        for _ in 0..(60.0 / TICK_SECONDS) as u32 {
            let (firs, jumps) = (w.firs.len(), w.jumps.len());
            w.tick(PHONE, true, PLANE_X, &mut Vec::new());
            seen.0 += w.firs.len().saturating_sub(firs);
            seen.1 += w.jumps.len().saturating_sub(jumps);
            // Groups and jumps never overlap: sorted by where they stand,
            // each starts after the last ended (firs of one group may
            // stand close).
            let mut spans: Vec<(f32, f32, bool)> = (w.firs.iter().map(|f| {
                let (l, r) = f.span(PHONE);
                (l, r, false)
            }))
            .chain(w.jumps.iter().map(|j| {
                let (l, r) = j.span(PHONE);
                (l, r, true)
            }))
            .collect();
            spans.sort_by(|a, b| a.0.total_cmp(&b.0));
            for pair in spans.windows(2) {
                if pair[0].2 || pair[1].2 {
                    assert!(pair[1].0 - pair[0].1 >= WINTER_ROOM - 1.0, "{pair:?}");
                }
            }
        }
        assert!((12..=30).contains(&seen.0), "firs: {}", seen.0);
        assert!((4..=10).contains(&seen.1), "ski jumps: {}", seen.1);
    }

    #[test]
    fn a_ski_jump_is_solid_on_its_inrun_tower_and_post_free_under_the_inrun() {
        let j = a_jump();
        let (lip, top) = j.inrun(PHONE);
        let jump = j.shape(PHONE);
        let mid = Vec2::new((lip.x + top.x) / 2.0, (lip.y + top.y) / 2.0);
        assert!(any_overlap(&probe(mid.x, mid.y + 3.0), &jump), "the inrun");
        assert!(any_overlap(&probe(top.x + 8.0, 300.0), &jump), "the tower");
        assert!(any_overlap(&probe(lip.x + 3.0, 300.0), &jump), "the post");
        assert!(
            !any_overlap(&probe(mid.x, mid.y + 40.0), &jump),
            "under the inrun"
        );
        assert!(!any_overlap(&probe(top.x, top.y - 15.0), &jump), "over it");
        assert!(
            !any_overlap(&probe(lip.x - 10.0, lip.y), &jump),
            "in front of the take-off"
        );
    }

    #[test]
    fn a_jumper_sets_off_as_the_plane_nears_races_down_and_flies_at_it() {
        let mut w = Winter::new(1);
        w.jumps.push(SkiJump {
            x: PLANE_X + 700.0,
            ..a_jump()
        });
        let mut events = Vec::new();
        let mut set_off_at = None;
        let mut on_inrun = Vec::new();
        while w.jumpers.is_empty() {
            w.tick(PHONE, false, PLANE_X, &mut events);
            let j = w.jumps[0];
            if j.slide == Some(0) {
                set_off_at = Some(j.x - PLANE_X);
            }
            if let Some((at, _)) = j.rider(PHONE) {
                on_inrun.push(at);
            }
        }
        let ahead = set_off_at.expect("set off");
        assert!((ahead - 450.0).abs() < 2.0, "sets off {ahead} px ahead");
        // Down the inrun, faster and faster.
        let first = on_inrun[0];
        let last = *on_inrun.last().unwrap();
        assert!(last.y > first.y + 40.0 && last.x < first.x);
        assert_eq!(events, vec![Event::SkiJump], "one jump sound");
        // Then in the air toward the plane: up off the take-off a little,
        // then down in a long arc, landing in the snow.
        let lip = w.jumps[0].inrun(PHONE).0;
        let mut path = Vec::new();
        while w.jumpers[0].state == JumperState::Flying {
            path.push(w.jumpers[0].pos);
            w.tick(PHONE, false, PLANE_X, &mut Vec::new());
        }
        let peak = path.iter().map(|p| p.y).fold(f32::MAX, f32::min);
        assert!(peak < lip.y && peak > lip.y - 15.0, "a small rise: {peak}");
        let landed = w.jumpers[0].pos;
        assert_eq!(landed.y, WINTER_GROUND * PHONE.height);
        assert!(landed.x < lip.x - 300.0, "far: {} from {}", landed.x, lip.x);
        let secs = path.len() as f32 * TICK_SECONDS;
        assert!((1.2..2.4).contains(&secs), "in the air {secs} s");
    }

    #[test]
    fn jumpers_cross_the_plane_at_many_heights() {
        // Taking off 250 to 480 px ahead, they pass the plane anywhere
        // from about the take-off's height down to the snow.
        let mut heights = Vec::new();
        for start in [330.0, 400.0, 480.0, 560.0] {
            let mut w = Winter::new(1);
            w.jumps.push(SkiJump {
                x: PLANE_X + 700.0,
                start,
                ..a_jump()
            });
            for _ in 0..(8.0 / TICK_SECONDS) as u32 {
                let before = w.jumpers.first().map(|j| j.pos);
                w.tick(PHONE, false, PLANE_X, &mut Vec::new());
                if let (Some(a), Some(j)) = (before, w.jumpers.first()) {
                    if a.x >= PLANE_X && j.pos.x < PLANE_X {
                        heights.push(j.pos.y);
                    }
                }
            }
        }
        let (lo, hi) = (
            heights.iter().cloned().fold(f32::MAX, f32::min),
            heights.iter().cloned().fold(f32::MIN, f32::max),
        );
        assert!(heights.len() >= 3, "{heights:?}");
        assert!(hi - lo > 80.0, "{heights:?}");
    }

    #[test]
    fn a_struck_jumper_tumbles_down_and_is_gone() {
        let mut w = Winter::new(1);
        w.jumpers.push(Jumper {
            pos: Vec2::new(400.0, 200.0),
            vel: Vec2::new(-SKI_SPEED, -SKI_LIFT),
            state: JumperState::Flying,
        });
        assert_eq!(w.jumper_shapes(PHONE).len(), 1);
        w.knock_down(Ok(0), PHONE);
        assert!(w.jumpers[0].is_down());
        assert!(w.jumper_shapes(PHONE).is_empty(), "no obstacle any more");
        run(&mut w, 3.0, false);
        assert!(w.jumpers.is_empty());
    }

    #[test]
    fn a_jumper_struck_on_the_inrun_is_thrown_off_it() {
        let mut w = Winter::new(1);
        w.jumps.push(SkiJump {
            slide: Some(10),
            ..a_jump()
        });
        let shapes = w.jumper_shapes(PHONE);
        assert_eq!(shapes.len(), 1);
        w.knock_down(shapes[0].0, PHONE);
        assert!(w.jumps[0].rider(PHONE).is_none());
        assert!(w.jumpers[0].is_down());
        let events = run(&mut w, 2.0, false);
        assert!(!events.contains(&Event::SkiJump), "no jump after it");
    }

    #[test]
    fn storms_come_now_and_then_and_fade() {
        let mut w = Winter::new(5);
        let mut events = Vec::new();
        let mut strengths = Vec::new();
        for _ in 0..(60.0 / TICK_SECONDS) as u32 {
            w.tick(PHONE, true, PLANE_X, &mut events);
            strengths.push(w.storm());
        }
        let storms = events.iter().filter(|e| **e == Event::Blizzard).count();
        assert!((3..=6).contains(&storms), "{storms}");
        assert!(strengths.contains(&1.0));
        assert!(strengths.iter().any(|&s| s > 0.2 && s < 0.8), "it fades");
        for pair in strengths.windows(2) {
            assert!((pair[1] - pair[0]).abs() < 0.05, "smooth");
        }
        let calm = strengths.iter().filter(|&&s| s == 0.0).count() as f32;
        assert!(calm / strengths.len() as f32 > 0.5, "mostly calm");
    }

    #[test]
    fn same_seed_same_winter() {
        let (mut a, mut b) = (Winter::new(9), Winter::new(9));
        let (ea, eb) = (run(&mut a, 40.0, true), run(&mut b, 40.0, true));
        assert_eq!(a, b);
        assert_eq!(ea, eb);
    }
}
