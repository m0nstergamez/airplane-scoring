//! Thin rendering shell. All game rules live in `airplane-scoring-core`.
//! This file should only: read input, call `World::tick`, draw state.

use airplane_scoring_core::Vec2 as Vec2Core;
use airplane_scoring_core::{
    background_x, pick_steer, scroll_distance, start_level, steer_from_keys, Balloon, Bounds,
    Clock, Event, Gull, Highscores, Hut, Input, Pad, Tilt, Tower, World, BALLOON_RADIUS,
    BASKET_BELOW, HIGHSCORE_COUNT, HUT_EAVES, HUT_INSET, LIVES, PAD_HEIGHT, PAD_WIDTH,
    PLANE_COCKPIT, PLANE_FUSELAGE, PLANE_NOSE, PLANE_RADIUS, PLANE_TAIL, PLANE_WING, TICK_SECONDS,
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
    // The best scores on this device; where the last flight landed in them.
    let mut highscores = Highscores::from_scores(page_js::load_highscores(HIGHSCORE_COUNT));
    let mut result: Option<FlightResult> = None;

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
            result = None;
        }

        // Fixed ticks; the game stands still until "Tap to fly".
        for _ in 0..clock.ticks_for(get_frame_time(), page_js::started()) {
            // Recomputed each step: steering toward a finger depends on where
            // the plane is now.
            let pointer = pointer_y.map(|y| world.steer_toward(y));
            let tilted = gravity.and_then(|(gx, gy)| tilt.update(gx, gy));
            world.tick(&Input::new(pick_steer(pointer, keys, tilted)));
            for &event in world.events() {
                audio.play(event);
                if event == Event::Crash {
                    crash_flash = 1.0;
                }
                if event == Event::GameOver {
                    // Core ranks the score; the page keeps the list.
                    let mut fanfare = Vec::new();
                    let place = highscores.add(world.score(), &mut fanfare);
                    page_js::save_highscores(HIGHSCORE_COUNT, highscores.list());
                    for e in &fanfare {
                        audio.play(*e);
                    }
                    result = Some(FlightResult {
                        place,
                        record: !fanfare.is_empty(),
                    });
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
        // Core counts the distance at phone size; the picture is scaled to
        // the screen height, so the distance is too.
        let scale = world.bounds.scale();
        let distance = scroll_distance(world.ticks) * scale as f64;
        // Sizes of what flies come from core's depth (`Bounds::depth`):
        // small up high, big down low, matching the hit boxes.
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
            let size = world.bounds.depth(m.pos.y);
            draw_circle(m.pos.x, m.pos.y, PLANE_RADIUS * size, LIME);
            draw_circle(m.pos.x - 8.0, m.pos.y - 6.0, 4.0, BLACK);
            draw_circle(m.pos.x + 8.0, m.pos.y - 6.0, 4.0, BLACK);
        }

        // Launch pads and balloons, then the huts in the foreground in
        // front of them, then what flies.
        for pad in &world.balloons.pads {
            draw_pad(pad, world.bounds);
        }
        for balloon in &world.balloons.list {
            // Big near the ground, small up high (core's `Balloon::size`).
            draw_balloon(balloon, balloon.size(world.bounds));
        }
        for hut in &world.huts.list {
            draw_hut(hut, world.bounds);
        }
        for tower in &world.towers.list {
            draw_tower(tower, world.bounds);
        }

        for gull in &world.gulls.list {
            draw_gull(gull, world.bounds.depth(gull.pos.y));
        }

        // A safe plane blinks (5 times a second), on real time so it
        // also blinks smoothly between ticks.
        let blink_off = world.player.is_safe() && (get_time() * 10.0) as u64 % 2 == 1;
        if !blink_off {
            draw_player(
                world.player.pos.x,
                world.player.pos.y,
                // Small up high, big down low, like the balloons.
                world.bounds.plane_depth(world.player.pos.y),
                world.player.is_crashed().then_some(world.ticks),
            );
        }
        draw_rain(rain, world.weather.wind, world.ticks, scale);

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
            let size = 72.0 * scale;
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
            draw_game_over(&world, &highscores, result); // shows the score itself
        } else {
            draw_score(&world, highscores.best());
        }

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
fn draw_score(world: &World, best: u64) {
    let scale = world.bounds.scale();
    // The best score so far, small under the score.
    if best > 0 {
        let text = format!("Best {}", best.max(world.score()));
        let size = 22.0 * scale;
        let dims = measure_text(&text, None, size as u16, 1.0);
        let colour = Color::from_rgba(255, 220, 90, 230);
        let x = (screen_width() - dims.width) / 2.0;
        draw_text_on_landscape(&text, x, 64.0 * scale, size, colour);
    }
    let text = format!("Score {}", world.score());
    let size = 40.0 * scale;
    let dims = measure_text(&text, None, size as u16, 1.0);
    let colour = Color::from_rgba(255, 244, 214, 255);
    draw_text_on_landscape(
        &text,
        (screen_width() - dims.width) / 2.0,
        40.0 * scale,
        size,
        colour,
    );
}

/// One small plane per life, top right, left of the speaker button. Lost
/// lives are grey; the one just lost blinks while the plane is crashed or
/// safe.
fn draw_lives(world: &World) {
    let w = screen_width();
    let scale = world.bounds.scale() * 0.6;
    let step = 64.0 * scale;
    // The speaker button (web/index.html) is 44 px wide, 8 px from the
    // edge, and moves in for a phone's notch at the side; keep clear of both.
    let right = w - (w * 0.06).max(8.0) - 44.0 - 16.0 - 30.0 * scale;
    let left = right - (LIVES - 1) as f32 * step;
    // Level with the speaker button (centre 30 px from the top), but
    // never cut off at the top.
    let y = (8.0 + 22.0_f32).max(6.0 + 20.0 * scale);
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

/// Where a finished flight landed in the highscores.
#[derive(Clone, Copy)]
struct FlightResult {
    /// Its place in the list (0 = best), if it made it.
    place: Option<usize>,
    /// Better than every earlier flight.
    record: bool,
}

/// "Game over", the final score, the best scores on this device (this
/// flight highlighted), and how to fly again.
fn draw_game_over(world: &World, highscores: &Highscores, result: Option<FlightResult>) {
    let (w, h) = (screen_width(), screen_height());
    draw_rectangle(0.0, 0.0, w, h, Color::new(0.08, 0.09, 0.12, 0.65));
    let s = world.bounds.scale();
    let centred = |text: &str, y: f32, size: f32, colour: Color| {
        let size = size * s;
        let dims = measure_text(text, None, size as u16, 1.0);
        draw_text_on_landscape(text, (w - dims.width) / 2.0, y, size, colour);
    };
    let gold = Color::from_rgba(255, 220, 90, 255);
    let mut y = h * 0.2;
    centred("Game over", y, 64.0, gold);
    y += 40.0 * s;
    centred(&format!("Score {}", world.score()), y, 30.0, WHITE);
    if result.is_some_and(|r| r.record) {
        y += 30.0 * s;
        // Blinks, to celebrate.
        let on = ((get_time() * 3.0) as u64).is_multiple_of(2);
        let colour = if on {
            gold
        } else {
            Color::from_rgba(255, 150, 60, 255)
        };
        centred("New highscore!", y, 28.0, colour);
    }
    // The best scores, this flight highlighted.
    y += 34.0 * s;
    centred("Highscores", y, 22.0, Color::from_rgba(200, 210, 225, 255));
    for (i, score) in highscores.list().iter().enumerate() {
        y += 22.0 * s;
        let mine = result.and_then(|r| r.place) == Some(i);
        let colour = if mine {
            gold
        } else {
            Color::from_rgba(235, 235, 225, 255)
        };
        let text = format!("{}.  {}{}", i + 1, score, if mine { "  <" } else { "" });
        centred(&text, y, 22.0, colour);
    }
    if world.can_restart() {
        centred(
            "Tap or click to fly again",
            h - 24.0 * s,
            26.0,
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
fn draw_rain(rain: f32, wind: f32, ticks: u64, scale: f32) {
    let (w, h) = (screen_width() as f64, screen_height() as f64);
    let seconds = ticks as f64 * TICK_SECONDS as f64;
    let count = (rain as f64 * w * h / 2000.0) as u32;
    let colour = Color::new(0.62, 0.72, 0.86, 0.55);
    let slant = 0.15 + 0.5 * wind;
    for i in 0..count {
        // Two fixed pseudo-random numbers in 0..1 per streak.
        let a = (i.wrapping_mul(2_654_435_761) >> 8) as f64 / (1 << 24) as f64;
        let b = (i.wrapping_mul(40_503).wrapping_add(977) % 4096) as f64 / 4096.0;
        let fall = (620.0 + 260.0 * b) * scale as f64;
        let y = ((b * h + seconds * fall) % (h + 40.0)) as f32 - 20.0;
        let x = ((a * w - seconds * fall * slant as f64).rem_euclid(w + 40.0)) as f32 - 20.0;
        let len = (10.0 + 8.0 * b as f32) * scale;
        draw_line(x, y, x - len * slant, y + len, 1.5 * scale, colour);
    }
}

/// A wooden beach hut: plank walls, a gable roof with a little overhang,
/// a door and a window. Fills core's `Hut::rect` (roof ridge to ground).
fn draw_hut(hut: &Hut, bounds: Bounds) {
    let (l, top, r, ground) = hut.rect(bounds);
    let s = bounds.scale();
    let (w, h) = (r - l, ground - top);
    let (wall, plank, door) = match hut.style {
        0 => (
            Color::from_rgba(150, 104, 66, 255),
            Color::from_rgba(120, 80, 50, 255),
            Color::from_rgba(184, 58, 48, 255),
        ),
        1 => (
            Color::from_rgba(236, 236, 228, 255),
            Color::from_rgba(70, 130, 190, 255),
            Color::from_rgba(52, 104, 168, 255),
        ),
        _ => (
            Color::from_rgba(232, 204, 120, 255),
            Color::from_rgba(196, 164, 86, 255),
            Color::from_rgba(70, 128, 88, 255),
        ),
    };
    let roof = Color::from_rgba(92, 62, 48, 255);
    // Same proportions as core's `Hut::shape`, which crashes use.
    let eaves = top + h * HUT_EAVES;
    let inset = w * HUT_INSET;
    // Walls with vertical planks.
    draw_rectangle(l + inset, eaves, w - 2.0 * inset, ground - eaves, wall);
    let planks = 5;
    for i in 1..planks {
        let x = l + inset + (w - 2.0 * inset) * i as f32 / planks as f32;
        draw_line(x, eaves, x, ground, 1.5 * s, plank);
    }
    // Gable roof, a bit wider than the walls.
    draw_triangle(
        vec2(l, eaves),
        vec2(r, eaves),
        vec2((l + r) / 2.0, top),
        roof,
    );
    draw_rectangle(l, eaves - 1.5 * s, w, 3.0 * s, roof);
    // Door and a small window.
    let door_w = w * 0.26;
    draw_rectangle(l + w * 0.2, ground - h * 0.42, door_w, h * 0.42, door);
    let win = w * 0.18;
    draw_rectangle(
        l + w * 0.6,
        eaves + h * 0.12,
        win,
        win,
        Color::from_rgba(52, 62, 76, 255),
    );
    draw_rectangle_lines(l + w * 0.6, eaves + h * 0.12, win, win, 1.5 * s, plank);
}

/// A lifeguard tower on the beach: wooden legs with a cross brace, a
/// ladder, a platform, a cabin with a stripe and a window, and a roof.
/// Uses core's `Tower::parts`, which crashes use too.
fn draw_tower(tower: &Tower, bounds: Bounds) {
    let p = tower.parts(bounds);
    let s = tower.size(bounds); // smaller further back by the water
    let wood = Color::from_rgba(122, 86, 58, 255);
    let (cabin, roof, stripe) = match tower.style {
        0 => (
            Color::from_rgba(244, 240, 228, 255),
            Color::from_rgba(200, 52, 44, 255),
            Color::from_rgba(200, 52, 44, 255),
        ),
        _ => (
            Color::from_rgba(240, 200, 70, 255),
            Color::from_rgba(44, 96, 168, 255),
            Color::from_rgba(44, 96, 168, 255),
        ),
    };
    for (a, b) in p.legs {
        draw_line(a.x, a.y, b.x, b.y, p.leg_width, wood);
    }
    // A cross brace between the legs (thin, inside the gap's frame).
    let [(a0, b0), (a1, b1)] = p.legs;
    let mid = |a: Vec2Core, b: Vec2Core, t: f32| vec2(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t);
    let (l1, r1) = (mid(a0, b0, 0.25), mid(a1, b1, 0.7));
    let (l2, r2) = (mid(a0, b0, 0.7), mid(a1, b1, 0.25));
    draw_line(l1.x, l1.y, r1.x, r1.y, 1.5 * s, wood);
    draw_line(l2.x, l2.y, r2.x, r2.y, 1.5 * s, wood);
    // Ladder: two rails and rungs.
    let (lb, lt) = p.ladder;
    draw_line(lb.x, lb.y, lt.x, lt.y, p.leg_width * 0.75, wood);
    for i in 1..6 {
        let r = mid(lb, lt, i as f32 / 6.0);
        draw_line(r.x - 4.0 * s, r.y, r.x, r.y, 1.5 * s, wood);
    }
    let (l, t, w, h) = p.platform;
    draw_rectangle(l, t, w, h, wood);
    let (l, t, w, h) = p.cabin;
    draw_rectangle(l, t, w, h, cabin);
    draw_rectangle(l, t + h * 0.62, w, h * 0.16, stripe);
    draw_rectangle(
        l + w * 0.2,
        t + h * 0.15,
        w * 0.6,
        h * 0.32,
        Color::from_rgba(52, 62, 76, 255),
    );
    let [a, b, c] = p.roof;
    draw_triangle(vec2(a.x, a.y), vec2(b.x, b.y), vec2(c.x, c.y), roof);
}

/// A balloon launch pad: a flat wooden deck on the grass with a yellow
/// ring where the basket stands.
fn draw_pad(pad: &Pad, bounds: Bounds) {
    let s = bounds.scale();
    let (w, h) = (PAD_WIDTH * s, PAD_HEIGHT * s);
    let top = Pad::top(bounds);
    let l = pad.x - w / 2.0;
    draw_rectangle(l, top, w, h, Color::from_rgba(120, 92, 64, 255));
    draw_rectangle(l, top, w, h * 0.4, Color::from_rgba(176, 144, 104, 255));
    // Little legs into the grass.
    for x in [l + 4.0 * s, l + w - 7.0 * s] {
        draw_rectangle(
            x,
            top + h,
            3.0 * s,
            4.0 * s,
            Color::from_rgba(90, 68, 48, 255),
        );
    }
    draw_ellipse_lines(
        pad.x,
        top + h * 0.2,
        16.0 * s,
        2.5 * s,
        0.0,
        1.5 * s,
        YELLOW,
    );
}

/// A hot-air balloon: a striped envelope tapering to the burner, ropes
/// and a wicker basket. Sizes match core's `BALLOON_RADIUS`, `BASKET_BELOW`
/// and `BASKET_RADIUS`, times `scale` (core's `Balloon::size`).
fn draw_balloon(balloon: &Balloon, scale: f32) {
    let (x, y) = (balloon.pos.x, balloon.pos.y);
    let s = scale;
    let r = BALLOON_RADIUS * s;
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
    let basket_y = y + BASKET_BELOW * s;
    let neck_y = y + r * 1.15;
    // Ropes from the envelope's mouth to the basket.
    let rope = Color::from_rgba(70, 56, 44, 255);
    for side in [-1.0, 1.0] {
        let rx = x + side * 7.0 * s;
        draw_line(rx, neck_y, rx, basket_y - 6.0 * s, 1.5 * s, rope);
    }
    // Envelope: round top, tapering down to the mouth; stripes are
    // narrower copies on top.
    for (w, colour) in [(1.0, main), (0.62, stripe), (0.3, main)] {
        draw_ellipse(x, y, r * w, r * 1.05, 0.0, colour);
        draw_triangle(
            vec2(x - r * w * 0.93, y + r * 0.35),
            vec2(x + r * w * 0.93, y + r * 0.35),
            vec2(x, neck_y + 4.0 * s),
            colour,
        );
    }
    // Mouth, basket and its rim.
    let rect = |dx: f32, dy: f32, w: f32, h: f32, colour: Color| {
        draw_rectangle(x + dx * s, dy, w * s, h * s, colour);
    };
    rect(-8.0, neck_y, 16.0, 4.0, Color::from_rgba(60, 50, 44, 255));
    rect(
        -8.0,
        basket_y - 6.0 * s,
        16.0,
        13.0,
        Color::from_rgba(150, 102, 56, 255),
    );
    rect(
        -9.0,
        basket_y - 7.0 * s,
        18.0,
        3.0,
        Color::from_rgba(104, 68, 36, 255),
    );
}

/// A gull seen from the side, centred on its position, facing where it
/// flies. The wings beat; their size matches core's `GULL_RADIUS`, times
/// the screen's `scale`.
fn draw_gull(gull: &Gull, scale: f32) {
    let (x, y) = (gull.pos.x, gull.pos.y);
    let dir = gull.facing(); // -1 = facing left
                             // A point (dx, dy) design px from the gull's centre.
    let p = |dx: f32, dy: f32| vec2(x + dx * scale, y + dy * scale);
    let wing = Color::from_rgba(120, 128, 140, 255);
    let tip = Color::from_rgba(40, 42, 48, 255);
    let body = Color::from_rgba(250, 250, 246, 255);
    // Wings: a soft "M", shoulder to bend to tip, on both sides.
    for side in [-1.0_f32, 1.0] {
        // Where core puts the wing for crashes (`Gull::shape`).
        let ((bx, by), (ex, ey)) = gull.wing(side);
        let (bend, end) = (p(bx, by), p(ex, ey));
        draw_line(x, y, bend.x, bend.y, 4.0 * scale, wing);
        draw_line(bend.x, bend.y, end.x, end.y, 3.0 * scale, wing);
        let from = vec2(end.x - side * 3.0 * scale, end.y + (bend.y - end.y) * 0.3);
        draw_line(from.x, from.y, end.x, end.y, 3.0 * scale, tip);
    }
    // Body, head and a yellow beak towards the flight direction.
    draw_ellipse(x, y, 9.0 * scale, 4.0 * scale, 0.0, body);
    let head = p(dir * 8.0, -2.0);
    draw_circle(head.x, head.y, 3.5 * scale, body);
    draw_triangle(
        p(dir * 11.0, -3.0),
        p(dir * 16.0, -1.5),
        p(dir * 11.0, -0.5),
        Color::from_rgba(240, 180, 40, 255),
    );
}

/// Side view of a small plane flying right, centred on (x, y), at the
/// screen's `scale`. `crashed`: the tick count while it is out of control.
fn draw_player(x: f32, y: f32, scale: f32, crashed: Option<u64>) {
    let mut body = WHITE_PLANE;
    let mut accent = BLUE_PLANE;
    if let Some(ticks) = crashed {
        body = Color::from_rgba(255, 138, 92, 255);
        accent = Color::from_rgba(196, 48, 43, 255);
        // Smoke puffs trailing up and back.
        for i in 0..4u64 {
            let age = ((ticks + i * 5) % 20) as f32 / 20.0;
            draw_circle(
                x - (20.0 + 26.0 * age) * scale,
                y - (8.0 + 22.0 * age) * scale,
                (4.0 + 7.0 * age) * scale,
                Color::new(0.6, 0.6, 0.62, 0.6 * (1.0 - age)),
            );
        }
    }
    draw_plane(x, y, scale, body, accent);
}

/// The plane's shape, flying right, centred on (x, y), `scale` times its
/// size on a phone (the player's plane, and the lives top right).
fn draw_plane(x: f32, y: f32, scale: f32, body: Color, accent: Color) {
    // The same parts core uses for crashes (`Player::shape`).
    let p = |(dx, dy): (f32, f32)| vec2(x + dx * scale, y + dy * scale);
    let [a, b, c, d] = PLANE_FUSELAGE.map(p);
    draw_triangle(a, b, c, body);
    draw_triangle(a, c, d, body);
    let [a, b, c] = PLANE_NOSE.map(p);
    draw_triangle(a, b, c, body);
    let [a, b, c] = PLANE_WING.map(p);
    draw_triangle(a, b, c, accent);
    let [a, b, c] = PLANE_TAIL.map(p);
    draw_triangle(a, b, c, accent);
    let (centre, r) = PLANE_COCKPIT;
    let c = p(centre);
    draw_circle(c.x, c.y, r * scale, accent);
}
