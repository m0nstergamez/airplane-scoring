//! Sunrise (level 4): flocks of geese, columns of steam and the low sun.
//!
//! * **Geese** come at the plane in a V, fast, every few seconds
//!   ([`GOOSE_GAP_SECONDS`]). The V is aimed at the plane's height: its
//!   tip or one of its arms is where the plane is, so fly over or under
//!   the whole V. Each flock calls as it appears, never twice the same
//!   way in a row ([`GOOSE_CALLS`]). Fly into one and you crash;
//!   the goose is knocked down and tumbles to the ground.
//! * **Steam** rises from the chimney of a house in the valley: a column
//!   from the chimney up to some height, sliding past with the ground.
//!   Flying through it costs no life, but the warm air throws the plane
//!   up ([`STEAM_LIFT`]), so you have to catch it again. The house is
//!   small (smaller than the town's high-rises) but solid: flying into it
//!   is a crash.
//! * **High-rises** stand far back in the valley, black against the
//!   sun, like the one in the photo. They are scenery, not obstacles
//!   (the plane flies in front of them), and like everything else they
//!   only come while no runway is on screen, so none stands by a runway.
//! * **Glare**: now and then the low sun ahead blinds you for a moment
//!   ([`GLARE_SECONDS`]); the game washes the screen in its light.
//!
//! Positions are screen px; sizes are for a phone and grow with the
//! screen ([`Bounds::scale`]). Seeded, so the same seed gives the same
//! morning.

use crate::rng::Rng;
use crate::shape::Poly;
use crate::{Bounds, Event, Vec2, SCROLL_SPEED, TICK_SECONDS};

/// The level with the sunrise.
pub const SUNRISE_LEVEL: u32 = 4;

/// Where the sun stands, fixed in the sky ahead, as fractions of the
/// screen (from the left, from the top), and its radius in px. The game
/// draws it; the glare comes from there.
pub const SUN_AT: (f32, f32) = (0.78, 0.56);
pub const SUN_RADIUS: f32 = 20.0;

/// Time between two flocks of geese, shortest and longest, in seconds;
/// none in the first seconds of the level.
pub const GOOSE_GAP_SECONDS: (f32, f32) = (3.0, 5.0);
pub const FIRST_GOOSE_SECONDS: f32 = 3.0;
/// How fast a flock flies at the plane, in px per second (on its own,
/// the ground sliding by comes on top for nothing: geese fly in the air).
pub const GOOSE_SPEED: (f32, f32) = (190.0, 260.0);
/// How many geese fly in each arm of the V behind the leader.
pub const GOOSE_ARM: (u32, u32) = (2, 3);
/// The step from one goose to the next along an arm: back (right) and
/// out (up or down), in px.
pub const GOOSE_STEP: (f32, f32) = (24.0, 15.0);
/// How many different calls a flock can make as it appears (the game
/// has one sound for each; see `Event::GooseHonk`).
pub const GOOSE_CALLS: u8 = 4;
/// A goose's half length and half height (its body), in px.
pub const GOOSE_BODY: (f32, f32) = (11.0, 4.5);

/// A struck goose: how hard it is thrown up, how fast it then falls (px
/// per second squared) and how much of its speed it keeps.
pub const GOOSE_HIT_POP: f32 = 80.0;
pub const GOOSE_FALL: f32 = 650.0;
pub const GOOSE_HIT_KEEP: f32 = 0.3;

/// Time between two columns of steam, shortest and longest, in seconds;
/// the first this long after the level begins.
pub const STEAM_GAP_SECONDS: (f32, f32) = (5.0, 8.0);
pub const FIRST_STEAM_SECONDS: f32 = 4.0;
/// A column's width in px, and how high it reaches (its top, as a
/// fraction of the screen height from the top).
pub const STEAM_WIDTH: (f32, f32) = (40.0, 64.0);
pub const STEAM_TOP: (f32, f32) = (0.3, 0.55);
/// Where the houses stand: the ground line, as a fraction of the screen
/// height from the top.
pub const STEAM_GROUND: f32 = crate::huts::HUT_GROUND;
/// A house under a column of steam: its width and its walls' height,
/// narrowest and widest, lowest and highest, in px; its roof's height,
/// and the chimney (width, how far it sticks out over the roof).
pub const HOUSE_WIDTH: (f32, f32) = (56.0, 84.0);
pub const HOUSE_HEIGHT: (f32, f32) = (34.0, 56.0);
pub const HOUSE_ROOF: f32 = 16.0;
pub const CHIMNEY: (f32, f32) = (9.0, 16.0);
/// How hard the warm air throws the plane up while it is in a column,
/// in px per second, each second (at scale 1).
pub const STEAM_LIFT: f32 = 900.0;

/// High-rises far back: time between two, shortest and longest, and the
/// first, in seconds; width in px; their roof, as a fraction of the screen
/// height from the top (they stand on the ground line).
pub const FAR_HIGH_RISE_GAP_SECONDS: (f32, f32) = (7.0, 12.0);
pub const FIRST_FAR_HIGH_RISE_SECONDS: f32 = 1.5;
pub const FAR_HIGH_RISE_WIDTH: (f32, f32) = (90.0, 130.0);
pub const FAR_HIGH_RISE_TOP: (f32, f32) = (0.5, 0.6);
/// The least room between a high-rise and a house with its steam, in px:
/// one waits until the other has moved on, so they never stand in each
/// other.
pub const SUNRISE_ROOM: f32 = 30.0;

/// Time between two glares, shortest and longest, and the first one, in
/// seconds; how long one lasts, and how long it takes to come and go.
pub const GLARE_GAP_SECONDS: (f32, f32) = (10.0, 16.0);
pub const FIRST_GLARE_SECONDS: f32 = 8.0;
pub const GLARE_SECONDS: f32 = 2.5;
pub const GLARE_FADE_SECONDS: f32 = 0.6;

/// Geese start and are dropped this far outside the screen, in px.
const MARGIN: f32 = 40.0;

fn ticks(seconds: f32) -> u32 {
    (seconds / TICK_SECONDS).round().max(1.0) as u32
}

/// One goose of a V.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Goose {
    /// Its body's centre on screen, in px.
    pub pos: Vec2,
    /// px per second.
    pub vel: Vec2,
    /// Ticks since its flock appeared (for the wing beat).
    pub age: u32,
    /// Ticks since it was struck; `None` while it flies.
    pub down: Option<u32>,
    /// Its fall, px per second squared (0 while it flies).
    fall: f32,
}

impl Goose {
    /// Was it struck? Then it tumbles down and cannot be hit again.
    pub fn is_down(&self) -> bool {
        self.down.is_some()
    }

    /// Struck by the plane: thrown up a little, slowed, then it falls.
    pub fn knock_down(&mut self, scale: f32) {
        if self.is_down() {
            return;
        }
        self.down = Some(0);
        self.vel = Vec2::new(self.vel.x * GOOSE_HIT_KEEP, -GOOSE_HIT_POP * scale);
        self.fall = GOOSE_FALL * scale;
    }

    /// The wings' position, -1 (down) .. 1 (up): slow, strong beats; a
    /// struck goose's wings hang up.
    pub fn wing_beat(&self) -> f32 {
        if self.is_down() {
            return 0.8;
        }
        (self.age as f32 * TICK_SECONDS * std::f32::consts::TAU * 2.2).sin()
    }

    /// How far a struck goose has turned over, in radians.
    pub fn tumble(&self) -> f32 {
        -(self.down.unwrap_or(0) as f32 * TICK_SECONDS * 1.2 * std::f32::consts::TAU)
    }

    /// Where its parts are, relative to the centre at `scale`: the head
    /// (on a long neck, ahead, to the left) and the tip of the near wing.
    pub fn head(&self, scale: f32) -> Vec2 {
        Vec2::new(self.pos.x - 16.0 * scale, self.pos.y - 2.0 * scale)
    }

    pub fn wing_tip(&self, scale: f32) -> Vec2 {
        let beat = self.wing_beat();
        Vec2::new(
            self.pos.x + 2.0 * scale,
            self.pos.y - (3.0 + 11.0 * beat) * scale,
        )
    }

    /// The goose as drawn, for crashing: body, neck and head, the wing.
    pub fn shape(&self, scale: f32) -> Vec<Poly> {
        let s = scale;
        let (hl, hh) = (GOOSE_BODY.0 * s, GOOSE_BODY.1 * s);
        let head = self.head(s);
        vec![
            Poly::ellipse(self.pos, hl, hh),
            Poly::line(Vec2::new(self.pos.x - hl * 0.6, self.pos.y), head, 3.0 * s),
            Poly::ellipse(head, 3.0 * s, 2.5 * s),
            Poly::line(self.pos, self.wing_tip(s), 4.0 * s),
        ]
    }

    fn step(&mut self) {
        self.vel.y += self.fall * TICK_SECONDS;
        self.pos.x += self.vel.x * TICK_SECONDS;
        self.pos.y += self.vel.y * TICK_SECONDS;
        self.age += 1;
        if let Some(t) = &mut self.down {
            *t += 1;
        }
    }

    fn gone(&self, bounds: Bounds) -> bool {
        let m = 2.0 * MARGIN * bounds.scale();
        let p = self.pos;
        p.x < -m || p.y > bounds.height + m
    }
}

/// A house in the valley with a column of steam rising from its
/// chimney.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Steam {
    /// The chimney's (and the column's) middle on screen, in px.
    pub x: f32,
    /// The column's width in px at scale 1.
    pub width: f32,
    /// The column's top, as a fraction of the screen height.
    pub top: f32,
    /// The house: width and walls' height, in px at scale 1.
    pub house: (f32, f32),
    /// Ticks since it appeared (for the drawn billows).
    pub age: u32,
}

/// Where a house's parts are on screen, in px.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HouseParts {
    /// The walls: left, top, width, height.
    pub walls: (f32, f32, f32, f32),
    /// The roof's ridge (its top point); the roof runs from the walls'
    /// top corners up to it.
    pub ridge: Vec2,
    /// The chimney: left, top, width, height (from the roof's line up).
    pub chimney: (f32, f32, f32, f32),
}

impl Steam {
    /// Where the house is on this screen. The chimney stands on the
    /// house's left (front) half of the roof.
    pub fn house_parts(&self, bounds: Bounds) -> HouseParts {
        let s = bounds.scale();
        let ground = STEAM_GROUND * bounds.height;
        let (w, h) = (self.house.0 * s, self.house.1 * s);
        let left = self.x - 0.3 * w;
        let top = ground - h;
        let (cw, ch) = (CHIMNEY.0 * s, CHIMNEY.1 * s);
        // The chimney rises from the roof's slope to above the ridge.
        let roof = HOUSE_ROOF * s;
        let chimney_top = top - roof - ch + roof * 0.4;
        HouseParts {
            walls: (left, top, w, h),
            ridge: Vec2::new(left + w / 2.0, top - roof),
            chimney: (self.x - cw / 2.0, chimney_top, cw, top - chimney_top),
        }
    }

    /// The house as drawn, for crashing: walls, roof and chimney.
    pub fn house_shape(&self, bounds: Bounds) -> Vec<Poly> {
        let p = self.house_parts(bounds);
        let (l, t, w, h) = p.walls;
        let (cl, ct, cw, ch) = p.chimney;
        vec![
            Poly::rect(l, t, w, h),
            Poly::at(
                Vec2::new(0.0, 0.0),
                1.0,
                &[(l, t), (p.ridge.x, p.ridge.y), (l + w, t)],
            ),
            Poly::rect(cl, ct, cw, ch),
        ]
    }

    /// Where the column is: left, top, width, height, in px (from its top
    /// down to the chimney).
    pub fn rect(&self, bounds: Bounds) -> (f32, f32, f32, f32) {
        let w = self.width * bounds.scale();
        let top = self.top * bounds.height;
        let chimney = self.house_parts(bounds).chimney.1;
        (self.x - w / 2.0, top, w, (chimney - top).max(0.0))
    }

    /// The column, for finding out if the plane is in it.
    pub fn shape(&self, bounds: Bounds) -> Poly {
        let (l, t, w, h) = self.rect(bounds);
        Poly::rect(l, t, w, h)
    }

    fn right(&self, bounds: Bounds) -> f32 {
        self.span(bounds).1
    }

    /// Its leftmost and rightmost x, house and steam together, in px.
    pub fn span(&self, bounds: Bounds) -> (f32, f32) {
        let (l, _, w, _) = self.house_parts(bounds).walls;
        let (cl, _, cw, _) = self.rect(bounds);
        (l.min(cl), (l + w).max(cl + cw))
    }
}

/// A high-rise far back in the valley, in silhouette (scenery).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FarHighRise {
    /// Its middle on screen, in px.
    pub x: f32,
    /// Its width in px at scale 1.
    pub width: f32,
    /// Its roof, as a fraction of the screen height from the top.
    pub top: f32,
}

impl FarHighRise {
    /// Its walls: left, top, width, height, in px (down to the ground).
    pub fn rect(&self, bounds: Bounds) -> (f32, f32, f32, f32) {
        let w = self.width * bounds.scale();
        let top = self.top * bounds.height;
        (self.x - w / 2.0, top, w, STEAM_GROUND * bounds.height - top)
    }

    /// Its leftmost and rightmost x, with the roof's overhang, in px.
    pub fn span(&self, bounds: Bounds) -> (f32, f32) {
        let w = self.width * bounds.scale() * 1.2;
        (self.x - w / 2.0, self.x + w / 2.0)
    }
}

/// The morning's geese, steam and glare.
#[derive(Debug, Clone, PartialEq)]
pub struct Sunrise {
    /// Every goose in the air, all flocks together.
    pub geese: Vec<Goose>,
    /// The columns of steam on screen.
    pub steam: Vec<Steam>,
    /// High-rises far back (scenery), oldest first.
    pub high_rises: Vec<FarHighRise>,
    /// Ticks into the glare now; `None` between glares.
    glare: Option<u32>,
    next_goose: u32,
    next_steam: u32,
    next_high_rise: u32,
    next_glare: u32,
    /// The last flock's call, so the next one sounds different.
    last_call: u8,
    rng: Rng,
}

impl Sunrise {
    pub fn new(seed: u64) -> Self {
        Self {
            geese: Vec::new(),
            steam: Vec::new(),
            high_rises: Vec::new(),
            glare: None,
            next_goose: ticks(FIRST_GOOSE_SECONDS),
            next_steam: ticks(FIRST_STEAM_SECONDS),
            next_high_rise: ticks(FIRST_FAR_HIGH_RISE_SECONDS),
            next_glare: ticks(FIRST_GLARE_SECONDS),
            last_call: 0,
            // Its own stream, so the morning does not change other things.
            rng: Rng::new(seed ^ 0x5A2A_15E0_0000_0004),
        }
    }

    /// Clear it all away (a new level).
    pub fn clear(&mut self) {
        self.geese.clear();
        self.steam.clear();
        self.high_rises.clear();
        self.glare = None;
    }

    /// Everything slides further left by `dx` px (the game going faster).
    pub fn shift(&mut self, dx: f32) {
        self.geese.iter_mut().for_each(|g| g.pos.x += dx);
        self.steam.iter_mut().for_each(|c| c.x += dx);
        self.high_rises.iter_mut().for_each(|b| b.x += dx);
    }

    /// How strong the glare is now: 0 = none, 1 = full, fading in and
    /// out over [`GLARE_FADE_SECONDS`].
    pub fn glare(&self) -> f32 {
        let Some(t) = self.glare else { return 0.0 };
        let t = t as f32 * TICK_SECONDS;
        let fade_in = (t / GLARE_FADE_SECONDS).min(1.0);
        let fade_out = ((GLARE_SECONDS - t) / GLARE_FADE_SECONDS).clamp(0.0, 1.0);
        fade_in.min(fade_out)
    }

    /// Is the plane, as `me`, in a column of steam?
    pub fn in_steam(&self, bounds: Bounds, me: &[Poly]) -> bool {
        (self.steam.iter()).any(|c| crate::shape::any_overlap(me, &[c.shape(bounds)]))
    }

    /// Advance one tick. `active`: is the sunrise on screen (nothing new
    /// comes while it is not)? `plane_y`: where the geese aim. Honks go to
    /// `events`.
    pub fn tick(&mut self, bounds: Bounds, active: bool, plane_y: f32, events: &mut Vec<Event>) {
        let s = bounds.scale();
        for g in &mut self.geese {
            g.step();
        }
        self.geese.retain(|g| !g.gone(bounds));
        let dx = -SCROLL_SPEED * s * TICK_SECONDS;
        for c in &mut self.steam {
            c.x += dx;
            c.age += 1;
        }
        self.steam.retain(|c| c.right(bounds) > -MARGIN * s);
        for b in &mut self.high_rises {
            b.x += dx;
        }
        self.high_rises.retain(|b| b.span(bounds).1 > -MARGIN * s);
        if let Some(t) = &mut self.glare {
            *t += 1;
            if *t >= ticks(GLARE_SECONDS) {
                self.glare = None;
            }
        }

        if !active {
            self.next_goose = ticks(FIRST_GOOSE_SECONDS);
            self.next_steam = ticks(FIRST_STEAM_SECONDS);
            self.next_high_rise = ticks(FIRST_FAR_HIGH_RISE_SECONDS);
            self.next_glare = ticks(FIRST_GLARE_SECONDS);
            self.glare = None;
            return;
        }
        self.next_goose -= 1;
        if self.next_goose == 0 {
            self.spawn_flock(bounds, plane_y);
            // Another call than the last one.
            let other = (self.rng.unit() * (GOOSE_CALLS - 1) as f32) as u8 % (GOOSE_CALLS - 1);
            let call = (self.last_call + 1 + other) % GOOSE_CALLS;
            self.last_call = call;
            events.push(Event::GooseHonk { call });
            self.next_goose = ticks(self.rng.range(GOOSE_GAP_SECONDS.0, GOOSE_GAP_SECONDS.1));
        }
        self.next_steam -= 1;
        // A house or a high-rise waits until the last one has moved in far
        // enough: they come in from just off the right edge, so this keeps
        // room between them.
        let busy = (self.steam.iter().map(|c| c.span(bounds).1))
            .chain(self.high_rises.iter().map(|b| b.span(bounds).1))
            .fold(f32::MIN, f32::max)
            > bounds.width - SUNRISE_ROOM * s;
        if self.next_steam == 0 && busy {
            self.next_steam = ticks(0.2);
        }
        if self.next_steam == 0 {
            let r = &mut self.rng;
            let width = r.range(STEAM_WIDTH.0, STEAM_WIDTH.1);
            let house = (
                r.range(HOUSE_WIDTH.0, HOUSE_WIDTH.1),
                r.range(HOUSE_HEIGHT.0, HOUSE_HEIGHT.1),
            );
            self.steam.push(Steam {
                x: bounds.width + (width + house.0) * s,
                width,
                top: r.range(STEAM_TOP.0, STEAM_TOP.1),
                house,
                age: 0,
            });
            self.next_steam = ticks(r.range(STEAM_GAP_SECONDS.0, STEAM_GAP_SECONDS.1));
        }
        self.next_high_rise -= 1;
        let busy = (self.steam.iter().map(|c| c.span(bounds).1))
            .chain(self.high_rises.iter().map(|b| b.span(bounds).1))
            .fold(f32::MIN, f32::max)
            > bounds.width - SUNRISE_ROOM * s;
        if self.next_high_rise == 0 && busy {
            self.next_high_rise = ticks(0.2);
        }
        if self.next_high_rise == 0 {
            let r = &mut self.rng;
            let width = r.range(FAR_HIGH_RISE_WIDTH.0, FAR_HIGH_RISE_WIDTH.1);
            self.high_rises.push(FarHighRise {
                x: bounds.width + width * s,
                width,
                top: r.range(FAR_HIGH_RISE_TOP.0, FAR_HIGH_RISE_TOP.1),
            });
            let gap = r.range(FAR_HIGH_RISE_GAP_SECONDS.0, FAR_HIGH_RISE_GAP_SECONDS.1);
            self.next_high_rise = ticks(gap);
        }
        self.next_glare -= 1;
        if self.next_glare == 0 {
            self.glare = Some(0);
            self.next_glare = ticks(self.rng.range(GLARE_GAP_SECONDS.0, GLARE_GAP_SECONDS.1));
        }
    }

    /// A new V from just off the right edge, aimed at `plane_y`: its tip,
    /// its upper arm or its lower arm there, the whole V on screen.
    fn spawn_flock(&mut self, bounds: Bounds, plane_y: f32) {
        let s = bounds.scale();
        let r = &mut self.rng;
        let arm = r.range(GOOSE_ARM.0 as f32, GOOSE_ARM.1 as f32 + 1.0) as u32;
        let arm = arm.min(GOOSE_ARM.1);
        let (sx, sy) = (GOOSE_STEP.0 * s, GOOSE_STEP.1 * s);
        let reach = arm as f32 * sy;
        // Which part meets the plane: the tip, or a goose up or down an arm.
        let aim = match (r.unit() * 3.0) as u32 {
            0 => 0.0,
            1 => -r.range(1.0, arm as f32) * sy,
            _ => r.range(1.0, arm as f32) * sy,
        };
        let edge = 16.0 * s;
        let lowest = STEAM_GROUND * bounds.height - 30.0 * s;
        let tip_y = (plane_y - aim).clamp(reach + edge, lowest - reach);
        let speed = r.range(GOOSE_SPEED.0, GOOSE_SPEED.1) * s;
        let start = bounds.width + MARGIN * s;
        let mut add = |dx: f32, dy: f32| {
            self.geese.push(Goose {
                pos: Vec2::new(start + dx, tip_y + dy),
                vel: Vec2::new(-speed, 0.0),
                age: 0,
                down: None,
                fall: 0.0,
            });
        };
        add(0.0, 0.0);
        for i in 1..=arm {
            let i = i as f32;
            add(i * sx, -i * sy);
            add(i * sx, i * sy);
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

    fn run(m: &mut Sunrise, seconds: f32, active: bool, plane_y: f32) -> Vec<Event> {
        let mut events = Vec::new();
        for _ in 0..ticks(seconds) {
            m.tick(PHONE, active, plane_y, &mut events);
        }
        events
    }

    #[test]
    fn nothing_away_from_the_sunrise() {
        let mut m = Sunrise::new(1);
        let events = run(&mut m, 60.0, false, 200.0);
        assert!(m.geese.is_empty() && m.steam.is_empty() && events.is_empty());
        assert_eq!(m.glare(), 0.0);
    }

    /// The flocks as they appear: each a list of goose positions.
    fn flocks(seed: u64, seconds: f32, plane_y: f32) -> Vec<Vec<Vec2>> {
        let mut m = Sunrise::new(seed);
        let mut out = Vec::new();
        for _ in 0..ticks(seconds) {
            m.tick(PHONE, true, plane_y, &mut Vec::new());
            let fresh: Vec<Vec2> = (m.geese.iter())
                .filter(|g| g.age == 0)
                .map(|g| g.pos)
                .collect();
            if !fresh.is_empty() {
                out.push(fresh);
            }
        }
        out
    }

    #[test]
    fn geese_come_in_a_v_from_the_right_every_few_seconds_and_honk() {
        let mut m = Sunrise::new(2);
        let events = run(&mut m, FIRST_GOOSE_SECONDS - 0.1, true, 200.0);
        assert!(m.geese.is_empty() && events.is_empty());
        let all = flocks(2, 60.0, 200.0);
        // One every 3–5 s after the first 3 s.
        assert!((11..=20).contains(&all.len()), "{}", all.len());
        for v in &all {
            assert!(v.len() == 5 || v.len() == 7, "{}", v.len());
            let tip = v[0];
            assert!(tip.x > PHONE.width, "from off the right");
            for g in &v[1..] {
                assert!(g.x > tip.x, "the tip leads");
            }
            let above = v.iter().filter(|g| g.y < tip.y).count();
            let below = v.iter().filter(|g| g.y > tip.y).count();
            assert_eq!(above, below, "a V: two equal arms");
        }
        let mut m = Sunrise::new(2);
        let events = run(&mut m, 20.0, true, 200.0);
        let calls: Vec<u8> = (events.iter())
            .filter_map(|e| match e {
                Event::GooseHonk { call } => Some(*call),
                _ => None,
            })
            .collect();
        assert!(calls.len() >= 3);
        for pair in calls.windows(2) {
            assert_ne!(pair[0], pair[1], "never the same call twice in a row");
        }
        let mut m = Sunrise::new(2);
        let events = run(&mut m, 300.0, true, 200.0);
        for c in 0..GOOSE_CALLS {
            assert!(
                events.contains(&Event::GooseHonk { call: c }),
                "call {c} is heard"
            );
        }
    }

    #[test]
    fn a_v_is_aimed_at_the_planes_height_with_room_over_or_under_it() {
        for plane_y in [20.0, 120.0, 200.0, 330.0] {
            for v in flocks(5, 120.0, plane_y) {
                let top = v.iter().map(|g| g.y).fold(f32::MAX, f32::min);
                let bottom = v.iter().map(|g| g.y).fold(f32::MIN, f32::max);
                assert!(top > 10.0, "on screen");
                // Within reach of the plane, unless it hugs an edge.
                let near = (top - 20.0..=bottom + 20.0).contains(&plane_y);
                assert!(
                    near || plane_y < 40.0 || plane_y > 300.0,
                    "{plane_y}: {top}..{bottom}"
                );
                // Room to fly past it, above or below.
                let room = top.max(STEAM_GROUND * PHONE.height - bottom);
                assert!(room > 0.3 * PHONE.height, "{room}");
            }
        }
    }

    #[test]
    fn a_struck_goose_tumbles_down_and_is_dropped() {
        let mut m = Sunrise::new(3);
        run(&mut m, FIRST_GOOSE_SECONDS + 0.1, true, 200.0);
        m.geese[0].knock_down(PHONE.scale());
        assert!(m.geese[0].is_down());
        let y = m.geese[0].pos.y;
        run(&mut m, 0.5, false, 200.0);
        assert!(m.geese[0].pos.y > y, "falling");
        run(&mut m, 2.0, false, 200.0);
        assert!(
            !m.geese.iter().any(|g| g.is_down()),
            "it fell off the screen"
        );
        run(&mut m, 6.0, false, 200.0);
        assert!(m.geese.is_empty(), "the whole V is gone");
    }

    #[test]
    fn a_goose_is_hit_on_its_body_neck_and_wing() {
        let g = Goose {
            pos: Vec2::new(400.0, 200.0),
            vel: Vec2::new(-200.0, 0.0),
            age: 0,
            down: None,
            fall: 0.0,
        };
        let shape = g.shape(1.0);
        let hit = |x: f32, y: f32| any_overlap(&shape, &[Poly::rect(x - 1.0, y - 1.0, 2.0, 2.0)]);
        assert!(hit(400.0, 200.0), "body");
        assert!(hit(g.head(1.0).x, g.head(1.0).y), "head");
        assert!(!hit(400.0, 220.0), "under it");
    }

    #[test]
    fn steam_rises_from_a_houses_chimney_and_slides_off_with_the_ground() {
        let mut m = Sunrise::new(4);
        run(&mut m, FIRST_STEAM_SECONDS + 0.1, true, 200.0);
        assert_eq!(m.steam.len(), 1);
        let c = m.steam[0];
        let (l, t, _, h) = c.rect(PHONE);
        let house = c.house_parts(PHONE);
        assert!(house.walls.0 > PHONE.width, "from off the right edge");
        assert!(l > PHONE.width, "from off the right edge");
        assert!((t + h - house.chimney.1).abs() < 0.01, "out of the chimney");
        let (_, wt, _, wh) = house.walls;
        assert!(
            (wt + wh - STEAM_GROUND * PHONE.height).abs() < 0.01,
            "on the ground"
        );
        assert!(house.chimney.1 > 0.7 * PHONE.height, "a small house");
        assert!(t > 0.25 * PHONE.height, "room above it");
        let x = m.steam[0].x;
        run(&mut m, 1.0, false, 200.0);
        assert!(
            (x - m.steam[0].x - SCROLL_SPEED).abs() < 1.0,
            "with the ground"
        );
        run(&mut m, 20.0, false, 200.0);
        assert!(m.steam.is_empty());
    }

    #[test]
    fn the_house_is_solid_the_steam_is_not() {
        let c = Steam {
            x: 400.0,
            width: 50.0,
            top: 0.4,
            house: (70.0, 45.0),
            age: 0,
        };
        let house = c.house_shape(PHONE);
        let hit = |x: f32, y: f32| any_overlap(&house, &[Poly::rect(x - 1.0, y - 1.0, 2.0, 2.0)]);
        let p = c.house_parts(PHONE);
        let (l, t, w, h) = p.walls;
        assert!(hit(l + w / 2.0, t + h / 2.0), "the walls");
        assert!(hit(p.ridge.x, p.ridge.y + 3.0), "the roof");
        let (cl, ct, cw, _) = p.chimney;
        assert!(hit(cl + cw / 2.0, ct + 2.0), "the chimney");
        assert!(!hit(l - 4.0, t + 5.0), "beside it");
        let (sl, st, sw, sh) = c.rect(PHONE);
        assert!(!hit(sl + sw / 2.0, st + sh / 2.0), "the steam is no wall");
    }

    #[test]
    fn the_sun_glares_now_and_then_fading_in_and_out() {
        let mut m = Sunrise::new(6);
        run(&mut m, FIRST_GLARE_SECONDS - 0.1, true, 200.0);
        assert_eq!(m.glare(), 0.0);
        run(&mut m, GLARE_FADE_SECONDS / 2.0 + 0.1, true, 200.0);
        let rising = m.glare();
        assert!(rising > 0.2 && rising < 1.0, "{rising}");
        run(&mut m, GLARE_FADE_SECONDS, true, 200.0);
        assert_eq!(m.glare(), 1.0);
        run(&mut m, GLARE_SECONDS, true, 200.0);
        assert_eq!(m.glare(), 0.0, "over");
        // Several over a minute.
        let mut m = Sunrise::new(6);
        let mut starts = 0;
        let mut was = 0.0;
        for _ in 0..ticks(60.0) {
            m.tick(PHONE, true, 200.0, &mut Vec::new());
            starts += (was == 0.0 && m.glare() > 0.0) as u32;
            was = m.glare();
        }
        assert!((3..=6).contains(&starts), "{starts}");
    }

    #[test]
    fn high_rises_and_houses_never_stand_in_each_other() {
        for seed in 0..8 {
            let mut m = Sunrise::new(seed);
            let (mut houses, mut towers) = (0, 0);
            for _ in 0..ticks(300.0) {
                m.tick(PHONE, true, 200.0, &mut Vec::new());
                houses = houses.max(m.steam.len());
                towers = towers.max(m.high_rises.len());
                let mut spans: Vec<(f32, f32)> = (m.steam.iter())
                    .map(|c| c.span(PHONE))
                    .chain(m.high_rises.iter().map(|b| b.span(PHONE)))
                    .collect();
                spans.sort_by(|a, b| a.0.total_cmp(&b.0));
                for pair in spans.windows(2) {
                    let gap = pair[1].0 - pair[0].1;
                    assert!(gap >= SUNRISE_ROOM - 1.0, "seed {seed}: {gap}");
                }
            }
            assert!(houses > 0 && towers > 0, "both still come");
        }
    }

    #[test]
    fn same_seed_same_morning() {
        let (mut a, mut b) = (Sunrise::new(9), Sunrise::new(9));
        run(&mut a, 40.0, true, 150.0);
        run(&mut b, 40.0, true, 150.0);
        assert_eq!(a, b);
    }
}
