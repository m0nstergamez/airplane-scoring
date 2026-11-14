//! Pure game logic. No rendering, no input polling, no timers.
//! The game crate drives this with `World::tick` and reads state to draw.

/// Fixed simulation step in seconds. The renderer accumulates frame time
/// and calls `tick` zero or more times per frame.
pub const TICK_SECONDS: f32 = 1.0 / 60.0;
/// Ticks in one second, for counting whole seconds.
const TICKS_PER_SECOND: u64 = 60;

mod airport;
mod balloons;
mod best_times;
mod city;
mod clock;
mod gulls;
mod huts;
mod input;
mod level;
pub mod replay;
mod rng;
mod scenery;
mod shape;
mod sound;
mod sunrise;
mod tilt;
mod towers;
mod weather;
pub use airport::{
    Airport, TyreSmoke, APPROACH_DISTANCE, BRAKE_SECONDS, CLEAR_SECONDS, CLIMB_OUT_SECONDS,
    CLIMB_OUT_STEER, COUNTDOWN_SECONDS, FLARE_PITCH, GEAR_DELAY_SECONDS, GEAR_RETRACT_SECONDS,
    MAIN_WHEEL_X, NOSE_DOWN_SECONDS, NOSE_WHEEL_X, ROLL_SECONDS, RUNWAY_GROUND, RUNWAY_HEIGHT,
    RUNWAY_LENGTH, RUNWAY_LIT_LEVELS, STRUT_REACH, TOUCHDOWN_INTO, TYRE_SMOKE_SECONDS,
    WHEELS_BELOW, WHEEL_RADIUS,
};
pub use balloons::{
    Balloon, BalloonWay, Balloons, Pad, BALLOON_COLOURS, BALLOON_GAP_SECONDS, BALLOON_HIGH,
    BALLOON_RADIUS, BALLOON_RISE, BASKET_BELOW, BASKET_HALF, BASKET_RADIUS, FIRST_BALLOON_SECONDS,
    PAD_GROUND, PAD_HEIGHT, PAD_WIDTH, TAKEOFF_AT, TOUCHDOWN_AT,
};
pub use best_times::{BestTimes, Placed, BEST_TIMES_COUNT};
pub use city::{
    City, Crane, CraneParts, FlyingHeli, Heli, HeliParts, HeliState, HighRise, HighRiseParts,
    Wreck, CITY_GAP_SECONDS, CITY_LEVEL, CITY_ROOM, CRANE_SHARE, FLYING_HELI_LOWEST,
    HIGH_RISES_IN_A_ROW, HIGH_RISE_STYLES,
};
pub use clock::{Clock, MAX_FRAME_SECONDS};
pub use gulls::{
    Gull, GullStart, Gulls, FIRST_GULL_SECONDS, GULL_BOB, GULL_CALL_CHANCE, GULL_FROM_LEFT,
    GULL_FROM_RIGHT, GULL_GAP_SECONDS, GULL_SPEED,
};
pub use huts::{
    Hut, Huts, FIRST_HUT_SECONDS, HUT_EAVES, HUT_GAP_SECONDS, HUT_GROUND, HUT_HEIGHT, HUT_INSET,
    HUT_STYLES, HUT_WIDTH,
};
pub use input::{
    pick_steer, pick_throttle, steer_from_keys, steer_toward, Input, TOUCH_EASE_DISTANCE,
};
pub use level::{
    has_next_level, level_view, start_level, LevelView, LEVEL_BANNER_SECONDS, LEVEL_COUNT,
    LEVEL_DISTANCE, LEVEL_FADE_SECONDS, LEVEL_SECONDS, RAINY_LEVELS,
};
pub use scenery::{background_x, scroll_distance, SCROLL_SPEED};
pub use shape::{any_overlap, Poly};
pub use sound::{
    Mix, BALLOON_BURNER_VOLUME, BALLOON_BURST_VOLUME, BEACH_LEVEL, BEACH_VOLUME, CRASH_VOLUME,
    DIVE_WIND, ENGINE_FAR_VOLUME, ENGINE_VOLUME, EXPLOSION_VOLUME, GEAR_VOLUME, GOOSE_HONK_VOLUME,
    GULL_CALL_VOLUME, MIX_CHANGE_PER_SECOND, NEW_LEVEL_VOLUME, RAIN_VOLUME, RUMBLE_VOLUME,
    STEAM_VOLUME, TAKEOFF_VOLUME, VOICE_VOLUME, WARNING_VOLUME, WIND_VOLUME,
};
pub use sunrise::{
    FarHighRise, FarHighRiseParts, Goose, HouseParts, Steam, Sunrise, GLARE_SECONDS, GOOSE_BODY,
    GOOSE_CALLS, GOOSE_GAP_SECONDS, STEAM_LIFT, SUNRISE_LEVEL, SUN_AT, SUN_RADIUS,
};
pub use tilt::{
    Tilt, LEAN_DEAD_ZONE_DEG, LEAN_FULL_DEG, TILT_DEAD_ZONE_DEG, TILT_FULL_DEG, TILT_MIN_GRAVITY,
    TILT_SMOOTHING,
};
pub use towers::{
    Tower, TowerParts, Towers, FIRST_TOWER_SECONDS, TOWER_FAR_SIZE, TOWER_GAP_SECONDS,
    TOWER_GROUND, TOWER_HEIGHT, TOWER_STYLES, TOWER_WIDTH,
};
pub use weather::{
    Weather, CALM_WIND, DRY_SECONDS, FIRST_DRY_SECONDS, FLASH_SECONDS, GUST_WIND,
    RAIN_CHANGE_PER_SECOND, RAIN_PEAK, RAIN_SECONDS, RAIN_WIND, STORM_RAIN, STRIKE_GAP_SECONDS,
    THUNDER_DELAY_SECONDS, THUNDER_VOLUME,
};

/// The screen height that every size and speed in px below is made for:
/// a phone in landscape. On other screens they grow or shrink with the
/// height ([`Bounds::scale`]), like the background picture does, so the
/// game looks and plays the same on a phone and on a computer.
pub const DESIGN_HEIGHT: f32 = 390.0;

/// How big the plane and the obstacles are, compared with the sizes in px
/// below (which are how they were first drawn). Smaller leaves more room
/// to dodge; speeds do not change with it. See [`Bounds::size`].
pub const OBJECT_SIZE: f32 = 0.41;

/// How big the plane gets near the ground, compared with its first
/// design (balloons get to 1 there). See [`Bounds::plane_depth`].
pub const PLANE_NEAR_SIZE: f32 = 2.4;

/// The airspeed you set with the throttle, as a factor on the game's
/// speed: 1 = cruising, the lowest and the highest you can go.
pub const AIRSPEED_RANGE: (f32, f32) = (0.5, 2.0);
/// How fast full throttle changes the airspeed, per second.
pub const AIRSPEED_CHANGE: f32 = 0.6;
/// Without throttle the airspeed drifts back toward 1 this fast, per second.
pub const AIRSPEED_RETURN: f32 = 0.15;
/// Above this airspeed: "Overspeed" (warning tone and voice). Stay above
/// it for [`OVERSPEED_SECONDS`] and the plane breaks apart: every life
/// is gone at once.
pub const OVERSPEED_AT: f32 = 1.5;
pub const OVERSPEED_SECONDS: f32 = 3.0;
/// Below this airspeed the wings lose lift: "Pull up" (warning light,
/// tone and voice) and the plane sags, more the slower it is, up to
/// [`STALL_SINK`] px per second at the slowest.
pub const STALL_AT: f32 = 0.75;
pub const STALL_SINK: f32 = 180.0;
/// While a warning lasts, its voice repeats this often, in seconds.
pub const WARNING_REPEAT_SECONDS: f32 = 1.5;
/// How long a warning voice speaks, in seconds (the longer one,
/// "Overspeed!", is about 1.1 s).
pub const VOICE_SECONDS: f32 = 1.1;
/// While the voice speaks, its warning tone plays this loud (share).
pub const VOICE_DUCK: f32 = 0.2;

/// How quickly the plane takes up a new climb or dive: its vertical speed
/// closes this share of the gap per second (exponentially; full speed
/// after about a quarter of a second). Smaller is smoother.
pub const CLIMB_RESPONSE: f32 = 8.0;

/// How far the plane tilts its nose at full climb or dive, in degrees.
pub const MAX_PITCH_DEG: f32 = 20.0;

/// How quickly the tilt follows, like [`CLIMB_RESPONSE`].
pub const PITCH_RESPONSE: f32 = 10.0;

/// Vertical speed of the player plane at full steer, in px per second.
pub const PLAYER_CLIMB_SPEED: f32 = 300.0;

/// Half the player plane's drawn height; keeps the whole plane on screen.
pub const PLAYER_HALF_HEIGHT: f32 = 18.0;

/// The player's plane sits at this fraction of the screen width.
pub const PLAYER_X_FRACTION: f32 = 0.2;

/// Radius of the other planes, in px.
pub const PLANE_RADIUS: f32 = 24.0;

/// The player's plane as drawn, in px at size 1, centred on its position
/// and flying right: fuselage, nose, wing, tail fin. Crashes use exactly
/// these parts (and the game crate draws them), so only a real touch
/// counts. The round cockpit sits on the fuselage.
pub const PLANE_FUSELAGE: [(f32, f32); 4] =
    [(-26.0, -5.0), (18.0, -5.0), (18.0, 5.0), (-26.0, 5.0)];
pub const PLANE_NOSE: [(f32, f32); 3] = [(18.0, -5.0), (30.0, 0.0), (18.0, 5.0)];
pub const PLANE_WING: [(f32, f32); 3] = [(-6.0, 0.0), (6.0, 0.0), (-10.0, 16.0)];
pub const PLANE_TAIL: [(f32, f32); 3] = [(-26.0, -5.0), (-18.0, -5.0), (-28.0, -18.0)];
/// The cockpit: centre and radius, in px at size 1.
pub const PLANE_COCKPIT: ((f32, f32), f32) = ((10.0, -3.0), 3.5);

/// After a crash the plane is out of control for this long.
pub const CRASH_SECONDS: f32 = 1.5;

/// [`CRASH_SECONDS`] in ticks.
pub const CRASH_TICKS: u32 = (CRASH_SECONDS * 60.0) as u32;

/// A crashed plane falls this fast, in px per second.
pub const CRASH_FALL_SPEED: f32 = 120.0;

/// Lives at the start of a flight. Each crash costs one; when the last
/// one is gone, the game is over.
pub const LIVES: u32 = 3;

/// After a crash, once the plane flies again, it is safe for this long:
/// it blinks and cannot crash, so one flock of gulls costs only one life.
pub const SAFE_SECONDS: f32 = 2.0;

/// [`SAFE_SECONDS`] in ticks.
pub const SAFE_TICKS: u32 = (SAFE_SECONDS * 60.0) as u32;

/// The flight time as a clock, minutes and seconds: 125 → "2:05".
pub fn clock_text(seconds: u64) -> String {
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

/// A best time with milliseconds: 62_483 → "1:02.483".
pub fn best_time_text(millis: u64) -> String {
    format!("{}.{:03}", clock_text(millis / 1000), millis % 1000)
}

/// How much the plane smokes after losing lives: index = lives lost
/// (0 = none, 1 = a clear grey trail, 2 = thick dark smoke and a flame).
/// The last life lost is an explosion instead.
pub const DAMAGE_SMOKE: [f32; 3] = [0.0, 0.5, 1.0];

/// How far the screen shakes at the moment of the explosion, in px (on a
/// phone; it grows with the screen). It fades out over the explosion.
pub const SHAKE_PX: f32 = 10.0;

/// The game speeds up a little with every point: the landscape and
/// everything in it slide past this much faster per point (0.1 %: after
/// a minute, 120 points, about 12 % faster).
pub const SPEED_PER_SECOND: f32 = 0.002;
/// It never gets faster than this (1 = the start).
pub const MAX_SPEED: f32 = 1.5;

/// "Game over" shows at least this long before a tap starts again, so a
/// finger still steering does not skip it.
pub const RESTART_WAIT_SECONDS: f32 = 1.0;

/// Something that happened during a tick. The game crate reacts to these
/// with one-shot effects (a sound, a flash); they never change the rules.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Event {
    /// The player's plane hit another plane.
    Crash,
    /// Landing: the main wheels touch the runway (tyre smoke).
    MainWheelsTouch,
    /// Landing: the nose wheel comes down onto the runway (tyre smoke).
    NoseWheelTouch,
    /// Lightning struck. `distance` is 0 (overhead) to 1 (far away).
    Lightning { distance: f32 },
    /// Thunder from an earlier strike arrives. `volume` is 0..1.
    Thunder { volume: f32 },
    /// A new level begins (after the last one's landing; also when the
    /// game starts over with level 1).
    NewLevel { level: u32 },
    /// The plane has landed and stopped: `level` is done (its tune).
    LevelComplete { level: u32 },
    /// The landing gear comes down for the approach (its whir and clunk).
    GearDown,
    /// A gull calls as it appears.
    GullCall,
    /// A V of geese calls as it appears (level 4): one of
    /// [`GOOSE_CALLS`] calls, never the same twice in a row.
    GooseHonk { call: u8 },
    /// The plane flies into a column of steam: a rushing hiss as it is
    /// thrown up (level 4).
    SteamHiss,
    /// A hot-air balloon's burner roars as it appears.
    BalloonBurner,
    /// The plane flew into a balloon: it explodes (a bang) and falls.
    BalloonBurst,
    /// The plane flew into a helicopter: it explodes (a bang) and falls.
    HeliCrash,
    /// The last life is gone: the plane has fallen and the flight is over.
    GameOver,
    /// The last life is gone: the plane explodes (instead of a crash).
    Explosion,
    /// The takeoff roll begins: the engine's spool-up sound.
    Takeoff,
    /// The landing gear starts folding up after lift-off: its whir and
    /// clunk.
    GearUp,
    /// The plane got too fast: the "Overspeed" voice (also repeats).
    Overspeed,
    /// The plane got too slow: the "Pull up" voice (also repeats).
    PullUp,
}

/// Whole ticks in `seconds` (rounded, so 3 s is exactly 180 ticks).
fn seconds_to_ticks(seconds: f32) -> u32 {
    (seconds / TICK_SECONDS).round() as u32
}

/// A cockpit warning on screen.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Warning {
    /// Too fast; it breaks apart when `seconds_left` reaches 0.
    Overspeed { seconds_left: f32 },
    /// Too slow: the plane sags.
    PullUp,
}

/// How a level ends: the approach to the landing runway (the autopilot
/// flies), the rollout after touchdown, "Level complete", and the
/// countdown into the next level. Each counts its own ticks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Landing {
    Approach(u32),
    Rollout(u32),
    Complete(u32),
    Countdown(u32),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Plane {
    pub name: String,
    pub pos: Vec2,
    pub vel: Vec2,
}

/// The plane the player steers. It stays at a fixed x; steering moves it
/// up and down.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Player {
    pub pos: Vec2,
    /// The steer value used in the last tick (0 while crashed).
    pub steer: f32,
    /// Vertical speed, px per second (down is positive). It follows the
    /// steering smoothly ([`CLIMB_RESPONSE`]).
    pub vel_y: f32,
    /// How the plane is tilted, radians: negative = nose up (climbing),
    /// positive = nose down. Follows `vel_y` ([`MAX_PITCH_DEG`]).
    pub pitch: f32,
    /// Airspeed set with the throttle, 1 = cruising ([`AIRSPEED_RANGE`]).
    pub airspeed: f32,
    /// Ticks left until a crashed plane flies again; 0 = flying.
    pub crash_ticks: u32,
    /// Ticks left of the safe time after a crash; 0 = can crash.
    pub safe_ticks: u32,
    /// Overlapping another plane right now. A crash needs a *new* touch,
    /// so a plane that recovers inside another one does not crash again.
    touching: bool,
}

impl Player {
    /// A plane flying level at `pos`.
    pub const fn at(pos: Vec2) -> Self {
        Self {
            pos,
            steer: 0.0,
            vel_y: 0.0,
            pitch: 0.0,
            airspeed: 1.0,
            crash_ticks: 0,
            safe_ticks: 0,
            touching: false,
        }
    }

    /// Out of control after hitting another plane.
    pub fn is_crashed(&self) -> bool {
        self.crash_ticks > 0
    }

    /// The plane as drawn, for crashing, on this screen (its size follows
    /// [`Bounds::plane_depth`]).
    pub fn shape(&self, bounds: Bounds) -> Vec<Poly> {
        let size = bounds.plane_depth(self.pos.y);
        let at = |points: &[(f32, f32)]| Poly::at_turned(self.pos, size, self.pitch, points);
        let ((cx, cy), r) = PLANE_COCKPIT;
        let (sin, cos) = self.pitch.sin_cos();
        let (cx, cy) = (cx * size, cy * size);
        let cockpit = Vec2::new(
            self.pos.x + cx * cos - cy * sin,
            self.pos.y + cx * sin + cy * cos,
        );
        vec![
            at(&PLANE_FUSELAGE),
            at(&PLANE_NOSE),
            at(&PLANE_WING),
            at(&PLANE_TAIL),
            Poly::ellipse(cockpit, r * size, r * size),
        ]
    }

    /// Too fast: "Overspeed".
    pub fn is_overspeed(&self) -> bool {
        self.airspeed > OVERSPEED_AT && !self.is_crashed()
    }

    /// Too slow: "Pull up".
    pub fn is_stalling(&self) -> bool {
        self.airspeed < STALL_AT && !self.is_crashed()
    }

    /// Flying again after a crash, but cannot crash yet (it blinks).
    pub fn is_safe(&self) -> bool {
        self.safe_ticks > 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    pub width: f32,
    pub height: f32,
}

impl Bounds {
    /// How much bigger than at [`DESIGN_HEIGHT`] everything is on this
    /// screen: 1 on a phone in landscape, about 2.3 on a 900 px high
    /// window. Multiply every size and speed in px by it.
    pub fn scale(&self) -> f32 {
        self.height.max(1.0) / DESIGN_HEIGHT
    }

    /// The factor for sizes of the plane and obstacles (radii, heights)
    /// on this screen: [`Bounds::scale`] times [`OBJECT_SIZE`]. Speeds and
    /// distances use [`Bounds::scale`] alone.
    pub fn size(&self) -> f32 {
        self.scale() * OBJECT_SIZE
    }

    /// Depth: the size factor of balloons and gulls at height `y`
    /// (screen px). At the top of the screen they are far away and
    /// smallest ([`Bounds::size`]); lower down they come closer and grow
    /// steadily over the whole height, until they are full size
    /// ([`Bounds::scale`]) at the height of a balloon standing on a launch
    /// pad in the foreground, and stay full size below.
    pub fn depth(&self, y: f32) -> f32 {
        self.depth_up_to(y, 1.0)
    }

    /// The plane's depth: like [`Bounds::depth`], just as small at the
    /// top, but growing to [`PLANE_NEAR_SIZE`] near the ground.
    pub fn plane_depth(&self, y: f32) -> f32 {
        self.depth_up_to(y, PLANE_NEAR_SIZE)
    }

    /// How near something at height `y` is: 0 at the top of the screen
    /// (far away) to 1 from the height of a balloon standing on a pad
    /// down (in the foreground). Depth sizes and the engine's loudness
    /// follow it.
    pub fn nearness(&self, y: f32) -> f32 {
        let near_y = Balloon::standing_y(*self);
        if near_y > 0.0 {
            (y / near_y).clamp(0.0, 1.0)
        } else {
            1.0
        }
    }

    fn depth_up_to(&self, y: f32, near_size: f32) -> f32 {
        let near = self.nearness(y);
        self.scale() * (OBJECT_SIZE + (near_size - OBJECT_SIZE) * near)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct World {
    pub bounds: Bounds,
    pub player: Player,
    pub planes: Vec<Plane>,
    /// The obstacles of level 1: balloons in the air, huts on the ground.
    pub balloons: Balloons,
    pub huts: Huts,
    /// The obstacles at the beach: gulls in the air, lifeguard towers in
    /// the sand.
    pub gulls: Gulls,
    pub towers: Towers,
    /// The obstacles of level 3: high-rises and cranes.
    pub city: City,
    /// The sunrise (level 4): geese, steam and the low sun's glare.
    pub sunrise: Sunrise,
    /// Was the plane in a column of steam last tick? (It hisses on the
    /// way in.)
    in_steam: bool,
    pub weather: Weather,
    /// Volumes of the looping sounds, eased toward what fits the world now.
    pub mix: Mix,
    pub ticks: u64,
    /// The level the flight started in (1 ..= `LEVEL_COUNT`).
    pub start_level: u32,
    /// How many levels have been landed (completed) on this flight.
    completed: u32,
    /// Ticks flown in the level now: its time.
    level_ticks: u64,
    /// How far the plane has come in the level now, in px of landscape
    /// at scale 1. At [`LEVEL_DISTANCE`] it lands.
    level_distance: f32,
    /// The landing at the end of a level, while it goes on.
    pub landing: Option<Landing>,
    /// A tap asked to go on from "Level complete" now.
    go_on: bool,
    /// Lives left, [`LIVES`] at the start.
    pub lives: u32,
    /// Where the plane blew up when the last life went; `None` before.
    pub explosion: Option<Vec2>,
    /// Puffs of tyre smoke from the landing's touchdowns.
    pub tyre_smoke: Vec<TyreSmoke>,
    /// How far the landscape has slid past, in px at phone size (it
    /// speeds up with the time flown, see [`World::speed`]).
    pub distance: f64,
    /// The airport the flight starts from, while it is on screen.
    pub airport: Option<Airport>,
    /// Ticks since the takeoff began, while it lasts; `None` once the
    /// player flies (or for a flight that starts in the air).
    takeoff: Option<u32>,
    /// Ticks spent in overspeed / stalling in a row (0 = not now).
    overspeed_ticks: u32,
    stall_ticks: u32,
    /// Ticks since the game ended; 0 while flying.
    over_ticks: u32,
    events: Vec<Event>,
}

impl World {
    /// A world with the weather of seed 0.
    pub fn new(bounds: Bounds) -> Self {
        Self::with_seed(bounds, 0)
    }

    /// The seed decides the weather. Same seed and inputs, same game.
    pub fn with_seed(bounds: Bounds, seed: u64) -> Self {
        let player = Player::at(Vec2::new(
            bounds.width * PLAYER_X_FRACTION,
            bounds.height / 2.0,
        ));
        Self {
            bounds,
            player,
            planes: Vec::new(),
            balloons: Balloons::new(seed),
            huts: Huts::new(seed),
            gulls: Gulls::new(seed),
            towers: Towers::new(seed),
            city: City::new(seed),
            sunrise: Sunrise::new(seed),
            in_steam: false,
            weather: Weather::new(seed),
            mix: Mix::SILENT,
            ticks: 0,
            start_level: 1,
            completed: 0,
            level_ticks: 0,
            level_distance: 0.0,
            landing: None,
            go_on: false,
            lives: LIVES,
            explosion: None,
            tyre_smoke: Vec::new(),
            distance: 0.0,
            airport: None,
            takeoff: None,
            overspeed_ticks: 0,
            stall_ticks: 0,
            over_ticks: 0,
            events: Vec::new(),
        }
    }

    /// Which level is on screen now. A level ends with a landing and the
    /// next starts with a takeoff, so nothing fades over and no banner
    /// shows ("Level complete" and the countdown say it instead).
    pub fn levels(&self) -> LevelView {
        let start = start_level(self.start_level as i32);
        let level = (start - 1 + self.completed) % LEVEL_COUNT + 1;
        LevelView {
            level,
            next: level % LEVEL_COUNT + 1,
            fade: 0.0,
            banner: 0.0,
        }
    }

    /// "Level N complete" is showing: the level just done.
    pub fn level_complete(&self) -> Option<u32> {
        match self.landing {
            // (The next level begins only after it.)
            Some(Landing::Complete(_)) => Some(self.levels().level),
            _ => None,
        }
    }

    /// The countdown into the next level: 3, 2, 1 (seconds left).
    pub fn countdown(&self) -> Option<u32> {
        match self.landing {
            Some(Landing::Countdown(t)) => {
                let left = COUNTDOWN_SECONDS - t as f32 * TICK_SECONDS;
                Some(left.ceil().max(1.0) as u32)
            }
            _ => None,
        }
    }

    /// A tap at "Level complete": go on to the next level now.
    pub fn go_on(&mut self) {
        if matches!(self.landing, Some(Landing::Complete(_))) {
            self.go_on = true;
        }
    }

    /// Is the autopilot flying (approach and rollout)?
    pub fn autopilot(&self) -> bool {
        matches!(
            self.landing,
            Some(Landing::Approach(_) | Landing::Rollout(_))
        )
    }

    /// Does the plane fly on its own now (takeoff, approach, rollout)? Then
    /// the player cannot react, so the speed warnings stay off and it
    /// cannot stall: the airspeed just settles back to cruising.
    fn hands_off(&self) -> bool {
        self.autopilot() || self.takeoff.is_some()
    }

    /// Is the flight over? True once the last life is gone and the plane
    /// has fallen. Then nothing moves any more.
    /// How much the plane smokes now, 0..1 ([`DAMAGE_SMOKE`]): more with
    /// every life lost. 0 once it has exploded.
    pub fn smoke(&self) -> f32 {
        if self.explosion.is_some() {
            return 0.0;
        }
        let lost = LIVES.saturating_sub(self.lives) as usize;
        DAMAGE_SMOKE[lost.min(DAMAGE_SMOKE.len() - 1)]
    }

    /// How far the explosion has gone, 0 (just blew up) .. 1 (over);
    /// `None` without an explosion.
    pub fn explosion_progress(&self) -> Option<f32> {
        self.explosion?;
        Some(1.0 - self.player.crash_ticks as f32 / CRASH_TICKS as f32)
    }

    /// How far to shift the landscape and everything in it this tick, in
    /// screen px: strongest at the explosion's bang, fading to nothing by
    /// its end; (0, 0) otherwise. The direction jumps around with the
    /// ticks, so it shakes, and follows from them, so it is repeatable.
    pub fn shake(&self) -> Vec2 {
        let Some(t) = self.explosion_progress() else {
            return Vec2::new(0.0, 0.0);
        };
        let strength = SHAKE_PX * self.bounds.scale() * (1.0 - t).powi(2);
        let k = self.ticks as f32;
        Vec2::new(
            strength * libm::sinf(k * 1.7),
            strength * 0.7 * libm::cosf(k * 2.3 + 1.0),
        )
    }

    /// How fast the game goes now: 1 at the start, a little faster with
    /// every second flown ([`SPEED_PER_SECOND`], at most [`MAX_SPEED`]), times the
    /// airspeed you set with the throttle.
    pub fn speed(&self) -> f32 {
        let flown = (1.0 + SPEED_PER_SECOND * self.seconds_flown() as f32).min(MAX_SPEED);
        flown * self.player.airspeed * self.rolling()
    }

    /// Start this flight on the airport's runway (the game does this for
    /// every new flight, in whatever level it starts).
    pub fn with_takeoff(mut self) -> Self {
        self.airport = Some(Airport::at_start(self.bounds));
        self.takeoff = Some(0);
        // On the runway right away, also while a countdown still runs.
        self.player.pos.y = self.runway_y();
        self
    }

    /// While the plane rolls down the runway, the landscape speeds up from
    /// standing still: 0..1 over [`ROLL_SECONDS`]; 1 otherwise.
    fn rolling(&self) -> f32 {
        match (self.takeoff, self.landing) {
            (_, Some(Landing::Rollout(t))) => {
                (1.0 - t as f32 * TICK_SECONDS / BRAKE_SECONDS).max(0.0)
            }
            (_, Some(Landing::Complete(_) | Landing::Countdown(_))) => 0.0,
            (Some(t), _) => (t as f32 * TICK_SECONDS / ROLL_SECONDS).min(1.0),
            (None, _) => 1.0,
        }
    }

    /// Is the plane still on the runway (not yet lifted off)?
    pub fn on_runway(&self) -> bool {
        self.takeoff
            .is_some_and(|t| (t as f32 * TICK_SECONDS) < ROLL_SECONDS)
    }

    /// Rolling out after touchdown (braking on the landing runway).
    pub fn rolling_out(&self) -> bool {
        matches!(self.landing, Some(Landing::Rollout(_)))
    }

    /// The landing gear: 1 = down (on the runway and just after lift-off),
    /// folding up to 0 over [`GEAR_RETRACT_SECONDS`].
    pub fn gear(&self) -> f32 {
        match self.landing {
            // Coming down for the approach.
            Some(Landing::Approach(t)) => {
                return (t as f32 * TICK_SECONDS / GEAR_RETRACT_SECONDS).min(1.0)
            }
            Some(_) => return 1.0,
            None => {}
        }
        let Some(t) = self.takeoff else { return 0.0 };
        let since = t as f32 * TICK_SECONDS - ROLL_SECONDS - GEAR_DELAY_SECONDS;
        (1.0 - since / GEAR_RETRACT_SECONDS).clamp(0.0, 1.0)
    }

    /// Is the takeoff still going (rolling or climbing out on its own)?
    pub fn taking_off(&self) -> bool {
        self.takeoff.is_some()
    }

    /// The nose-up pitch right after touchdown, lowering to 0 over
    /// [`NOSE_DOWN_SECONDS`]; 0 otherwise.
    fn flare_pitch(&self) -> f32 {
        match self.landing {
            Some(Landing::Rollout(t)) => {
                let down = t as f32 * TICK_SECONDS / NOSE_DOWN_SECONDS;
                FLARE_PITCH * (1.0 - down).max(0.0)
            }
            _ => 0.0,
        }
    }

    /// Where the plane's centre is with its main wheels on the runway and
    /// the nose at `pitch` (the gear turns with the plane, as drawn).
    fn runway_y_at(&self, pitch: f32) -> f32 {
        let top = Airport::runway_top(self.bounds);
        let size = self.bounds.plane_depth(top);
        let (sin, cos) = pitch.sin_cos();
        let main = MAIN_WHEEL_X * sin + STRUT_REACH * cos + WHEEL_RADIUS;
        let nose = NOSE_WHEEL_X * sin + STRUT_REACH * cos + WHEEL_RADIUS;
        top - main.max(nose) * size
    }

    /// Tyre smoke where the wheel at `wheel_x` (see [`MAIN_WHEEL_X`])
    /// meets the runway, the plane at `pitch`.
    fn puff_at(&mut self, wheel_x: f32, pitch: f32) {
        let top = Airport::runway_top(self.bounds);
        let size = self.bounds.plane_depth(top);
        let (sin, cos) = pitch.sin_cos();
        let x = self.player.pos.x + (wheel_x * cos - STRUT_REACH * sin) * size;
        self.tyre_smoke.push(TyreSmoke {
            pos: Vec2::new(x, top),
            ticks: 0,
        });
    }

    /// Are the runway's lights on? In the dark levels
    /// ([`RUNWAY_LIT_LEVELS`]).
    pub fn runway_lit(&self) -> bool {
        RUNWAY_LIT_LEVELS.contains(&self.levels().level)
    }

    /// Where the plane's centre is while it stands on the runway.
    fn runway_y(&self) -> f32 {
        let y = Airport::runway_top(self.bounds);
        // Its size there follows the depth at that height (close enough
        // at the bottom of the screen, where it is full size).
        y - WHEELS_BELOW * self.bounds.plane_depth(y)
    }

    /// Is a warning voice speaking now? `ticks`: how long that warning has
    /// lasted (its voice starts at tick 1 and repeats).
    fn voice_speaking(&self, ticks: u32) -> bool {
        if ticks == 0 {
            return false;
        }
        let repeat = (WARNING_REPEAT_SECONDS / TICK_SECONDS).round() as u32;
        let spoken = (VOICE_SECONDS / TICK_SECONDS).round() as u32;
        (ticks - 1) % repeat < spoken
    }

    /// The plane is destroyed at once: it explodes where it is, and every
    /// life is gone (overspeed for too long, or stalling into the ground).
    fn break_apart(&mut self) {
        self.lives = 0;
        self.player.crash_ticks = CRASH_TICKS;
        self.player.steer = 0.0;
        self.explosion = Some(self.player.pos);
        self.events.push(Event::Explosion);
    }

    /// Count the time in overspeed and stalling, send their voices when
    /// they start and every [`WARNING_REPEAT_SECONDS`], and break the plane
    /// apart after [`OVERSPEED_SECONDS`] of overspeed.
    fn warnings(&mut self) {
        let repeat = (WARNING_REPEAT_SECONDS / TICK_SECONDS).round() as u32;
        let flying = self.explosion.is_none() && !self.is_over() && !self.hands_off();
        let says = |ticks: u32| ticks >= 1 && (ticks - 1).is_multiple_of(repeat);
        if flying && self.player.is_overspeed() {
            self.overspeed_ticks += 1;
            if says(self.overspeed_ticks) {
                self.events.push(Event::Overspeed);
            }
            if self.overspeed_ticks as f32 * TICK_SECONDS >= OVERSPEED_SECONDS {
                // Too fast for too long: it breaks apart.
                self.break_apart();
                self.overspeed_ticks = 0;
            }
        } else {
            self.overspeed_ticks = 0;
        }
        if flying && self.player.is_stalling() {
            self.stall_ticks += 1;
            if says(self.stall_ticks) {
                self.events.push(Event::PullUp);
            }
        } else {
            self.stall_ticks = 0;
        }
    }

    /// The warning to show now, if any.
    pub fn warning(&self) -> Option<Warning> {
        if self.explosion.is_some() || self.hands_off() {
            None
        } else if self.player.is_overspeed() {
            let limit = (OVERSPEED_SECONDS / TICK_SECONDS).round() as u32;
            let left = limit.saturating_sub(self.overspeed_ticks) as f32 * TICK_SECONDS;
            Some(Warning::Overspeed { seconds_left: left })
        } else if self.player.is_stalling() {
            Some(Warning::PullUp)
        } else {
            None
        }
    }

    pub fn is_over(&self) -> bool {
        self.lives == 0 && !self.player.is_crashed()
    }

    /// May a tap start a new flight? Only once "Game over" has shown for
    /// [`RESTART_WAIT_SECONDS`].
    pub fn can_restart(&self) -> bool {
        self.is_over() && self.over_ticks as f32 * TICK_SECONDS >= RESTART_WAIT_SECONDS
    }

    /// Full seconds flown in this flight, all levels together (the game
    /// speeds up with them, see [`World::speed`]). Stands still at "Level
    /// complete" and game over, since time stands still then.
    pub fn seconds_flown(&self) -> u64 {
        self.ticks / TICKS_PER_SECOND
    }

    /// Full seconds flown in the level now: the clock on screen (see
    /// [`clock_text`]) and the time for its best times. Starts again at
    /// 0:00 with every level; stands still like [`World::seconds_flown`].
    pub fn seconds_in_level(&self) -> u64 {
        self.level_ticks / TICKS_PER_SECOND
    }

    /// The time in the level now, in milliseconds: at "Level complete",
    /// the time for its best times ([`best_time_text`]).
    pub fn millis_in_level(&self) -> u64 {
        (self.level_ticks * 1000 + TICKS_PER_SECOND / 2) / TICKS_PER_SECOND
    }

    /// How much of the level is behind the plane: 0 at the start, 1 when
    /// the landing begins.
    pub fn level_progress(&self) -> f32 {
        (self.level_distance / LEVEL_DISTANCE).min(1.0)
    }

    /// Steering toward a finger at `target_y` (screen px), from where the
    /// plane is now. Eases off within [`TOUCH_EASE_DISTANCE`], measured at
    /// [`DESIGN_HEIGHT`] like every other size.
    /// The plane's momentum counts: it aims from where it will soon be,
    /// so it settles on the finger without shooting past.
    pub fn steer_toward(&self, target_y: f32) -> f32 {
        let scale = self.bounds.scale();
        let ahead = self.player.pos.y + self.player.vel_y / CLIMB_RESPONSE;
        steer_toward(target_y / scale, ahead / scale)
    }

    /// What happened in the last tick. Read it after every `tick`:
    /// the next one starts a new list.
    pub fn events(&self) -> &[Event] {
        &self.events
    }

    pub fn spawn_plane(&mut self, name: impl Into<String>, pos: Vec2, vel: Vec2) -> usize {
        self.planes.push(Plane {
            name: name.into(),
            pos,
            vel,
        });
        self.planes.len() - 1
    }

    /// Advance the simulation by exactly one fixed step.
    /// Deterministic: same inputs, same output, every time.
    pub fn tick(&mut self, input: &Input) {
        let Bounds { width, height } = self.bounds;
        let scale = self.bounds.scale();
        self.events.clear();

        // The takeoff: only for a flight that starts in level 1. The plane
        // stands on the runway, rolls, lifts off and climbs out on its own;
        // the controls count only after that.
        // The level's end: "complete", then the countdown; nothing moves.
        match self.landing {
            Some(Landing::Complete(t)) => {
                // Waits for the player: "Next level" (or "Menu" in the game).
                if self.go_on {
                    self.next_level();
                } else {
                    self.landing = Some(Landing::Complete(t + 1));
                }
                self.quiet_down();
                return;
            }
            Some(Landing::Countdown(t)) => {
                let done = t + 1 >= seconds_to_ticks(COUNTDOWN_SECONDS);
                self.landing = (!done).then_some(Landing::Countdown(t + 1));
                self.quiet_down();
                return;
            }
            _ => {}
        }
        // Time's up in this level: the landing runway comes in and the
        // autopilot takes over (gear down).
        let level_over = self.level_distance >= LEVEL_DISTANCE;
        if level_over
            && self.landing.is_none()
            && self.takeoff.is_none()
            && self.explosion.is_none()
            && !self.is_over()
            && !self.player.is_crashed()
        {
            self.landing = Some(Landing::Approach(0));
            self.airport = Some(Airport {
                x: width + APPROACH_DISTANCE * scale,
            });
            self.events.push(Event::GearDown);
        }

        let takeoff = self.takeoff;
        let autopilot = self.autopilot_steer();
        let input = &match (takeoff, autopilot) {
            (_, Some(steer)) => Input::new(steer),
            (Some(t), _) if (t as f32 * TICK_SECONDS) < ROLL_SECONDS => Input::NONE,
            (Some(_), _) => Input::new(CLIMB_OUT_STEER),
            (None, None) => *input,
        };
        if takeoff == Some(0) {
            self.events.push(Event::Takeoff);
        }
        let gear_up = ((ROLL_SECONDS + GEAR_DELAY_SECONDS) / TICK_SECONDS).round() as u32;
        if takeoff == Some(gear_up) {
            self.events.push(Event::GearUp);
        }
        if let Some(t) = takeoff {
            let done = (ROLL_SECONDS + CLIMB_OUT_SECONDS) / TICK_SECONDS;
            self.takeoff = (((t + 1) as f32) < done).then_some(t + 1);
        }

        // Game over: everything stands still and the sound fades out.
        if self.is_over() {
            self.over_ticks += 1;
            self.mix = self
                .mix
                .approach(Mix::SILENT, MIX_CHANGE_PER_SECOND * TICK_SECONDS);
            return;
        }

        // A crashed plane ignores the controls and falls until it recovers;
        // then it is safe for a while (with the last life gone, it is over).
        let wanted = if self.player.is_crashed() {
            self.player.crash_ticks -= 1;
            self.player.steer = 0.0;
            if self.player.crash_ticks == 0 {
                if self.lives == 0 {
                    self.events.push(Event::GameOver);
                    return;
                }
                self.player.safe_ticks = SAFE_TICKS;
            }
            CRASH_FALL_SPEED * scale
        } else {
            self.player.safe_ticks = self.player.safe_ticks.saturating_sub(1);
            self.player.steer = input.steer();
            input.steer() * PLAYER_CLIMB_SPEED * scale
        };
        // Throttle: gas and brake change the airspeed; without, it drifts
        // back to cruising. A crashed plane cannot be steered at all.
        let throttle = if self.player.is_crashed() {
            0.0
        } else {
            input.throttle()
        };
        let a = &mut self.player.airspeed;
        if throttle != 0.0 {
            *a += throttle * AIRSPEED_CHANGE * TICK_SECONDS;
        } else {
            let back = AIRSPEED_RETURN * TICK_SECONDS;
            *a += (1.0 - *a).clamp(-back, back);
        }
        *a = a.clamp(AIRSPEED_RANGE.0, AIRSPEED_RANGE.1);
        // Too slow: the wings lose lift and the plane sags.
        let wanted = if self.player.is_stalling() && !self.hands_off() {
            let deep = (STALL_AT - self.player.airspeed) / (STALL_AT - AIRSPEED_RANGE.0);
            wanted + STALL_SINK * scale * deep.clamp(0.0, 1.0)
        } else {
            wanted
        };
        self.warnings();
        // A crashed plane drops at once; otherwise the vertical speed
        // follows the steering smoothly, and the nose tilts with it.
        let player = &mut self.player;
        player.vel_y = if player.is_crashed() {
            wanted
        } else {
            let ease = 1.0 - libm::expf(-CLIMB_RESPONSE * TICK_SECONDS);
            player.vel_y + (wanted - player.vel_y) * ease
        };
        let full = (PLAYER_CLIMB_SPEED * scale).max(1e-3);
        let max_pitch = MAX_PITCH_DEG.to_radians();
        let tilt = (player.vel_y / full).clamp(-1.0, 1.0) * max_pitch;
        let ease = 1.0 - libm::expf(-PITCH_RESPONSE * TICK_SECONDS);
        player.pitch += (tilt - player.pitch) * ease;
        let fall = player.vel_y;
        // The plane is small up high and big down low (depth), so it
        // reaches each edge with its size there.
        let top = PLAYER_HALF_HEIGHT * self.bounds.plane_depth(0.0);
        let bottom = height - PLAYER_HALF_HEIGHT * self.bounds.plane_depth(height);
        let p = &mut self.player.pos;
        let y = p.y + fall * TICK_SECONDS;
        p.y = if bottom > top {
            y.clamp(top, bottom)
        } else {
            height / 2.0 // too small to fly in; never panic in clamp
        };
        let hit_ground = y > p.y; // stopped by the bottom edge
        let at_edge = p.y != y;
        p.x = width * PLAYER_X_FRACTION; // follow resizes / rotation
        if at_edge {
            self.player.vel_y = 0.0; // at the edge: no speed piles up
        }
        // Sagging without lift into the ground: it crashes and explodes.
        if hit_ground && self.player.is_stalling() && self.explosion.is_none() && !self.hands_off()
        {
            self.break_apart();
        }
        // Landing: touch down once over the runway and down at it, then
        // roll out (braking) and stop: the level is done.
        match self.landing {
            Some(Landing::Approach(t)) => {
                let over = self
                    .airport
                    .is_some_and(|a| a.x + TOUCHDOWN_INTO * scale <= self.player.pos.x);
                if over && self.player.pos.y >= self.runway_y() - 2.0 * scale {
                    self.landing = Some(Landing::Rollout(0));
                    self.events.push(Event::MainWheelsTouch);
                    self.puff_at(MAIN_WHEEL_X, FLARE_PITCH);
                } else {
                    self.landing = Some(Landing::Approach(t + 1));
                }
            }
            Some(Landing::Rollout(t)) => {
                if t + 1 == seconds_to_ticks(NOSE_DOWN_SECONDS) {
                    self.events.push(Event::NoseWheelTouch);
                    self.puff_at(NOSE_WHEEL_X, 0.0);
                }
                if t + 1 >= seconds_to_ticks(BRAKE_SECONDS) {
                    let level = self.levels().level;
                    self.landing = Some(Landing::Complete(0));
                    self.events.push(Event::LevelComplete { level });
                } else {
                    self.landing = Some(Landing::Rollout(t + 1));
                }
            }
            _ => {}
        }
        // Standing and rolling on the runway: wheels on the tarmac, level.
        let on_ground = takeoff.is_some_and(|t| (t as f32 * TICK_SECONDS) < ROLL_SECONDS)
            || matches!(
                self.landing,
                Some(Landing::Rollout(_) | Landing::Complete(_))
            );
        if on_ground {
            // Just after touchdown the nose is still up and comes down;
            // the main wheels stay on the runway meanwhile.
            let pitch = self.flare_pitch();
            let y = self.runway_y_at(pitch);
            let player = &mut self.player;
            (player.pos.y, player.vel_y, player.pitch) = (y, 0.0, pitch);
        }
        for m in &mut self.planes {
            m.pos.x += m.vel.x * TICK_SECONDS;
            m.pos.y += m.vel.y * TICK_SECONDS;

            // Bounce off the edges.
            if m.pos.x < 0.0 {
                m.pos.x = -m.pos.x;
                m.vel.x = -m.vel.x;
            } else if m.pos.x > width {
                m.pos.x = 2.0 * width - m.pos.x;
                m.vel.x = -m.vel.x;
            }
            if m.pos.y < 0.0 {
                m.pos.y = -m.pos.y;
                m.vel.y = -m.vel.y;
            } else if m.pos.y > height {
                m.pos.y = 2.0 * height - m.pos.y;
                m.vel.y = -m.vel.y;
            }
        }

        // Balloons rise only while level 1 is fully on screen, gulls fly
        // only while the beach is.
        let levels = self.levels();
        // Nothing new comes in while the airport's runway still reaches the
        // right edge: no hut or launch pad on the runway.
        let runway_in = self
            .airport
            .is_some_and(|a| a.x + RUNWAY_LENGTH * scale > width - 40.0 * scale);
        // Nor in the last seconds before a landing, nor during it, nor
        // while the autopilot takes off (the player cannot dodge then).
        let clearing = self.landing.is_some()
            || self.hands_off()
            || self.level_distance >= LEVEL_DISTANCE - SCROLL_SPEED * CLEAR_SECONDS;
        let coast = levels.level == 1 && !runway_in && !clearing;
        let plane_y = self.player.pos.y;
        let pads: Vec<f32> = self.balloons.pads.iter().map(|p| p.x).collect();
        self.huts.tick(self.bounds, coast, &pads);
        let huts: Vec<f32> = self.huts.list.iter().map(|h| h.x).collect();
        let speed = self.speed();
        self.balloons.set_speed(speed);
        let events = &mut self.events;
        (self.balloons).tick(self.bounds, coast, plane_y, &huts, events);
        let beach = levels.level == BEACH_LEVEL && !runway_in && !clearing;
        self.gulls.tick(self.bounds, beach, &mut self.events);
        self.towers.tick(self.bounds, beach);
        let dawn = levels.level == SUNRISE_LEVEL && !runway_in && !clearing;
        (self.sunrise).tick(self.bounds, dawn, plane_y, &mut self.events);
        let town = levels.level == CITY_LEVEL && !runway_in && !clearing;
        self.city.set_speed(speed);
        self.city.tick(self.bounds, town, plane_y);
        // Faster than at the start: everything slides past by the extra
        // too (far towers less, as they move slower); slower while rolling.
        self.distance += (SCROLL_SPEED * speed * TICK_SECONDS) as f64;
        self.level_distance += SCROLL_SPEED * speed * TICK_SECONDS;
        // Tyre smoke stays on the ground: it slides off with it.
        for puff in &mut self.tyre_smoke {
            puff.pos.x -= SCROLL_SPEED * scale * speed * TICK_SECONDS;
            puff.ticks += 1;
        }
        self.tyre_smoke.retain(|p| p.progress() < 1.0);
        if let Some(a) = &mut self.airport {
            a.x -= SCROLL_SPEED * scale * speed * TICK_SECONDS;
        }
        if self.airport.is_some_and(|a| a.gone(self.bounds)) {
            self.airport = None;
        }
        let extra = (speed - 1.0) * SCROLL_SPEED * scale * TICK_SECONDS;
        if extra != 0.0 {
            self.huts.list.iter_mut().for_each(|h| h.x -= extra);
            self.balloons.pads.iter_mut().for_each(|p| p.x -= extra);
            self.balloons.list.iter_mut().for_each(|b| b.pos.x -= extra);
            for t in &mut self.towers.list {
                t.x -= extra * t.size(self.bounds) / scale;
            }
            self.gulls.shift(-extra);
            self.city.shift(-extra);
            self.sunrise.shift(-extra);
        }

        // Crash: the plane as drawn newly overlaps another plane, a
        // balloon, a launch pad, a hut or a gull as drawn. It costs a
        // life; a crashed or safe plane cannot crash.
        let bounds = self.bounds;
        let me = self.player.shape(bounds);
        let hits = |them: &[Poly]| any_overlap(&me, them);
        // Gulls the plane flies into are knocked down (a falling one is
        // no obstacle any more).
        // While the autopilot flies, the player cannot dodge: nothing
        // still around can hit the plane then.
        let open = self.explosion.is_none() && !self.hands_off();
        let struck: Vec<usize> = if open {
            (self.gulls.list.iter().enumerate())
                .filter(|(_, g)| !g.is_down() && hits(&g.shape(bounds.depth(g.pos.y))))
                .map(|(i, _)| i)
                .collect()
        } else {
            Vec::new()
        };
        // Balloons too: one the plane flies into explodes and falls.
        let burst: Vec<usize> = if open {
            (self.balloons.list.iter().enumerate())
                .filter(|(_, b)| !b.is_burst() && hits(&b.shape(b.size(bounds))))
                .map(|(i, _)| i)
                .collect()
        } else {
            Vec::new()
        };
        // Geese too: one the plane flies into is knocked down.
        let geese: Vec<usize> = if open {
            (self.sunrise.geese.iter().enumerate())
                .filter(|(_, g)| !g.is_down() && hits(&g.shape(scale)))
                .map(|(i, _)| i)
                .collect()
        } else {
            Vec::new()
        };
        // The houses under the steam and the valley's high-rises are solid.
        let house = (self.sunrise.steam.iter()).any(|c| hits(&c.house_shape(bounds)))
            || (self.sunrise.high_rises.iter()).any(|b| hits(&b.shape(bounds)));
        // Steam costs no life: the warm air throws the plane up.
        let in_steam = open && self.sunrise.in_steam(bounds, &me);
        // Helicopters too (parked on a roof or flying): one the plane
        // flies into crashes and burns.
        let (roof_helis, flyers): (Vec<usize>, Vec<usize>) = if open {
            (
                (self.city.high_rises.iter().enumerate())
                    .filter(|(_, b)| hits(&b.heli_shape(bounds)))
                    .map(|(i, _)| i)
                    .collect(),
                (self.city.flyers.iter().enumerate())
                    .filter(|(_, f)| hits(&f.shape(bounds)))
                    .map(|(i, _)| i)
                    .collect(),
            )
        } else {
            (Vec::new(), Vec::new())
        };
        let touching = (self.planes.iter()).any(|m| {
            let r = PLANE_RADIUS * bounds.depth(m.pos.y);
            me.iter().any(|part| part.overlaps_circle(m.pos, r))
        }) || !struck.is_empty()
            || !burst.is_empty()
            || !geese.is_empty()
            || house
            || !roof_helis.is_empty()
            || !flyers.is_empty()
            || (self.balloons.pads.iter()).any(|p| hits(&p.shape(bounds)))
            || (self.huts.list.iter()).any(|h| hits(&h.shape(bounds)))
            || (self.towers.list.iter()).any(|t| hits(&t.shape(bounds)))
            || (self.city.high_rises.iter()).any(|b| hits(&b.shape(bounds)))
            || (self.city.cranes.iter()).any(|c| hits(&c.shape(bounds)));
        let player = &mut self.player;
        let hit = touching && !player.touching && open;
        if hit && !player.is_crashed() && !player.is_safe() {
            player.crash_ticks = CRASH_TICKS;
            self.lives = self.lives.saturating_sub(1);
            player.steer = 0.0;
            if self.lives == 0 {
                // The last life: it blows up where it was hit.
                self.explosion = Some(player.pos);
                self.events.push(Event::Explosion);
            } else {
                self.events.push(Event::Crash);
            }
        }
        player.touching = touching;
        for &i in &struck {
            self.gulls.list[i].knock_down(scale);
        }
        if !struck.is_empty() {
            self.events.push(Event::GullCall); // a squawk
        }
        for &i in &burst {
            self.balloons.list[i].burst(scale);
        }
        if !burst.is_empty() {
            self.events.push(Event::BalloonBurst);
        }
        for &i in &geese {
            self.sunrise.geese[i].knock_down(scale);
        }
        if in_steam {
            self.player.vel_y -= STEAM_LIFT * scale * TICK_SECONDS;
            if !self.in_steam {
                self.events.push(Event::SteamHiss);
            }
        }
        self.in_steam = in_steam;
        for &i in &roof_helis {
            self.city.down_roof_heli(i, bounds);
        }
        for &i in flyers.iter().rev() {
            self.city.down_flyer(i, bounds);
        }
        if !roof_helis.is_empty() || !flyers.is_empty() {
            self.events.push(Event::HeliCrash);
        }

        // Rain (and storms) only where a rainy level shows; none yet.
        self.weather.tick(&mut self.events, levels.rain_room());
        let mut target = Mix::target(
            &self.player,
            &self.weather,
            &self.levels(),
            self.bounds.nearness(self.player.pos.y),
        );
        // While a warning voice speaks, its tone steps back so the words
        // come through (like a real cockpit).
        // The plane flies itself: no warning tones (see `hands_off`).
        if self.hands_off() {
            target.overspeed = 0.0;
            target.stall = 0.0;
        }
        if self.voice_speaking(self.stall_ticks) {
            target.stall *= VOICE_DUCK;
        }
        if self.voice_speaking(self.overspeed_ticks) {
            target.overspeed *= VOICE_DUCK;
        }
        // Rolling down the runway: the engine spools up, the wheels rumble.
        if self.on_runway() {
            target = target.takeoff(self.rolling());
        }
        // Rolling out after touchdown: rumble dying down, engine idling.
        if self.rolling_out() {
            target = target.rollout(self.rolling());
        }
        let step = MIX_CHANGE_PER_SECOND * TICK_SECONDS;
        self.mix = self.mix.approach(target, step);
        self.ticks += 1;
        self.level_ticks += 1;
    }

    /// The autopilot's steering on the approach and rollout: down a gentle
    /// slope so the wheels meet the runway at the touchdown point, then
    /// stay on it. `None` when the player flies.
    fn autopilot_steer(&self) -> Option<f32> {
        if !self.autopilot() {
            return None;
        }
        let scale = self.bounds.scale();
        let a = self.airport?;
        let target = self.runway_y();
        let ahead = a.x + TOUCHDOWN_INTO * scale - self.player.pos.x;
        let ground = (SCROLL_SPEED * scale * self.speed()).max(1.0);
        let wanted_vel = if ahead > 0.0 {
            (target - self.player.pos.y) / (ahead / ground).max(0.4)
        } else {
            (target - self.player.pos.y) * 4.0
        };
        // Lead for the plane's smooth response (like the finger).
        let steer = wanted_vel / (PLAYER_CLIMB_SPEED * scale);
        Some(steer.clamp(-0.6, 0.6))
    }

    /// The level is done: switch to the next level's landscape, set the
    /// plane on its runway for the takeoff, clear everything away, and
    /// start the countdown.
    fn next_level(&mut self) {
        self.completed += 1;
        self.level_ticks = 0;
        self.level_distance = 0.0;
        self.go_on = false;
        self.landing = Some(Landing::Countdown(0));
        self.balloons.list.clear();
        self.balloons.pads.clear();
        self.huts.list.clear();
        self.gulls.list.clear();
        self.towers.list.clear();
        self.city.clear();
        self.sunrise.clear();
        self.tyre_smoke.clear();
        // A fresh start in every level: all lives back (and so no smoke).
        self.lives = self.lives.max(LIVES);
        self.player.airspeed = 1.0;
        self.player.vel_y = 0.0;
        self.player.pitch = 0.0;
        self.player.safe_ticks = 0;
        self.airport = Some(Airport::at_start(self.bounds));
        self.takeoff = Some(0);
        self.player.pos.y = self.runway_y();
        let level = self.levels().level;
        self.events.push(Event::NewLevel { level });
    }

    /// While the world waits ("complete", countdown): the sound settles to
    /// a quiet idle.
    fn quiet_down(&mut self) {
        let step = MIX_CHANGE_PER_SECOND * TICK_SECONDS;
        self.mix = self.mix.approach(Mix::SILENT, step);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world() -> World {
        World::new(Bounds {
            width: 100.0,
            height: 50.0,
        })
    }

    #[test]
    fn tick_moves_plane_by_velocity_times_dt() {
        let mut w = world();
        w.spawn_plane("m", Vec2::new(10.0, 10.0), Vec2::new(60.0, -120.0));
        w.tick(&Input::NONE);
        let m = &w.planes[0];
        assert!((m.pos.x - 11.0).abs() < 1e-5);
        assert!((m.pos.y - 8.0).abs() < 1e-5);
        assert_eq!(w.ticks, 1);
    }

    #[test]
    fn plane_bounces_off_right_edge() {
        let mut w = world();
        w.spawn_plane("m", Vec2::new(99.5, 10.0), Vec2::new(60.0, 0.0));
        w.tick(&Input::NONE); // would reach 100.5, reflects to 99.5
        let m = &w.planes[0];
        assert!((m.pos.x - 99.5).abs() < 1e-5);
        assert_eq!(m.vel.x, -60.0);
    }

    #[test]
    fn player_starts_on_the_left_at_mid_height() {
        let w = world();
        assert_eq!(w.player.pos, Vec2::new(100.0 * PLAYER_X_FRACTION, 25.0));
    }

    /// A phone-sized world (scale 1) with a plane parked `gap` px to the
    /// right of the player.
    fn world_with_plane_ahead(gap: f32) -> World {
        let mut w = World::new(Bounds {
            width: 800.0,
            height: DESIGN_HEIGHT,
        });
        let p = w.player.pos;
        w.spawn_plane("m", Vec2::new(p.x + gap, p.y), Vec2::new(0.0, 0.0));
        w
    }

    /// How close another plane at the player's starting height may come:
    /// it touches the tip of the player's nose.
    fn mid_height_reach() -> f32 {
        let b = World::new(Bounds {
            width: 800.0,
            height: DESIGN_HEIGHT,
        })
        .bounds;
        let y = DESIGN_HEIGHT / 2.0;
        PLANE_RADIUS * b.depth(y) + PLANE_NOSE[1].0 * b.plane_depth(y)
    }

    #[test]
    fn touching_another_plane_is_a_crash() {
        let reach = mid_height_reach();
        let mut w = world_with_plane_ahead(reach - 1.0);
        w.tick(&Input::NONE);
        assert_eq!(w.events(), [Event::Crash]);
        assert!(w.player.is_crashed());
        assert_eq!(w.player.crash_ticks, CRASH_TICKS);
    }

    #[test]
    fn a_near_miss_is_not_a_crash() {
        let reach = mid_height_reach();
        let mut w = world_with_plane_ahead(reach + 1.0);
        for _ in 0..100 {
            w.tick(&Input::NONE);
            assert_eq!(w.events(), []);
        }
        assert!(!w.player.is_crashed());
    }

    #[test]
    fn a_crashed_plane_falls_and_ignores_the_controls() {
        let mut w = world_with_plane_ahead(0.0);
        w.tick(&Input::NONE);
        let y = w.player.pos.y;
        w.tick(&Input::new(-1.0)); // full climb: no effect
        assert!((w.player.pos.y - (y + CRASH_FALL_SPEED * TICK_SECONDS)).abs() < 1e-4);
        assert_eq!(w.player.steer, 0.0);
    }

    #[test]
    fn the_plane_flies_again_after_the_crash_time() {
        let mut w = world_with_plane_ahead(0.0);
        w.tick(&Input::NONE); // crash
        for _ in 0..CRASH_TICKS {
            assert!(w.player.is_crashed());
            w.tick(&Input::NONE);
        }
        assert!(!w.player.is_crashed());
        let y = w.player.pos.y;
        w.tick(&Input::new(-1.0));
        assert!(w.player.pos.y < y, "controls work again");
    }

    #[test]
    fn one_touch_is_one_crash() {
        // The other plane stays on top of the player the whole time.
        let mut w = world_with_plane_ahead(0.0);
        w.planes[0].vel = Vec2::new(0.0, CRASH_FALL_SPEED); // falls along
        let mut crashes = 0;
        for _ in 0..10 * CRASH_TICKS {
            w.tick(&Input::NONE);
            crashes += w.events().iter().filter(|e| **e == Event::Crash).count();
            w.planes[0].pos = w.player.pos;
        }
        assert_eq!(crashes, 1);
    }

    #[test]
    fn a_second_touch_after_flying_free_crashes_again() {
        let mut w = world_with_plane_ahead(0.0);
        w.tick(&Input::NONE);
        w.planes[0].pos.x += 500.0; // far away
        for _ in 0..=CRASH_TICKS + SAFE_TICKS {
            w.tick(&Input::NONE);
        }
        assert!(!w.player.is_crashed() && !w.player.is_safe());
        w.planes[0].pos = w.player.pos;
        w.tick(&Input::NONE);
        assert_eq!(w.events(), [Event::Crash]);
    }

    /// Crash into the plane parked on the player, then move it far away.
    fn crash(w: &mut World) -> Vec<Event> {
        w.planes[0].pos = w.player.pos;
        w.tick(&Input::NONE);
        w.planes[0].pos.x += 500.0;
        w.events().to_vec()
    }

    /// Tick until the plane flies again and its safe time is over.
    fn recover(w: &mut World) {
        for _ in 0..CRASH_TICKS + SAFE_TICKS {
            w.tick(&Input::NONE);
        }
        assert!(!w.player.is_crashed() && !w.player.is_safe());
    }

    #[test]
    fn each_crash_costs_a_life() {
        let mut w = world_with_plane_ahead(300.0);
        assert_eq!(w.lives, LIVES);
        assert_eq!(crash(&mut w), [Event::Crash]);
        assert_eq!(w.lives, LIVES - 1);
        recover(&mut w);
        crash(&mut w);
        assert_eq!(w.lives, LIVES - 2);
        assert!(!w.is_over(), "one life left");
    }

    #[test]
    fn after_a_crash_the_plane_is_safe_for_a_while() {
        let mut w = world_with_plane_ahead(300.0);
        crash(&mut w);
        for _ in 0..CRASH_TICKS {
            w.tick(&Input::NONE);
        }
        assert!(
            !w.player.is_crashed() && w.player.is_safe(),
            "flies, blinking"
        );
        let y = w.player.pos.y;
        w.tick(&Input::new(-1.0));
        assert!(w.player.pos.y < y, "steers while safe");
        // Flying into something while safe costs nothing.
        assert_eq!(crash(&mut w), []);
        assert_eq!(w.lives, LIVES - 1);
        for _ in 0..SAFE_TICKS {
            w.tick(&Input::NONE);
        }
        assert!(!w.player.is_safe());
        assert_eq!(crash(&mut w), [Event::Crash], "safe time is over");
        assert_eq!(w.lives, LIVES - 2);
    }

    #[test]
    fn the_last_crash_ends_the_game_once_the_plane_has_fallen() {
        let mut w = world_with_plane_ahead(300.0);
        for _ in 1..LIVES {
            crash(&mut w);
            recover(&mut w);
        }
        crash(&mut w);
        assert_eq!(w.lives, 0);
        let mut game_overs = 0;
        for _ in 0..CRASH_TICKS {
            assert!(!w.is_over(), "still falling");
            w.tick(&Input::NONE);
            game_overs += w.events().iter().filter(|e| **e == Event::GameOver).count();
        }
        assert!(w.is_over());
        assert_eq!(game_overs, 1);
    }

    #[test]
    fn the_plane_smokes_more_with_each_life_lost_and_explodes_at_the_last() {
        let mut w = world_with_plane_ahead(300.0);
        assert_eq!(w.smoke(), 0.0);
        assert_eq!(crash(&mut w), [Event::Crash]);
        assert_eq!(w.smoke(), DAMAGE_SMOKE[1]);
        recover(&mut w);
        assert_eq!(w.smoke(), DAMAGE_SMOKE[1], "the smoke stays");
        assert_eq!(crash(&mut w), [Event::Crash]);
        assert!(w.smoke() > DAMAGE_SMOKE[1], "more smoke");
        recover(&mut w);
        let at = w.player.pos;
        assert_eq!(crash(&mut w), [Event::Explosion], "no plain crash");
        assert_eq!(w.explosion, Some(at), "where it was hit");
        assert_eq!(w.smoke(), 0.0, "nothing left to smoke");
        // The explosion runs for the crash time, then the game is over.
        let mut last = -1.0;
        while !w.is_over() {
            let p = w.explosion_progress().unwrap();
            assert!(p > last && p <= 1.0);
            last = p;
            w.tick(&Input::NONE);
        }
        assert_eq!(w.explosion_progress(), Some(1.0));
    }

    #[test]
    fn the_flight_starts_on_the_runway_rolls_and_lifts_off() {
        let mut w = World::new(PHONE).with_takeoff();
        let parked = w.player.pos.y;
        assert!(
            parked > 0.8 * PHONE.height,
            "on the runway before the first tick"
        );
        assert!(w.airport.is_some() && w.taking_off());
        w.tick(&Input::new(-1.0)); // the controls do not count yet
        assert!(w.on_runway());
        let on_runway = w.player.pos.y;
        assert!(
            on_runway > 0.8 * PHONE.height,
            "down on the runway: {on_runway}"
        );
        // The landscape starts from standing and speeds up while rolling.
        let first = w.speed();
        assert!(first < 0.05, "{first}");
        let roll = (ROLL_SECONDS / TICK_SECONDS) as u32;
        for _ in 0..roll / 2 {
            w.tick(&Input::new(1.0));
        }
        assert!(w.speed() > 0.4 && w.speed() < 0.6, "{}", w.speed());
        assert_eq!(w.player.pos.y, on_runway, "still rolling");
        assert_eq!(w.player.pitch, 0.0);
        // Lift-off: it climbs away on its own, nose up.
        for _ in 0..roll / 2 + 30 {
            w.tick(&Input::new(1.0));
        }
        assert!(!w.on_runway() && w.taking_off());
        assert!(w.player.pos.y < on_runway - 3.0, "airborne");
        assert!(w.player.pitch < 0.0, "nose up");
        // A gentle climb-out, not a steep one.
        let mut steepest = 0.0_f32;
        let mut probe = w.clone();
        while probe.taking_off() {
            probe.tick(&Input::NONE);
            steepest = steepest.min(probe.player.pitch);
        }
        assert!(steepest > -0.5 * MAX_PITCH_DEG.to_radians(), "{steepest}");
        // Then the player has the controls.
        for _ in 0..(CLIMB_OUT_SECONDS / TICK_SECONDS) as u32 {
            w.tick(&Input::NONE);
        }
        assert!(!w.taking_off());
        let y = w.player.pos.y;
        for _ in 0..30 {
            w.tick(&Input::new(1.0));
        }
        assert!(w.player.pos.y > y, "steers now");
        assert_eq!(w.lives, LIVES, "no crash on the way");
    }

    #[test]
    fn nothing_stands_on_the_runway() {
        for seed in 0..6 {
            let mut w = World::with_seed(PHONE, seed).with_takeoff();
            w.lives = u32::MAX;
            for _ in 0..60 * 20 {
                w.tick(&Input::NONE);
                let Some(a) = w.airport else { break };
                let (l, r) = (a.x, a.x + RUNWAY_LENGTH);
                for h in &w.huts.list {
                    let (hl, _, hr, _) = h.rect(PHONE);
                    assert!(hr < l || hl > r, "hut on the runway, seed {seed}");
                }
                for p in &w.balloons.pads {
                    assert!(p.x + PAD_WIDTH / 2.0 < l || p.x - PAD_WIDTH / 2.0 > r);
                }
            }
        }
    }

    #[test]
    fn the_gear_folds_up_after_lift_off_with_its_sound() {
        let mut w = World::new(PHONE).with_takeoff();
        let mut gear_ups = 0;
        let mut last = 1.0;
        let mut was_up_while_taking_off = false;
        while w.taking_off() {
            w.tick(&Input::NONE);
            gear_ups += w.events().iter().filter(|e| **e == Event::GearUp).count();
            if w.on_runway() {
                assert_eq!(w.gear(), 1.0, "down on the runway");
            }
            assert!(w.gear() <= last, "only ever folds up");
            last = w.gear();
            was_up_while_taking_off |= w.taking_off() && w.gear() == 0.0;
        }
        assert_eq!(gear_ups, 1, "one whir and clunk");
        assert!(was_up_while_taking_off, "up before the player takes over");
        assert_eq!(w.gear(), 0.0);
        assert_eq!(World::new(PHONE).gear(), 0.0, "in the air: up");
    }

    #[test]
    fn the_takeoff_sounds_like_one() {
        let mut w = World::new(PHONE).with_takeoff();
        let roll = (ROLL_SECONDS / TICK_SECONDS) as u32;
        w.tick(&Input::NONE);
        assert_eq!(w.events(), [Event::Takeoff], "the spool-up starts");
        for _ in 1..30 {
            w.tick(&Input::NONE);
            assert!(!w.events().contains(&Event::Takeoff), "only once");
        }
        let early_rumble = w.mix.rumble;
        assert!(early_rumble > 0.0, "wheels on the runway");
        assert_eq!(w.mix.engine_power, 0.0, "the spool-up sound carries it");
        for _ in 30..roll - 5 {
            w.tick(&Input::NONE);
        }
        assert!(w.mix.rumble > early_rumble, "faster, louder rumble");
        for _ in 0..30 {
            w.tick(&Input::NONE);
        }
        assert!(!w.on_runway());
        assert!(
            w.mix.rumble < early_rumble * 0.5,
            "off the ground: it stops"
        );
        assert!(w.mix.engine_power > 0.0, "the engine loops take over");
        for _ in 0..60 {
            w.tick(&Input::NONE);
        }
        assert_eq!(w.mix.rumble, 0.0);
    }

    #[test]
    fn the_airport_slides_away_and_a_flight_in_level_2_takes_off_too() {
        let mut w = World::new(PHONE).with_takeoff();
        let x = w.airport.unwrap().x;
        for _ in 0..60 * 6 {
            w.tick(&Input::NONE);
        }
        assert!(w.airport.is_none_or(|a| a.x < x - 100.0), "slides off");
        for _ in 0..60 * 20 {
            w.balloons.list.clear();
            w.huts.list.clear();
            w.tick(&Input::NONE);
        }
        assert!(w.airport.is_none(), "gone once off screen");
        let mut beach = World::new(PHONE).with_takeoff();
        beach.start_level = BEACH_LEVEL;
        beach.tick(&Input::new(-1.0));
        assert!(
            beach.airport.is_some() && beach.on_runway(),
            "from the beach's runway"
        );
        assert_eq!(beach.levels().level, BEACH_LEVEL);
    }

    #[test]
    fn the_game_speeds_up_a_little_with_every_point() {
        let mut w = World::new(PHONE);
        w.lives = u32::MAX;
        assert_eq!(w.speed(), 1.0);
        let step = |w: &mut World| {
            let d = w.distance;
            w.tick(&Input::NONE);
            w.distance - d
        };
        let start = step(&mut w);
        assert!((start - (SCROLL_SPEED * TICK_SECONDS) as f64).abs() < 1e-6);
        // A minute later: 12 % faster, only a little.
        w.ticks = 60 * 60; // one minute of ticks
        assert_eq!(w.seconds_flown(), 60);
        assert!((w.speed() - 1.12).abs() < 1e-5, "{}", w.speed());
        let later = step(&mut w);
        assert!((later / start - w.speed() as f64).abs() < 0.01);
        // It stops getting faster.
        w.ticks = 3600 * 60; // an hour
        assert_eq!(w.speed(), MAX_SPEED);
    }

    #[test]
    fn faster_means_huts_and_gulls_come_by_faster_too() {
        let moved = |ticks: u64| {
            let mut w = World::new(PHONE);
            w.lives = u32::MAX;
            w.ticks = ticks;
            w.huts.list.push(Hut {
                x: 600.0,
                width: 100.0,
                height: 90.0,
                style: 0,
            });
            w.tick(&Input::NONE);
            600.0 - w.huts.list[0].x
        };
        let (slow, fast) = (moved(0), moved(60 * 60));
        assert!((fast / slow - 1.12).abs() < 0.01, "{slow} {fast}");
    }

    /// Fly with a throttle (and level steering) for `seconds`, nothing to
    /// hit; returns every event.
    fn throttle(w: &mut World, gas: f32, seconds: f32) -> Vec<Event> {
        let mut all = Vec::new();
        for _ in 0..(seconds / TICK_SECONDS).round() as u32 {
            w.balloons.list.clear();
            w.huts.list.clear();
            w.tick(&Input::with_throttle(0.0, gas));
            all.extend_from_slice(w.events());
        }
        all
    }

    #[test]
    fn gas_and_brake_change_the_airspeed_and_it_drifts_back() {
        let mut w = open_sky();
        assert_eq!(w.player.airspeed, 1.0);
        throttle(&mut w, 1.0, 0.5);
        assert!((w.player.airspeed - (1.0 + 0.5 * AIRSPEED_CHANGE)).abs() < 0.01);
        assert!(
            (w.speed() - w.player.airspeed).abs() < 0.01,
            "the game goes faster"
        );
        throttle(&mut w, 0.0, 10.0);
        assert!((w.player.airspeed - 1.0).abs() < 1e-4, "back to cruising");
        throttle(&mut w, -1.0, 0.5);
        assert!(w.player.airspeed < 1.0);
        throttle(&mut w, 1.0, 2.5); // (before overspeed breaks it apart)
        assert_eq!(
            w.player.airspeed, AIRSPEED_RANGE.1,
            "never faster than the top"
        );
    }

    #[test]
    fn overspeed_warns_counts_down_and_breaks_the_plane_apart() {
        let mut w = open_sky();
        w.lives = LIVES;
        // Gas until overspeed: it says so once.
        let to_limit = (OVERSPEED_AT - 1.0) / AIRSPEED_CHANGE;
        let events = throttle(&mut w, 1.0, to_limit + 0.1);
        assert_eq!(events.iter().filter(|e| **e == Event::Overspeed).count(), 1);
        let Some(Warning::Overspeed { seconds_left }) = w.warning() else {
            panic!("{:?}", w.warning())
        };
        assert!(seconds_left > OVERSPEED_SECONDS - 0.2 && seconds_left <= OVERSPEED_SECONDS);
        assert!(w.mix.overspeed > 0.0, "the warning tone");
        // Keep the gas on: the voice repeats, then it breaks apart.
        let events = throttle(&mut w, 1.0, OVERSPEED_SECONDS + 0.1);
        assert!(events.iter().filter(|e| **e == Event::Overspeed).count() >= 1);
        assert!(events.contains(&Event::Explosion), "{events:?}");
        assert_eq!(w.lives, 0, "every life gone at once");
        assert!(w.explosion.is_some());
        throttle(&mut w, 0.0, CRASH_SECONDS + 0.1);
        assert!(w.is_over());
    }

    #[test]
    fn braking_out_of_overspeed_in_time_saves_the_plane() {
        let mut w = open_sky();
        w.lives = LIVES;
        throttle(&mut w, 1.0, (OVERSPEED_AT - 1.0) / AIRSPEED_CHANGE + 0.1);
        assert!(matches!(w.warning(), Some(Warning::Overspeed { .. })));
        let events = throttle(&mut w, -1.0, 1.0);
        assert!(!events.contains(&Event::Explosion));
        assert_eq!(w.warning(), None);
        assert_eq!(w.lives, LIVES);
        // Going over again starts the count from the beginning.
        let events = throttle(&mut w, 1.0, 1.5);
        assert!(events.contains(&Event::Overspeed));
        let Some(Warning::Overspeed { seconds_left }) = w.warning() else {
            panic!()
        };
        assert!(seconds_left > 1.5, "{seconds_left}");
    }

    #[test]
    fn the_warning_tone_steps_back_while_the_voice_speaks() {
        let mut w = open_sky();
        let to_stall = (1.0 - STALL_AT) / AIRSPEED_CHANGE;
        throttle(&mut w, -1.0, to_stall + 0.3); // the voice just started
        assert!(w.mix.stall > 0.0);
        assert!(
            w.mix.stall <= WARNING_VOLUME * VOICE_DUCK + 1e-3,
            "{}",
            w.mix.stall
        );
        // After the words, the tone is back to full, before the next call.
        let mut full = 0.0_f32;
        for _ in 0..(WARNING_REPEAT_SECONDS / TICK_SECONDS) as u32 {
            w.tick(&Input::with_throttle(-1.0, -1.0)); // pull up, keep braking
            full = full.max(w.mix.stall);
        }
        assert!((full - WARNING_VOLUME).abs() < 1e-3, "{full}");
    }

    #[test]
    fn stalling_into_the_ground_blows_the_plane_up() {
        let mut w = open_sky();
        w.lives = LIVES;
        let mut events = Vec::new();
        // Brake and keep braking: it sags all the way down.
        for _ in 0..(6.0 / TICK_SECONDS) as u32 {
            w.balloons.list.clear();
            w.huts.list.clear();
            w.tick(&Input::with_throttle(0.0, -1.0));
            events.extend_from_slice(w.events());
            if w.explosion.is_some() {
                break;
            }
        }
        assert!(events.contains(&Event::PullUp));
        assert!(events.contains(&Event::Explosion), "it hit the ground");
        assert_eq!(w.lives, 0, "every life gone at once");
    }

    #[test]
    fn diving_to_the_ground_at_normal_speed_is_no_crash() {
        let mut w = open_sky();
        w.lives = LIVES;
        fly(&mut w, 1.0, 3.0);
        assert_eq!(w.explosion, None);
        assert_eq!(w.lives, LIVES);
    }

    #[test]
    fn too_slow_says_pull_up_and_the_plane_sags() {
        let mut w = open_sky();
        let y = w.player.pos.y;
        let to_stall = (1.0 - STALL_AT) / AIRSPEED_CHANGE;
        let events = throttle(&mut w, -1.0, to_stall + 0.1);
        assert_eq!(events.iter().filter(|e| **e == Event::PullUp).count(), 1);
        assert_eq!(w.warning(), Some(Warning::PullUp));
        assert!(w.mix.stall > 0.0, "the warning tone");
        throttle(&mut w, -1.0, 0.4);
        assert!(
            w.player.vel_y > 0.0 && w.player.pos.y > y,
            "sags without steering"
        );
        // Keep braking but pull up hard (so it does not hit the ground).
        let mut events = Vec::new();
        for _ in 0..(WARNING_REPEAT_SECONDS / TICK_SECONDS).round() as u32 + 1 {
            w.tick(&Input::with_throttle(-1.0, -1.0));
            events.extend_from_slice(w.events());
        }
        assert!(events.contains(&Event::PullUp), "the voice repeats");
        assert_eq!(w.explosion, None);
        // Gas: the warning stops and it flies level again.
        throttle(&mut w, 1.0, 1.0);
        assert_eq!(w.warning(), None);
        assert!(w.player.airspeed >= STALL_AT);
    }

    #[test]
    fn the_screen_shakes_at_the_explosion_and_calms_down() {
        let mut w = world_with_plane_ahead(300.0);
        w.lives = 1;
        assert_eq!(w.shake(), Vec2::new(0.0, 0.0), "no shake while flying");
        crash(&mut w);
        let size = |v: Vec2| (v.x * v.x + v.y * v.y).sqrt();
        let max = SHAKE_PX * w.bounds.scale() * 1.25; // x and 0.7 y together
        let mut early = 0.0_f32;
        let mut late = 0.0_f32;
        let mut directions = std::collections::HashSet::new();
        for i in 0..CRASH_TICKS {
            let s = w.shake();
            assert!(size(s) <= max, "{s:?}");
            if i < 10 {
                early = early.max(size(s));
                directions.insert((s.x > 0.0, s.y > 0.0));
            }
            if i > CRASH_TICKS - 10 {
                late = late.max(size(s));
            }
            w.tick(&Input::NONE);
        }
        assert!(early > SHAKE_PX * 0.5, "strong at the bang: {early}");
        assert!(late < early * 0.05, "calm at the end: {late}");
        assert!(directions.len() >= 3, "shakes around: {directions:?}");
        assert!(w.is_over());
        assert_eq!(w.shake(), Vec2::new(0.0, 0.0), "still at game over");
    }

    #[test]
    fn after_game_over_nothing_moves_and_the_sound_fades_out() {
        let mut w = world_with_plane_ahead(300.0);
        w.lives = 1;
        crash(&mut w);
        for _ in 0..CRASH_TICKS {
            w.tick(&Input::NONE);
        }
        assert!(w.is_over());
        let (ticks, flown, player) = (w.ticks, w.seconds_flown(), w.player);
        let plane = w.planes[0].clone();
        for _ in 0..120 {
            w.tick(&Input::new(-1.0));
            assert_eq!(w.events(), []);
        }
        assert_eq!(
            (w.ticks, w.seconds_flown(), w.player),
            (ticks, flown, player)
        );
        assert_eq!(w.planes[0], plane);
        assert_eq!(w.mix, Mix::SILENT);
    }

    #[test]
    fn a_new_flight_can_start_only_after_game_over_has_shown_a_moment() {
        let mut w = world_with_plane_ahead(300.0);
        w.lives = 1;
        crash(&mut w);
        assert!(!w.can_restart(), "still flying");
        for _ in 0..CRASH_TICKS {
            w.tick(&Input::NONE);
        }
        let wait = (RESTART_WAIT_SECONDS / TICK_SECONDS).round() as u32;
        for _ in 0..wait - 1 {
            w.tick(&Input::NONE);
            assert!(!w.can_restart());
        }
        w.tick(&Input::NONE);
        assert!(w.can_restart());
    }

    #[test]
    fn the_clock_starts_again_with_every_level() {
        let mut w = World::with_seed(PHONE, 2);
        w.lives = u32::MAX;
        while w.level_complete().is_none() {
            w.balloons.list.clear();
            w.tick(&Input::NONE);
        }
        let (in_level, flown) = (w.seconds_in_level(), w.seconds_flown());
        assert!(in_level >= 60, "a whole level: {in_level}");
        assert_eq!(in_level, flown, "the first level is the whole flight");
        w.go_on();
        while w.countdown().is_some() || w.level_complete().is_some() {
            w.tick(&Input::NONE);
        }
        assert_eq!(w.seconds_in_level(), 0, "level 2 starts at 0:00");
        for _ in 0..5 * 60 {
            w.tick(&Input::NONE);
        }
        assert_eq!(w.seconds_in_level(), 5);
        assert_eq!(w.seconds_flown(), flown + 5, "the speed-up goes on");
    }

    #[test]
    fn the_flight_time_counts_whole_seconds() {
        let mut w = world_with_plane_ahead(300.0);
        assert_eq!(w.seconds_flown(), 0);
        let mut last = 0;
        for second in 1..=10 {
            for _ in 0..60 {
                w.tick(&Input::NONE);
                assert!(w.seconds_flown() >= last, "never goes down");
                last = w.seconds_flown();
            }
            assert_eq!(w.seconds_flown(), second);
        }
    }

    #[test]
    fn the_clock_shows_minutes_and_seconds() {
        assert_eq!(clock_text(0), "0:00");
        assert_eq!(clock_text(9), "0:09");
        assert_eq!(clock_text(59), "0:59");
        assert_eq!(clock_text(60), "1:00");
        assert_eq!(clock_text(125), "2:05");
        assert_eq!(clock_text(3600), "60:00");
    }

    #[test]
    fn events_last_for_one_tick() {
        let mut w = world_with_plane_ahead(0.0);
        w.tick(&Input::NONE);
        assert_eq!(w.events().len(), 1);
        w.tick(&Input::NONE);
        assert_eq!(w.events(), []);
    }

    #[test]
    fn climbing_away_makes_the_engine_quieter() {
        let mut w = World::new(PHONE);
        let engine = |w: &World| (w.mix.engine_idle.powi(2) + w.mix.engine_power.powi(2)).sqrt();
        for _ in 0..60 {
            w.tick(&Input::NONE); // settle, level flight mid-screen
        }
        let middle = engine(&w);
        for _ in 0..120 {
            w.balloons.list.clear();
            w.tick(&Input::new(-1.0)); // up to the top
        }
        for _ in 0..60 {
            w.balloons.list.clear();
            w.tick(&Input::NONE); // level again: same throttle as before
        }
        assert!(engine(&w) < middle * 0.8, "{} vs {middle}", engine(&w));
    }

    #[test]
    fn the_engine_fades_in_at_the_start_and_cuts_out_in_a_crash() {
        let mut w = world_with_plane_ahead(300.0);
        assert_eq!(w.mix, Mix::SILENT);
        w.tick(&Input::NONE);
        let step = MIX_CHANGE_PER_SECOND * TICK_SECONDS;
        assert!((w.mix.engine_idle - step).abs() < 1e-6, "fades, no jump");
        for _ in 0..60 {
            w.tick(&Input::NONE);
        }
        let near = w.bounds.nearness(w.player.pos.y);
        assert_eq!(w.mix, Mix::target(&w.player, &w.weather, &w.levels(), near));
        assert!(w.mix.engine_idle > 0.0 && w.mix.engine_power > 0.0);

        w.planes[0].pos = w.player.pos;
        w.tick(&Input::NONE);
        assert!(w.mix.engine_idle > 0.0, "fades out, no jump");
        for _ in 0..30 {
            w.tick(&Input::NONE);
        }
        assert_eq!((w.mix.engine_idle, w.mix.engine_power), (0.0, 0.0));
    }

    fn new_levels(w: &mut World, seconds: f32) -> Vec<(u64, u32)> {
        let mut seen = Vec::new();
        for _ in 0..(seconds / TICK_SECONDS).round() as u64 {
            w.go_on(); // "Next level" whenever a level is complete
            w.tick(&Input::NONE);
            for e in w.events() {
                if let Event::NewLevel { level } = *e {
                    seen.push((w.ticks, level));
                }
            }
        }
        seen
    }

    #[test]
    fn each_level_ends_with_a_landing_then_the_next_begins() {
        let mut w = World::with_seed(PHONE, 1);
        w.lives = u32::MAX; // nothing must end the flight
        let mut seen = Vec::new();
        for _ in 0..(7.0 * LEVEL_SECONDS / TICK_SECONDS) as u32 {
            w.go_on();
            w.tick(&Input::NONE);
            for e in w.events() {
                match *e {
                    Event::NewLevel { level } => seen.push(("new", level)),
                    Event::LevelComplete { level } => seen.push(("done", level)),
                    _ => {}
                }
            }
        }
        // Level 1 landed, level 2 begins; landed, level 3; landed, level
        // 4; landed, level 1 again.
        assert_eq!(
            &seen[..8],
            [
                ("done", 1),
                ("new", 2),
                ("done", 2),
                ("new", 3),
                ("done", 3),
                ("new", 4),
                ("done", 4),
                ("new", 1)
            ]
        );
    }

    /// Fly (and land, and take off) until `level` has begun and its
    /// countdown is over.
    fn to_level(w: &mut World, level: u32) {
        for _ in 0..(3.0 * LEVEL_SECONDS / TICK_SECONDS) as u32 {
            if w.levels().level == level && w.landing.is_none() {
                return;
            }
            w.go_on();
            w.tick(&Input::NONE);
        }
        panic!("never reached level {level}");
    }

    #[test]
    fn a_new_level_starts_with_all_lives_again() {
        let mut w = world_with_plane_ahead(300.0);
        crash(&mut w); // one life lost in level 1
        assert_eq!(w.lives, LIVES - 1);
        assert!(w.smoke() > 0.0);
        w.planes.clear();
        // Fly on to level 2 with nothing to hit on the way.
        for _ in 0..(3.0 * LEVEL_SECONDS / TICK_SECONDS) as u32 {
            if w.levels().level == BEACH_LEVEL && w.landing.is_none() {
                break;
            }
            w.balloons.list.clear();
            w.balloons.pads.clear();
            w.huts.list.clear();
            w.go_on();
            w.tick(&Input::NONE);
        }
        assert_eq!(w.levels().level, BEACH_LEVEL);
        assert_eq!(w.lives, LIVES, "full lives in level 2");
        assert_eq!(w.smoke(), 0.0, "no smoke either");
    }

    /// Fly with `gas` (nothing to hit) until the approach begins.
    fn until_approach(w: &mut World, gas: f32) {
        while w.landing.is_none() {
            w.balloons.list.clear();
            w.balloons.pads.clear();
            w.huts.list.clear();
            w.gulls.list.clear();
            w.towers.list.clear();
            w.city.clear();
            w.tick(&Input::with_throttle(0.0, gas));
        }
    }

    #[test]
    fn the_main_wheels_touch_down_first_then_the_nose_wheel_with_tyre_smoke() {
        let mut w = World::with_seed(PHONE, 3);
        until_approach(&mut w, 0.0);
        let mut seen = Vec::new();
        let mut nose_up = false;
        while !w.level_complete().is_some() {
            w.tick(&Input::NONE);
            for e in w.events() {
                if matches!(e, Event::MainWheelsTouch | Event::NoseWheelTouch) {
                    seen.push((*e, w.ticks, w.tyre_smoke.len()));
                }
            }
            if w.rolling_out() {
                // The main wheels stay on the runway while the nose comes
                // down.
                let size = PHONE.plane_depth(Airport::runway_top(PHONE));
                let (sin, cos) = w.player.pitch.sin_cos();
                let main =
                    w.player.pos.y + (MAIN_WHEEL_X * sin + STRUT_REACH * cos + WHEEL_RADIUS) * size;
                assert!((main - Airport::runway_top(PHONE)).abs() < 0.01, "{main}");
                nose_up |= w.player.pitch < -0.05;
            }
        }
        assert!(nose_up, "touches down nose up");
        assert_eq!(w.player.pitch, 0.0, "the nose is down by the stop");
        let kinds: Vec<Event> = seen.iter().map(|s| s.0).collect();
        assert_eq!(kinds, [Event::MainWheelsTouch, Event::NoseWheelTouch]);
        let gap = (seen[1].1 - seen[0].1) as f32 * TICK_SECONDS;
        assert!((gap - NOSE_DOWN_SECONDS).abs() < 0.05, "{gap}");
        assert_eq!((seen[0].2, seen[1].2), (1, 2), "a puff for each");
        // The puffs are gone after a while.
        assert!(w.tyre_smoke.is_empty());
    }

    #[test]
    fn tyre_smoke_stays_on_the_ground_and_fades() {
        let mut w = World::with_seed(PHONE, 3);
        until_approach(&mut w, 0.0);
        while w.tyre_smoke.is_empty() {
            w.tick(&Input::NONE);
        }
        let first = w.tyre_smoke[0];
        assert!(first.pos.x < w.player.pos.x, "behind the plane's centre");
        assert_eq!(first.pos.y, Airport::runway_top(PHONE));
        w.tick(&Input::NONE);
        let later = w.tyre_smoke[0];
        assert!(later.pos.x < first.pos.x, "slides off with the ground");
        assert!(later.progress() > first.progress());
    }

    #[test]
    fn the_runways_are_lit_at_sunset_at_night_and_at_sunrise() {
        for (level, lit) in [
            (1, false),
            (BEACH_LEVEL, true),
            (CITY_LEVEL, true),
            (SUNRISE_LEVEL, true),
        ] {
            let mut w = world();
            w.start_level = level;
            assert_eq!(w.runway_lit(), lit, "level {level}");
        }
    }

    #[test]
    fn a_crash_on_the_approach_still_ends_the_level() {
        for late in [0.0, 1.0, 2.0, 3.0] {
            let mut w = World::with_seed(PHONE, 3);
            until_approach(&mut w, 0.0);
            for _ in 0..(late / TICK_SECONDS) as u32 {
                w.tick(&Input::NONE);
            }
            // Hit by a gull from behind (a life left): it falls a moment.
            w.lives = 2;
            w.player.crash_ticks = CRASH_TICKS;
            let mut done = false;
            for _ in 0..(40.0 / TICK_SECONDS) as u32 {
                w.tick(&Input::NONE);
                if w.level_complete().is_some() {
                    done = true;
                    break;
                }
            }
            assert!(done, "hit {late} s into the approach: never landed");
        }
    }

    #[test]
    fn no_obstacles_come_while_the_autopilot_takes_off() {
        for level in [1, BEACH_LEVEL, CITY_LEVEL] {
            let mut w = World::with_seed(PHONE, 7);
            w.start_level = level;
            let mut w = w.with_takeoff();
            while w.taking_off() {
                assert!(
                    w.gulls.list.is_empty()
                        && w.towers.list.is_empty()
                        && w.balloons.list.is_empty()
                        && w.huts.list.is_empty()
                        && w.city.high_rises.is_empty()
                        && w.city.cranes.is_empty(),
                    "level {level}: an obstacle during the takeoff"
                );
                w.tick(&Input::NONE);
            }
            assert_eq!(w.lives, LIVES, "level {level}: no crash on the way up");
        }
    }

    #[test]
    fn a_gull_still_around_cannot_hit_the_plane_while_the_autopilot_flies() {
        let mut w = World::with_seed(PHONE, 3);
        w.start_level = BEACH_LEVEL;
        until_approach(&mut w, 0.0);
        let mut ahead = w.clone();
        ahead.tick(&Input::NONE);
        let at = ahead.player.pos;
        let mut gulls = Gulls::new(1);
        while gulls.list.is_empty() {
            gulls.tick(PHONE, true, &mut Vec::new());
        }
        let mut gull = gulls.list[0].clone();
        gull.place_at(at);
        w.gulls.list = vec![gull];
        w.tick(&Input::NONE);
        assert_eq!(w.lives, LIVES, "the autopilot cannot dodge: no life lost");
        assert!(!w.player.is_crashed());
    }

    /// Fly a level through at `airspeed` (nothing in the way) and land:
    /// its time in milliseconds.
    fn level_time_at(airspeed: f32) -> u64 {
        let mut w = World::with_seed(PHONE, 3).with_takeoff();
        assert_eq!(w.level_progress(), 0.0);
        let mut last = 0.0;
        while w.level_complete().is_none() {
            assert!(w.level_progress() >= last, "the progress only grows");
            last = w.level_progress();
            w.balloons.list.clear();
            w.balloons.pads.clear();
            w.huts.list.clear();
            if w.landing.is_none() && !w.hands_off() {
                w.player.airspeed = airspeed;
            }
            w.tick(&Input::NONE);
        }
        assert_eq!(w.level_progress(), 1.0, "all the way at the landing");
        w.millis_in_level()
    }

    #[test]
    fn a_level_is_a_distance_so_more_speed_lands_sooner() {
        let cruising = level_time_at(1.0);
        let fast = level_time_at(1.4);
        // Roughly a minute of flying plus takeoff and landing.
        assert!((60_000..90_000).contains(&cruising), "{cruising}");
        assert!(fast + 10_000 < cruising, "{fast} vs {cruising}");
        assert_eq!(level_time_at(1.0), cruising, "same flight, same time");
    }

    #[test]
    fn the_level_time_is_in_milliseconds() {
        let mut w = World::with_seed(PHONE, 3);
        for _ in 0..90 {
            w.tick(&Input::NONE);
        }
        assert_eq!(w.millis_in_level(), 1500);
        w.tick(&Input::NONE);
        assert_eq!(w.millis_in_level(), 1517, "one tick: 1/60 s");
        assert_eq!(best_time_text(62_483), "1:02.483");
        assert_eq!(best_time_text(5_007), "0:05.007");
    }

    #[test]
    fn overspeed_at_the_end_of_a_level_does_not_wreck_the_landing() {
        let mut w = World::with_seed(PHONE, 3);
        // Gas on just before the landing: overspeed as the approach begins.
        // About 1.5 s of cruising before the landing.
        let left = 1.5 * SCROLL_SPEED / LEVEL_DISTANCE;
        while w.level_progress() < 1.0 - left {
            w.balloons.list.clear();
            w.balloons.pads.clear();
            w.huts.list.clear();
            w.tick(&Input::NONE);
        }
        until_approach(&mut w, 1.0);
        assert!(
            w.player.airspeed > OVERSPEED_AT,
            "overspeed at the approach"
        );
        assert_eq!(w.warning(), None, "no warning while the autopilot flies");
        for _ in 0..(2.0 / TICK_SECONDS) as u32 {
            w.tick(&Input::with_throttle(0.0, 1.0));
            assert!(!w.events().contains(&Event::Overspeed));
        }
        assert_eq!(w.mix.overspeed, 0.0, "the warning tone has faded out");
        for _ in 0..(20.0 / TICK_SECONDS) as u32 {
            w.tick(&Input::with_throttle(0.0, 1.0));
            if w.level_complete().is_some() {
                break;
            }
        }
        assert_eq!(w.explosion, None, "the autopilot takes care of the speed");
        assert_eq!(w.level_complete(), Some(1));
    }

    #[test]
    fn braking_into_the_landing_does_not_crash_it() {
        let mut w = World::with_seed(PHONE, 3);
        // About 1.0 s of cruising before the landing.
        let left = 1.0 * SCROLL_SPEED / LEVEL_DISTANCE;
        while w.level_progress() < 1.0 - left {
            w.balloons.list.clear();
            w.balloons.pads.clear();
            w.huts.list.clear();
            w.tick(&Input::NONE);
        }
        until_approach(&mut w, -1.0);
        for _ in 0..(20.0 / TICK_SECONDS) as u32 {
            w.tick(&Input::with_throttle(0.0, -1.0));
            if w.level_complete().is_some() {
                break;
            }
        }
        assert_eq!(w.explosion, None);
        assert_eq!(w.lives, LIVES);
        assert_eq!(w.level_complete(), Some(1));
    }

    #[test]
    fn the_landing_goes_from_approach_to_touchdown_to_complete() {
        let mut w = World::with_seed(PHONE, 2);
        w.lives = u32::MAX;
        let mut events = Vec::new();
        while w.landing.is_none() {
            // Balloons come at the plane even up there: out of the way.
            w.balloons.list.clear();
            w.tick(&Input::new(-1.0)); // even flying high, the autopilot comes
            events.extend_from_slice(w.events());
        }
        assert!(events.contains(&Event::GearDown));
        assert!(w.airport.is_some(), "the landing runway comes in");
        assert!(w.autopilot());
        // The autopilot flies; the player's steering does not count.
        let mut touched = false;
        let mut done = false;
        for _ in 0..(30.0 / TICK_SECONDS) as u32 {
            w.tick(&Input::new(-1.0));
            touched |= w.rolling_out();
            if w.events()
                .iter()
                .any(|e| matches!(e, Event::LevelComplete { level: 1 }))
            {
                done = true;
                break;
            }
        }
        assert!(
            touched && done,
            "touched {touched} done {done} y {} runway {} airport {:?} landing {:?}",
            w.player.pos.y,
            w.runway_y(),
            w.airport,
            w.landing
        );
        assert_eq!(w.lives, u32::MAX, "no crash on the way down");
        assert_eq!(w.level_complete(), Some(1));
        assert_eq!(w.speed(), 0.0, "standing still");
        let flown = w.seconds_flown();
        // "Level complete" waits for the player (Next level or Menu).
        for _ in 0..60 * 30 {
            w.tick(&Input::NONE);
        }
        assert_eq!(w.level_complete(), Some(1), "still waiting");
        assert_eq!(w.seconds_flown(), flown, "the clock stands still");
        w.go_on();
        w.tick(&Input::NONE);
        assert_eq!(w.levels().level, BEACH_LEVEL);
        assert_eq!(w.countdown(), Some(3));
        for _ in 0..seconds_to_ticks(COUNTDOWN_SECONDS) {
            w.tick(&Input::NONE);
        }
        assert_eq!(w.countdown(), None);
        assert!(w.taking_off() && w.on_runway(), "takes off at the beach");
    }

    #[test]
    fn a_flight_started_in_the_last_level_moves_on_to_level_1() {
        let mut w = world();
        w.start_level = LEVEL_COUNT;
        w.lives = u32::MAX; // the town must not end the flight
        assert_eq!(w.levels().level, LEVEL_COUNT);
        let seen = new_levels(&mut w, 1.5 * LEVEL_SECONDS);
        assert_eq!(seen.iter().map(|&(_, l)| l).collect::<Vec<_>>(), [1]);
    }

    #[test]
    fn the_waves_come_with_the_beach() {
        let mut w = World::with_seed(PHONE, 1);
        w.lives = u32::MAX;
        new_levels(&mut w, LEVEL_SECONDS - 1.0);
        assert_eq!(w.mix.beach, 0.0);
        to_level(&mut w, BEACH_LEVEL);
        new_levels(&mut w, 1.0);
        assert!((w.mix.beach - BEACH_VOLUME).abs() < 1e-6, "{}", w.mix.beach);
    }

    /// A phone in landscape, in logical px.
    const PHONE: Bounds = Bounds {
        width: 844.0,
        height: 390.0,
    };

    #[test]
    fn touching_a_gull_is_a_crash() {
        let mut w = World::with_seed(PHONE, 1);
        w.start_level = BEACH_LEVEL;
        // Wait for a gull in the open sky, far from the plane.
        let in_sky = |g: &Gull| {
            g.pos.x > PHONE.width / 2.0
                && g.pos.x < PHONE.width
                && g.pos.y > 2.0 * PLAYER_HALF_HEIGHT
                && g.pos.y < PHONE.height - 2.0 * PLAYER_HALF_HEIGHT
        };
        // ... at a calm moment: not crashed, no gull near the plane.
        let calm = |w: &World| {
            !w.player.is_crashed()
                && !w.player.is_safe()
                && w.gulls.list.iter().all(|g| {
                    (g.pos.x - w.player.pos.x).abs() + (g.pos.y - w.player.pos.y).abs() > 100.0
                })
        };
        while !(w.gulls.list.iter().any(in_sky) && calm(&w)) {
            w.tick(&Input::NONE);
        }
        // Move the plane to where that gull will be next tick.
        let i = w.gulls.list.iter().position(in_sky).unwrap();
        let mut ahead = w.clone();
        ahead.tick(&Input::NONE);
        let target = ahead.gulls.list[i].pos;
        w.player.pos.y = target.y;
        w.bounds.width = target.x / PLAYER_X_FRACTION; // the plane's x follows the width
        w.tick(&Input::NONE);
        assert!(w.player.is_crashed());
        assert!(w.events().contains(&Event::Crash));
        // The gull is hit too: it tumbles down, no longer in the way.
        let gull = &w.gulls.list[i];
        assert!(gull.is_down(), "the gull is knocked down");
        assert!(w.events().contains(&Event::GullCall), "with a squawk");
        let y = gull.pos.y;
        for _ in 0..30 {
            w.tick(&Input::NONE);
        }
        assert!(w.gulls.list.iter().any(|g| g.is_down() && g.pos.y > y));
    }

    #[test]
    fn a_falling_gull_is_no_obstacle() {
        let mut w = World::with_seed(PHONE, 1);
        w.start_level = BEACH_LEVEL;
        while w.gulls.list.is_empty() {
            w.tick(&Input::NONE);
        }
        w.gulls.list.truncate(1);
        w.gulls.list[0].knock_down(PHONE.scale());
        let mut ahead = w.clone();
        ahead.tick(&Input::NONE);
        let target = ahead.gulls.list[0].pos;
        w.player.pos.y = target.y;
        w.bounds.width = target.x / PLAYER_X_FRACTION;
        w.tick(&Input::NONE);
        assert!(!w.player.is_crashed());
        assert_eq!(w.lives, LIVES);
    }

    #[test]
    fn flying_into_a_balloon_is_a_crash() {
        let mut w = World::with_seed(PHONE, 2);
        while w.balloons.list.is_empty() {
            w.tick(&Input::NONE);
        }
        // Park the plane right where the balloon will be next tick.
        let mut ahead = w.clone();
        ahead.tick(&Input::NONE);
        let target = ahead.balloons.list[0].pos;
        w.player.pos.y = target.y;
        w.bounds.width = target.x / PLAYER_X_FRACTION; // the plane's x follows the width
        w.tick(&Input::NONE);
        assert!(w.events().contains(&Event::Crash));
        assert_eq!(w.lives, LIVES - 1);
        // The balloon explodes and falls; it is no obstacle any more.
        assert!(w.balloons.list[0].is_burst(), "the balloon bursts");
        assert!(w.events().contains(&Event::BalloonBurst), "with a bang");
        let y = w.balloons.list[0].pos.y;
        let mut again = w.clone();
        again.player.crash_ticks = 0;
        again.player.safe_ticks = 0;
        again.player.touching = false;
        again.tick(&Input::NONE);
        assert_eq!(again.lives, LIVES - 1, "a burst balloon is no crash");
        for _ in 0..30 {
            w.tick(&Input::NONE);
        }
        assert!(w.balloons.list.iter().any(|b| b.is_burst() && b.pos.y > y));
    }

    #[test]
    fn balloons_rise_in_level_1_and_none_at_the_beach() {
        let mut w = World::with_seed(PHONE, 3);
        w.lives = u32::MAX;
        let mut burners = 0;
        for _ in 0..((LEVEL_SECONDS - CLEAR_SECONDS) / TICK_SECONDS) as u32 {
            w.tick(&Input::NONE);
            burners += (w.events().iter())
                .filter(|e| **e == Event::BalloonBurner)
                .count();
        }
        assert!(burners >= 5, "{burners} in level 1");
        // Landed and off to the beach: no balloons there.
        to_level(&mut w, BEACH_LEVEL);
        assert!(w.balloons.list.is_empty());
        for _ in 0..(40.0 / TICK_SECONDS) as u32 {
            w.tick(&Input::NONE);
            assert!(!w.events().contains(&Event::BalloonBurner));
        }
    }

    #[test]
    fn flying_low_into_a_hut_is_a_crash_and_flying_over_is_not() {
        let fly = |steer: f32| {
            let mut w = World::with_seed(PHONE, 2);
            w.lives = u32::MAX;
            let mut crashes = 0;
            for _ in 0..(20.0 / TICK_SECONDS) as u32 {
                w.balloons.list.clear(); // only the huts here
                w.tick(&Input::new(steer));
                crashes += w.events().iter().filter(|e| **e == Event::Crash).count();
            }
            assert!(!w.huts.list.is_empty() || crashes > 0, "huts came");
            crashes
        };
        assert!(fly(1.0) > 0, "low over the grass");
        assert_eq!(fly(-1.0), 0, "high in the sky");
    }

    #[test]
    fn skimming_over_a_hut_is_no_crash_but_touching_it_is() {
        let hut = Hut {
            x: 400.0,
            width: 110.0,
            height: 95.0,
            style: 0,
        };
        let (_, ridge, _, _) = hut.rect(PHONE);
        // The wing tip is the plane's lowest point: put it right over the
        // ridge, then just above and just into it.
        let plane_over = |gap: f32| {
            // Find the height where the wing tip is `gap` above the ridge.
            let mut y = ridge - 60.0;
            loop {
                let size = PHONE.plane_depth(y);
                let (tx, ty) = PLANE_WING[2];
                if y + ty * size >= ridge - gap {
                    return Player::at(Vec2::new(400.0 - tx * size, y));
                }
                y += 0.05;
            }
        };
        let touches = |p: Player| any_overlap(&p.shape(PHONE), &hut.shape(PHONE));
        assert!(!touches(plane_over(1.5)), "a near miss is a miss");
        assert!(touches(plane_over(-1.5)), "a touch is a crash");
        // Flying low beside the hut, in the empty corner by the roof: free.
        // (The nose tip 3 px short of the hut's left edge.)
        let y = ridge + 20.0;
        let nose = PLANE_NOSE[1].0 * PHONE.plane_depth(y);
        let corner = Player::at(Vec2::new(400.0 - 110.0 / 2.0 - nose - 3.0, y));
        assert!(!touches(corner));
    }

    #[test]
    fn flying_into_a_launch_pad_is_a_crash() {
        let mut w = World::with_seed(PHONE, 2);
        w.huts.list.clear();
        w.balloons.list.clear();
        let x = w.player.pos.x;
        w.balloons.pads.push(Pad { x });
        w.player.pos.y = Pad::top(PHONE) - 2.0; // skimming the deck
        w.tick(&Input::NONE);
        assert!(w.events().contains(&Event::Crash));
        assert_eq!(w.lives, LIVES - 1);
    }

    #[test]
    fn no_huts_or_pads_at_the_beach() {
        let mut w = World::with_seed(PHONE, 3);
        w.lives = u32::MAX;
        to_level(&mut w, BEACH_LEVEL);
        for _ in 0..(40.0 / TICK_SECONDS) as u32 {
            w.tick(&Input::NONE);
            assert!(w.huts.list.is_empty() && w.balloons.pads.is_empty());
        }
    }

    #[test]
    fn lifeguard_towers_stand_only_at_the_beach_and_flying_low_hits_them() {
        let mut w = World::with_seed(PHONE, 5);
        w.lives = u32::MAX;
        new_levels(&mut w, LEVEL_SECONDS - 0.1);
        assert!(w.towers.list.is_empty(), "none in level 1");
        // At the beach, skimming the sand: a tower comes and is a crash.
        let mut w = World::with_seed(PHONE, 5);
        w.lives = u32::MAX;
        w.start_level = BEACH_LEVEL;
        let mut crashes = 0;
        for _ in 0..(25.0 / TICK_SECONDS) as u32 {
            w.gulls.list.clear(); // only the towers here
            w.tick(&Input::new(1.0));
            if !w.towers.list.is_empty() {
                crashes += w.events().iter().filter(|e| **e == Event::Crash).count();
            }
        }
        assert!(crashes > 0);
    }

    #[test]
    fn the_town_stands_only_in_level_3() {
        let mut w = World::with_seed(PHONE, 5);
        w.lives = u32::MAX;
        for _ in 0..((2.0 * LEVEL_SECONDS + 20.0) / TICK_SECONDS) as u32 {
            w.go_on();
            w.tick(&Input::NONE);
            let town = !w.city.high_rises.is_empty() || !w.city.cranes.is_empty();
            if w.levels().level != CITY_LEVEL {
                assert!(!town, "level {}", w.levels().level);
            }
        }
        let mut w = World::with_seed(PHONE, 5);
        w.start_level = CITY_LEVEL;
        let mut w = w.with_takeoff();
        let mut both = (false, false);
        for _ in 0..(40.0 / TICK_SECONDS) as u32 {
            w.lives = LIVES;
            w.tick(&Input::NONE);
            both.0 |= !w.city.high_rises.is_empty();
            both.1 |= !w.city.cranes.is_empty();
        }
        assert_eq!(both, (true, true), "high-rises and cranes in the town");
    }

    /// Flying in level 4, past the takeoff, nothing on screen yet.
    fn in_the_sunrise() -> World {
        let mut w = World::with_seed(PHONE, 5);
        w.start_level = SUNRISE_LEVEL;
        let mut w = w.with_takeoff();
        while w.hands_off() {
            w.tick(&Input::NONE);
        }
        w.sunrise.clear();
        w
    }

    #[test]
    fn the_sunrise_is_only_in_level_4() {
        let mut w = World::with_seed(PHONE, 5);
        w.lives = u32::MAX;
        for _ in 0..((3.0 * LEVEL_SECONDS + 20.0) / TICK_SECONDS) as u32 {
            w.go_on();
            w.tick(&Input::NONE);
            if w.levels().level != SUNRISE_LEVEL {
                assert!(w.sunrise.geese.is_empty() && w.sunrise.steam.is_empty());
                assert_eq!(w.sunrise.glare(), 0.0);
            }
        }
        let mut w = in_the_sunrise();
        let mut seen = (false, false);
        for _ in 0..(20.0 / TICK_SECONDS) as u32 {
            w.lives = LIVES;
            w.tick(&Input::NONE);
            seen.0 |= !w.sunrise.geese.is_empty();
            seen.1 |= !w.sunrise.steam.is_empty();
        }
        assert_eq!(seen, (true, true), "geese and steam at sunrise");
    }

    #[test]
    fn at_sunrise_high_rises_stand_far_back_but_never_by_a_runway() {
        let mut w = World::with_seed(PHONE, 7);
        w.start_level = SUNRISE_LEVEL;
        let mut w = w.with_takeoff();
        let mut seen = 0;
        while w.level_complete().is_none() {
            w.lives = LIVES;
            w.sunrise.geese.clear();
            w.sunrise.steam.clear();
            w.tick(&Input::NONE);
            seen = seen.max(w.sunrise.high_rises.len());
            let Some(a) = w.airport else { continue };
            let (from, to) = (a.x, a.x + RUNWAY_LENGTH * PHONE.scale());
            for b in &w.sunrise.high_rises {
                let (l, r) = b.span(PHONE);
                assert!(r < from || l > to, "a high-rise by the runway");
            }
        }
        assert!(seen > 0, "there are high-rises at sunrise");
    }

    #[test]
    fn flying_into_a_goose_costs_a_life_and_knocks_it_down() {
        let mut w = in_the_sunrise();
        let mut crashed = false;
        for _ in 0..(20.0 / TICK_SECONDS) as u32 {
            w.sunrise.steam.clear();
            w.lives = LIVES;
            // Hold the plane where the next V is aimed.
            w.tick(&Input::NONE);
            if w.events().contains(&Event::Crash) {
                crashed = true;
                assert!(w.sunrise.geese.iter().any(|g| g.is_down()));
                break;
            }
        }
        assert!(
            crashed,
            "a V aimed at the plane hits it if it does not dodge"
        );
    }

    #[test]
    fn flying_into_a_high_rise_at_sunrise_costs_a_life() {
        let mut w = in_the_sunrise();
        w.sunrise.high_rises.push(FarHighRise {
            x: w.player.pos.x + 120.0,
            width: 110.0,
            top: 0.55,
        });
        let (_, top, _, h) = w.sunrise.high_rises[0].rect(PHONE);
        let y = top + h / 3.0;
        let lives = w.lives;
        for _ in 0..(2.5 / TICK_SECONDS) as u32 {
            w.sunrise.geese.clear();
            w.sunrise.steam.clear();
            w.player.pos.y = y;
            w.player.vel_y = 0.0;
            w.tick(&Input::NONE);
        }
        assert_eq!(w.lives, lives - 1, "the high-rise is solid");
    }

    #[test]
    fn flying_into_a_house_under_the_steam_costs_a_life() {
        let mut w = in_the_sunrise();
        w.sunrise.steam.push(Steam {
            x: w.player.pos.x + 80.0,
            width: 50.0,
            top: 0.4,
            house: (70.0, 45.0),
            age: 0,
        });
        let (_, top, _, h) = w.sunrise.steam[0].house_parts(PHONE).walls;
        let y = top + h / 2.0;
        let lives = w.lives;
        for _ in 0..(2.0 / TICK_SECONDS) as u32 {
            w.sunrise.geese.clear();
            w.player.pos.y = y;
            w.player.vel_y = 0.0;
            w.tick(&Input::NONE);
        }
        assert_eq!(w.lives, lives - 1, "the house is solid");
    }

    #[test]
    fn steam_throws_the_plane_up_but_costs_no_life() {
        let mut w = in_the_sunrise();
        let y = 0.6 * PHONE.height;
        w.player.pos.y = y;
        w.player.vel_y = 0.0;
        w.sunrise.steam.push(Steam {
            x: w.player.pos.x + 40.0,
            width: 60.0,
            top: 0.3,
            house: (70.0, 45.0),
            age: 0,
        });
        let lives = w.lives;
        let mut hissed = 0;
        for _ in 0..(0.6 / TICK_SECONDS) as u32 {
            w.sunrise.geese.clear();
            w.tick(&Input::NONE);
            hissed += w
                .events()
                .iter()
                .filter(|e| **e == Event::SteamHiss)
                .count();
        }
        assert_eq!(w.lives, lives, "no life lost");
        assert!(
            w.player.pos.y < y - 15.0,
            "thrown up: {} from {y}",
            w.player.pos.y
        );
        assert_eq!(hissed, 1, "one hiss on the way in");
    }

    /// The town with one crane in the plane's way (its mast 150 px ahead),
    /// the plane at `y`.
    fn crane_ahead(y: f32) -> World {
        let mut w = World::with_seed(PHONE, 5);
        w.start_level = CITY_LEVEL;
        let mut w = w.with_takeoff();
        while w.hands_off() {
            w.tick(&Input::NONE);
        }
        w.city.clear();
        w.city.cranes.push(Crane {
            x: w.player.pos.x + 150.0,
            reach: 200.0,
            load_width: 90.0,
            jib_at: 0.42,
            load_drop: 40.0,
        });
        w.player.pos.y = y;
        w.player.vel_y = 0.0;
        w
    }

    /// Fly level at `y` for `seconds` with no flying helicopters; the
    /// lives lost.
    fn hold(w: &mut World, y: f32, seconds: f32) -> u32 {
        let lives = w.lives;
        for _ in 0..(seconds / TICK_SECONDS) as u32 {
            w.city.flyers.clear();
            w.player.pos.y = y;
            w.player.vel_y = 0.0;
            w.tick(&Input::NONE);
        }
        lives - w.lives
    }

    #[test]
    fn low_down_a_cranes_mast_is_in_the_way() {
        let low = 0.8 * PHONE.height;
        let mut w = crane_ahead(low);
        assert_eq!(hold(&mut w, low, 3.0), 1, "the mast");
    }

    #[test]
    fn over_a_crane_is_the_way_through() {
        let probe = crane_ahead(0.0);
        let over = probe.city.cranes[0].top(PHONE) - 0.1 * PHONE.height;
        let mut w = crane_ahead(over);
        assert_eq!(hold(&mut w, over, 3.0), 0, "no crash over the crane");
        let mast = (w.city.cranes.iter().map(|c| c.parts(PHONE).mast.0.x)).fold(f32::MAX, f32::min);
        assert!(mast < w.player.pos.x, "flew past the mast");
    }

    #[test]
    fn right_at_the_top_helicopters_come_at_you_and_crash() {
        let mut w = World::with_seed(PHONE, 5);
        w.start_level = CITY_LEVEL;
        let mut w = w.with_takeoff();
        while w.hands_off() {
            w.tick(&Input::NONE);
        }
        let top = PLAYER_HALF_HEIGHT * PHONE.plane_depth(0.0);
        let mut crashes = 0;
        for _ in 0..(40.0 / TICK_SECONDS) as u32 {
            w.lives = LIVES;
            w.player.pos.y = top; // hug the top
            w.player.vel_y = 0.0;
            w.tick(&Input::NONE);
            if w.events().contains(&Event::HeliCrash) {
                crashes += 1;
                assert!(w.events().contains(&Event::Crash), "costs a life");
                assert!(!w.city.wrecks.is_empty(), "it falls, burning");
            }
        }
        assert!(crashes >= 3, "{crashes}");
    }

    #[test]
    fn flying_into_a_parked_helicopter_crashes_it() {
        let mut w = crane_ahead(0.0);
        w.city.clear();
        w.city.high_rises.push(HighRise {
            x: w.player.pos.x + 200.0,
            width: 80.0,
            height: 0.4,
            style: 0,
            heli: Some(Heli {
                off: Vec2::new(0.0, -10.0),
                state: HeliState::Landed(0),
                age: 0,
            }),
        });
        let heli_y = w.city.high_rises[0].heli_parts(PHONE).unwrap().body.0.y;
        let mut events = Vec::new();
        for _ in 0..(4.0 / TICK_SECONDS) as u32 {
            w.city.flyers.clear();
            w.player.pos.y = heli_y;
            w.player.vel_y = 0.0;
            w.tick(&Input::NONE);
            events.extend_from_slice(w.events());
        }
        assert!(events.contains(&Event::HeliCrash), "{events:?}");
    }

    #[test]
    fn no_gulls_in_level_1() {
        let mut w = world();
        new_levels(&mut w, LEVEL_SECONDS - 0.1);
        assert!(w.gulls.list.is_empty());
    }

    #[test]
    fn no_rain_and_no_thunderstorms_in_any_level() {
        for seed in 0..4 {
            let mut w = World::with_seed(PHONE, seed);
            w.lives = u32::MAX; // keep flying through both levels
            for _ in 0..5 * 3600 {
                w.tick(&Input::NONE);
                assert_eq!((w.weather.rain, w.mix.rain), (0.0, 0.0), "seed {seed}");
                for e in w.events() {
                    match e {
                        Event::Lightning { .. } | Event::Thunder { .. } => {
                            panic!("seed {seed}: {e:?}")
                        }
                        Event::Crash
                        | Event::NewLevel { .. }
                        | Event::GullCall
                        | Event::BalloonBurner
                        | Event::BalloonBurst
                        | Event::HeliCrash
                        | Event::GooseHonk { .. }
                        | Event::SteamHiss => {}
                        Event::GameOver => panic!("lives cannot run out"),
                        Event::Explosion => panic!("lives cannot run out"),
                        Event::Overspeed | Event::PullUp => panic!("no throttle given"),
                        // 5 minutes: several levels, each landed and taken off.
                        Event::Takeoff
                        | Event::GearUp
                        | Event::GearDown
                        | Event::MainWheelsTouch
                        | Event::NoseWheelTouch
                        | Event::LevelComplete { .. } => {}
                    }
                }
            }
        }
    }

    /// A phone-sized world with nothing to hit, mid-screen.
    fn open_sky() -> World {
        let mut w = World::new(PHONE);
        w.lives = u32::MAX;
        w
    }

    fn fly(w: &mut World, steer: f32, seconds: f32) {
        for _ in 0..(seconds / TICK_SECONDS).round() as u32 {
            w.balloons.list.clear();
            w.huts.list.clear();
            w.tick(&Input::new(steer));
        }
    }

    #[test]
    fn a_dive_starts_gently_and_reaches_full_speed() {
        let mut w = open_sky();
        let y = w.player.pos.y;
        w.tick(&Input::new(1.0));
        let first = w.player.pos.y - y;
        let full = PLAYER_CLIMB_SPEED * TICK_SECONDS;
        assert!(first > 0.0 && first < 0.25 * full, "gentle: {first}");
        fly(&mut w, 1.0, 0.6);
        assert!(
            (w.player.vel_y - PLAYER_CLIMB_SPEED).abs() < 3.0,
            "{}",
            w.player.vel_y
        );
    }

    #[test]
    fn half_steering_settles_at_half_speed() {
        let mut w = open_sky();
        fly(&mut w, -0.5, 0.6);
        let half = -0.5 * PLAYER_CLIMB_SPEED;
        assert!((w.player.vel_y - half).abs() < 2.0, "{}", w.player.vel_y);
    }

    #[test]
    fn the_nose_tilts_up_to_climb_and_down_to_dive_and_levels_out() {
        let mut w = open_sky();
        assert_eq!(w.player.pitch, 0.0);
        let max = MAX_PITCH_DEG.to_radians();
        fly(&mut w, -1.0, 0.1);
        let early = w.player.pitch;
        assert!(
            early < 0.0 && early > -max * 0.6,
            "starts tilting up: {early}"
        );
        fly(&mut w, -1.0, 0.6);
        assert!(
            (w.player.pitch + max).abs() < 0.02,
            "nose up: {}",
            w.player.pitch
        );
        fly(&mut w, 0.0, 1.0);
        assert!(
            w.player.pitch.abs() < 0.02,
            "level again: {}",
            w.player.pitch
        );
        fly(&mut w, 0.5, 1.0);
        let down = w.player.pitch;
        assert!((down - max * 0.5).abs() < 0.02, "half down: {down}");
    }

    #[test]
    fn the_crash_shape_tilts_with_the_plane() {
        let mut p = Player::at(Vec2::new(400.0, 200.0));
        let level = p.shape(PHONE);
        p.pitch = -MAX_PITCH_DEG.to_radians();
        let tilted = p.shape(PHONE);
        // The nose tip is higher when the nose is up.
        assert!(tilted[1].0[1].y < level[1].0[1].y - 3.0);
        assert!((tilted[1].0[1].x - level[1].0[1].x).abs() < 3.0);
    }

    #[test]
    fn player_cannot_leave_the_screen() {
        let mut w = world();
        for _ in 0..600 {
            w.balloons.list.clear(); // only the edges here
            w.huts.list.clear();
            w.tick(&Input::new(-1.0));
        }
        // Small at the top, big at the bottom.
        let top = PLAYER_HALF_HEIGHT * w.bounds.size();
        assert_eq!(w.player.pos.y, top);
        for _ in 0..600 {
            w.balloons.list.clear();
            w.huts.list.clear();
            w.tick(&Input::new(1.0));
        }
        let bottom = PLAYER_HALF_HEIGHT * w.bounds.scale() * PLANE_NEAR_SIZE;
        assert_eq!(w.player.pos.y, 50.0 - bottom);
    }

    #[test]
    fn following_a_finger_settles_on_it_without_overshooting() {
        let mut w = World::new(Bounds {
            width: 800.0,
            height: 400.0,
        });
        let target = 330.0;
        let mut last = w.player.pos.y;
        for _ in 0..600 {
            w.balloons.list.clear(); // only the steering here
            w.huts.list.clear();
            let steer = w.steer_toward(target);
            w.tick(&Input::new(steer));
            let y = w.player.pos.y;
            assert!(
                y >= last && y <= target,
                "overshot or reversed: {last} -> {y}"
            );
            last = y;
        }
        assert!((target - last).abs() < 0.5, "did not settle: {last}");
    }

    #[test]
    fn player_keeps_its_place_when_the_screen_resizes() {
        let mut w = world();
        w.bounds = Bounds {
            width: 400.0,
            height: 20.0,
        }; // e.g. phone rotated; the plane was at 25, below the new edge
        w.tick(&Input::NONE);
        let half = PLAYER_HALF_HEIGHT * w.bounds.scale() * PLANE_NEAR_SIZE; // down low
        assert_eq!(
            w.player.pos,
            Vec2::new(400.0 * PLAYER_X_FRACTION, 20.0 - half)
        );
    }

    #[test]
    fn the_plane_is_small_up_high_and_big_down_low_like_the_balloons() {
        let w = World::new(PHONE);
        let b = w.bounds;
        assert_eq!(b.depth(0.0), PHONE.size(), "smallest at the very top");
        assert_eq!(b.plane_depth(0.0), PHONE.size());
        assert_eq!(b.depth(0.95 * PHONE.height), PHONE.scale());
        assert_eq!(
            b.plane_depth(0.95 * PHONE.height),
            PHONE.scale() * PLANE_NEAR_SIZE
        );
        // Shrinking all the way up, without a flat part.
        let near_y = Balloon::standing_y(PHONE);
        for i in 1..=10 {
            let (hi, lo) = (near_y * (i - 1) as f32 / 10.0, near_y * i as f32 / 10.0);
            assert!(b.depth(hi) < b.depth(lo), "{hi} vs {lo}");
            assert!(b.plane_depth(hi) < b.plane_depth(lo));
            assert!(
                b.plane_depth(lo) >= b.depth(lo),
                "plane at least balloon size"
            );
        }
        // Balloons follow the same rule.
        for y in [50.0, 200.0, 260.0, 320.0, 380.0] {
            let b = Balloon::new(
                Vec2::new(400.0, y),
                Vec2::new(0.0, 0.0),
                0,
                BalloonWay::AtPlane,
            );
            assert_eq!(b.size(PHONE), w.bounds.depth(y));
        }
        // A plane parked low hits from further away than one up high.
        let reach_at = |y: f32| {
            let mut w = World::new(PHONE);
            w.player.pos.y = y;
            let mut gap = 0.0;
            loop {
                let mut t = w.clone();
                t.planes = vec![];
                let p = t.player.pos;
                t.spawn_plane("m", Vec2::new(p.x + gap, p.y), Vec2::new(0.0, 0.0));
                t.tick(&Input::NONE);
                if !t.events().contains(&Event::Crash) {
                    return gap;
                }
                gap += 1.0;
            }
        };
        assert!(reach_at(PHONE.height - 20.0) > reach_at(60.0));
    }

    #[test]
    fn a_screen_with_no_height_centres_the_plane_instead_of_crashing() {
        // The plane shrinks with the screen, so only a screen of (almost)
        // no height is too small to fly in, e.g. while a window opens.
        let mut w = World::new(Bounds {
            width: 100.0,
            height: 0.0,
        });
        w.tick(&Input::new(1.0));
        assert_eq!(w.player.pos.y, 0.0);
    }

    #[test]
    fn plane_and_obstacles_keep_their_size_relative_to_the_screen() {
        let phone = Bounds {
            width: 844.0,
            height: DESIGN_HEIGHT,
        };
        let computer = Bounds {
            width: 1440.0,
            height: 900.0,
        };
        assert_eq!(phone.scale(), 1.0);
        let s = computer.scale();
        assert!((s - 900.0 / 390.0).abs() < 1e-6);
        // One second of full climb crosses the same share of the screen.
        let share = |b: Bounds| {
            let mut w = World::new(b);
            for _ in 0..30 {
                w.tick(&Input::new(-1.0));
            }
            (b.height / 2.0 - w.player.pos.y) / b.height
        };
        assert!((share(phone) - share(computer)).abs() < 1e-4);
        // A balloon touches at the same share of the screen.
        let balloon = |b: Bounds| {
            Balloon::new(
                Vec2::new(500.0, b.height / 2.0),
                Vec2::new(0.0, 0.0),
                0,
                BalloonWay::AtPlane,
            )
        };
        let mut shares = Vec::new();
        for b in [phone, computer] {
            // The plane's nose tip meets the envelope's side.
            let y = b.height / 2.0;
            let reach = PLANE_NOSE[1].0 * b.plane_depth(y) + BALLOON_RADIUS * balloon(b).size(b);
            let touches = |gap: f32| {
                let me = Player::at(Vec2::new(500.0 - gap, y)).shape(b);
                any_overlap(&me, &balloon(b).shape(balloon(b).size(b)))
            };
            assert!(touches(reach - 1.0) && !touches(reach + 1.0));
            shares.push(reach / b.height);
        }
        assert!((shares[0] - shares[1]).abs() < 1e-4);
        // A finger pulls the plane over the same share of the screen.
        for b in [phone, computer] {
            let w = World::new(b);
            let ease = TOUCH_EASE_DISTANCE * b.scale();
            assert!((w.steer_toward(w.player.pos.y + ease / 2.0) - 0.5).abs() < 1e-5);
        }
    }

    #[test]
    fn simulation_is_deterministic() {
        let bounds = Bounds {
            width: 100.0,
            height: 50.0,
        };
        let mut a = World::with_seed(bounds, 4);
        let mut b = World::with_seed(bounds, 4);
        let mut logs = [Vec::new(), Vec::new()];
        for (w, log) in [&mut a, &mut b].into_iter().zip(&mut logs) {
            w.spawn_plane("m", Vec2::new(5.0, 5.0), Vec2::new(37.0, 91.0));
            w.lives = u32::MAX; // long enough for both levels
            for i in 0..10_000 {
                w.go_on();
                w.tick(&Input::new(((i % 7) as f32 - 3.0) / 3.0));
                log.extend_from_slice(w.events());
            }
        }
        assert_eq!(a, b);
        assert_eq!(logs[0], logs[1]);
        // The run is long enough to cover crashes and both levels.
        assert!(logs[0].contains(&Event::Crash));
        assert!(logs[0].contains(&Event::NewLevel { level: 2 }));
    }

    #[test]
    fn plane_stays_in_bounds_over_time() {
        let mut w = world();
        w.spawn_plane("m", Vec2::new(5.0, 5.0), Vec2::new(300.0, 500.0));
        for _ in 0..10_000 {
            w.tick(&Input::NONE);
            let m = &w.planes[0];
            assert!(
                m.pos.x >= 0.0 && m.pos.x <= 100.0,
                "x out of bounds: {}",
                m.pos.x
            );
            assert!(
                m.pos.y >= 0.0 && m.pos.y <= 50.0,
                "y out of bounds: {}",
                m.pos.y
            );
        }
    }
}
