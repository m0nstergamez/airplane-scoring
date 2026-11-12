//! Huts: big wooden beach huts in the foreground grass of level 1, close
//! to the viewer. They stand at the bottom of the screen and come now and
//! then, so they only matter when you fly close to the ground. Fly into
//! one and you crash.
//!
//! They slide past with the landscape and belong to it: their sizes are
//! for a phone and grow with the screen like the background
//! ([`Bounds::scale`]), not with [`OBJECT_SIZE`](crate::OBJECT_SIZE), which
//! shrinks only the plane and what flies. Seeded, so the same seed gives
//! the same huts.

use crate::rng::Rng;
use crate::shape::Poly;
use crate::{Bounds, SCROLL_SPEED, TICK_SECONDS};

/// Where the huts stand: the ground line, as a fraction of the screen
/// height from the top (right at the bottom, in the closest grass).
pub const HUT_GROUND: f32 = 0.98;
/// How wide a hut is, narrowest and widest, in px.
pub const HUT_WIDTH: (f32, f32) = (96.0, 124.0);
/// How tall a hut is from the ground to the roof's ridge, in px.
pub const HUT_HEIGHT: (f32, f32) = (80.0, 104.0);
/// Time between two huts, shortest and longest, in seconds.
pub const HUT_GAP_SECONDS: (f32, f32) = (4.0, 7.0);
/// No hut in the first seconds of level 1.
pub const FIRST_HUT_SECONDS: f32 = 4.0;
/// How many looks (wall and door colours) there are; the game draws them.
pub const HUT_STYLES: u32 = 3;
/// Where the roof meets the walls, as a share of the hut's height from
/// the ridge down.
pub const HUT_EAVES: f32 = 0.4;
/// How far the walls are in from the roof's edges, as a share of the
/// hut's width on each side.
pub const HUT_INSET: f32 = 0.07;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hut {
    /// Centre x on screen, in px.
    pub x: f32,
    /// Width and height in px at [`Bounds::scale`] 1.
    pub width: f32,
    pub height: f32,
    /// Which look, 0 .. [`HUT_STYLES`].
    pub style: u32,
}

impl Hut {
    /// The hut's box on screen: (left, top, right, ground) in px. The top
    /// is the roof's ridge.
    pub fn rect(&self, bounds: Bounds) -> (f32, f32, f32, f32) {
        let scale = bounds.scale();
        let half = self.width * scale / 2.0;
        let ground = HUT_GROUND * bounds.height;
        (
            self.x - half,
            ground - self.height * scale,
            self.x + half,
            ground,
        )
    }

    /// The hut as drawn, for crashing: walls, the gable roof (a
    /// triangle, so the empty corners beside its slopes are free) and the
    /// strip along the eaves. The game crate draws these same parts.
    pub fn shape(&self, bounds: Bounds) -> Vec<Poly> {
        let (l, top, r, ground) = self.rect(bounds);
        let s = bounds.scale();
        let (w, h) = (r - l, ground - top);
        let eaves = top + h * HUT_EAVES;
        let inset = w * HUT_INSET;
        vec![
            Poly::rect(l + inset, eaves, w - 2.0 * inset, ground - eaves),
            Poly(vec![
                crate::Vec2::new(l, eaves),
                crate::Vec2::new(r, eaves),
                crate::Vec2::new((l + r) / 2.0, top),
            ]),
            Poly::rect(l, eaves - 1.5 * s, w, 3.0 * s),
        ]
    }
}

fn ticks(seconds: f32) -> u32 {
    (seconds / TICK_SECONDS).round().max(1.0) as u32
}

#[derive(Debug, Clone, PartialEq)]
pub struct Huts {
    /// The huts on screen right now, oldest (leftmost) first.
    pub list: Vec<Hut>,
    next_ticks: u32,
    rng: Rng,
}

impl Huts {
    pub fn new(seed: u64) -> Self {
        Self {
            list: Vec::new(),
            next_ticks: ticks(FIRST_HUT_SECONDS),
            // Its own stream, so huts do not change the balloons or gulls.
            rng: Rng::new(seed ^ 0x4075_0000_0000_0005),
        }
    }

    /// Advance one tick. `active`: is level 1 fully on screen? Huts keep
    /// sliding off when it is not. `pads`: the x of the launch pads on
    /// screen, so a hut never stands on one.
    pub fn tick(&mut self, bounds: Bounds, active: bool, pads: &[f32]) {
        let size = bounds.size();
        let margin = HUT_WIDTH.1 * bounds.scale();
        for hut in &mut self.list {
            hut.x -= SCROLL_SPEED * bounds.scale() * TICK_SECONDS;
        }
        self.list.retain(|h| h.x > -margin);

        if !active {
            self.next_ticks = ticks(FIRST_HUT_SECONDS);
            return;
        }
        self.next_ticks = self.next_ticks.saturating_sub(1);
        if self.next_ticks > 0 {
            return;
        }
        let x = bounds.width + margin;
        let clear =
            crate::balloons::PAD_WIDTH / 2.0 * size + (HUT_WIDTH.1 / 2.0 + 20.0) * bounds.scale();
        if pads.iter().any(|&px| (px - x).abs() < clear) {
            self.next_ticks = ticks(0.5); // a launch pad is there: a bit later
            return;
        }
        let r = &mut self.rng;
        self.list.push(Hut {
            x,
            width: r.range(HUT_WIDTH.0, HUT_WIDTH.1),
            height: r.range(HUT_HEIGHT.0, HUT_HEIGHT.1),
            style: (r.unit() * HUT_STYLES as f32) as u32 % HUT_STYLES,
        });
        self.next_ticks = ticks(r.range(HUT_GAP_SECONDS.0, HUT_GAP_SECONDS.1));
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

    fn run(h: &mut Huts, seconds: f32, active: bool, pads: &[f32]) {
        for _ in 0..ticks(seconds) {
            h.tick(PHONE, active, pads);
        }
    }

    #[test]
    fn no_huts_outside_level_1() {
        let mut h = Huts::new(1);
        run(&mut h, 120.0, false, &[]);
        assert!(h.list.is_empty());
    }

    #[test]
    fn the_first_hut_comes_after_a_few_seconds_from_the_right() {
        let mut h = Huts::new(1);
        run(&mut h, FIRST_HUT_SECONDS - 0.1, true, &[]);
        assert!(h.list.is_empty());
        run(&mut h, 0.2, true, &[]);
        assert_eq!(h.list.len(), 1);
        assert!(h.list[0].rect(PHONE).0 > PHONE.width, "starts off screen");
    }

    #[test]
    fn huts_slide_with_the_landscape() {
        let mut h = Huts::new(1);
        run(&mut h, FIRST_HUT_SECONDS, true, &[]);
        let x = h.list[0].x;
        run(&mut h, 1.0, true, &[]);
        assert!((x - h.list[0].x - SCROLL_SPEED).abs() < 0.01);
    }

    #[test]
    fn huts_are_low_rare_and_apart() {
        let mut h = Huts::new(2);
        let mut seen = 0;
        for _ in 0..ticks(120.0) {
            h.tick(PHONE, true, &[]);
            for hut in &h.list {
                let (_, top, _, ground) = hut.rect(PHONE);
                assert_eq!(ground, HUT_GROUND * PHONE.height);
                // Big, but only the lowest part of the screen.
                assert!(top > 0.7 * PHONE.height, "{top}");
                assert!(ground - top >= HUT_HEIGHT.0 - 1e-3);
            }
            for pair in h.list.windows(2) {
                let gap = pair[1].x - pair[0].x;
                assert!(gap >= SCROLL_SPEED * HUT_GAP_SECONDS.0 - 1.0, "{gap}");
            }
            seen = seen.max(h.list.len());
        }
        assert!((1..=3).contains(&seen), "{seen}");
    }

    #[test]
    fn a_hut_waits_for_a_launch_pad_to_pass() {
        let mut h = Huts::new(1);
        let pad = PHONE.width + HUT_WIDTH.1 * PHONE.scale();
        run(&mut h, FIRST_HUT_SECONDS + 0.1, true, &[pad]);
        assert!(h.list.is_empty(), "not on the pad");
        run(&mut h, 0.5, true, &[]);
        assert_eq!(h.list.len(), 1);
    }

    /// A tiny 2×2 px probe at (x, y).
    fn probe(x: f32, y: f32) -> Vec<Poly> {
        vec![Poly::rect(x - 1.0, y - 1.0, 2.0, 2.0)]
    }

    #[test]
    fn only_touching_the_hut_itself_is_a_hit() {
        let hut = Hut {
            x: 400.0,
            width: 100.0,
            height: 90.0,
            style: 0,
        };
        let shape = hut.shape(PHONE);
        let (l, top, r, ground) = hut.rect(PHONE);
        let eaves = top + (ground - top) * HUT_EAVES;
        let hit = |x: f32, y: f32| any_overlap(&shape, &probe(x, y));
        // The ridge and the walls.
        assert!(hit(400.0, top + 2.0));
        assert!(hit(l + (r - l) * 0.5, ground - 5.0));
        // Just above the ridge: free.
        assert!(!hit(400.0, top - 2.0));
        // The empty corners beside the roof's slopes: free, although they
        // are inside the hut's box.
        assert!(!hit(l + 3.0, top + 3.0));
        assert!(!hit(r - 3.0, top + 3.0));
        // Beside the walls under the eaves' overhang: free.
        let inset = (r - l) * HUT_INSET;
        assert!(!hit(l + inset / 3.0, eaves + 10.0));
        // Just beside the hut: free.
        assert!(!hit(r + 2.0, ground - 5.0));
    }

    #[test]
    fn same_seed_same_huts() {
        let (mut a, mut b) = (Huts::new(4), Huts::new(4));
        run(&mut a, 60.0, true, &[]);
        run(&mut b, 60.0, true, &[]);
        assert_eq!(a, b);
    }
}
