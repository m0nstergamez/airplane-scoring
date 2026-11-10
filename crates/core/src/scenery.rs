//! The scrolling background. The plane stays at a fixed x, so the world
//! moves past it: the background slides left at [`SCROLL_SPEED`] and its
//! image repeats side by side (the image's left and right edges match).

use crate::TICK_SECONDS;

/// How fast the background slides past, in screen px per second.
pub const SCROLL_SPEED: f32 = 90.0;

/// How far the plane has flown after `ticks` ticks, in screen px.
/// Computed from the tick count, so it needs no state and stays
/// deterministic.
pub fn scroll_distance(ticks: u64) -> f64 {
    ticks as f64 * (SCROLL_SPEED as f64 * TICK_SECONDS as f64)
}

/// Where the background image starts, in screen px: draw one copy at the
/// returned x (0 or less) and more copies every `tile_width` to the right
/// until the screen is covered. The value is in `(-tile_width, 0]`.
pub fn background_x(distance: f64, tile_width: f32) -> f32 {
    if tile_width <= 0.0 {
        return 0.0; // image not loaded or zero size; never divide by zero
    }
    -(distance.rem_euclid(tile_width as f64) as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_grows_with_scroll_speed() {
        assert_eq!(scroll_distance(0), 0.0);
        // One second of ticks moves the background by SCROLL_SPEED.
        assert!((scroll_distance(60) - SCROLL_SPEED as f64).abs() < 1e-3);
    }

    #[test]
    fn background_starts_at_the_left_edge() {
        assert_eq!(background_x(0.0, 1000.0), 0.0);
    }

    #[test]
    fn background_slides_left() {
        assert_eq!(background_x(30.0, 1000.0), -30.0);
    }

    #[test]
    fn background_wraps_after_one_image_width() {
        assert_eq!(background_x(1000.0, 1000.0), 0.0);
        assert_eq!(background_x(1250.0, 1000.0), -250.0);
    }

    #[test]
    fn background_x_stays_in_range_after_a_long_flight() {
        let ten_hours = scroll_distance(60 * 60 * 60 * 10);
        let x = background_x(ten_hours, 1733.0);
        assert!(x <= 0.0 && x > -1733.0, "{x}");
    }

    #[test]
    fn missing_image_does_not_panic() {
        assert_eq!(background_x(500.0, 0.0), 0.0);
        assert_eq!(background_x(500.0, -1.0), 0.0);
    }
}
