//! Pure game logic. No rendering, no input polling, no timers.
//! The game crate drives this with `World::tick` and reads state to draw.

/// Fixed simulation step in seconds. The renderer accumulates frame time
/// and calls `tick` zero or more times per frame.
pub const TICK_SECONDS: f32 = 1.0 / 60.0;

mod input;
pub use input::{pick_steer, steer_from_keys, steer_toward, Input, TOUCH_EASE_DISTANCE};

/// Vertical speed of the player plane at full steer, in px per second.
pub const PLAYER_CLIMB_SPEED: f32 = 300.0;

/// Half the player plane's drawn height; keeps the whole plane on screen.
pub const PLAYER_HALF_HEIGHT: f32 = 18.0;

/// The player's plane sits at this fraction of the screen width.
pub const PLAYER_X_FRACTION: f32 = 0.2;

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
    pub ticks: u64,
}

impl World {
    pub fn new(bounds: Bounds) -> Self {
        let player = Player {
            pos: Vec2::new(bounds.width * PLAYER_X_FRACTION, bounds.height / 2.0),
        };
        Self {
            bounds,
            player,
            planes: Vec::new(),
            ticks: 0,
        }
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

        let p = &mut self.player.pos;
        let y = p.y + input.steer() * PLAYER_CLIMB_SPEED * TICK_SECONDS;
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
        self.ticks += 1;
    }

    /// Greeting shown on screen. Lives in core so it is unit-tested.
    pub fn greeting(&self) -> String {
        match self.planes.first() {
            Some(m) => format!("hello {}", m.name),
            None => "hello nobody".to_string(),
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
    fn greeting_names_first_plane() {
        let mut w = world();
        assert_eq!(w.greeting(), "hello nobody");
        w.spawn_plane("plane", Vec2::new(10.0, 10.0), Vec2::new(0.0, 0.0));
        assert_eq!(w.greeting(), "hello plane");
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
        let mut a = world();
        let mut b = world();
        for w in [&mut a, &mut b] {
            w.spawn_plane("m", Vec2::new(5.0, 5.0), Vec2::new(37.0, 91.0));
            for i in 0..1000 {
                w.tick(&Input::new(((i % 7) as f32 - 3.0) / 3.0));
            }
        }
        assert_eq!(a, b);
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
