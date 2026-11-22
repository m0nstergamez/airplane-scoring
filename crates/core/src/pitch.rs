//! The football pitch (level 6): footballs kicked up in an arc, tall
//! floodlight masts and lawn sprinklers that come on and off.
//!
//! * **Footballs** are kicked up from the grass ahead of the plane
//!   ([`BALL_KICK_AT`]) and fly a parabola: up to a peak
//!   ([`BALL_PEAK`]), then down again, where they bounce
//!   ([`BALL_BOUNCE`]) and roll off. You see each one leave the ground,
//!   so the arc is never a surprise. Fly into one and you crash; it is
//!   knocked away. Some are aimed ([`BALL_AIMED_SHARE`]): a high ball
//!   that peaks just where the plane will be, so flying along the top
//!   is not safe either.
//! * **Floodlight masts** stand at the edge of the pitch, rarely
//!   ([`MAST_GAP_SECONDS`]): a thin pole with a bank of lamps on top.
//!   They are solid: fly over them.
//! * **Sprinklers** sit in the grass and throw a jet of water up
//!   ([`SPRINKLER_TOP`]) for [`SPRINKLER_ON_SECONDS`], then rest for
//!   [`SPRINKLER_OFF_SECONDS`]. The jet is solid while it is up: fly
//!   through while it rests, or over it.
//!
//! Masts and sprinklers stand on the ground and keep [`PITCH_ROOM`]
//! between them, so there is always a gap to come down into. Positions
//! are screen px; sizes are for a phone and grow with the screen
//! ([`Bounds::scale`]). Seeded, so the same seed gives the same pitch.

use crate::rng::Rng;
use crate::shape::Poly;
use crate::{Bounds, Event, Vec2, SCROLL_SPEED, TICK_SECONDS};

/// The level with the football pitch.
pub const PITCH_LEVEL: u32 = 6;

/// Where the grass is: the ground line, as a fraction of the screen
/// height from the top.
pub const PITCH_GROUND: f32 = crate::huts::HUT_GROUND;
/// The least free ground between two things standing on it (masts,
/// sprinklers), in px.
pub const PITCH_ROOM: f32 = 110.0;

/// Time between two kicks, shortest and longest, and the first, in
/// seconds.
pub const BALL_GAP_SECONDS: (f32, f32) = (2.0, 4.0);
pub const FIRST_BALL_SECONDS: f32 = 3.0;
/// Where a ball is kicked from, as a fraction of the screen width from
/// the left (well ahead of the plane, so you see it rise).
pub const BALL_KICK_AT: (f32, f32) = (0.6, 0.95);
/// How high a ball flies at its peak, as a fraction of the screen height
/// from the top (highest and lowest).
pub const BALL_PEAK: (f32, f32) = (0.22, 0.6);
/// How fast a ball rolls along the pitch by itself, in px per second
/// (less than 0: toward the plane), on top of the ground sliding past.
pub const BALL_DRIFT: (f32, f32) = (-60.0, 30.0);
/// How hard a ball falls back, in px per second, each second (at scale 1).
pub const BALL_GRAVITY: f32 = 300.0;
/// A ball's radius, in px.
pub const BALL_RADIUS: f32 = 8.0;
/// How much of its speed a ball keeps when it bounces on the grass, and
/// how often it bounces before it just rolls.
pub const BALL_BOUNCE: f32 = 0.45;
pub const BALL_BOUNCES: u32 = 2;
/// How many of the kicks are aimed at the plane: kicked nearly straight
/// up (drifting [`BALL_AIM_DRIFT`]) from where the ball peaks at the
/// plane's height just as it gets there. Only while the plane flies high
/// enough that the kick is at least [`BALL_AIM_AHEAD`] (a fraction of the
/// screen width) ahead of it; else it is an ordinary kick.
pub const BALL_AIMED_SHARE: f32 = 0.5;
pub const BALL_AIM_DRIFT: f32 = -30.0;
pub const BALL_AIM_AHEAD: f32 = 0.15;
/// An aimed ball's peak is up to this far above or below the plane, in px.
pub const BALL_AIM_SPREAD: f32 = 10.0;

/// Time between two floodlight masts, shortest and longest, and the
/// first, in seconds. (Rare: a pitch has only a few.)
pub const MAST_GAP_SECONDS: (f32, f32) = (8.0, 13.0);
pub const FIRST_MAST_SECONDS: f32 = 7.0;
/// The top of a mast's lamps, as a fraction of the screen height from the
/// top (tallest and shortest).
pub const MAST_TOP: (f32, f32) = (0.34, 0.48);
/// The bank of lamps: width and height, in px; the pole's width.
pub const MAST_LAMPS: (f32, f32) = (44.0, 22.0);
pub const MAST_POLE: f32 = 7.0;

/// Time between two sprinklers, shortest and longest, and the first, in
/// seconds.
pub const SPRINKLER_GAP_SECONDS: (f32, f32) = (2.6, 4.6);
pub const FIRST_SPRINKLER_SECONDS: f32 = 5.0;
/// How long a sprinkler throws water and how long it rests, in seconds;
/// how long its jet takes to rise and to fall.
pub const SPRINKLER_ON_SECONDS: f32 = 1.4;
pub const SPRINKLER_OFF_SECONDS: f32 = 1.6;
pub const SPRINKLER_RISE_SECONDS: f32 = 0.3;
/// How high a full jet reaches, as a fraction of the screen height from
/// the top (highest and lowest).
pub const SPRINKLER_TOP: (f32, f32) = (0.45, 0.62);
/// The jet's width, in px, and the spray at its top: width and height.
pub const SPRINKLER_JET: f32 = 14.0;
pub const SPRINKLER_SPRAY: (f32, f32) = (40.0, 16.0);
/// Below this much of its height, a jet is only a trickle: no obstacle.
const TRICKLE: f32 = 0.08;

/// Things start and are dropped this far outside the screen, in px.
const MARGIN: f32 = 40.0;

fn ticks(seconds: f32) -> u32 {
    (seconds / TICK_SECONDS).round().max(1.0) as u32
}

/// A football.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ball {
    /// Its centre on screen, in px.
    pub pos: Vec2,
    /// Its own speed, in px per second: across on top of the ground
    /// sliding past, and up (less than 0) or down.
    pub vel: Vec2,
    /// How often it has bounced on the grass.
    pub bounces: u32,
    /// The plane hit it: it flies off and cannot be hit again.
    pub struck: bool,
    /// Ticks since it was kicked (for its spin).
    pub age: u32,
}

impl Ball {
    /// How far it has turned, in radians: it spins as it flies or rolls.
    pub fn spin(&self) -> f32 {
        -(self.age as f32) * TICK_SECONDS * 6.0
    }

    /// The ball as drawn, for crashing.
    pub fn shape(&self, scale: f32) -> Poly {
        let r = BALL_RADIUS * scale;
        Poly::ellipse(self.pos, r, r)
    }

    /// Hit by the plane: it flies off up and away.
    pub fn knock_away(&mut self, scale: f32) {
        if self.struck {
            return;
        }
        self.struck = true;
        self.vel = Vec2::new(160.0 * scale, -220.0 * scale);
    }
}

/// A floodlight mast.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mast {
    /// Its pole's x on screen, in px.
    pub x: f32,
    /// The top of its lamps, as a fraction of the screen height.
    pub top: f32,
}

impl Mast {
    /// The bank of lamps: left, top, width, height, in px.
    pub fn lamps(&self, bounds: Bounds) -> (f32, f32, f32, f32) {
        let s = bounds.scale();
        let (w, h) = (MAST_LAMPS.0 * s, MAST_LAMPS.1 * s);
        (self.x - w / 2.0, self.top * bounds.height, w, h)
    }

    /// The pole under the lamps down to the grass: left, top, width,
    /// height, in px.
    pub fn pole(&self, bounds: Bounds) -> (f32, f32, f32, f32) {
        let (_, t, _, h) = self.lamps(bounds);
        let w = MAST_POLE * bounds.scale();
        let ground = PITCH_GROUND * bounds.height;
        (self.x - w / 2.0, t + h, w, ground - t - h)
    }

    /// The mast as drawn, for crashing: lamps and pole.
    pub fn shape(&self, bounds: Bounds) -> Vec<Poly> {
        let (l, t, w, h) = self.lamps(bounds);
        let (pl, pt, pw, ph) = self.pole(bounds);
        vec![Poly::rect(l, t, w, h), Poly::rect(pl, pt, pw, ph)]
    }

    /// Its leftmost and rightmost x, in px.
    pub fn span(&self, bounds: Bounds) -> (f32, f32) {
        let (l, _, w, _) = self.lamps(bounds);
        (l, l + w)
    }
}

/// A lawn sprinkler.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sprinkler {
    /// Its x on screen, in px.
    pub x: f32,
    /// How high its full jet reaches, as a fraction of the screen height.
    pub top: f32,
    /// Ticks into its on-and-off cycle (it starts at a random point, so
    /// two sprinklers do not go in step).
    pub phase: u32,
}

impl Sprinkler {
    /// How much of its jet is up now: 0 = resting, 1 = full. It rises and
    /// falls over [`SPRINKLER_RISE_SECONDS`].
    pub fn water(&self) -> f32 {
        let t = self.phase as f32 * TICK_SECONDS;
        if t >= SPRINKLER_ON_SECONDS {
            return 0.0;
        }
        let rise = (t / SPRINKLER_RISE_SECONDS).min(1.0);
        let fall = ((SPRINKLER_ON_SECONDS - t) / SPRINKLER_RISE_SECONDS).min(1.0);
        rise.min(fall)
    }

    /// The jet as it is now: left, top, width, height, in px (height 0
    /// while it rests).
    pub fn jet(&self, bounds: Bounds) -> (f32, f32, f32, f32) {
        let ground = PITCH_GROUND * bounds.height;
        let full = ground - self.top * bounds.height;
        let h = full * self.water();
        let w = SPRINKLER_JET * bounds.scale();
        (self.x - w / 2.0, ground - h, w, h)
    }

    /// The spray at the jet's top: left, top, width, height, in px.
    pub fn spray(&self, bounds: Bounds) -> (f32, f32, f32, f32) {
        let s = bounds.scale();
        let (_, t, _, _) = self.jet(bounds);
        let (w, h) = (SPRINKLER_SPRAY.0 * s, SPRINKLER_SPRAY.1 * s);
        (self.x - w / 2.0, t - h / 2.0, w, h)
    }

    /// The water as drawn, for crashing: the jet and its spray, or
    /// nothing while it rests (or only trickles).
    pub fn shape(&self, bounds: Bounds) -> Vec<Poly> {
        if self.water() < TRICKLE {
            return Vec::new();
        }
        let (l, t, w, h) = self.jet(bounds);
        let (sl, st, sw, sh) = self.spray(bounds);
        vec![Poly::rect(l, t, w, h), Poly::rect(sl, st, sw, sh)]
    }

    /// Its leftmost and rightmost x, in px (as wide as its spray).
    pub fn span(&self, bounds: Bounds) -> (f32, f32) {
        let half = SPRINKLER_SPRAY.0 * bounds.scale() / 2.0;
        (self.x - half, self.x + half)
    }
}

/// Which ground thing a timer is for.
#[derive(Clone, Copy)]
enum Which {
    Mast,
    Sprinkler,
}

/// The pitch's balls, masts and sprinklers.
#[derive(Debug, Clone, PartialEq)]
pub struct Pitch {
    /// Footballs in the air or rolling, oldest first.
    pub balls: Vec<Ball>,
    /// Floodlight masts.
    pub masts: Vec<Mast>,
    /// Lawn sprinklers.
    pub sprinklers: Vec<Sprinkler>,
    next_ball: u32,
    next_mast: u32,
    next_sprinkler: u32,
    /// The game's speed (1 = normal), see [`Pitch::set_speed`].
    speed: f32,
    rng: Rng,
}

impl Pitch {
    pub fn new(seed: u64) -> Self {
        Self {
            balls: Vec::new(),
            masts: Vec::new(),
            sprinklers: Vec::new(),
            next_ball: ticks(FIRST_BALL_SECONDS),
            next_mast: ticks(FIRST_MAST_SECONDS),
            next_sprinkler: ticks(FIRST_SPRINKLER_SECONDS),
            speed: 1.0,
            // Its own stream, so the pitch does not change other things.
            rng: Rng::new(seed ^ 0xF007_BA11_0000_0006),
        }
    }

    /// Clear it all away (a new level).
    pub fn clear(&mut self) {
        self.balls.clear();
        self.masts.clear();
        self.sprinklers.clear();
    }

    /// Everything slides further left by `dx` px (the game going faster):
    /// it all stands or rolls on the ground.
    pub fn shift(&mut self, dx: f32) {
        self.balls.iter_mut().for_each(|b| b.pos.x += dx);
        self.masts.iter_mut().for_each(|m| m.x += dx);
        self.sprinklers.iter_mut().for_each(|s| s.x += dx);
    }

    /// How fast the game goes now (1 = normal; the world slides the pitch
    /// by the extra, see [`Pitch::shift`]): aimed balls allow for it.
    pub fn set_speed(&mut self, speed: f32) {
        self.speed = speed.max(0.1);
    }

    /// Advance one tick. `active`: is the pitch on screen (nothing new
    /// comes while it is not)? `plane`: the plane's centre, where aimed
    /// balls go. Kicks go to `events`.
    pub fn tick(&mut self, bounds: Bounds, active: bool, plane: Vec2, events: &mut Vec<Event>) {
        let s = bounds.scale();
        let dx = -SCROLL_SPEED * s * TICK_SECONDS;
        let ground = PITCH_GROUND * bounds.height;
        for b in &mut self.balls {
            Self::step_ball(b, s, dx, ground);
        }
        for m in &mut self.masts {
            m.x += dx;
        }
        let cycle = ticks(SPRINKLER_ON_SECONDS + SPRINKLER_OFF_SECONDS);
        for sp in &mut self.sprinklers {
            sp.x += dx;
            sp.phase = (sp.phase + 1) % cycle;
        }
        let m = MARGIN * s;
        let h = bounds.height;
        (self.balls).retain(|b| b.pos.x > -m && b.pos.y < h + m && b.pos.y > -4.0 * m);
        self.masts.retain(|x| x.span(bounds).1 > -m);
        self.sprinklers.retain(|x| x.span(bounds).1 > -m);

        if !active {
            self.next_ball = ticks(FIRST_BALL_SECONDS);
            self.next_mast = ticks(FIRST_MAST_SECONDS);
            self.next_sprinkler = ticks(FIRST_SPRINKLER_SECONDS);
            return;
        }
        // Ground things wait until the last one has moved in by the room,
        // then come in with their left edge just off the screen.
        let spawn_x = |width: f32| bounds.width + (MARGIN + width / 2.0) * s;
        if self.due_on_ground(Which::Mast, bounds) {
            let top = self.rng.range(MAST_TOP.0, MAST_TOP.1);
            let x = spawn_x(MAST_LAMPS.0);
            self.masts.push(Mast { x, top });
            self.next_mast = ticks(self.rng.range(MAST_GAP_SECONDS.0, MAST_GAP_SECONDS.1));
        }
        if self.due_on_ground(Which::Sprinkler, bounds) {
            let r = &mut self.rng;
            let top = r.range(SPRINKLER_TOP.0, SPRINKLER_TOP.1);
            let phase = (r.unit() * cycle as f32) as u32 % cycle;
            let x = spawn_x(SPRINKLER_SPRAY.0);
            self.sprinklers.push(Sprinkler { x, top, phase });
            self.next_sprinkler = ticks(r.range(SPRINKLER_GAP_SECONDS.0, SPRINKLER_GAP_SECONDS.1));
        }
        self.next_ball -= 1;
        if self.next_ball == 0 {
            let kick = self
                .aimed_kick(bounds, plane)
                .or_else(|| self.plain_kick(bounds));
            let Some((x, peak, drift)) = kick else {
                self.next_ball = ticks(0.2); // all blocked: a moment later
                return;
            };
            let y = ground - BALL_RADIUS * s;
            let up = Self::launch(y - peak, s);
            self.balls.push(Ball {
                pos: Vec2::new(x, y),
                vel: Vec2::new(drift * s, -up),
                bounces: 0,
                struck: false,
                age: 0,
            });
            events.push(Event::BallKick);
            let r = &mut self.rng;
            self.next_ball = ticks(r.range(BALL_GAP_SECONDS.0, BALL_GAP_SECONDS.1));
        }
    }

    /// How fast a ball must go up to rise `rise` px. (Plus half a tick of
    /// falling: the ball falls before it moves in every tick, so it peaks
    /// exactly that high.)
    fn launch(rise: f32, s: f32) -> f32 {
        let g = BALL_GRAVITY * s;
        (2.0 * g * rise.max(0.0)).sqrt() + g * TICK_SECONDS / 2.0
    }

    /// An ordinary kick: where from, how high (y of the peak) and its
    /// drift. Never out of a mast or a sprinkler: a few spots are tried.
    fn plain_kick(&mut self, bounds: Bounds) -> Option<(f32, f32, f32)> {
        let r = &mut self.rng;
        let spots: Vec<f32> = (0..4)
            .map(|_| r.range(BALL_KICK_AT.0, BALL_KICK_AT.1) * bounds.width)
            .collect();
        let x = spots.into_iter().find(|&x| self.clear_to_kick(x, bounds))?;
        let r = &mut self.rng;
        let peak = r.range(BALL_PEAK.0, BALL_PEAK.1) * bounds.height;
        Some((x, peak, r.range(BALL_DRIFT.0, BALL_DRIFT.1)))
    }

    /// An aimed kick ([`BALL_AIMED_SHARE`]), if this one is and the plane
    /// is high enough for it: from where the ball, sliding toward the
    /// plane, reaches it just at its peak.
    fn aimed_kick(&mut self, bounds: Bounds, plane: Vec2) -> Option<(f32, f32, f32)> {
        let r = &mut self.rng;
        if r.unit() >= BALL_AIMED_SHARE {
            return None;
        }
        let s = bounds.scale();
        let spread = r.range(-BALL_AIM_SPREAD, BALL_AIM_SPREAD) * s;
        let peak = (plane.y + spread).max(BALL_RADIUS * s);
        let ground = PITCH_GROUND * bounds.height - BALL_RADIUS * s;
        let rise_time = Self::launch(ground - peak, s) / (BALL_GRAVITY * s);
        let closing = SCROLL_SPEED * s * self.speed - BALL_AIM_DRIFT * s;
        let x = plane.x + closing * rise_time;
        let far_enough = x >= plane.x + BALL_AIM_AHEAD * bounds.width;
        let on_screen = x <= BALL_KICK_AT.1 * bounds.width;
        (far_enough && on_screen && self.clear_to_kick(x, bounds)).then_some((
            x,
            peak,
            BALL_AIM_DRIFT,
        ))
    }

    /// Count down a mast's or a sprinkler's timer; true when one comes in
    /// now. While the last ground thing is still too close to the right
    /// edge, it waits a moment longer.
    fn due_on_ground(&mut self, which: Which, bounds: Bounds) -> bool {
        let crowded = self.crowded(bounds);
        let timer = match which {
            Which::Mast => &mut self.next_mast,
            Which::Sprinkler => &mut self.next_sprinkler,
        };
        *timer -= 1;
        if *timer == 0 && crowded {
            *timer = ticks(0.2);
        }
        *timer == 0
    }

    /// Is the grass at `x` free of masts and sprinklers, with room for a
    /// ball beside them?
    fn clear_to_kick(&self, x: f32, bounds: Bounds) -> bool {
        let room = 2.0 * BALL_RADIUS * bounds.scale();
        (self.masts.iter().map(|m| m.span(bounds)))
            .chain(self.sprinklers.iter().map(|sp| sp.span(bounds)))
            .all(|(l, r)| x < l - room || x > r + room)
    }

    /// Is the last mast or sprinkler still too close to the right edge for
    /// another one to come in?
    fn crowded(&self, bounds: Bounds) -> bool {
        let last = (self.masts.iter().map(|x| x.span(bounds).1))
            .chain(self.sprinklers.iter().map(|x| x.span(bounds).1))
            .fold(f32::MIN, f32::max);
        last > bounds.width - PITCH_ROOM * bounds.scale()
    }

    /// Move a ball on by one tick: with the ground (`dx`), by its own
    /// speed and falling; it bounces on the grass, then rolls.
    fn step_ball(b: &mut Ball, s: f32, dx: f32, ground: f32) {
        b.age += 1;
        b.vel.y += BALL_GRAVITY * s * TICK_SECONDS;
        b.pos.x += dx + b.vel.x * TICK_SECONDS;
        b.pos.y += b.vel.y * TICK_SECONDS;
        let floor = ground - BALL_RADIUS * s;
        if b.struck || b.pos.y < floor || b.vel.y <= 0.0 {
            return; // (a struck ball falls through, off the screen)
        }
        b.pos.y = floor;
        if b.bounces < BALL_BOUNCES {
            b.bounces += 1;
            b.vel.y = -b.vel.y * BALL_BOUNCE;
            b.vel.x *= 0.7;
        } else {
            b.vel.y = 0.0;
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
    /// A plane low over the grass: too low for aimed balls.
    const LOW: Vec2 = Vec2 {
        x: PLANE_X,
        y: 330.0,
    };

    fn run(p: &mut Pitch, seconds: f32, active: bool) -> Vec<Event> {
        let mut events = Vec::new();
        for _ in 0..ticks(seconds) {
            p.tick(PHONE, active, LOW, &mut events);
        }
        events
    }

    fn probe(x: f32, y: f32) -> Vec<Poly> {
        vec![Poly::rect(x - 1.0, y - 1.0, 2.0, 2.0)]
    }

    #[test]
    fn nothing_away_from_the_pitch() {
        let mut p = Pitch::new(1);
        let events = run(&mut p, 60.0, false);
        assert!(p.balls.is_empty() && p.masts.is_empty() && p.sprinklers.is_empty());
        assert!(events.is_empty());
    }

    #[test]
    fn a_ball_is_kicked_up_from_the_grass_ahead_of_the_plane() {
        let mut p = Pitch::new(2);
        let events = run(&mut p, FIRST_BALL_SECONDS + 0.01, true);
        assert_eq!(p.balls.len(), 1);
        assert_eq!(events, vec![Event::BallKick]);
        let b = p.balls[0];
        let ground = PITCH_GROUND * PHONE.height;
        assert!(b.pos.y > ground - 2.0 * BALL_RADIUS, "from the grass");
        assert!(b.pos.x > PLANE_X + 300.0, "well ahead: {}", b.pos.x);
        assert!(b.vel.y < 0.0, "going up");
    }

    /// The balls kicked (where from) and how often one passed within
    /// `near` px of a plane holding still at `plane`, in `seconds`.
    fn aimed_at(plane: Vec2, seconds: f32, near: f32) -> (Vec<f32>, usize) {
        let mut p = Pitch::new(9);
        let mut kicks = Vec::new();
        let mut met = std::collections::BTreeSet::new(); // by kick tick
        for t in 0..ticks(seconds) {
            let before = p.balls.len();
            p.tick(PHONE, true, plane, &mut Vec::new());
            if p.balls.len() > before {
                kicks.push(p.balls.last().unwrap().pos.x);
            }
            for b in &p.balls {
                if libm::hypotf(b.pos.x - plane.x, b.pos.y - plane.y) < near {
                    met.insert(t - b.age);
                }
            }
        }
        (kicks, met.len())
    }

    #[test]
    fn flying_along_the_top_is_not_safe() {
        // Holding still up high, the plane is met by aimed balls again
        // and again: up to every second kick (one every 2..4 s).
        for y in [12.0, 40.0, 80.0, 140.0] {
            let plane = Vec2::new(PLANE_X, y);
            let (kicks, close) = aimed_at(plane, 120.0, BALL_RADIUS + 6.0);
            let share = close as f32 / kicks.len() as f32;
            // (Fewer lower down: a mast or a sprinkler often stands
            // where an aimed kick would be, so it is an ordinary one.)
            let least = if y < 50.0 { 0.4 } else { 0.2 };
            let msg = format!("{y}: {close} of {}", kicks.len());
            assert!(share >= least && share < 0.7, "{msg}");
        }
    }

    #[test]
    fn aimed_balls_are_kicked_well_ahead_and_only_at_a_high_plane() {
        let ahead = PLANE_X + BALL_AIM_AHEAD * PHONE.width;
        for y in [12.0, 80.0, 160.0, 220.0, 330.0] {
            let (kicks, _) = aimed_at(Vec2::new(PLANE_X, y), 120.0, 1.0);
            assert!(kicks.iter().all(|&x| x >= ahead), "{y}: {kicks:?}");
        }
        // Low over the grass, every kick is an ordinary one.
        let (kicks, close) = aimed_at(LOW, 120.0, BALL_RADIUS + 6.0);
        assert!(kicks.iter().all(|&x| x >= BALL_KICK_AT.0 * PHONE.width));
        assert!(close < kicks.len() / 5, "{close} of {}", kicks.len());
    }

    #[test]
    fn aimed_balls_allow_for_the_speed() {
        // Twice as fast: kicked further ahead, and still on target.
        let plane = Vec2::new(PLANE_X, 40.0);
        let mut slow = Pitch::new(9);
        let mut fast = Pitch::new(9);
        fast.set_speed(2.0);
        let first = |p: &mut Pitch, speed: f32| loop {
            p.tick(PHONE, true, plane, &mut Vec::new());
            if p.balls.iter().any(|b| b.vel.x == BALL_AIM_DRIFT) {
                let b = *p.balls.iter().find(|b| b.vel.x == BALL_AIM_DRIFT).unwrap();
                p.balls.retain(|x| *x == b);
                // Follow it alone, the ground sliding by the extra too.
                let extra = (speed - 1.0) * SCROLL_SPEED * TICK_SECONDS;
                let mut best = f32::MAX;
                for _ in 0..ticks(4.0) {
                    p.tick(PHONE, false, plane, &mut Vec::new());
                    p.shift(-extra);
                    if let Some(b) = p.balls.first() {
                        best = best.min(libm::hypotf(b.pos.x - plane.x, b.pos.y - plane.y));
                    }
                }
                return (b.pos.x, best);
            }
        };
        let (x_slow, miss_slow) = first(&mut slow, 1.0);
        let (x_fast, miss_fast) = first(&mut fast, 2.0);
        assert!(x_fast > x_slow + 100.0, "{x_slow} {x_fast}");
        let near = BALL_AIM_SPREAD + 4.0;
        assert!(
            miss_slow < near && miss_fast < near,
            "{miss_slow} {miss_fast}"
        );
    }

    #[test]
    fn a_ball_flies_an_arc_to_its_peak_and_comes_down_again() {
        for seed in 0..20 {
            let mut p = Pitch::new(seed);
            run(&mut p, FIRST_BALL_SECONDS + 0.01, true);
            let mut top = f32::MAX;
            let mut came_down = false;
            for _ in 0..ticks(5.0) {
                p.tick(PHONE, false, LOW, &mut Vec::new());
                let Some(b) = p.balls.first() else { break };
                top = top.min(b.pos.y);
                came_down |= b.bounces > 0;
            }
            let (hi, lo) = (BALL_PEAK.0 * PHONE.height, BALL_PEAK.1 * PHONE.height);
            assert!(top > hi - 1.0 && top < lo + 1.0, "peak {top} in {hi}..{lo}");
            assert!(came_down, "it lands and bounces");
        }
    }

    #[test]
    fn a_ball_bounces_a_few_times_then_rolls_off() {
        let mut p = Pitch::new(3);
        run(&mut p, FIRST_BALL_SECONDS + 0.01, true);
        let mut rolled = false;
        for _ in 0..ticks(12.0) {
            p.tick(PHONE, false, LOW, &mut Vec::new());
            if let Some(b) = p.balls.first() {
                assert!(b.bounces <= BALL_BOUNCES);
                rolled |= b.bounces == BALL_BOUNCES && b.vel.y == 0.0;
            }
        }
        assert!(rolled, "it ends up rolling");
        assert!(p.balls.is_empty(), "and rolls off the screen");
    }

    #[test]
    fn balls_come_about_every_few_seconds() {
        let mut p = Pitch::new(4);
        let kicks = run(&mut p, 120.0, true).len();
        let (lo, hi) = (120.0 / BALL_GAP_SECONDS.1, 120.0 / BALL_GAP_SECONDS.0);
        assert!((lo as usize..=hi as usize + 1).contains(&kicks), "{kicks}");
    }

    #[test]
    fn a_struck_ball_flies_off_and_is_gone() {
        let mut p = Pitch::new(2);
        run(&mut p, FIRST_BALL_SECONDS + 0.5, true);
        p.balls[0].knock_away(PHONE.scale());
        assert!(p.balls[0].struck);
        run(&mut p, 6.0, false);
        assert!(p.balls.is_empty());
    }

    #[test]
    fn a_ball_is_hit_on_it_not_beside_it() {
        let b = Ball {
            pos: Vec2::new(300.0, 200.0),
            vel: Vec2::new(0.0, 0.0),
            bounces: 0,
            struck: false,
            age: 0,
        };
        let shape = [b.shape(1.0)];
        assert!(any_overlap(
            &shape,
            &probe(300.0, 200.0 + BALL_RADIUS - 2.0)
        ));
        assert!(!any_overlap(
            &shape,
            &probe(300.0 + BALL_RADIUS + 3.0, 200.0)
        ));
    }

    #[test]
    fn masts_are_rare_and_leave_room_above() {
        let mut p = Pitch::new(5);
        let mut placed = Vec::new();
        for tick in 0..ticks(240.0) {
            let before = p.masts.len();
            p.tick(PHONE, true, LOW, &mut Vec::new());
            if p.masts.len() > before {
                placed.push(tick as f32 * TICK_SECONDS);
            }
        }
        assert!(placed.len() >= 10, "{}", placed.len());
        for pair in placed.windows(2) {
            let gap = pair[1] - pair[0];
            assert!(gap >= MAST_GAP_SECONDS.0 - 0.01, "too close: {gap}");
        }
        let m = Mast {
            x: 400.0,
            top: MAST_TOP.0,
        };
        let (_, t, _, _) = m.lamps(PHONE);
        assert!(t > 0.3 * PHONE.height, "room above the tallest: {t}");
    }

    #[test]
    fn a_mast_is_hit_on_its_lamps_and_pole_not_beside_or_above() {
        let m = Mast { x: 400.0, top: 0.4 };
        let shape = m.shape(PHONE);
        let hit = |x: f32, y: f32| any_overlap(&shape, &probe(x, y));
        let (l, t, w, h) = m.lamps(PHONE);
        assert!(hit(l + 3.0, t + 3.0), "the lamps");
        let (pl, pt, pw, ph) = m.pole(PHONE);
        assert!(hit(pl + pw / 2.0, pt + ph / 2.0), "the pole");
        assert!(!hit(400.0, t - 4.0), "above it");
        assert!(!hit(l + w + 4.0, t + h / 2.0), "beside the lamps");
        assert!(!hit(pl - 8.0, pt + ph / 2.0), "beside the pole");
    }

    #[test]
    fn a_sprinkler_throws_water_then_rests_and_its_jet_is_solid_only_when_up() {
        let mut sp = Sprinkler {
            x: 400.0,
            top: 0.5,
            phase: 0,
        };
        assert_eq!(sp.water(), 0.0, "rising from nothing");
        assert!(sp.shape(PHONE).is_empty());
        sp.phase = ticks(SPRINKLER_ON_SECONDS / 2.0);
        assert_eq!(sp.water(), 1.0);
        let (_, t, _, _) = sp.jet(PHONE);
        assert!((t - 0.5 * PHONE.height).abs() < 0.5, "full height: {t}");
        let shape = sp.shape(PHONE);
        assert!(any_overlap(&shape, &probe(400.0, t + 20.0)), "in the jet");
        assert!(
            !any_overlap(&shape, &probe(400.0, t - 20.0)),
            "above the spray"
        );
        sp.phase = ticks(SPRINKLER_ON_SECONDS + 0.1);
        assert_eq!(sp.water(), 0.0, "resting");
        assert!(sp.shape(PHONE).is_empty(), "fly through while it rests");
    }

    #[test]
    fn a_sprinkler_rests_long_enough_every_cycle_to_fly_through() {
        let mut p = Pitch::new(6);
        run(&mut p, FIRST_SPRINKLER_SECONDS + 0.01, true);
        assert_eq!(p.sprinklers.len(), 1);
        // Over two cycles: the longest stretch without water.
        let mut dry = 0u32;
        let mut longest = 0u32;
        let mut wet = false;
        for _ in 0..ticks(2.0 * (SPRINKLER_ON_SECONDS + SPRINKLER_OFF_SECONDS)) {
            p.tick(PHONE, false, LOW, &mut Vec::new());
            let s = p.sprinklers[0];
            if s.shape(PHONE).is_empty() {
                dry += 1;
                longest = longest.max(dry);
            } else {
                dry = 0;
                wet = true;
            }
        }
        assert!(wet, "it comes on");
        let secs = longest as f32 * TICK_SECONDS;
        assert!(secs >= SPRINKLER_OFF_SECONDS, "rests {secs} s");
    }

    #[test]
    fn sprinklers_do_not_go_in_step() {
        let mut p = Pitch::new(7);
        run(&mut p, 30.0, true);
        let phases: Vec<u32> = p.sprinklers.iter().map(|s| s.phase).collect();
        assert!(phases.len() >= 2, "{phases:?}");
        assert!(phases.windows(2).any(|w| w[0] != w[1]), "{phases:?}");
    }

    #[test]
    fn things_on_the_ground_leave_room_between_them() {
        let mut p = Pitch::new(8);
        for _ in 0..ticks(240.0) {
            p.tick(PHONE, true, LOW, &mut Vec::new());
            let mut spans: Vec<(f32, f32)> = (p.masts.iter().map(|m| m.span(PHONE)))
                .chain(p.sprinklers.iter().map(|s| s.span(PHONE)))
                .collect();
            spans.sort_by(|a, b| a.0.total_cmp(&b.0));
            for pair in spans.windows(2) {
                let gap = pair[1].0 - pair[0].1;
                assert!(gap >= PITCH_ROOM - 2.0, "{gap}");
            }
        }
    }

    #[test]
    fn everything_slides_with_the_ground() {
        let mut p = Pitch::new(9);
        run(&mut p, 20.0, true);
        // The newest of each: still well on the screen a second later.
        let mast = p.masts.last().unwrap().x;
        let sp = p.sprinklers.last().unwrap().x;
        run(&mut p, 1.0, false);
        assert!((mast - p.masts.last().unwrap().x - SCROLL_SPEED).abs() < 1.0);
        assert!((sp - p.sprinklers.last().unwrap().x - SCROLL_SPEED).abs() < 1.0);
    }

    #[test]
    fn a_ball_is_never_kicked_out_of_a_mast_or_a_sprinkler() {
        for seed in 0..10 {
            let mut p = Pitch::new(seed);
            for _ in 0..ticks(120.0) {
                let before = p.balls.len();
                p.tick(PHONE, true, LOW, &mut Vec::new());
                if p.balls.len() > before {
                    let x = p.balls.last().unwrap().pos.x;
                    for (l, r) in (p.masts.iter().map(|m| m.span(PHONE)))
                        .chain(p.sprinklers.iter().map(|s| s.span(PHONE)))
                    {
                        assert!(x < l || x > r, "seed {seed}: kicked at {x} in {l}..{r}");
                    }
                }
            }
        }
    }

    #[test]
    fn same_seed_same_pitch() {
        let (mut a, mut b) = (Pitch::new(9), Pitch::new(9));
        run(&mut a, 40.0, true);
        run(&mut b, 40.0, true);
        assert_eq!(a, b);
    }
}
