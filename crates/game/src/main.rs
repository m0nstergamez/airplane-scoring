//! Thin rendering shell. All game rules live in `airplane-scoring-core`.
//! This file should only: read input, call `World::tick`, draw state.

use airplane_scoring_core::{
    background_x, pick_steer, scroll_distance, start_level, steer_from_keys, steer_toward, Balloon,
    Bounds, Clock, Event, Gull, Input, Tilt, World, BALLOON_RADIUS, BASKET_BELOW, LIVES,
    PLANE_RADIUS, TICK_SECONDS,
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

    let mut world = new_world();

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

        // Game over: a tap, click or Enter starts a new flight.
        if world.can_restart() && restart_pressed() {
            world = new_world();
            crash_flash = 0.0;
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

        for balloon in &world.balloons.list {
            draw_balloon(balloon);
        }

        for gull in &world.gulls.list {
            draw_gull(gull);
        }

        // A safe plane blinks (5 times a second), on real time so it
        // also blinks smoothly between ticks.
        let blink_off = world.player.is_safe() && (get_time() * 10.0) as u64 % 2 == 1;
        if !blink_off {
            draw_player(
                world.player.pos.x,
                world.player.pos.y,
                world.player.is_crashed().then_some(world.ticks),
            );
        }
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

        draw_lives(&world);
        if world.is_over() {
            draw_game_over(&world); // shows the score itself
        } else {
            draw_score(&world);
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

/// A new flight: full lives, level 1 (or `?level=`), fresh weather.
fn new_world() -> World {
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
    world
}

/// A new tap, click, Enter or Space this frame (not a finger still held).
fn restart_pressed() -> bool {
    is_mouse_button_pressed(MouseButton::Left)
        || touches().iter().any(|t| t.phase == TouchPhase::Started)
        || is_key_pressed(KeyCode::Enter)
        || is_key_pressed(KeyCode::Space)
}

/// The score, top centre, all game long.
fn draw_score(world: &World) {
    let text = format!("Score {}", world.score());
    let size = 40.0;
    let dims = measure_text(&text, None, size as u16, 1.0);
    let colour = Color::from_rgba(255, 244, 214, 255);
    draw_text_on_landscape(
        &text,
        (screen_width() - dims.width) / 2.0,
        40.0,
        size,
        colour,
    );
}

/// One small plane per life, top right, left of the speaker button. Lost
/// lives are grey; the one just lost blinks while the plane is crashed or
/// safe.
fn draw_lives(world: &World) {
    let (w, h) = (screen_width(), screen_height());
    let scale = (h / 390.0).clamp(0.6, 1.2) * 0.6;
    let step = 64.0 * scale;
    // The speaker button (web/index.html) is 44 px wide, 8 px from the
    // edge, and moves in for a phone's notch at the side; keep clear of both.
    let right = w - (w * 0.06).max(8.0) - 44.0 - 16.0 - 30.0 * scale;
    let left = right - (LIVES - 1) as f32 * step;
    let y = 30.0;
    let just_lost = world.player.is_crashed() || world.player.is_safe();
    let blink = ((get_time() * 4.0) as u64).is_multiple_of(2);
    for i in 0..LIVES {
        let x = left + i as f32 * step;
        let alive = i < world.lives;
        let (body, accent) = if alive || (just_lost && i == world.lives && blink) {
            (WHITE_PLANE, BLUE_PLANE)
        } else {
            (
                Color::new(0.55, 0.57, 0.6, 0.55),
                Color::new(0.4, 0.42, 0.45, 0.55),
            )
        };
        // A soft shadow keeps the icons readable on the bright sky.
        draw_plane(
            x + 2.0,
            y + 2.0,
            scale,
            Color::new(0.0, 0.0, 0.0, 0.3),
            Color::new(0.0, 0.0, 0.0, 0.3),
        );
        draw_plane(x, y, scale, body, accent);
    }
}

/// "Game over", the final score, and how to fly again.
fn draw_game_over(world: &World) {
    let (w, h) = (screen_width(), screen_height());
    draw_rectangle(0.0, 0.0, w, h, Color::new(0.08, 0.09, 0.12, 0.6));
    let centred = |text: &str, y: f32, size: f32, colour: Color| {
        let dims = measure_text(text, None, size as u16, 1.0);
        draw_text_on_landscape(text, (w - dims.width) / 2.0, y, size, colour);
    };
    centred(
        "Game over",
        h * 0.4,
        80.0,
        Color::from_rgba(255, 220, 90, 255),
    );
    centred(
        &format!("Score {}", world.score()),
        h * 0.4 + 52.0,
        32.0,
        WHITE,
    );
    if world.can_restart() {
        centred(
            "Tap or click to fly again",
            h * 0.4 + 100.0,
            28.0,
            Color::from_rgba(160, 200, 255, 255),
        );
    }
}

const WHITE_PLANE: Color = Color::new(0.94, 0.957, 0.98, 1.0);
const BLUE_PLANE: Color = Color::new(0.122, 0.435, 0.92, 1.0);

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

/// A hot-air balloon: a striped envelope tapering to the burner, ropes
/// and a wicker basket. Sizes match core's `BALLOON_RADIUS`, `BASKET_BELOW`
/// and `BASKET_RADIUS`.
fn draw_balloon(balloon: &Balloon) {
    let (x, y) = (balloon.pos.x, balloon.pos.y);
    let r = BALLOON_RADIUS;
    let (main, stripe) = match balloon.colour {
        0 => (
            Color::from_rgba(214, 64, 52, 255),
            Color::from_rgba(250, 206, 72, 255),
        ),
        1 => (
            Color::from_rgba(40, 104, 196, 255),
            Color::from_rgba(244, 244, 236, 255),
        ),
        2 => (
            Color::from_rgba(232, 128, 40, 255),
            Color::from_rgba(122, 64, 150, 255),
        ),
        _ => (
            Color::from_rgba(56, 150, 96, 255),
            Color::from_rgba(246, 232, 190, 255),
        ),
    };
    let basket_y = y + BASKET_BELOW;
    let neck_y = y + r * 1.15;
    // Ropes from the envelope's mouth to the basket.
    let rope = Color::from_rgba(70, 56, 44, 255);
    draw_line(x - 7.0, neck_y, x - 7.0, basket_y - 6.0, 1.5, rope);
    draw_line(x + 7.0, neck_y, x + 7.0, basket_y - 6.0, 1.5, rope);
    // Envelope: round top, tapering down to the mouth.
    draw_triangle(
        vec2(x - r * 0.93, y + r * 0.35),
        vec2(x + r * 0.93, y + r * 0.35),
        vec2(x, neck_y + 4.0),
        main,
    );
    draw_ellipse(x, y, r, r * 1.05, 0.0, main);
    // Stripes: narrower ellipses and the matching wedges below them.
    for (w, colour) in [(0.62, stripe), (0.3, main)] {
        draw_ellipse(x, y, r * w, r * 1.05, 0.0, colour);
        draw_triangle(
            vec2(x - r * w * 0.93, y + r * 0.35),
            vec2(x + r * w * 0.93, y + r * 0.35),
            vec2(x, neck_y + 4.0),
            colour,
        );
    }
    // Mouth, basket and its rim.
    draw_rectangle(
        x - 8.0,
        neck_y,
        16.0,
        4.0,
        Color::from_rgba(60, 50, 44, 255),
    );
    draw_rectangle(
        x - 8.0,
        basket_y - 6.0,
        16.0,
        13.0,
        Color::from_rgba(150, 102, 56, 255),
    );
    draw_rectangle(
        x - 9.0,
        basket_y - 7.0,
        18.0,
        3.0,
        Color::from_rgba(104, 68, 36, 255),
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
    let mut body = WHITE_PLANE;
    let mut accent = BLUE_PLANE;
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
    draw_plane(x, y, 1.0, body, accent);
}

/// The plane's shape, flying right, centred on (x, y), `scale` times its
/// size in the game (for the lives, top right).
fn draw_plane(x: f32, y: f32, scale: f32, body: Color, accent: Color) {
    let p = |dx: f32, dy: f32| vec2(x + dx * scale, y + dy * scale);
    // fuselage
    let corner = p(-26.0, -5.0);
    draw_rectangle(corner.x, corner.y, 44.0 * scale, 10.0 * scale, body);
    draw_triangle(p(18.0, -5.0), p(30.0, 0.0), p(18.0, 5.0), body);
    // wing and tail fin
    draw_triangle(p(-6.0, 0.0), p(6.0, 0.0), p(-10.0, 16.0), accent);
    draw_triangle(p(-26.0, -5.0), p(-18.0, -5.0), p(-28.0, -18.0), accent);
    // cockpit
    let c = p(10.0, -3.0);
    draw_circle(c.x, c.y, 3.5 * scale, accent);
}
