//! Thin rendering shell. All game rules live in `airplane-scoring-core`.
//! This file should only: read input, call `World::tick`, draw state.

use airplane_scoring_core::{Bounds, Vec2, World, TICK_SECONDS};
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

        // Fixed-step simulation; cap to avoid spiral of death after a tab sleeps.
        accumulator += get_frame_time().min(0.25);
        while accumulator >= TICK_SECONDS {
            world.tick();
            accumulator -= TICK_SECONDS;
        }

        clear_background(Color::from_rgba(20, 24, 32, 255));

        for m in &world.planes {
            draw_circle(m.pos.x, m.pos.y, 24.0, LIME);
            draw_circle(m.pos.x - 8.0, m.pos.y - 6.0, 4.0, BLACK);
            draw_circle(m.pos.x + 8.0, m.pos.y - 6.0, 4.0, BLACK);
        }

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
            format!("ticks: {}", world.ticks),
            12.0,
            screen_height() - 12.0,
            20.0,
            GRAY,
        );

        next_frame().await
    }
}
