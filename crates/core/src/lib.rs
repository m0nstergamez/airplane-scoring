//! Pure game logic. No rendering, no input polling, no timers.
//! The game crate drives this with `World::tick` and reads state to draw.

/// Fixed simulation step in seconds. The renderer accumulates frame time
/// and calls `tick` zero or more times per frame.
pub const TICK_SECONDS: f32 = 1.0 / 60.0;

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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct World {
    pub bounds: Bounds,
    pub planes: Vec<Plane>,
    pub ticks: u64,
}

impl World {
    pub fn new(bounds: Bounds) -> Self {
        Self { bounds, planes: Vec::new(), ticks: 0 }
    }

    pub fn spawn_plane(&mut self, name: impl Into<String>, pos: Vec2, vel: Vec2) -> usize {
        self.planes.push(Plane { name: name.into(), pos, vel });
        self.planes.len() - 1
    }

    /// Advance the simulation by exactly one fixed step.
    /// Deterministic: same inputs, same output, every time.
    pub fn tick(&mut self) {
        let Bounds { width, height } = self.bounds;
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
        World::new(Bounds { width: 100.0, height: 50.0 })
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
        w.tick();
        let m = &w.planes[0];
        assert!((m.pos.x - 11.0).abs() < 1e-5);
        assert!((m.pos.y - 8.0).abs() < 1e-5);
        assert_eq!(w.ticks, 1);
    }

    #[test]
    fn plane_bounces_off_right_edge() {
        let mut w = world();
        w.spawn_plane("m", Vec2::new(99.5, 10.0), Vec2::new(60.0, 0.0));
        w.tick(); // would reach 100.5, reflects to 99.5
        let m = &w.planes[0];
        assert!((m.pos.x - 99.5).abs() < 1e-5);
        assert_eq!(m.vel.x, -60.0);
    }

    #[test]
    fn simulation_is_deterministic() {
        let mut a = world();
        let mut b = world();
        for w in [&mut a, &mut b] {
            w.spawn_plane("m", Vec2::new(5.0, 5.0), Vec2::new(37.0, 91.0));
            for _ in 0..1000 {
                w.tick();
            }
        }
        assert_eq!(a, b);
    }

    #[test]
    fn plane_stays_in_bounds_over_time() {
        let mut w = world();
        w.spawn_plane("m", Vec2::new(5.0, 5.0), Vec2::new(300.0, 500.0));
        for _ in 0..10_000 {
            w.tick();
            let m = &w.planes[0];
            assert!(m.pos.x >= 0.0 && m.pos.x <= 100.0, "x out of bounds: {}", m.pos.x);
            assert!(m.pos.y >= 0.0 && m.pos.y <= 50.0, "y out of bounds: {}", m.pos.y);
        }
    }
}
