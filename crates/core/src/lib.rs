//! Pure game logic. No rendering, no input polling, no timers.
//! The game crate drives this with `World::tick` and reads state to draw.

/// Fixed simulation step in seconds. The renderer accumulates frame time
/// and calls `tick` zero or more times per frame.
pub const TICK_SECONDS: f32 = 1.0 / 60.0;

mod clock;
mod gulls;
mod input;
mod level;
mod rng;
mod scenery;
mod sound;
mod tilt;
mod towers;
mod weather;
pub use clock::{Clock, MAX_FRAME_SECONDS};
pub use gulls::{
    Gull, GullStart, Gulls, FIRST_GULL_SECONDS, GULL_BOB, GULL_CALL_CHANCE, GULL_FROM_LEFT,
    GULL_FROM_RIGHT, GULL_GAP_SECONDS, GULL_RADIUS, GULL_SPEED,
};
pub use input::{pick_steer, steer_from_keys, steer_toward, Input, TOUCH_EASE_DISTANCE};
pub use level::{
    level_view, start_level, LevelView, LEVEL_BANNER_SECONDS, LEVEL_COUNT, LEVEL_FADE_SECONDS,
    LEVEL_SECONDS,
};
pub use scenery::{background_x, scroll_distance, SCROLL_SPEED};
pub use sound::{
    Mix, BEACH_LEVEL, BEACH_VOLUME, CRASH_VOLUME, DIVE_WIND, ENGINE_VOLUME, GULL_CALL_VOLUME,
    MIX_CHANGE_PER_SECOND, NEW_LEVEL_VOLUME, RAIN_VOLUME, WIND_VOLUME,
};
pub use tilt::{Tilt, TILT_DEAD_ZONE_DEG, TILT_FULL_DEG, TILT_MIN_GRAVITY, TILT_SMOOTHING};
pub use towers::{
    Tower, Towers, FIRST_TOWER_SECONDS, TOWER_GAP_SECONDS, TOWER_GROUND, TOWER_HEIGHT, TOWER_WIDTH,
};
pub use weather::{
    Weather, CALM_WIND, DRY_SECONDS, FIRST_DRY_SECONDS, FLASH_SECONDS, GUST_WIND,
    RAIN_CHANGE_PER_SECOND, RAIN_PEAK, RAIN_SECONDS, RAIN_WIND, STORM_RAIN, STRIKE_GAP_SECONDS,
    THUNDER_DELAY_SECONDS, THUNDER_VOLUME,
};

/// Vertical speed of the player plane at full steer, in px per second.
pub const PLAYER_CLIMB_SPEED: f32 = 300.0;

/// Half the player plane's drawn height; keeps the whole plane on screen.
pub const PLAYER_HALF_HEIGHT: f32 = 18.0;

/// The player's plane sits at this fraction of the screen width.
pub const PLAYER_X_FRACTION: f32 = 0.2;

/// Radius of the other planes, in px.
pub const PLANE_RADIUS: f32 = 24.0;

/// The player's plane crashes when another plane's centre comes closer
/// than this plus [`PLANE_RADIUS`].
pub const PLAYER_HIT_RADIUS: f32 = 16.0;

/// After a crash the plane is out of control for this long.
pub const CRASH_SECONDS: f32 = 1.5;

/// [`CRASH_SECONDS`] in ticks.
pub const CRASH_TICKS: u32 = (CRASH_SECONDS * 60.0) as u32;

/// A crashed plane falls this fast, in px per second.
pub const CRASH_FALL_SPEED: f32 = 120.0;

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
            touching: false,
        }
    }

    /// Out of control after hitting another plane.
    pub fn is_crashed(&self) -> bool {
        self.crash_ticks > 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct World {
    pub bounds: Bounds,
    pub player: Player,
    pub planes: Vec<Plane>,
    /// The obstacles at the beach.
    pub gulls: Gulls,
    /// The obstacles of level 1.
    pub towers: Towers,
    pub weather: Weather,
    /// Volumes of the looping sounds, eased toward what fits the world now.
    pub mix: Mix,
    pub ticks: u64,
    /// The level the flight started in (1 ..= `LEVEL_COUNT`).
    pub start_level: u32,
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
            gulls: Gulls::new(seed),
            towers: Towers::new(seed),
            weather: Weather::new(seed),
            mix: Mix::SILENT,
            ticks: 0,
            start_level: 1,
            events: Vec::new(),
        }
    }

    /// Which level is on screen now, and how far the next has faded in.
    pub fn levels(&self) -> LevelView {
        level_view(self.ticks, self.start_level)
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
        self.events.clear();

        // A crashed plane ignores the controls and falls until it recovers.
        let fall = if self.player.is_crashed() {
            self.player.crash_ticks -= 1;
            self.player.steer = 0.0;
            CRASH_FALL_SPEED
        } else {
            self.player.steer = input.steer();
            input.steer() * PLAYER_CLIMB_SPEED
        };
        let p = &mut self.player.pos;
        let y = p.y + fall * TICK_SECONDS;
        p.y = if height > 2.0 * PLAYER_HALF_HEIGHT {
            y.clamp(PLAYER_HALF_HEIGHT, height - PLAYER_HALF_HEIGHT)
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

        // Gulls fly only while the beach is fully on screen.
        let levels = self.levels();
        let beach = levels.level == BEACH_LEVEL && levels.fade == 0.0;
        self.gulls.tick(self.bounds, beach, &mut self.events);
        // Lifeguard towers stand only while level 1 is fully on screen.
        let coast = levels.level == 1 && levels.fade == 0.0;
        self.towers.tick(self.bounds, coast);

        // Crash: the player's plane newly touches another plane, a gull
        // or a tower.
        let bounds = self.bounds;
        let player = &mut self.player;
        let near = |pos: Vec2, radius: f32| {
            let reach = radius + PLAYER_HIT_RADIUS;
            let (dx, dy) = (pos.x - player.pos.x, pos.y - player.pos.y);
            dx * dx + dy * dy < reach * reach
        };
        let touching = self.planes.iter().any(|m| near(m.pos, PLANE_RADIUS))
            || self.gulls.list.iter().any(|g| near(g.pos, GULL_RADIUS))
            || (self.towers.list.iter()).any(|t| t.touches(player.pos, PLAYER_HIT_RADIUS, bounds));
        if touching && !player.touching && !player.is_crashed() {
            player.crash_ticks = CRASH_TICKS;
            player.steer = 0.0;
            self.events.push(Event::Crash);
        }
        player.touching = touching;

        // The beach is always dry: rain only where level 1 shows.
        let max_rain = 1.0 - levels.presence(BEACH_LEVEL);
        self.weather.tick(&mut self.events, max_rain);
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

    /// A big world with a plane parked `gap` px to the right of the player.
    fn world_with_plane_ahead(gap: f32) -> World {
        let mut w = World::new(Bounds {
            width: 800.0,
            height: 400.0,
        });
        let p = w.player.pos;
        w.spawn_plane("m", Vec2::new(p.x + gap, p.y), Vec2::new(0.0, 0.0));
        w
    }

    #[test]
    fn touching_another_plane_is_a_crash() {
        let mut w = world_with_plane_ahead(PLANE_RADIUS + PLAYER_HIT_RADIUS - 1.0);
        w.tick(&Input::NONE);
        assert_eq!(w.events(), [Event::Crash]);
        assert!(w.player.is_crashed());
        assert_eq!(w.player.crash_ticks, CRASH_TICKS);
    }

    #[test]
    fn a_near_miss_is_not_a_crash() {
        let mut w = world_with_plane_ahead(PLANE_RADIUS + PLAYER_HIT_RADIUS + 1.0);
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
        for _ in 0..=CRASH_TICKS {
            w.tick(&Input::NONE);
        }
        assert!(!w.player.is_crashed());
        w.planes[0].pos = w.player.pos;
        w.tick(&Input::NONE);
        assert_eq!(w.events(), [Event::Crash]);
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
        let level_ticks = (LEVEL_SECONDS / TICK_SECONDS).round() as u64;
        let seen = new_levels(&mut w, 2.5 * LEVEL_SECONDS);
        // Not at the start, then level 2, then level 1 again.
        assert_eq!(seen, [(level_ticks, 2), (2 * level_ticks, 1)]);
    }

    #[test]
    fn a_flight_started_in_level_2_moves_on_to_level_1() {
        let mut w = world();
        w.start_level = 2;
        assert_eq!(w.levels().level, 2);
        let seen = new_levels(&mut w, 1.5 * LEVEL_SECONDS);
        assert_eq!(seen.iter().map(|&(_, l)| l).collect::<Vec<_>>(), [1]);
    }

    #[test]
    fn the_beach_fades_into_the_mix_with_level_2() {
        let mut w = world();
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
    fn the_beach_is_dry_and_has_no_thunderstorms() {
        for seed in 0..8 {
            let mut w = World::with_seed(PHONE, seed);
            w.start_level = BEACH_LEVEL;
            // Until level 1 starts to fade back in.
            for _ in 0..((LEVEL_SECONDS - LEVEL_FADE_SECONDS) / TICK_SECONDS) as u32 {
                w.tick(&Input::NONE);
                assert_eq!(w.weather.rain, 0.0, "seed {seed}");
                for e in w.events() {
                    assert!(
                        !matches!(e, Event::Lightning { .. } | Event::Thunder { .. }),
                        "seed {seed}: {e:?}"
                    );
                }
            }
        }
    }

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
    fn flying_into_a_lifeguard_tower_is_a_crash_and_flying_over_is_not() {
        let mut w = World::with_seed(PHONE, 1);
        // Low and slow: steer to the bottom and wait for the first tower.
        let down = Input::new(1.0);
        while !(w.towers.list.iter()).any(|t| t.rect(w.bounds).0 < w.player.pos.x + 40.0) {
            w.tick(&down);
        }
        let mut crashed = false;
        for _ in 0..120 {
            w.tick(&down);
            crashed |= w.events().contains(&Event::Crash);
        }
        assert!(crashed, "a low plane hits the tower");

        // The same flight high up passes every tower.
        let mut w = World::with_seed(PHONE, 1);
        let up = Input::new(-1.0);
        for _ in 0..(50.0 / TICK_SECONDS) as u32 {
            w.tick(&up);
            assert!(!w.events().contains(&Event::Crash));
        }
        assert!(!w.towers.list.is_empty());
    }

    #[test]
    fn no_towers_at_the_beach() {
        let mut w = World::with_seed(PHONE, 1);
        w.start_level = BEACH_LEVEL;
        new_levels(&mut w, LEVEL_SECONDS - LEVEL_FADE_SECONDS);
        assert!(w.towers.list.is_empty());
    }

    #[test]
    fn no_gulls_in_level_1() {
        let mut w = world();
        new_levels(&mut w, LEVEL_SECONDS - 0.1);
        assert!(w.gulls.list.is_empty());
    }

    #[test]
    fn thunderstorms_reach_the_world_events_and_the_rain_mix() {
        let mut w = World::with_seed(
            Bounds {
                width: 800.0,
                height: 400.0,
            },
            1,
        );
        let (mut lightning, mut thunder, mut loudest_rain) = (0, 0, 0.0_f32);
        for _ in 0..5 * 3600 {
            w.tick(&Input::NONE);
            for e in w.events() {
                match e {
                    Event::Lightning { .. } => lightning += 1,
                    Event::Thunder { .. } => thunder += 1,
                    // 5 minutes: both levels; at the beach gulls can hit the plane.
                    Event::Crash | Event::NewLevel { .. } | Event::GullCall => {}
                }
            }
            loudest_rain = loudest_rain.max(w.mix.rain);
        }
        assert!(lightning > 0 && thunder > 0, "{lightning} {thunder}");
        assert!(loudest_rain >= RAIN_VOLUME * STORM_RAIN);
    }

    #[test]
    fn full_dive_moves_player_down_by_speed_times_dt() {
        let mut w = world();
        w.tick(&Input::new(1.0));
        let expected = 25.0 + PLAYER_CLIMB_SPEED * TICK_SECONDS;
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
        let expected = 25.0 - 0.5 * PLAYER_CLIMB_SPEED * TICK_SECONDS;
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
            w.tick(&Input::new(-1.0));
        }
        assert_eq!(w.player.pos.y, PLAYER_HALF_HEIGHT);
        for _ in 0..600 {
            w.tick(&Input::new(1.0));
        }
        assert_eq!(w.player.pos.y, 50.0 - PLAYER_HALF_HEIGHT);
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
            w.towers.list.clear(); // about steering, not obstacles
            let steer = steer_toward(target, w.player.pos.y);
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
            height: 40.0,
        }; // e.g. phone rotated
        w.tick(&Input::NONE);
        assert_eq!(
            w.player.pos,
            Vec2::new(400.0 * PLAYER_X_FRACTION, 40.0 - PLAYER_HALF_HEIGHT)
        );
    }

    #[test]
    fn screen_shorter_than_the_plane_centres_it_instead_of_crashing() {
        let mut w = World::new(Bounds {
            width: 100.0,
            height: 10.0,
        });
        w.tick(&Input::new(1.0));
        assert_eq!(w.player.pos.y, 5.0);
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
            for i in 0..10_000 {
                w.tick(&Input::new(((i % 7) as f32 - 3.0) / 3.0));
                log.extend_from_slice(w.events());
            }
        }
        assert_eq!(a, b);
        assert_eq!(logs[0], logs[1]);
        // The run is long enough to cover crashes and a thunderstorm.
        assert!(logs[0].contains(&Event::Crash));
        assert!(logs[0].iter().any(|e| matches!(e, Event::Thunder { .. })));
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
