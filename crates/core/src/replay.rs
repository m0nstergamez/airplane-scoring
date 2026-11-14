//! Replays: a flight written down as its seed and inputs, so that
//! anyone (the leaderboard server) can fly it again and see the same
//! result. Core is deterministic: same seed, same inputs, same flight.
//!
//! The game records with a [`Recorder`]; the server calls [`replay`] on
//! the bytes and trusts only what comes out of it ([`Outcome`]), never a
//! score the client claims. The score is the same as for the local best
//! times ([`crate::BestTimes`]): the time of each level landed on the
//! flight ([`World::millis_in_level`] at "Level complete"); a game over
//! or a flight left early counts no time.
//!
//! What a flight depends on, and so what is recorded:
//! - the seed, the start level and the screen size at the start ([`Start`]),
//! - every tick's steering and throttle, rounded to 1/127 steps
//!   ([`quantize`]) so one byte each holds exactly what the world saw,
//! - every change of the screen size (sizes and speeds scale with it),
//! - every "Next level" tap ([`World::go_on`]).
//!
//! Bytes: `"APR1"`, seed (u64), level (u8), width, height (f32), then
//! records: `0 steer throttle count` (the same input for `count` ticks),
//! `1 width height` (resize), `2` (go on). Numbers are little endian.

use crate::{has_next_level, start_level, Bounds, Event, Input, World};

const MAGIC: [u8; 4] = *b"APR1";
const HEADER_LEN: usize = 4 + 8 + 1 + 4 + 4;
const TICKS: u8 = 0;
const RESIZE: u8 = 1;
const GO_ON: u8 = 2;

/// A replay may run this many ticks at most (three hours): a limit on
/// the work a replay can ask of the server.
pub const MAX_REPLAY_TICKS: u64 = 3 * 60 * 60 * 60;

/// Screens a flight may be flown on, in px. Sizes and speeds scale with
/// the height, so odd shapes could make the game easier.
pub const BOUNDS_WIDTH: (f32, f32) = (200.0, 8000.0);
pub const BOUNDS_HEIGHT: (f32, f32) = (150.0, 5000.0);
/// Width / height: from an upright window to an ultra-wide screen.
pub const BOUNDS_ASPECT: (f32, f32) = (0.4, 4.5);

/// Steering or throttle as the byte a replay keeps: -127 ..= 127.
/// NaN or infinite counts as 0, like in [`Input`].
pub fn quantize(v: f32) -> i8 {
    if v.is_finite() {
        (v.clamp(-1.0, 1.0) * 127.0).round() as i8
    } else {
        0
    }
}

/// Steering and throttle the way a replay keeps them: rounded to
/// 1/127 steps. What [`Recorder::tick`] returns, for flights that are
/// not recorded.
pub fn rounded_input(steer: f32, throttle: f32) -> Input {
    input_of(quantize(steer), quantize(throttle))
}

/// The input the world sees for a recorded step.
fn input_of(steer: i8, throttle: i8) -> Input {
    let unit = |q: i8| f32::from(q.max(-127)) / 127.0;
    Input::with_throttle(unit(steer), unit(throttle))
}

/// Everything that decides how a flight begins.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Start {
    pub seed: u64,
    /// The level the flight starts in (anything invalid means level 1).
    pub level: u32,
    pub bounds: Bounds,
}

impl Start {
    /// The world at the start of this flight: on the runway, ready to
    /// take off. The game and [`replay`] both begin here.
    pub fn world(&self) -> World {
        let mut world = World::with_seed(self.bounds, self.seed);
        world.start_level = start_level(self.level as i32);
        world.with_takeoff()
    }
}

/// Writes a flight down while it is flown. Call it for everything that
/// changes the world, in the order it happens.
#[derive(Debug, Clone)]
pub struct Recorder {
    bytes: Vec<u8>,
    bounds: Bounds,
    /// The input of the last ticks and how many there were (not yet
    /// written).
    run: Option<(i8, i8, u8)>,
}

impl Recorder {
    pub fn new(start: &Start) -> Self {
        let mut bytes = Vec::with_capacity(4096);
        bytes.extend_from_slice(&MAGIC);
        bytes.extend_from_slice(&start.seed.to_le_bytes());
        bytes.push(start_level(start.level as i32) as u8);
        push_bounds(&mut bytes, start.bounds);
        Self {
            bytes,
            bounds: start.bounds,
            run: None,
        }
    }

    /// The screen has this size now. Writes it down if it changed.
    pub fn resize(&mut self, bounds: Bounds) {
        if same_bits(bounds, self.bounds) {
            return;
        }
        self.flush();
        self.bytes.push(RESIZE);
        push_bounds(&mut self.bytes, bounds);
        self.bounds = bounds;
    }

    /// The player tapped "Next level" ([`World::go_on`]).
    pub fn go_on(&mut self) {
        self.flush();
        self.bytes.push(GO_ON);
    }

    /// One tick with this steering and throttle. Returns the input to
    /// hand to [`World::tick`]: the rounded one, which is what a replay
    /// will fly with.
    pub fn tick(&mut self, steer: f32, throttle: f32) -> Input {
        let (s, t) = (quantize(steer), quantize(throttle));
        match &mut self.run {
            Some((rs, rt, n)) if *rs == s && *rt == t && *n < u8::MAX => *n += 1,
            _ => {
                self.flush();
                self.run = Some((s, t, 1));
            }
        }
        input_of(s, t)
    }

    /// The replay so far.
    pub fn bytes(&self) -> Vec<u8> {
        let mut done = self.clone();
        done.flush();
        done.bytes
    }

    fn flush(&mut self) {
        if let Some((s, t, n)) = self.run.take() {
            self.bytes.extend_from_slice(&[TICKS, s as u8, t as u8, n]);
        }
    }
}

/// A level landed on a flight, and how long it took.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LevelTime {
    pub level: u32,
    /// [`World::millis_in_level`] at "Level complete": the score.
    pub millis: u64,
}

/// What a replay turned out to be, once flown again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub seed: u64,
    /// The level the flight started in.
    pub level: u32,
    /// Every level landed on this flight, in the order they were flown.
    pub completed: Vec<LevelTime>,
    /// Did the flight end (last life gone)? `false`: the player left it,
    /// or it is still going (a replay sent at "Level complete").
    pub over: bool,
    /// Ticks the replay ran in all, with takeoffs, landings and waiting.
    /// A real flight took at least this long ([`crate::TICK_SECONDS`] each).
    pub steps: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayError {
    /// Not a replay, or one from another version.
    BadHeader,
    /// Ends in the middle of a record.
    Truncated,
    /// A record of a kind this version does not know.
    UnknownRecord(u8),
    /// A screen size no real screen has ([`BOUNDS_WIDTH`] and so on).
    BadBounds,
    /// More than [`MAX_REPLAY_TICKS`].
    TooLong,
    /// "Next level" after the last level, which the game does not offer.
    NoNextLevel,
}

impl std::fmt::Display for ReplayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadHeader => write!(f, "not a replay of this version"),
            Self::Truncated => write!(f, "the replay ends in the middle of a record"),
            Self::UnknownRecord(kind) => write!(f, "unknown record {kind}"),
            Self::BadBounds => write!(f, "impossible screen size"),
            Self::TooLong => write!(f, "the replay is too long"),
            Self::NoNextLevel => write!(f, "there is no level after the last one"),
        }
    }
}

impl std::error::Error for ReplayError {}

/// The start a replay claims, without flying it.
pub fn start_of(bytes: &[u8]) -> Result<Start, ReplayError> {
    if bytes.len() < HEADER_LEN || bytes[..4] != MAGIC {
        return Err(ReplayError::BadHeader);
    }
    let seed = u64::from_le_bytes(bytes[4..12].try_into().expect("8 bytes"));
    let level = u32::from(bytes[12]);
    if level != start_level(level as i32) {
        return Err(ReplayError::BadHeader);
    }
    let bounds = read_bounds(&bytes[13..HEADER_LEN])?;
    Ok(Start {
        seed,
        level,
        bounds,
    })
}

/// Fly a recorded flight again and say how it went. Never panics, on
/// any bytes.
pub fn replay(bytes: &[u8]) -> Result<Outcome, ReplayError> {
    let start = start_of(bytes)?;
    let mut world = start.world();
    let mut steps = 0_u64;
    let mut completed = Vec::new();
    let mut rest = &bytes[HEADER_LEN..];
    while let Some((&kind, tail)) = rest.split_first() {
        rest = match kind {
            TICKS => {
                let (&[s, t, n], tail) = tail.split_first_chunk().ok_or(ReplayError::Truncated)?;
                steps += u64::from(n);
                if steps > MAX_REPLAY_TICKS {
                    return Err(ReplayError::TooLong);
                }
                let input = input_of(s as i8, t as i8);
                for _ in 0..n {
                    world.tick(&input);
                    for event in world.events() {
                        if let Event::LevelComplete { level } = *event {
                            completed.push(LevelTime {
                                level,
                                millis: world.millis_in_level(),
                            });
                        }
                    }
                }
                tail
            }
            RESIZE => {
                let (size, tail) = tail
                    .split_first_chunk::<8>()
                    .ok_or(ReplayError::Truncated)?;
                world.bounds = read_bounds(size)?;
                tail
            }
            GO_ON => {
                if !offers_next_level(&world) {
                    return Err(ReplayError::NoNextLevel);
                }
                world.go_on();
                tail
            }
            other => return Err(ReplayError::UnknownRecord(other)),
        };
    }
    Ok(Outcome {
        seed: start.seed,
        level: start.level,
        completed,
        over: world.is_over(),
        steps,
    })
}

/// May "Next level" be tapped now? Not at "Level complete" of the last
/// level: the game offers only "Menu" there, while core alone would go
/// round to level 1 again.
fn offers_next_level(world: &World) -> bool {
    world.level_complete().is_none_or(has_next_level)
}

fn same_bits(a: Bounds, b: Bounds) -> bool {
    a.width.to_bits() == b.width.to_bits() && a.height.to_bits() == b.height.to_bits()
}

fn push_bounds(bytes: &mut Vec<u8>, bounds: Bounds) {
    bytes.extend_from_slice(&bounds.width.to_le_bytes());
    bytes.extend_from_slice(&bounds.height.to_le_bytes());
}

fn read_bounds(bytes: &[u8]) -> Result<Bounds, ReplayError> {
    let f = |b: &[u8]| f32::from_le_bytes(b.try_into().expect("4 bytes"));
    let bounds = Bounds {
        width: f(&bytes[..4]),
        height: f(&bytes[4..8]),
    };
    if is_playable(bounds) {
        Ok(bounds)
    } else {
        Err(ReplayError::BadBounds)
    }
}

/// Within [`BOUNDS_WIDTH`], [`BOUNDS_HEIGHT`] and [`BOUNDS_ASPECT`].
fn is_playable(Bounds { width, height }: Bounds) -> bool {
    let within = |v: f32, (lo, hi): (f32, f32)| v.is_finite() && (lo..=hi).contains(&v);
    within(width, BOUNDS_WIDTH)
        && within(height, BOUNDS_HEIGHT)
        && within(width / height, BOUNDS_ASPECT)
}

/// The screen size the world uses: the real one, moved into the sizes a
/// replay accepts ([`BOUNDS_WIDTH`] and so on). A window dragged very
/// short for a moment then does not make a ranked flight void. Sizes that
/// are fine come back unchanged.
pub fn playable_bounds(bounds: Bounds) -> Bounds {
    if is_playable(bounds) {
        return bounds;
    }
    let fix = |v: f32, (lo, hi): (f32, f32)| if v.is_finite() { v.clamp(lo, hi) } else { lo };
    let height = fix(bounds.height, BOUNDS_HEIGHT);
    // A hair inside the aspect limits, so rounding in `width / height`
    // cannot land just outside them.
    let (lo, hi) = BOUNDS_ASPECT;
    let width = fix(bounds.width, BOUNDS_WIDTH).clamp(height * lo * 1.0001, height * hi * 0.9999);
    Bounds { width, height }
}

/// How many ticks a replay runs, read from its records without flying
/// it: cheap, so a server can refuse a replay before the real work.
/// The same as [`Outcome::steps`] for a replay [`replay`] accepts.
pub fn steps_of(bytes: &[u8]) -> Result<u64, ReplayError> {
    start_of(bytes)?;
    let mut steps = 0_u64;
    let mut rest = &bytes[HEADER_LEN..];
    while let Some((&kind, tail)) = rest.split_first() {
        let len = match kind {
            TICKS => 3,
            RESIZE => 8,
            GO_ON => 0,
            other => return Err(ReplayError::UnknownRecord(other)),
        };
        let body = tail.get(..len).ok_or(ReplayError::Truncated)?;
        if kind == TICKS {
            steps += u64::from(body[2]);
            if steps > MAX_REPLAY_TICKS {
                return Err(ReplayError::TooLong);
            }
        }
        rest = &tail[len..];
    }
    Ok(steps)
}

/// How often the demo pilot decides, and how far it looks ahead.
const DEMO_DECIDE_TICKS: u64 = 12;
const DEMO_LOOKAHEAD_TICKS: u32 = 75;

/// A pilot for tests and tools that looks ahead: every few ticks it
/// tries its steering on a copy of the world and keeps it while that
/// loses no life, else takes the first choice that does not (in a seeded
/// random order, so pilots differ). Asked once per tick, it flies a real
/// world as well as a made-up one: the game's test build
/// (`--features autopilot`) lets it fly in the browser.
#[derive(Debug, Clone)]
pub struct DemoPilot {
    rng: crate::rng::Rng,
    /// Its own airspeed, well clear of stall and overspeed, so pilots
    /// land with different times.
    cruise: f32,
    steer: f32,
    throttle: f32,
    ticks: u64,
}

impl DemoPilot {
    pub fn new(pilot_seed: u64) -> Self {
        let mut rng = crate::rng::Rng::new(pilot_seed);
        let cruise = rng.range(0.95, 1.3);
        Self {
            rng,
            cruise,
            steer: 0.0,
            throttle: 0.0,
            ticks: 0,
        }
    }

    /// Steering and throttle for the next tick of `world`.
    pub fn controls(&mut self, world: &World) -> (f32, f32) {
        if self.ticks.is_multiple_of(DEMO_DECIDE_TICKS) {
            self.throttle = ((self.cruise - world.player.airspeed) * 4.0).clamp(-1.0, 1.0);
            let throttle = self.throttle;
            let mut choices = [-1.0_f32, -0.5, 0.0, 0.5, 1.0];
            let turn = (self.rng.unit() * choices.len() as f32) as usize;
            choices.rotate_left(turn);
            let survives = |s: f32| {
                let mut ahead = world.clone();
                let input = rounded_input(s, throttle);
                (0..DEMO_LOOKAHEAD_TICKS).all(|_| {
                    ahead.tick(&input);
                    ahead.lives == world.lives
                })
            };
            if !survives(self.steer) {
                self.steer = choices
                    .into_iter()
                    .find(|&s| survives(s))
                    .unwrap_or(self.steer);
            }
        }
        self.ticks += 1;
        (self.steer, self.throttle)
    }
}

/// A made-up flight for tests and tools, flown by a [`DemoPilot`]. Flies
/// until the flight is over, `max_steps` ticks ran or the last level is
/// landed, tapping "Next level" along the way, and turns the screen once
/// half way. Returns the replay.
pub fn demo_flight(start: &Start, pilot_seed: u64, max_steps: u64) -> Vec<u8> {
    demo_flight_seen(start, pilot_seed, max_steps).0
}

/// [`demo_flight`], and the times its own world showed at every "Level
/// complete" (what the game would have shown and kept).
fn demo_flight_seen(start: &Start, pilot_seed: u64, max_steps: u64) -> (Vec<u8>, Vec<LevelTime>) {
    let mut seen = Vec::new();
    let mut pilot = DemoPilot::new(pilot_seed);
    let mut world = start.world();
    let mut rec = Recorder::new(start);
    for step in 0..max_steps {
        if world.is_over() {
            break;
        }
        if step == max_steps / 2 {
            // A turned phone or resized window, once.
            let bounds = Bounds {
                width: start.bounds.width * 0.9,
                height: start.bounds.height,
            };
            world.bounds = bounds;
            rec.resize(bounds);
        }
        if world.level_complete().is_some() {
            if !offers_next_level(&world) {
                break;
            }
            world.go_on();
            rec.go_on();
        }
        let (steer, throttle) = pilot.controls(&world);
        world.tick(&rec.tick(steer, throttle));
        if let Some(&Event::LevelComplete { level }) = world
            .events()
            .iter()
            .find(|e| matches!(e, Event::LevelComplete { .. }))
        {
            seen.push(LevelTime {
                level,
                millis: world.millis_in_level(),
            });
        }
    }
    (rec.bytes(), seen)
}
#[cfg(test)]
mod tests {
    use super::*;

    const PHONE: Bounds = Bounds {
        width: 844.0,
        height: 390.0,
    };

    fn start(seed: u64) -> Start {
        Start {
            seed,
            level: 1,
            bounds: PHONE,
        }
    }

    #[test]
    fn a_replay_flies_exactly_the_recorded_flight() {
        let start = start(7);
        let mut world = start.world();
        let mut rec = Recorder::new(&start);
        let mut pilot = crate::rng::Rng::new(99);
        for _ in 0..3_000 {
            world.tick(&rec.tick(pilot.range(-1.0, 1.0), pilot.range(-1.0, 1.0)));
        }
        let outcome = replay(&rec.bytes()).expect("a valid replay");
        assert_eq!(outcome.over, world.is_over());
        assert_eq!(outcome.steps, 3_000);
        assert_eq!((outcome.seed, outcome.level), (7, 1));
        assert_eq!(start_of(&rec.bytes()), Ok(start));
    }

    /// Long enough for the demo pilot to land level 1 (about 70 s).
    const ONE_LEVEL: u64 = 6_000;

    #[test]
    fn a_landed_level_has_the_time_the_game_showed() {
        let (bytes, seen) = demo_flight_seen(&start(3), 5, ONE_LEVEL);
        let outcome = replay(&bytes).expect("a valid replay");
        assert_eq!(outcome.completed, seen);
        let first = outcome.completed.first().expect("level 1 landed");
        assert_eq!(first.level, 1);
        assert!(first.millis > 30_000, "a level takes a while: {first:?}");
        assert!(!outcome.over);
    }

    #[test]
    fn a_whole_demo_flight_replays_the_same_every_time() {
        let bytes = demo_flight(&start(3), 5, ONE_LEVEL);
        let outcome = replay(&bytes).expect("a valid replay");
        assert!(!outcome.completed.is_empty());
        assert_eq!(replay(&bytes), Ok(outcome));
    }

    #[test]
    fn a_flight_that_lands_nothing_has_no_time() {
        let start = start(1);
        let mut world = start.world();
        let mut rec = Recorder::new(&start);
        // Straight into the ground, three times.
        while !world.is_over() {
            world.tick(&rec.tick(1.0, 0.0));
        }
        let outcome = replay(&rec.bytes()).expect("a valid replay");
        assert!(outcome.over);
        assert_eq!(outcome.completed, []);
    }

    #[test]
    fn another_seed_or_pilot_gives_another_flight() {
        let a = replay(&demo_flight(&start(1), 5, 2_000)).expect("valid");
        let b = replay(&demo_flight(&start(2), 5, 2_000)).expect("valid");
        let c = replay(&demo_flight(&start(1), 6, 2_000)).expect("valid");
        assert!(a != b || a != c);
    }

    #[test]
    fn held_input_takes_few_bytes() {
        let start = start(1);
        let mut rec = Recorder::new(&start);
        for _ in 0..600 {
            rec.tick(-1.0, 0.0);
        }
        // 600 ticks in runs of 255: three records of four bytes.
        assert_eq!(rec.bytes().len(), HEADER_LEN + 3 * 4);
    }

    #[test]
    fn the_world_sees_the_rounded_input() {
        let mut rec = Recorder::new(&start(1));
        let input = rec.tick(0.3, f32::NAN);
        assert_eq!(input.steer(), 38.0 / 127.0);
        assert_eq!(input.throttle(), 0.0);
        assert_eq!(rounded_input(0.3, f32::NAN), input);
        assert_eq!((quantize(2.0), quantize(-2.0)), (127, -127));
    }

    #[test]
    fn broken_replays_are_refused_not_flown() {
        let good = demo_flight(&start(1), 5, 500);
        assert_eq!(replay(b"nope"), Err(ReplayError::BadHeader));
        assert_eq!(replay(&good[..good.len() - 1]), Err(ReplayError::Truncated));
        let mut unknown = good.clone();
        unknown.push(9);
        assert_eq!(replay(&unknown), Err(ReplayError::UnknownRecord(9)));
        let mut level = good.clone();
        level[12] = 200;
        assert_eq!(replay(&level), Err(ReplayError::BadHeader));
    }

    #[test]
    fn there_is_no_next_level_after_the_last() {
        // Landing the quick way (lives to spare, no pilot), in the level
        // before the last and in the last: only the first goes on.
        for level in [crate::LEVEL_COUNT - 1, crate::LEVEL_COUNT] {
            let mut world = Start { level, ..start(3) }.world();
            world.lives = u32::MAX;
            assert!(offers_next_level(&world), "not at Level complete yet");
            while world.level_complete().is_none() {
                world.tick(&Input::NONE);
            }
            let last = level == crate::LEVEL_COUNT;
            assert_eq!(offers_next_level(&world), !last, "level {level}");
        }
        // `replay` refuses a "Next level" where this says no (see there);
        // anywhere but at "Level complete" the record changes nothing.
        let mut bytes = demo_flight(&start(1), 5, 100);
        let flown = replay(&bytes);
        bytes.push(GO_ON);
        assert_eq!(replay(&bytes), flown);
    }

    #[test]
    fn steps_are_read_without_flying() {
        let bytes = demo_flight(&start(3), 5, ONE_LEVEL);
        let outcome = replay(&bytes).expect("a valid replay");
        assert_eq!(steps_of(&bytes), Ok(outcome.steps));
        assert_eq!(
            steps_of(&bytes[..bytes.len() - 1]),
            Err(ReplayError::Truncated)
        );
        assert_eq!(steps_of(b"nope"), Err(ReplayError::BadHeader));
    }

    #[test]
    fn odd_screens_are_moved_into_the_playable_sizes() {
        let odd = [
            (1200.0, 140.0),
            (844.0, 1.0),
            (8000.0, 150.0),
            (100.0, 4000.0),
            (20_000.0, 20_000.0),
            (f32::NAN, 390.0),
            (844.0, f32::INFINITY),
            (0.0, 0.0),
        ];
        for (width, height) in odd {
            let fixed = playable_bounds(Bounds { width, height });
            assert!(is_playable(fixed), "{width}x{height} -> {fixed:?}");
        }
        // Real screens stay exactly as they are.
        for bounds in [
            PHONE,
            Bounds {
                width: 1366.0,
                height: 768.0,
            },
        ] {
            assert_eq!(playable_bounds(bounds), bounds);
        }
        // A flight recorded through them is accepted.
        let start = start(1);
        let mut rec = Recorder::new(&start);
        rec.tick(0.0, 0.0);
        rec.resize(playable_bounds(Bounds {
            width: 1200.0,
            height: 140.0,
        }));
        rec.tick(0.0, 0.0);
        assert!(replay(&rec.bytes()).is_ok());
    }

    #[test]
    fn core_maths_is_the_same_in_wasm_and_native() {
        // std's sin, cos, exp ... use the system's maths library natively
        // and Rust's own in wasm: a replay could fly differently on the
        // server. Core uses `libm` for them. (tilt.rs runs in the game
        // only, never in a replay.)
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src");
        let banned = [
            ".sin()",
            ".cos()",
            ".tan()",
            ".sin_cos()",
            ".exp()",
            ".exp2()",
            ".ln()",
            ".log(",
            ".log2()",
            ".log10()",
            ".powf(",
            ".asin()",
            ".acos()",
            ".atan()",
            ".atan2(",
            ".sinh()",
            ".cosh()",
            ".tanh()",
            ".hypot(",
            ".cbrt()",
            ".exp_m1()",
            ".ln_1p()",
        ];
        for entry in std::fs::read_dir(dir).expect("core's sources") {
            let path = entry.expect("a file").path();
            if path.file_name().is_some_and(|n| n == "tilt.rs") {
                continue;
            }
            let text = std::fs::read_to_string(&path).expect("readable");
            for (i, line) in text.lines().enumerate() {
                let code = line.split("//").next().unwrap_or("");
                for call in banned {
                    // (Quoted, as in the list above, it is not a call.)
                    let quoted = format!("\"{call}");
                    assert!(
                        !code.contains(call) || code.contains(&quoted),
                        "{}:{}: `{call}` from std; use libm",
                        path.display(),
                        i + 1
                    );
                }
            }
        }
    }

    #[test]
    fn impossible_screens_are_refused() {
        for bounds in [
            Bounds {
                width: f32::NAN,
                height: 390.0,
            },
            Bounds {
                width: 8000.0,
                height: 150.0,
            },
            Bounds {
                width: 844.0,
                height: 1.0,
            },
        ] {
            let bytes = Recorder::new(&Start {
                seed: 1,
                level: 1,
                bounds,
            })
            .bytes();
            assert_eq!(replay(&bytes), Err(ReplayError::BadBounds), "{bounds:?}");
        }
    }

    #[test]
    fn endless_replays_are_cut_off() {
        let mut bytes = Recorder::new(&start(1)).bytes();
        // Far more ticks than allowed, cheaply: the check comes first.
        for _ in 0..=(MAX_REPLAY_TICKS / 255) {
            bytes.extend_from_slice(&[TICKS, 0, 0, 255]);
        }
        assert_eq!(replay(&bytes), Err(ReplayError::TooLong));
    }
}
