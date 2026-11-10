//! Thin rendering shell. All game rules live in `airplane-scoring-core`.
//! This file should only: read input, call `World::tick`, draw state.

use airplane_scoring_core::{
    pick_steer, steer_from_keys, steer_toward, Bounds, Input, Vec2, World, TICK_SECONDS,
};
use macroquad::prelude::*;

fn window_conf() -> Conf {
    Conf {
        window_title: "airplane scoring".to_owned(),
        window_width: 800,
        window_height: 450,
        high_dpi: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut world = World::new(Bounds {
        width: screen_width(),
        height: screen_height(),
    });
    world.spawn_plane(
        "plane",
        Vec2::new(screen_width() / 2.0, screen_height() / 2.0),
        Vec2::new(120.0, 90.0),
    );

    let mut accumulator = 0.0_f32;

    loop {
        // Keep the logical bounds in sync with the canvas (browser resizes).
        world.bounds = Bounds {
            width: screen_width(),
            height: screen_height(),
        };

        let pointer_y = pointer_y();
        let keys = steer_from_keys(
            is_key_down(KeyCode::Up) || is_key_down(KeyCode::W),
            is_key_down(KeyCode::Down) || is_key_down(KeyCode::S),
        );

        // Fixed-step simulation; cap to avoid spiral of death after a tab sleeps.
        accumulator += get_frame_time().min(0.25);
        while accumulator >= TICK_SECONDS {
            // Recomputed each step: steering toward a finger depends on where
            // the plane is now.
            let pointer = pointer_y.map(|y| steer_toward(y, world.player.pos.y));
            world.tick(&Input::new(pick_steer(pointer, keys)));
            accumulator -= TICK_SECONDS;
        }

        clear_background(Color::from_rgba(20, 24, 32, 255));

        for m in &world.planes {
            draw_circle(m.pos.x, m.pos.y, 24.0, LIME);
            draw_circle(m.pos.x - 8.0, m.pos.y - 6.0, 4.0, BLACK);
            draw_circle(m.pos.x + 8.0, m.pos.y - 6.0, 4.0, BLACK);
        }

        draw_player(world.player.pos.x, world.player.pos.y);

        let text = world.greeting();
        let size = 48.0;
        let dims = measure_text(&text, None, size as u16, 1.0);
        draw_text(
            &text,
            (screen_width() - dims.width) / 2.0,
            64.0,
            size,
            WHITE,
        );
        draw_text(
            format!("drag or arrow keys to fly  |  ticks: {}", world.ticks),
            12.0,
            screen_height() - 12.0,
            20.0,
            GRAY,
        );

        next_frame().await
    }
}

/// Height of the first finger on screen, or of the mouse while held.
/// (Raw device state only; what it means for steering lives in core.)
fn pointer_y() -> Option<f32> {
    if let Some(t) = touches().first() {
        // macroquad 0.4.16 reports touches in physical pixels but mouse and
        // drawing in logical ones (src/input.rs vs src/lib.rs touch_event).
        return Some(t.position.y / macroquad::miniquad::window::dpi_scale());
    }
    is_mouse_button_down(MouseButton::Left).then(|| mouse_position().1)
}

/// Side view of a small plane flying right, centred on (x, y).
fn draw_player(x: f32, y: f32) {
    let body = Color::from_rgba(240, 244, 250, 255);
    let accent = Color::from_rgba(31, 111, 235, 255);
    // fuselage
    draw_rectangle(x - 26.0, y - 5.0, 44.0, 10.0, body);
    draw_triangle(
        vec2(18.0 + x, y - 5.0),
        vec2(30.0 + x, y),
        vec2(18.0 + x, y + 5.0),
        body,
    );
    // wing and tail fin
    draw_triangle(
        vec2(x - 6.0, y),
        vec2(x + 6.0, y),
        vec2(x - 10.0, y + 16.0),
        accent,
    );
    draw_triangle(
        vec2(x - 26.0, y - 5.0),
        vec2(x - 18.0, y - 5.0),
        vec2(x - 28.0, y - 18.0),
        accent,
    );
    // cockpit
    draw_circle(x + 10.0, y - 3.0, 3.5, accent);
}
