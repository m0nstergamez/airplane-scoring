//! Thin rendering shell. All game rules live in `airplane-scoring-core`.
//! This file should only: read input, call `World::tick`, draw state.

use airplane_scoring_core::{
    background_x, pick_steer, scroll_distance, steer_from_keys, steer_toward, Bounds, Event, Input,
    Tilt, Vec2, World, PLANE_RADIUS, TICK_SECONDS,
};
use macroquad::prelude::*;

mod audio;
mod scenery;
mod sound_js;
mod tilt_js;

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
    // Native: find `sounds/` and `levels/` where the web page does, next
    // to index.html.
    #[cfg(not(target_arch = "wasm32"))]
    set_pc_assets_folder("web");
    let mut audio = audio::Audio::start();
    let mut scenery = scenery::Scenery::start();

    // The seed decides the weather; the clock makes every flight different.
    let seed = (macroquad::miniquad::date::now() * 1000.0) as u64;
    let mut world = World::with_seed(
        Bounds {
            width: screen_width(),
            height: screen_height(),
        },
        seed,
    );
    world.spawn_plane(
        "plane",
        Vec2::new(screen_width() / 2.0, screen_height() / 2.0),
        Vec2::new(120.0, 90.0),
    );

    let mut accumulator = 0.0_f32;
    let mut tilt = Tilt::default();
    let mut tilt_epoch = tilt_js::epoch();
    let mut crash_flash = 0.0_f32; // screen effect only, fades out

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
        if tilt_js::epoch() != tilt_epoch {
            tilt_epoch = tilt_js::epoch();
            tilt.recalibrate();
        }
        let gravity = tilt_js::sample();
        if is_key_pressed(KeyCode::M) {
            sound_js::toggle_mute();
        }

        // Fixed-step simulation; cap to avoid spiral of death after a tab sleeps.
        accumulator += get_frame_time().min(0.25);
        while accumulator >= TICK_SECONDS {
            // Recomputed each step: steering toward a finger depends on where
            // the plane is now.
            let pointer = pointer_y.map(|y| steer_toward(y, world.player.pos.y));
            let tilted = gravity.and_then(|(gx, gy)| tilt.update(gx, gy));
            world.tick(&Input::new(pick_steer(pointer, keys, tilted)));
            for &event in world.events() {
                audio.play(event);
                if event == Event::Crash {
                    crash_flash = 1.0;
                }
            }
            accumulator -= TICK_SECONDS;
        }
        audio.update(world.mix, sound_js::muted());

        // The landscape scrolls past; until its image has loaded, a plain sky.
        let rain = world.weather.rain;
        let x = background_x(scroll_distance(world.ticks), scenery.tile_width());
        if scenery.draw(x) {
            // Rain clouds darken the landscape.
            draw_rectangle(
                0.0,
                0.0,
                screen_width(),
                screen_height(),
                Color::new(0.16, 0.18, 0.22, 0.45 * rain),
            );
        } else {
            // Rain clouds grey the sky.
            clear_background(Color::new(
                0.078 + 0.06 * rain,
                0.094 + 0.06 * rain,
                0.125 + 0.06 * rain,
                1.0,
            ));
        }

        for m in &world.planes {
            draw_circle(m.pos.x, m.pos.y, PLANE_RADIUS, LIME);
            draw_circle(m.pos.x - 8.0, m.pos.y - 6.0, 4.0, BLACK);
            draw_circle(m.pos.x + 8.0, m.pos.y - 6.0, 4.0, BLACK);
        }

        draw_player(
            world.player.pos.x,
            world.player.pos.y,
            world.player.is_crashed().then_some(world.ticks),
        );
        draw_rain(rain, world.weather.wind, world.ticks);

        // Lightning lights the whole sky; a crash flashes red.
        let (w, h) = (screen_width(), screen_height());
        draw_rectangle(
            0.0,
            0.0,
            w,
            h,
            Color::new(1.0, 1.0, 1.0, 0.7 * world.weather.flash),
        );
        draw_rectangle(
            0.0,
            0.0,
            w,
            h,
            Color::new(1.0, 0.2, 0.1, 0.35 * crash_flash),
        );
        crash_flash = (crash_flash - get_frame_time() / 0.3).max(0.0);

        let text = world.greeting();
        let size = 48.0;
        let dims = measure_text(&text, None, size as u16, 1.0);
        draw_text_on_landscape(
            &text,
            (screen_width() - dims.width) / 2.0,
            64.0,
            size,
            WHITE,
        );
        draw_text_on_landscape(
            &format!(
                "{}  |  ticks: {}",
                if gravity.is_some() {
                    "tilt or drag to fly"
                } else {
                    "drag or arrow keys to fly"
                },
                world.ticks
            ),
            12.0,
            screen_height() - 12.0,
            20.0,
            Color::from_rgba(235, 235, 225, 255),
        );

        next_frame().await
    }
}

/// Text with a dark shadow, so it stays readable on light sky and on grass.
fn draw_text_on_landscape(text: &str, x: f32, y: f32, size: f32, colour: Color) {
    draw_text(
        text,
        x + 2.0,
        y + 2.0,
        size,
        Color::new(0.0, 0.0, 0.0, 0.55),
    );
    draw_text(text, x, y, size, colour);
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

/// Rain streaks. Where each one is follows from its number and the tick
/// count alone, so there is no state to keep; heavier rain shows more of
/// them and wind slants them.
fn draw_rain(rain: f32, wind: f32, ticks: u64) {
    let (w, h) = (screen_width() as f64, screen_height() as f64);
    let seconds = ticks as f64 * TICK_SECONDS as f64;
    let count = (rain as f64 * w * h / 2000.0) as u32;
    let colour = Color::new(0.62, 0.72, 0.86, 0.55);
    let slant = 0.15 + 0.5 * wind;
    for i in 0..count {
        // Two fixed pseudo-random numbers in 0..1 per streak.
        let a = (i.wrapping_mul(2_654_435_761) >> 8) as f64 / (1 << 24) as f64;
        let b = (i.wrapping_mul(40_503).wrapping_add(977) % 4096) as f64 / 4096.0;
        let fall = 620.0 + 260.0 * b;
        let y = ((b * h + seconds * fall) % (h + 40.0)) as f32 - 20.0;
        let x = ((a * w - seconds * fall * slant as f64).rem_euclid(w + 40.0)) as f32 - 20.0;
        let len = 10.0 + 8.0 * b as f32;
        draw_line(x, y, x - len * slant, y + len, 1.5, colour);
    }
}

/// Side view of a small plane flying right, centred on (x, y).
/// `crashed`: the tick count while the plane is out of control.
fn draw_player(x: f32, y: f32, crashed: Option<u64>) {
    let mut body = Color::from_rgba(240, 244, 250, 255);
    let mut accent = Color::from_rgba(31, 111, 235, 255);
    if let Some(ticks) = crashed {
        body = Color::from_rgba(255, 138, 92, 255);
        accent = Color::from_rgba(196, 48, 43, 255);
        // Smoke puffs trailing up and back.
        for i in 0..4u64 {
            let age = ((ticks + i * 5) % 20) as f32 / 20.0;
            draw_circle(
                x - 20.0 - 26.0 * age,
                y - 8.0 - 22.0 * age,
                4.0 + 7.0 * age,
                Color::new(0.6, 0.6, 0.62, 0.6 * (1.0 - age)),
            );
        }
    }
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
