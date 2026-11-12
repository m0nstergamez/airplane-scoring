//! Thin rendering shell. All game rules live in `airplane-scoring-core`.
//! This file should only: read input, call `World::tick`, draw state.

use airplane_scoring_core::{
    background_x, pick_steer, scroll_distance, start_level, steer_from_keys, steer_toward, Bounds,
    Clock, Event, Gull, Input, Tilt, Tower, World, PLANE_RADIUS, TICK_SECONDS,
};
use macroquad::prelude::*;

mod audio;
mod page_js;
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
    world.start_level = start_level(page_js::requested_level());

    let mut clock = Clock::default();
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

        // Fixed ticks; the game stands still until "Tap to fly".
        for _ in 0..clock.ticks_for(get_frame_time(), page_js::started()) {
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
        }
        audio.update(world.mix, sound_js::muted());

        // The landscape scrolls past; at the end of a level the next one
        // fades in over it. Until an image has loaded, a plain sky that
        // rain clouds grey.
        let rain = world.weather.rain;
        clear_background(Color::new(
            0.078 + 0.06 * rain,
            0.094 + 0.06 * rain,
            0.125 + 0.06 * rain,
            1.0,
        ));
        scenery.update();
        let levels = world.levels();
        let distance = scroll_distance(world.ticks);
        let mut landscape = false;
        for (level, alpha) in [(levels.level, 1.0), (levels.next, levels.fade)] {
            if alpha > 0.0 {
                let x = background_x(distance, scenery.tile_width(level));
                landscape |= scenery.draw(level, x, alpha);
            }
        }
        if landscape {
            // Rain clouds darken the landscape.
            draw_rectangle(
                0.0,
                0.0,
                screen_width(),
                screen_height(),
                Color::new(0.16, 0.18, 0.22, 0.45 * rain),
            );
        }

        for m in &world.planes {
            draw_circle(m.pos.x, m.pos.y, PLANE_RADIUS, LIME);
            draw_circle(m.pos.x - 8.0, m.pos.y - 6.0, 4.0, BLACK);
            draw_circle(m.pos.x + 8.0, m.pos.y - 6.0, 4.0, BLACK);
        }

        for tower in &world.towers.list {
            draw_tower(tower, world.bounds);
        }
        for gull in &world.gulls.list {
            draw_gull(gull);
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

        // "Level N" at the start of each level.
        if levels.banner > 0.0 {
            let text = format!("Level {}", levels.level);
            let size = 72.0;
            let dims = measure_text(&text, None, size as u16, 1.0);
            draw_text_on_landscape(
                &text,
                (w - dims.width) / 2.0,
                h * 0.42,
                size,
                Color::new(1.0, 0.86, 0.35, levels.banner),
            );
        }

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
        Color::new(0.0, 0.0, 0.0, 0.55 * colour.a),
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

/// A lifeguard tower on stilts. Its box comes from core (`Tower::rect`):
/// the roof's tip is the top, the feet stand on the ground.
fn draw_tower(tower: &Tower, bounds: Bounds) {
    let (l, top, r, ground) = tower.rect(bounds);
    let (w, x) = (r - l, (l + r) / 2.0);
    let wood = Color::from_rgba(122, 86, 52, 255);
    let wood_dark = Color::from_rgba(92, 62, 36, 255);
    let red = Color::from_rgba(206, 52, 44, 255);
    let white = Color::from_rgba(246, 244, 236, 255);
    let roof_h = 0.35 * w;
    let cabin_h = 0.8 * w;
    let floor = top + roof_h + cabin_h; // platform under the cabin
    let post = (0.07 * w).max(2.0);

    // Legs, splayed a little, with two cross braces.
    let (top_l, top_r) = (x - 0.38 * w, x + 0.38 * w);
    let (foot_l, foot_r) = (x - 0.55 * w, x + 0.55 * w);
    draw_line(top_l, floor, foot_l, ground, post, wood);
    draw_line(top_r, floor, foot_r, ground, post, wood);
    let at = |a: f32, b: f32, t: f32| a + (b - a) * t;
    for (t0, t1) in [(0.15, 0.55), (0.55, 0.95)] {
        let y0 = at(floor, ground, t0);
        let y1 = at(floor, ground, t1);
        draw_line(
            at(top_l, foot_l, t0),
            y0,
            at(top_r, foot_r, t1),
            y1,
            post * 0.6,
            wood_dark,
        );
        draw_line(
            at(top_r, foot_r, t0),
            y0,
            at(top_l, foot_l, t1),
            y1,
            post * 0.6,
            wood_dark,
        );
    }
    // Ladder down the front (right, where the plane comes from).
    let (lad_top, lad_foot) = (x + 0.3 * w, x + 0.95 * w);
    draw_line(lad_top, floor, lad_foot, ground, post * 0.6, wood_dark);
    draw_line(
        lad_top + 0.2 * w,
        floor,
        lad_foot + 0.2 * w,
        ground,
        post * 0.6,
        wood_dark,
    );
    let rungs = ((ground - floor) / (0.25 * w)).max(1.0) as u32;
    for i in 1..rungs {
        let t = i as f32 / rungs as f32;
        let y = at(floor, ground, t);
        let lx = at(lad_top, lad_foot, t);
        draw_line(lx, y, lx + 0.2 * w, y, post * 0.5, wood_dark);
    }
    // Platform, cabin with a red stripe and a dark window, roof.
    draw_rectangle(x - 0.65 * w, floor - post, 1.3 * w, post * 1.5, wood);
    draw_rectangle(l, top + roof_h, w, cabin_h, white);
    draw_rectangle(l, top + roof_h + 0.62 * cabin_h, w, 0.18 * cabin_h, red);
    draw_rectangle(
        l + 0.18 * w,
        top + roof_h + 0.15 * cabin_h,
        0.64 * w,
        0.35 * cabin_h,
        Color::from_rgba(44, 58, 72, 255),
    );
    draw_triangle(
        vec2(x - 0.65 * w, top + roof_h),
        vec2(x + 0.65 * w, top + roof_h),
        vec2(x, top),
        red,
    );
}

/// A gull seen from the side, centred on its position, facing where it
/// flies. The wings beat; their size matches core's `GULL_RADIUS`.
fn draw_gull(gull: &Gull) {
    let (x, y) = (gull.pos.x, gull.pos.y);
    let dir = if gull.vel.x < 0.0 { -1.0 } else { 1.0 }; // -1 = facing left
    let beat = (gull.age as f32 * TICK_SECONDS * std::f32::consts::TAU * 3.0).sin();
    let tip_y = y - 4.0 - 9.0 * beat; // wing tips up and down
    let wing = Color::from_rgba(120, 128, 140, 255);
    let tip = Color::from_rgba(40, 42, 48, 255);
    let body = Color::from_rgba(250, 250, 246, 255);
    // Wings: a soft "M", shoulder to bend to tip, on both sides.
    for side in [-1.0_f32, 1.0] {
        let bend = vec2(x + side * 8.0, y - 3.0 - 4.0 * beat);
        let end = vec2(x + side * 17.0, tip_y);
        draw_line(x, y, bend.x, bend.y, 4.0, wing);
        draw_line(bend.x, bend.y, end.x, end.y, 3.0, wing);
        draw_line(
            end.x - side * 3.0,
            end.y + (bend.y - end.y) * 0.3,
            end.x,
            end.y,
            3.0,
            tip,
        );
    }
    // Body, head and a yellow beak towards the flight direction.
    draw_ellipse(x, y, 9.0, 4.0, 0.0, body);
    draw_circle(x + dir * 8.0, y - 2.0, 3.5, body);
    draw_triangle(
        vec2(x + dir * 11.0, y - 3.0),
        vec2(x + dir * 16.0, y - 1.5),
        vec2(x + dir * 11.0, y - 0.5),
        Color::from_rgba(240, 180, 40, 255),
    );
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
