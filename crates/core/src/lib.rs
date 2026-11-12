//! Pure game logic. No rendering, no input polling, no timers.
//! The game crate drives this with `World::tick` and reads state to draw.

/// Fixed simulation step in seconds. The renderer accumulates frame time
/// and calls `tick` zero or more times per frame.
pub const TICK_SECONDS: f32 = 1.0 / 60.0;

mod balloons;
mod clock;
mod gulls;
mod huts;
mod input;
mod level;
mod rng;
mod scenery;
mod shape;
mod sound;
mod tilt;
mod weather;
pub use balloons::{
    Balloon, BalloonWay, Balloons, Pad, BALLOON_COLOURS, BALLOON_GAP_SECONDS, BALLOON_HIGH,
    BALLOON_RADIUS, BALLOON_RISE, BASKET_BELOW, BASKET_HALF, BASKET_RADIUS, FIRST_BALLOON_SECONDS,
    PAD_GROUND, PAD_HEIGHT, PAD_WIDTH, TAKEOFF_AT, TOUCHDOWN_AT,
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
pub use input::{pick_steer, steer_from_keys, steer_toward, Input, TOUCH_EASE_DISTANCE};
pub use level::{
    level_view, start_level, LevelView, LEVEL_BANNER_SECONDS, LEVEL_COUNT, LEVEL_FADE_SECONDS,
    LEVEL_SECONDS, RAINY_LEVELS,
};
pub use scenery::{background_x, scroll_distance, SCROLL_SPEED};
pub use shape::{any_overlap, Poly};
pub use sound::{
    Mix, BALLOON_BURNER_VOLUME, BEACH_LEVEL, BEACH_VOLUME, CRASH_VOLUME, DIVE_WIND, ENGINE_VOLUME,
    GULL_CALL_VOLUME, MIX_CHANGE_PER_SECOND, NEW_LEVEL_VOLUME, RAIN_VOLUME, WIND_VOLUME,
};
pub use tilt::{Tilt, TILT_DEAD_ZONE_DEG, TILT_FULL_DEG, TILT_MIN_GRAVITY, TILT_SMOOTHING};
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
pub const PLANE_NEAR_SIZE: f32 = 1.5;

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

/// Points for staying in the air: the score grows by this much every
/// second of the flight (also while crashed; it stops at game over).
pub const SCORE_PER_SECOND: u64 = 2;

/// "Game over" shows at least this long before a tap starts again, so a
/// finger still steering does not skip it.
pub const RESTART_WAIT_SECONDS: f32 = 1.0;

/// Something that happened during a tick. The game crate reacts to these
/// with one-shot effects (a sound, a flash); they never change the rules.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Event {
    /// The player's plane hit another plane.
    Crash,
    /// Lightning struck. `distance` is 0 (overhead) to 1 (far away).
    Lightning { distance: f32 },
    /// Thunder from an earlier strike arrives. `volume` is 0..1.
    Thunder { volume: f32 },
    /// A new level begins (also when the game starts over with level 1).
    NewLevel { level: u32 },
    /// A gull calls as it appears.
    GullCall,
    /// A hot-air balloon's burner roars as it appears.
    BalloonBurner,
    /// The last life is gone: the plane has fallen and the flight is over.
    GameOver,
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
        let at = |points: &[(f32, f32)]| Poly::at(self.pos, size, points);
        let ((cx, cy), r) = PLANE_COCKPIT;
        let cockpit = Vec2::new(self.pos.x + cx * size, self.pos.y + cy * size);
        vec![
            at(&PLANE_FUSELAGE),
            at(&PLANE_NOSE),
            at(&PLANE_WING),
            at(&PLANE_TAIL),
            Poly::ellipse(cockpit, r * size, r * size),
        ]
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

    fn depth_up_to(&self, y: f32, near_size: f32) -> f32 {
        let near_y = Balloon::standing_y(*self);
        let near = if near_y > 0.0 {
            (y / near_y).clamp(0.0, 1.0)
        } else {
            1.0
        };
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
    /// The obstacles at the beach.
    pub gulls: Gulls,
    pub weather: Weather,
    /// Volumes of the looping sounds, eased toward what fits the world now.
    pub mix: Mix,
    pub ticks: u64,
    /// The level the flight started in (1 ..= `LEVEL_COUNT`).
    pub start_level: u32,
    /// Lives left, [`LIVES`] at the start.
    pub lives: u32,
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
            weather: Weather::new(seed),
            mix: Mix::SILENT,
            ticks: 0,
            start_level: 1,
            lives: LIVES,
            over_ticks: 0,
            events: Vec::new(),
        }
    }

    /// Which level is on screen now, and how far the next has faded in.
    pub fn levels(&self) -> LevelView {
        level_view(self.ticks, self.start_level)
    }

    /// Is the flight over? True once the last life is gone and the plane
    /// has fallen. Then nothing moves any more.
    pub fn is_over(&self) -> bool {
        self.lives == 0 && !self.player.is_crashed()
    }

    /// May a tap start a new flight? Only once "Game over" has shown for
    /// [`RESTART_WAIT_SECONDS`].
    pub fn can_restart(&self) -> bool {
        self.is_over() && self.over_ticks as f32 * TICK_SECONDS >= RESTART_WAIT_SECONDS
    }

    /// Points so far: [`SCORE_PER_SECOND`] for every full second flown.
    /// Stops growing at game over, since time stands still then.
    pub fn score(&self) -> u64 {
        let ticks_per_second = (1.0 / TICK_SECONDS).round() as u64;
        self.ticks * SCORE_PER_SECOND / ticks_per_second
    }

    /// Steering toward a finger at `target_y` (screen px), from where the
    /// plane is now. Eases off within [`TOUCH_EASE_DISTANCE`], measured at
    /// [`DESIGN_HEIGHT`] like every other size.
    pub fn steer_toward(&self, target_y: f32) -> f32 {
        let scale = self.bounds.scale();
        steer_toward(target_y / scale, self.player.pos.y / scale)
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
        let fall = if self.player.is_crashed() {
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
        p.x = width * PLAYER_X_FRACTION; // follow resizes / rotation
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
        let coast = levels.level == 1 && levels.fade == 0.0;
        let plane_y = self.player.pos.y;
        let pads: Vec<f32> = self.balloons.pads.iter().map(|p| p.x).collect();
        self.huts.tick(self.bounds, coast, &pads);
        let huts: Vec<f32> = self.huts.list.iter().map(|h| h.x).collect();
        let events = &mut self.events;
        (self.balloons).tick(self.bounds, coast, plane_y, &huts, events);
        let beach = levels.level == BEACH_LEVEL && levels.fade == 0.0;
        self.gulls.tick(self.bounds, beach, &mut self.events);

        // Crash: the plane as drawn newly overlaps another plane, a
        // balloon, a launch pad, a hut or a gull as drawn. It costs a
        // life; a crashed or safe plane cannot crash.
        let bounds = self.bounds;
        let me = self.player.shape(bounds);
        let hits = |them: &[Poly]| any_overlap(&me, them);
        let touching = (self.planes.iter()).any(|m| {
            let r = PLANE_RADIUS * bounds.depth(m.pos.y);
            me.iter().any(|part| part.overlaps_circle(m.pos, r))
        }) || (self.gulls.list.iter())
            .any(|g| hits(&g.shape(bounds.depth(g.pos.y))))
            || (self.balloons.list.iter()).any(|b| hits(&b.shape(b.size(bounds))))
            || (self.balloons.pads.iter()).any(|p| hits(&p.shape(bounds)))
            || (self.huts.list.iter()).any(|h| hits(&h.shape(bounds)));
        let player = &mut self.player;
        if touching && !player.touching && !player.is_crashed() && !player.is_safe() {
            player.crash_ticks = CRASH_TICKS;
            self.lives = self.lives.saturating_sub(1);
            player.steer = 0.0;
            self.events.push(Event::Crash);
        }
        player.touching = touching;

        // Rain (and storms) only where a rainy level shows; none yet.
        self.weather.tick(&mut self.events, levels.rain_room());
        self.mix = self.mix.approach(
            Mix::target(&self.player, &self.weather, &self.levels()),
            MIX_CHANGE_PER_SECOND * TICK_SECONDS,
        );
        let level = self.levels().level;
        self.ticks += 1;
        let now = self.levels().level;
        if now != level {
            self.events.push(Event::NewLevel { level: now });
        }
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
    fn after_game_over_nothing_moves_and_the_sound_fades_out() {
        let mut w = world_with_plane_ahead(300.0);
        w.lives = 1;
        crash(&mut w);
        for _ in 0..CRASH_TICKS {
            w.tick(&Input::NONE);
        }
        assert!(w.is_over());
        let (ticks, score, player) = (w.ticks, w.score(), w.player);
        let plane = w.planes[0].clone();
        for _ in 0..120 {
            w.tick(&Input::new(-1.0));
            assert_eq!(w.events(), []);
        }
        assert_eq!((w.ticks, w.score(), w.player), (ticks, score, player));
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
    fn the_score_grows_by_two_every_second() {
        let mut w = world_with_plane_ahead(300.0);
        assert_eq!(w.score(), 0);
        let mut last = 0;
        for second in 1..=10 {
            for _ in 0..60 {
                w.tick(&Input::NONE);
                assert!(w.score() >= last, "never goes down");
                last = w.score();
            }
            assert_eq!(w.score(), SCORE_PER_SECOND * second);
        }
        assert_eq!(SCORE_PER_SECOND, 2);
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
    fn the_engine_fades_in_at_the_start_and_cuts_out_in_a_crash() {
        let mut w = world_with_plane_ahead(300.0);
        assert_eq!(w.mix, Mix::SILENT);
        w.tick(&Input::NONE);
        let step = MIX_CHANGE_PER_SECOND * TICK_SECONDS;
        assert!((w.mix.engine_idle - step).abs() < 1e-6, "fades, no jump");
        for _ in 0..60 {
            w.tick(&Input::NONE);
        }
        assert_eq!(w.mix, Mix::target(&w.player, &w.weather, &w.levels()));
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
    fn each_new_level_is_one_event_when_it_begins() {
        let mut w = world();
        w.lives = u32::MAX; // gulls must not end the flight
        let level_ticks = (LEVEL_SECONDS / TICK_SECONDS).round() as u64;
        let seen = new_levels(&mut w, 2.5 * LEVEL_SECONDS);
        // Not at the start, then level 2, then level 1 again.
        assert_eq!(seen, [(level_ticks, 2), (2 * level_ticks, 1)]);
    }

    #[test]
    fn a_flight_started_in_level_2_moves_on_to_level_1() {
        let mut w = world();
        w.start_level = 2;
        w.lives = u32::MAX; // gulls must not end the flight
        assert_eq!(w.levels().level, 2);
        let seen = new_levels(&mut w, 1.5 * LEVEL_SECONDS);
        assert_eq!(seen.iter().map(|&(_, l)| l).collect::<Vec<_>>(), [1]);
    }

    #[test]
    fn the_beach_fades_into_the_mix_with_level_2() {
        let mut w = world();
        w.lives = u32::MAX; // balloons must not end the flight
        new_levels(&mut w, LEVEL_SECONDS - LEVEL_FADE_SECONDS);
        assert_eq!(w.mix.beach, 0.0);
        new_levels(&mut w, LEVEL_FADE_SECONDS + 1.0);
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
    }

    #[test]
    fn balloons_rise_in_level_1_and_none_at_the_beach() {
        let mut w = World::with_seed(PHONE, 3);
        w.lives = u32::MAX;
        let mut burners = 0;
        for _ in 0..((LEVEL_SECONDS - LEVEL_FADE_SECONDS) / TICK_SECONDS) as u32 {
            w.tick(&Input::NONE);
            burners += (w.events().iter())
                .filter(|e| **e == Event::BalloonBurner)
                .count();
        }
        assert!(burners >= 5, "{burners} in level 1");
        // Past the fade into the beach and its banner: no new balloons, and
        // the last ones have drifted away.
        new_levels(&mut w, LEVEL_FADE_SECONDS + 12.0);
        assert_eq!(w.levels().level, BEACH_LEVEL);
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
        let corner = Player::at(Vec2::new(400.0 - 110.0 / 2.0 - 40.0, ridge + 20.0));
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
        new_levels(&mut w, LEVEL_SECONDS + 15.0);
        assert_eq!(w.levels().level, BEACH_LEVEL);
        for _ in 0..(40.0 / TICK_SECONDS) as u32 {
            w.tick(&Input::NONE);
            assert!(w.huts.list.is_empty() && w.balloons.pads.is_empty());
        }
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
                        | Event::BalloonBurner => {}
                        Event::GameOver => panic!("lives cannot run out"),
                    }
                }
            }
        }
    }

    #[test]
    fn full_dive_moves_player_down_by_speed_times_dt() {
        let mut w = world();
        w.tick(&Input::new(1.0));
        let expected = 25.0 + PLAYER_CLIMB_SPEED * w.bounds.scale() * TICK_SECONDS;
        assert!(
            (w.player.pos.y - expected).abs() < 1e-4,
            "y = {}",
            w.player.pos.y
        );
    }

    #[test]
    fn half_climb_moves_player_up_at_half_speed() {
        let mut w = world();
        w.tick(&Input::new(-0.5));
        let expected = 25.0 - 0.5 * PLAYER_CLIMB_SPEED * w.bounds.scale() * TICK_SECONDS;
        assert!(
            (w.player.pos.y - expected).abs() < 1e-4,
            "y = {}",
            w.player.pos.y
        );
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
            let b = Balloon {
                pos: Vec2::new(400.0, y),
                vel: Vec2::new(0.0, 0.0),
                colour: 0,
                way: BalloonWay::AtPlane,
                takeoff: None,
                touchdown: None,
            };
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
        let balloon = |b: Bounds| Balloon {
            pos: Vec2::new(500.0, b.height / 2.0),
            vel: Vec2::new(0.0, 0.0),
            colour: 0,
            way: BalloonWay::AtPlane,
            takeoff: None,
            touchdown: None,
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
