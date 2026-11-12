//! Lifeguard towers: the obstacles of level 1. They stand on the grass in
//! front of the coast and slide past with the landscape, so the plane has
//! to fly over them. Fly into one and you crash.
//!
//! Sizes follow the screen height (like the background, which is scaled
//! to it), so a tower looks and plays the same on a phone and a computer
//! and stays right when the window is resized. Seeded, so the same seed
//! gives the same towers.

use crate::rng::Rng;
use crate::{Bounds, Vec2, SCROLL_SPEED, TICK_SECONDS};

/// Where the towers stand, as a fraction of the screen height from the
/// top (in the grass of level 1's picture, which starts at 0.72).
pub const TOWER_GROUND: f32 = 0.9;
/// How tall a tower is, shortest and tallest, as a fraction of the screen
/// height. The tallest still leaves 40 % of the sky to fly through.
pub const TOWER_HEIGHT: (f32, f32) = (0.25, 0.5);
/// How wide a tower's cabin is, as a fraction of the screen height.
pub const TOWER_WIDTH: f32 = 0.085;
/// Time between two towers, shortest and longest, in seconds.
pub const TOWER_GAP_SECONDS: (f32, f32) = (2.2, 4.5);
/// No tower in the first seconds of level 1, while "Level 1" shows.
pub const FIRST_TOWER_SECONDS: f32 = 2.5;

/// Towers start and are dropped this far outside the screen, in tower
/// widths.
const MARGIN_WIDTHS: f32 = 1.5;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tower {
    /// Centre x on screen, in px.
    pub x: f32,
    /// Height as a fraction of the screen height (see [`TOWER_HEIGHT`]).
    pub height: f32,
}

impl Tower {
    /// The tower's box on screen: (left, top, right, bottom) in px.
    /// Bottom is the ground; top is the tip of the roof.
    pub fn rect(&self, bounds: Bounds) -> (f32, f32, f32, f32) {
        let h = bounds.height;
        let half = TOWER_WIDTH * h / 2.0;
        let ground = TOWER_GROUND * h;
        (
            self.x - half,
            ground - self.height * h,
            self.x + half,
            ground,
        )
    }

    /// Does a circle (the player's plane) touch the tower? Below the
    /// ground counts too, down to the screen's edge: the plane cannot slip
    /// under a tower through the grass in front of it.
    pub fn touches(&self, centre: Vec2, radius: f32, bounds: Bounds) -> bool {
        let (l, t, r, _) = self.rect(bounds);
        let b = bounds.height;
        let nx = centre.x.clamp(l, r);
        let ny = centre.y.clamp(t, b);
        let (dx, dy) = (centre.x - nx, centre.y - ny);
        dx * dx + dy * dy < radius * radius
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
            // Its own stream, so towers do not change the weather or gulls.
            rng: Rng::new(seed ^ 0x7031_E125_0000_0003),
        }
    }

    /// Advance one tick. `active`: is level 1 fully on screen? Towers
    /// already standing keep sliding off when it is not.
    pub fn tick(&mut self, bounds: Bounds, active: bool) {
        let margin = MARGIN_WIDTHS * TOWER_WIDTH * bounds.height;
        for t in &mut self.list {
            t.x -= SCROLL_SPEED * TICK_SECONDS;
        }
        self.list.retain(|t| t.x > -margin);

        if !active {
            self.next_ticks = ticks(FIRST_TOWER_SECONDS);
            return;
        }
        self.next_ticks -= 1;
        if self.next_ticks == 0 {
            let height = self.rng.range(TOWER_HEIGHT.0, TOWER_HEIGHT.1);
            self.list.push(Tower {
                x: bounds.width + margin,
                height,
            });
            self.next_ticks = ticks(self.rng.range(TOWER_GAP_SECONDS.0, TOWER_GAP_SECONDS.1));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PLAYER_HALF_HEIGHT;

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
    fn no_towers_outside_level_1() {
        let mut t = Towers::new(1);
        run(&mut t, 120.0, false);
        assert!(t.list.is_empty());
    }

    #[test]
    fn the_first_tower_waits_for_the_level_banner_and_comes_from_the_right() {
        let mut t = Towers::new(1);
        run(&mut t, FIRST_TOWER_SECONDS - 0.1, true);
        assert!(t.list.is_empty());
        run(&mut t, 0.2, true);
        assert_eq!(t.list.len(), 1);
        assert!(t.list[0].rect(PHONE).0 > PHONE.width, "starts off screen");
    }

    #[test]
    fn towers_slide_with_the_landscape() {
        let mut t = Towers::new(1);
        run(&mut t, FIRST_TOWER_SECONDS, true);
        let x = t.list[0].x;
        run(&mut t, 1.0, true);
        assert!((x - t.list[0].x - SCROLL_SPEED).abs() < 0.01);
    }

    #[test]
    fn towers_stand_on_the_ground_and_leave_room_to_fly_over() {
        let mut t = Towers::new(2);
        let mut seen = 0;
        for _ in 0..ticks(120.0) {
            t.tick(PHONE, true);
            for tower in &t.list {
                let (_, top, _, bottom) = tower.rect(PHONE);
                assert_eq!(bottom, TOWER_GROUND * PHONE.height);
                assert!(top >= PHONE.height * (TOWER_GROUND - TOWER_HEIGHT.1) - 1e-3);
                // The plane fits above even the tallest tower.
                assert!(top > 4.0 * PLAYER_HALF_HEIGHT, "{top}");
                seen += 1;
            }
        }
        assert!(seen > 0);
    }

    #[test]
    fn towers_keep_their_distance_and_are_dropped_off_screen() {
        let mut t = Towers::new(3);
        let mut most = 0;
        for _ in 0..ticks(120.0) {
            t.tick(PHONE, true);
            for pair in t.list.windows(2) {
                let gap = pair[1].x - pair[0].x;
                assert!(gap >= SCROLL_SPEED * TOWER_GAP_SECONDS.0 - 1.0, "{gap}");
            }
            most = most.max(t.list.len());
        }
        // Screen plus margins / shortest gap: never more than a handful.
        assert!(most <= 6, "{most}");
        run(&mut t, 20.0, false);
        assert!(t.list.is_empty(), "all slid off");
    }

    #[test]
    fn touching_means_overlapping_the_box() {
        let tower = Tower {
            x: 400.0,
            height: 0.4,
        };
        let (l, top, r, _) = tower.rect(PHONE);
        let r16 = 16.0;
        // Right above the roof: free. Touching the roof: hit.
        assert!(!tower.touches(Vec2::new(400.0, top - r16 - 1.0), r16, PHONE));
        assert!(tower.touches(Vec2::new(400.0, top - r16 + 1.0), r16, PHONE));
        // Beside the legs: free; into the side: hit.
        let mid = top + 40.0;
        assert!(!tower.touches(Vec2::new(l - r16 - 1.0, mid), r16, PHONE));
        assert!(tower.touches(Vec2::new(r + r16 - 1.0, mid), r16, PHONE));
        // No slipping under it through the grass.
        let lowest = PHONE.height - PLAYER_HALF_HEIGHT;
        assert!(tower.touches(Vec2::new(400.0, lowest), r16, PHONE));
    }

    #[test]
    fn same_seed_same_towers() {
        let (mut a, mut b) = (Towers::new(4), Towers::new(4));
        run(&mut a, 60.0, true);
        run(&mut b, 60.0, true);
        assert_eq!(a, b);
    }
}
