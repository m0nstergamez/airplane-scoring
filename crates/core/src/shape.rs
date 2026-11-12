//! Shapes for crashing: the plane and every obstacle are a few convex
//! polygons that match what is drawn (the game crate draws the same
//! points). A crash needs a real overlap, so a near miss is a miss.
//!
//! Overlap uses the separating axis test: two convex shapes do not touch
//! exactly when some edge direction of one of them separates them.

use crate::Vec2;

/// A convex polygon, corners in order (either direction).
#[derive(Debug, Clone, PartialEq)]
pub struct Poly(pub Vec<Vec2>);

impl Poly {
    /// A polygon from points relative to `at`, each `size` times bigger.
    pub fn at(at: Vec2, size: f32, points: &[(f32, f32)]) -> Poly {
        Poly(
            points
                .iter()
                .map(|&(x, y)| Vec2::new(at.x + x * size, at.y + y * size))
                .collect(),
        )
    }

    /// An upright rectangle from its left top corner and size.
    pub fn rect(left: f32, top: f32, width: f32, height: f32) -> Poly {
        let (r, b) = (left + width, top + height);
        Poly(vec![
            Vec2::new(left, top),
            Vec2::new(r, top),
            Vec2::new(r, b),
            Vec2::new(left, b),
        ])
    }

    /// An ellipse around `centre` with radii `rx`, `ry`, as 16 corners
    /// (all on the ellipse, so never bigger than it).
    pub fn ellipse(centre: Vec2, rx: f32, ry: f32) -> Poly {
        Poly(
            (0..16)
                .map(|i| {
                    let a = i as f32 * std::f32::consts::TAU / 16.0;
                    Vec2::new(centre.x + rx * a.cos(), centre.y + ry * a.sin())
                })
                .collect(),
        )
    }

    /// A line from `a` to `b`, `width` thick, as a thin rectangle.
    pub fn line(a: Vec2, b: Vec2, width: f32) -> Poly {
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let len = (dx * dx + dy * dy).sqrt().max(1e-6);
        let (nx, ny) = (-dy / len * width / 2.0, dx / len * width / 2.0);
        Poly(vec![
            Vec2::new(a.x + nx, a.y + ny),
            Vec2::new(b.x + nx, b.y + ny),
            Vec2::new(b.x - nx, b.y - ny),
            Vec2::new(a.x - nx, a.y - ny),
        ])
    }

    /// Range of the corners along `axis`.
    fn project(&self, axis: Vec2) -> (f32, f32) {
        let mut lo = f32::MAX;
        let mut hi = f32::MIN;
        for p in &self.0 {
            let d = p.x * axis.x + p.y * axis.y;
            lo = lo.min(d);
            hi = hi.max(d);
        }
        (lo, hi)
    }

    fn axes(&self) -> impl Iterator<Item = Vec2> + '_ {
        let n = self.0.len();
        (0..n).map(move |i| {
            let (a, b) = (self.0[i], self.0[(i + 1) % n]);
            Vec2::new(a.y - b.y, b.x - a.x)
        })
    }

    /// Do the two polygons overlap (more than just touching edges)?
    pub fn overlaps(&self, other: &Poly) -> bool {
        if self.0.is_empty() || other.0.is_empty() {
            return false;
        }
        for axis in self.axes().chain(other.axes()) {
            let (a0, a1) = self.project(axis);
            let (b0, b1) = other.project(axis);
            if a1 <= b0 || b1 <= a0 {
                return false;
            }
        }
        true
    }

    /// Does the polygon overlap a circle?
    pub fn overlaps_circle(&self, centre: Vec2, radius: f32) -> bool {
        if self.0.is_empty() {
            return false;
        }
        let circle = |axis: Vec2| {
            let len = (axis.x * axis.x + axis.y * axis.y).sqrt();
            let d = centre.x * axis.x + centre.y * axis.y;
            (d - radius * len, d + radius * len)
        };
        // The edge axes, and the axis towards the nearest corner.
        let nearest = self
            .0
            .iter()
            .min_by(|a, b| {
                let da = (a.x - centre.x).powi(2) + (a.y - centre.y).powi(2);
                let db = (b.x - centre.x).powi(2) + (b.y - centre.y).powi(2);
                da.total_cmp(&db)
            })
            .copied()
            .expect("not empty");
        let to_corner = Vec2::new(nearest.x - centre.x, nearest.y - centre.y);
        for axis in self.axes().chain(std::iter::once(to_corner)) {
            if axis.x == 0.0 && axis.y == 0.0 {
                continue;
            }
            let (a0, a1) = self.project(axis);
            let (b0, b1) = circle(axis);
            if a1 <= b0 || b1 <= a0 {
                return false;
            }
        }
        true
    }
}

/// Does any part of `a` overlap any part of `b`?
pub fn any_overlap(a: &[Poly], b: &[Poly]) -> bool {
    a.iter().any(|p| b.iter().any(|q| p.overlaps(q)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(x: f32, y: f32) -> Poly {
        Poly::rect(x, y, 10.0, 10.0)
    }

    #[test]
    fn overlapping_squares_touch_and_apart_ones_do_not() {
        assert!(square(0.0, 0.0).overlaps(&square(5.0, 5.0)));
        assert!(!square(0.0, 0.0).overlaps(&square(10.5, 0.0)));
        assert!(!square(0.0, 0.0).overlaps(&square(0.0, 10.5)));
    }

    #[test]
    fn a_triangle_roof_misses_what_is_beside_its_slope() {
        // Roof over 0..100, ridge at the top middle.
        let roof = Poly(vec![
            Vec2::new(0.0, 50.0),
            Vec2::new(100.0, 50.0),
            Vec2::new(50.0, 0.0),
        ]);
        // In the empty corner above the eaves, beside the slope: free,
        // although it is inside the roof's bounding box.
        assert!(!roof.overlaps(&Poly::rect(2.0, 2.0, 10.0, 10.0)));
        assert!(!roof.overlaps_circle(Vec2::new(8.0, 8.0), 5.0));
        // On the slope: hit.
        assert!(roof.overlaps(&Poly::rect(40.0, 10.0, 10.0, 10.0)));
        assert!(roof.overlaps_circle(Vec2::new(50.0, 30.0), 5.0));
    }

    #[test]
    fn circles_near_a_corner_miss_and_on_it_hit() {
        let s = square(0.0, 0.0);
        // Diagonal from the corner (10, 10): sqrt(2) * 3 ≈ 4.2 away.
        assert!(!s.overlaps_circle(Vec2::new(13.0, 13.0), 4.0));
        assert!(s.overlaps_circle(Vec2::new(13.0, 13.0), 4.5));
        assert!(s.overlaps_circle(Vec2::new(5.0, 5.0), 1.0), "inside");
    }

    #[test]
    fn a_thin_line_is_only_as_wide_as_drawn() {
        let l = Poly::line(Vec2::new(0.0, 0.0), Vec2::new(20.0, 0.0), 2.0);
        assert!(l.overlaps(&Poly::rect(5.0, 0.5, 2.0, 2.0)));
        assert!(!l.overlaps(&Poly::rect(5.0, 1.5, 2.0, 2.0)));
    }

    #[test]
    fn an_ellipse_stays_inside_its_box() {
        let e = Poly::ellipse(Vec2::new(0.0, 0.0), 10.0, 5.0);
        assert!(
            !e.overlaps(&Poly::rect(8.0, 4.0, 3.0, 3.0)),
            "corner of box"
        );
        assert!(e.overlaps(&Poly::rect(9.0, -1.0, 3.0, 2.0)), "at its side");
    }
}
