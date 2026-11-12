//! Lifeguard towers: wooden towers in the foreground sand of the beach
//! (level 2), close to the viewer like the huts in level 1. They are rare
//! and not tall, so they only matter when you fly low. Fly into one and
//! you crash.
//!
//! They slide past with the landscape and belong to it: their sizes are
//! for a phone and grow with the screen ([`Bounds::scale`]), not with
//! [`OBJECT_SIZE`](crate::OBJECT_SIZE). Seeded, so the same seed gives the
//! same towers.

use crate::rng::Rng;
use crate::shape::Poly;
use crate::{Bounds, Vec2, SCROLL_SPEED, TICK_SECONDS};

/// Where the towers stand: the ground line, as a fraction of the screen
/// height from the top (in the closest sand, like the huts).
pub const TOWER_GROUND: f32 = crate::huts::HUT_GROUND;
/// How wide a tower's cabin and platform are, narrowest and widest, in px.
pub const TOWER_WIDTH: (f32, f32) = (46.0, 56.0);
/// How tall a tower is from the sand to the tip of its roof, in px: not
/// much taller than a hut.
pub const TOWER_HEIGHT: (f32, f32) = (92.0, 112.0);
/// Time between two towers, shortest and longest, in seconds: rare.
pub const TOWER_GAP_SECONDS: (f32, f32) = (6.0, 10.0);
/// No tower in the first seconds of the beach.
pub const FIRST_TOWER_SECONDS: f32 = 4.0;
/// How many looks (colours) there are; the game draws them.
pub const TOWER_STYLES: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tower {
    /// Centre x on screen, in px.
    pub x: f32,
    /// Width and height in px at [`Bounds::scale`] 1.
    pub width: f32,
    pub height: f32,
    /// Which look, 0 .. [`TOWER_STYLES`].
    pub style: u32,
}

/// Where a tower's parts are on screen, in px. Crashes and drawing both
/// use these, so what you see is what you hit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TowerParts {
    /// The two legs, each from the sand to under the platform, and
    /// their thickness.
    pub legs: [(Vec2, Vec2); 2],
    pub leg_width: f32,
    /// The ladder up the right side.
    pub ladder: (Vec2, Vec2),
    /// The platform: left, top, width, height.
    pub platform: (f32, f32, f32, f32),
    /// The cabin on the platform: left, top, width, height.
    pub cabin: (f32, f32, f32, f32),
    /// The roof: left eave, right eave, tip.
    pub roof: [Vec2; 3],
}

impl Tower {
    /// The tower's box on screen: (left, top, right, ground) in px.
    pub fn rect(&self, bounds: Bounds) -> (f32, f32, f32, f32) {
        let s = bounds.scale();
        let half = self.width * s / 2.0;
        let ground = TOWER_GROUND * bounds.height;
        (
            self.x - half,
            ground - self.height * s,
            self.x + half,
            ground,
        )
    }

    /// Where each part is on this screen.
    pub fn parts(&self, bounds: Bounds) -> TowerParts {
        let s = bounds.scale();
        let (w, h) = (self.width * s, self.height * s);
        let (x, g) = (self.x, TOWER_GROUND * bounds.height);
        let top = g - h;
        let deck = g - 0.55 * h; // top of the platform
        let eaves = top + 0.18 * h;
        TowerParts {
            legs: [
                (Vec2::new(x - 0.5 * w, g), Vec2::new(x - 0.32 * w, deck)),
                (Vec2::new(x + 0.5 * w, g), Vec2::new(x + 0.32 * w, deck)),
            ],
            leg_width: 4.0 * s,
            ladder: (Vec2::new(x + 0.62 * w, g), Vec2::new(x + 0.45 * w, deck)),
            platform: (x - 0.5 * w, deck, w, 4.0 * s),
            cabin: (x - 0.38 * w, eaves, 0.76 * w, deck - eaves),
            roof: [
                Vec2::new(x - 0.5 * w, eaves),
                Vec2::new(x + 0.5 * w, eaves),
                Vec2::new(x, top),
            ],
        }
    }

    /// The tower as drawn, for crashing: legs, ladder, platform, cabin,
    /// roof. The gaps between the legs and beside the roof are free.
    pub fn shape(&self, bounds: Bounds) -> Vec<Poly> {
        let p = self.parts(bounds);
        let rect = |(l, t, w, h): (f32, f32, f32, f32)| Poly::rect(l, t, w, h);
        vec![
            Poly::line(p.legs[0].0, p.legs[0].1, p.leg_width),
            Poly::line(p.legs[1].0, p.legs[1].1, p.leg_width),
            Poly::line(p.ladder.0, p.ladder.1, p.leg_width * 0.75),
            rect(p.platform),
            rect(p.cabin),
            Poly(p.roof.to_vec()),
        ]
    }
}

fn ticks(seconds: f32) -> u32 {
    (seconds / TICK_SECONDS).round().max(1.0) as u32
}

#[derive(Debug, Clone, PartialEq)]
pub struct Towers {
    /// The towers on screen right now, oldest (leftmost) first.
    pub list: Vec<Tower>,
    next_ticks: u32,
    rng: Rng,
}

impl Towers {
    pub fn new(seed: u64) -> Self {
        Self {
            list: Vec::new(),
            next_ticks: ticks(FIRST_TOWER_SECONDS),
            // Its own stream, so towers do not change the gulls.
            rng: Rng::new(seed ^ 0x7031_E125_0000_0006),
        }
    }

    /// Advance one tick. `active`: is the beach fully on screen? Towers
    /// keep sliding off when it is not.
    pub fn tick(&mut self, bounds: Bounds, active: bool) {
        let s = bounds.scale();
        let margin = TOWER_WIDTH.1 * s;
        for t in &mut self.list {
            t.x -= SCROLL_SPEED * s * TICK_SECONDS;
        }
        self.list.retain(|t| t.x > -margin);

        if !active {
            self.next_ticks = ticks(FIRST_TOWER_SECONDS);
            return;
        }
        self.next_ticks = self.next_ticks.saturating_sub(1);
        if self.next_ticks > 0 {
            return;
        }
        let r = &mut self.rng;
        self.list.push(Tower {
            x: bounds.width + margin,
            width: r.range(TOWER_WIDTH.0, TOWER_WIDTH.1),
            height: r.range(TOWER_HEIGHT.0, TOWER_HEIGHT.1),
            style: (r.unit() * TOWER_STYLES as f32) as u32 % TOWER_STYLES,
        });
        self.next_ticks = ticks(r.range(TOWER_GAP_SECONDS.0, TOWER_GAP_SECONDS.1));
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

    fn run(t: &mut Towers, seconds: f32, active: bool) {
        for _ in 0..ticks(seconds) {
            t.tick(PHONE, active);
        }
    }

    #[test]
    fn no_towers_away_from_the_beach() {
        let mut t = Towers::new(1);
        run(&mut t, 120.0, false);
        assert!(t.list.is_empty());
    }

    #[test]
    fn the_first_tower_comes_after_a_few_seconds_from_the_right() {
        let mut t = Towers::new(1);
        run(&mut t, FIRST_TOWER_SECONDS - 0.1, true);
        assert!(t.list.is_empty());
        run(&mut t, 0.2, true);
        assert_eq!(t.list.len(), 1);
        assert!(t.list[0].rect(PHONE).0 > PHONE.width, "starts off screen");
    }

    #[test]
    fn towers_are_rare_not_tall_and_slide_with_the_landscape() {
        let mut t = Towers::new(2);
        let mut most = 0;
        let mut came = 0;
        let mut last_x = f32::MIN;
        for _ in 0..ticks(120.0) {
            t.tick(PHONE, true);
            for tower in &t.list {
                let (_, top, _, ground) = tower.rect(PHONE);
                assert_eq!(ground, TOWER_GROUND * PHONE.height);
                assert!(top > 0.68 * PHONE.height, "not tall: {top}");
            }
            for pair in t.list.windows(2) {
                let gap = pair[1].x - pair[0].x;
                assert!(gap >= SCROLL_SPEED * TOWER_GAP_SECONDS.0 - 1.0, "{gap}");
            }
            if let Some(n) = t.list.last() {
                if n.x > last_x + 1.0 {
                    came += 1;
                }
                last_x = n.x;
            }
            most = most.max(t.list.len());
        }
        // One every 6–10 s over two minutes (the first after 4 s).
        assert!((11..=20).contains(&came), "{came}");
        assert!(most <= 2, "{most} at once");
    }

    #[test]
    fn only_touching_the_tower_itself_is_a_hit() {
        let tower = Tower {
            x: 400.0,
            width: 50.0,
            height: 100.0,
            style: 0,
        };
        let shape = tower.shape(PHONE);
        let parts = tower.parts(PHONE);
        let probe = |x: f32, y: f32| vec![Poly::rect(x - 1.0, y - 1.0, 2.0, 2.0)];
        let hit = |x: f32, y: f32| any_overlap(&shape, &probe(x, y));
        let (_, top, _, ground) = tower.rect(PHONE);
        assert!(hit(400.0, top + 3.0), "roof");
        assert!(!hit(400.0, top - 2.0), "above the roof");
        let (_, deck, _, _) = parts.platform;
        assert!(hit(400.0, deck + 1.0), "platform");
        assert!(!hit(400.0, (deck + ground) / 2.0), "between the legs");
        assert!(!hit(400.0 - 22.0, top + 2.0), "beside the roof's slope");
        let leg = parts.legs[0].0;
        assert!(hit(leg.x + 0.5, leg.y - 1.5), "a leg");
    }

    #[test]
    fn same_seed_same_towers() {
        let (mut a, mut b) = (Towers::new(4), Towers::new(4));
        run(&mut a, 60.0, true);
        run(&mut b, 60.0, true);
        assert_eq!(a, b);
    }
}
