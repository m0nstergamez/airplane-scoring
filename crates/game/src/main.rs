//! Thin rendering shell. All game rules live in `airplane-scoring-core`.
//! This file should only: read input, call `World::tick`, draw state.

use airplane_scoring_core::Vec2 as Vec2Core;
use airplane_scoring_core::{
    background_x, best_time_text, clock_text, has_next_level, pick_steer, pick_throttle,
    start_level, steer_from_keys, Airport, Balloon, BestTimes, Bounds, Clock, Crane, Event,
    FarHighRise, Goose, Gull, HeliParts, HighRise, Hut, Input, Mix, Pad, Steam, Tilt, Tower,
    TyreSmoke, Warning, World, Wreck, AIRSPEED_RANGE, BALLOON_RADIUS, BASKET_BELOW,
    BEST_TIMES_COUNT, HUT_EAVES, HUT_INSET, LIVES, MAIN_WHEEL_X, NOSE_WHEEL_X, OVERSPEED_AT,
    PAD_HEIGHT, PAD_WIDTH, PLANE_COCKPIT, PLANE_FUSELAGE, PLANE_NOSE, PLANE_RADIUS, PLANE_TAIL,
    PLANE_WING, RUNWAY_HEIGHT, RUNWAY_LENGTH, STALL_AT, STRUT_REACH, SUNRISE_LEVEL, SUN_AT,
    SUN_RADIUS, TICK_SECONDS, WHEEL_RADIUS,
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
    let mut flight = page_js::new_flight();
    // The level's best times and where the last time landed in them, for
    // "Level complete" and "Game over".
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
        // Throttle keys: left/A slower, right/D faster (same maths as up/down).
        let gas = -steer_from_keys(
            is_key_down(KeyCode::Right) || is_key_down(KeyCode::D),
            is_key_down(KeyCode::Left) || is_key_down(KeyCode::A),
        );
        if tilt_js::epoch() != tilt_epoch {
            tilt_epoch = tilt_js::epoch();
            tilt.recalibrate();
        }
        let gravity = tilt_js::sample();
        if is_key_pressed(KeyCode::M) {
            sound_js::toggle_mute();
        }

        // The menu's "Menu" button went back to the start screen: get a
        // fresh flight ready for the next "Tap to fly".
        if page_js::new_flight() != flight {
            flight = page_js::new_flight();
            // Leaving a flight (Menu, Restart) counts no time: only a
            // completed level goes into the best times.
            world = new_world();
            crash_flash = 0.0;
            result = None;
        }

        // "Level complete": the buttons Next level and Menu (Enter: next;
        // after the last level only Menu).
        // Not while paused: a key pressed in the pause menu is not for them.
        let playing = page_js::started();
        if playing && world.level_complete().is_some() {
            match pressed_level_end_button(&world) {
                Some(LevelEndChoice::Next) => {
                    world.go_on();
                    result = None;
                }
                Some(LevelEndChoice::Menu) => page_js::show_menu(),
                None => {}
            }
        }

        // Game over: a tap, click or Enter starts a new flight.
        if playing && world.can_restart() && restart_pressed() {
            world = new_world();
            crash_flash = 0.0;
            result = None;
        }

        // Fixed ticks; the game stands still until "Tap to fly".
        for _ in 0..clock.ticks_for(get_frame_time(), page_js::started()) {
            // Recomputed each step: steering toward a finger depends on where
            // the plane is now.
            let pointer = pointer_y.map(|y| world.steer_toward(y));
            let tilted = gravity.and_then(|(gx, gy, _)| tilt.update(gx, gy));
            let leaned = gravity.and_then(|(gx, gy, gz)| tilt.lean(gx, gy, gz));
            let steer = pick_steer(pointer, keys, tilted);
            world.tick(&Input::with_throttle(steer, pick_throttle(gas, leaned)));
            for &event in world.events() {
                audio.play(event);
                if event == Event::Crash || event == Event::Explosion {
                    crash_flash = 1.0;
                }
                if let Event::LevelComplete { level } = event {
                    page_js::level_complete(level); // unlocks the next level
                    result = Some(record_time(&world));
                }
                if event == Event::GameOver {
                    // No time (the level was not completed): just the table.
                    result = Some(best_times_of(world.levels().level));
                }
            }
        }
        // Paused (menu open, counting down) or not started: silence.
        let mix = if page_js::started() {
            world.mix
        } else {
            Mix::SILENT
        };
        audio.update(mix, sound_js::muted(), sound_js::music_on());

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
        // The explosion shakes the landscape and everything in it (not
        // the time and lives): a camera moved by core's shake, zoomed in
        // a little so no empty edge shows.
        let shake = world.shake();
        if shake.x != 0.0 || shake.y != 0.0 {
            let (w, h) = (screen_width(), screen_height());
            let zoom = 1.06;
            let view = Rect::new(
                (w - w / zoom) / 2.0 - shake.x,
                (h - h / zoom) / 2.0 - shake.y,
                w / zoom,
                h / zoom,
            );
            // `from_display_rect` in macroquad 0.4.16 turns y upwards (the
            // picture comes out upside down); flip it back to screen y.
            let mut camera = Camera2D::from_display_rect(view);
            camera.zoom.y = -camera.zoom.y;
            set_camera(&camera);
        }
        scenery.update();
        let levels = world.levels();
        // Core counts the distance at phone size; the picture is scaled to
        // the screen height, so the distance is too.
        let scale = world.bounds.scale();
        // (`World::distance` grows faster as the game speeds up.)
        let distance = world.distance * scale as f64;
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

        // The low sun of the sunrise, fixed in the sky ahead.
        if levels.level == SUNRISE_LEVEL {
            draw_sun(world.bounds, world.ticks);
        }
        // High-rises in the valley, black against the sun (obstacles).
        for b in &world.sunrise.high_rises {
            draw_far_high_rise(b, world.bounds);
        }

        for m in &world.planes {
            let size = world.bounds.depth(m.pos.y);
            draw_circle(m.pos.x, m.pos.y, PLANE_RADIUS * size, LIME);
            draw_circle(m.pos.x - 8.0, m.pos.y - 6.0, 4.0, BLACK);
            draw_circle(m.pos.x + 8.0, m.pos.y - 6.0, 4.0, BLACK);
        }

        // Launch pads and balloons, then the huts in the foreground in
        // front of them, then what flies.
        if let Some(airport) = &world.airport {
            draw_airport(airport, world.bounds, world.ticks, world.runway_lit());
        }
        for puff in &world.tyre_smoke {
            draw_tyre_smoke(puff, world.bounds.scale());
        }
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
        // The town: cranes and high-rises on the ground, helicopters in
        // the air, crashed ones falling in flames.
        for crane in &world.city.cranes {
            draw_crane(crane, world.bounds);
        }
        for high_rise in &world.city.high_rises {
            draw_high_rise(high_rise, world.bounds);
        }
        let s = world.bounds.scale();
        for flyer in &world.city.flyers {
            draw_heli(true, flyer.age, &flyer.parts(world.bounds), s);
        }
        for wreck in &world.city.wrecks {
            draw_wreck(wreck, s);
        }

        for gull in &world.gulls.list {
            draw_gull(gull, world.bounds.depth(gull.pos.y));
        }
        // The sunrise: steam rising from the valley, geese in the air.
        for column in &world.sunrise.steam {
            draw_steam(column, world.bounds);
        }
        for goose in &world.sunrise.geese {
            draw_goose(goose, s);
        }

        // A safe plane blinks (5 times a second), on real time so it
        // also blinks smoothly between ticks.
        let blink_off = world.player.is_safe() && (get_time() * 10.0) as u64 % 2 == 1;
        // Small up high, big down low, like the balloons.
        let plane_size = world.bounds.plane_depth(world.player.pos.y);
        if let (Some(at), Some(t)) = (world.explosion, world.explosion_progress()) {
            draw_explosion(at.x, at.y, plane_size, t);
        } else {
            // Damage: a smoke trail that thickens with every life lost.
            draw_damage_smoke(
                world.player.pos.x,
                world.player.pos.y,
                plane_size,
                world.smoke(),
                world.ticks,
            );
            if !blink_off {
                draw_player(
                    world.player.pos.x,
                    world.player.pos.y,
                    plane_size,
                    world.player.pitch, // nose up to climb, down to dive
                    world.player.is_crashed().then_some(world.ticks),
                    world.gear(), // wheels out, folding up after lift-off
                );
            }
        }
        draw_rain(rain, world.weather.wind, world.ticks, scale);
        draw_glare(world.sunrise.glare(), world.bounds);

        set_default_camera(); // flashes and the time do not shake
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

        draw_airspeed(&world);
        draw_warning(&world);
        draw_lives(&world);
        if world.is_over() {
            draw_game_over(&world, result.as_ref()); // shows the time itself
        } else if world.level_complete().is_none() {
            // ("Level complete" shows the time itself, too.)
            draw_time(&world);
            draw_progress(&world);
        }
        draw_level_end(&world, result.as_ref());

        next_frame().await
    }
}

/// The time flown in the level now, in that level's best times: core
/// ranks it, the page keeps the table.
struct FlightResult {
    level: u32,
    times: BestTimes,
    placed: Option<airplane_scoring_core::Placed>,
}

/// A level's stored best times, with nothing new in them.
fn best_times_of(level: u32) -> FlightResult {
    FlightResult {
        level,
        times: BestTimes::from_millis(page_js::load_best_times(level, BEST_TIMES_COUNT)),
        placed: None,
    }
}

/// A completed level: its time goes into the level's best times (and is
/// saved if it made the table).
fn record_time(world: &World) -> FlightResult {
    let mut r = best_times_of(world.levels().level);
    r.placed = r.times.add(world.millis_in_level());
    if r.placed.is_some() {
        page_js::save_best_times(r.level, BEST_TIMES_COUNT, r.times.list());
    }
    r
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
    // Every flight starts from the airport (core skips it in level 2).
    world.with_takeoff()
}

/// A new tap, click, Enter or Space this frame (not a finger still held).
fn restart_pressed() -> bool {
    is_mouse_button_pressed(MouseButton::Left)
        || touches().iter().any(|t| t.phase == TouchPhase::Started)
        || is_key_pressed(KeyCode::Enter)
        || is_key_pressed(KeyCode::Space)
}

/// What the player chose at "Level complete".
#[derive(Clone, Copy, PartialEq)]
enum LevelEndChoice {
    Next,
    Menu,
}

/// Is the level just completed the last one? Then there is no "Next
/// level" button, only a note that more levels are coming, and "Menu".
fn last_level_done(world: &World) -> bool {
    world.level_complete().is_some_and(|l| !has_next_level(l))
}

/// The buttons under "Level complete": where they are and what they do.
/// After the last level only "Menu"; where "Next level" would be is the
/// note `NO_MORE_LEVELS` (see `no_more_levels_box`).
fn level_end_buttons(world: &World) -> Vec<(Rect, &'static str, LevelEndChoice)> {
    let (w, h) = (screen_width(), screen_height());
    let s = world.bounds.scale();
    let (bw, bh, gap) = (170.0 * s, 44.0 * s, 20.0 * s);
    // At the bottom, under the best times.
    let y = h - bh - 14.0 * s;
    let left = (w - 2.0 * bw - gap) / 2.0;
    if last_level_done(world) {
        let menu = Rect::new(left + bw + gap, y, bw, bh);
        return vec![(menu, "Menu", LevelEndChoice::Menu)];
    }
    vec![
        (
            Rect::new(left, y, bw, bh),
            "Next level",
            LevelEndChoice::Next,
        ),
        (
            Rect::new(left + bw + gap, y, bw, bh),
            "Menu",
            LevelEndChoice::Menu,
        ),
    ]
}

/// Shown after the last level where the "Next level" button would be.
const NO_MORE_LEVELS: [&str; 2] = ["No more levels yet,", "new ones are coming!"];

/// Where "Next level" would be: left of "Menu", same size.
fn no_more_levels_box(world: &World) -> Rect {
    let menu = level_end_buttons(world)[0].0;
    let gap = 20.0 * world.bounds.scale();
    Rect::new(menu.x - gap - menu.w, menu.y, menu.w, menu.h)
}

/// A tap or click on one of the level-end buttons this frame (or Enter
/// for the next level; after the last level for the menu).
fn pressed_level_end_button(world: &World) -> Option<LevelEndChoice> {
    if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::Space) {
        if last_level_done(world) {
            return Some(LevelEndChoice::Menu);
        }
        return Some(LevelEndChoice::Next);
    }
    let dpi = macroquad::miniquad::window::dpi_scale();
    let touch = touches()
        .into_iter()
        .find(|t| t.phase == TouchPhase::Started)
        .map(|t| t.position / dpi);
    let click = is_mouse_button_pressed(MouseButton::Left).then(|| Vec2::from(mouse_position()));
    let at = touch.or(click)?;
    level_end_buttons(world)
        .into_iter()
        .find(|(rect, _, _)| rect.contains(at))
        .map(|(_, _, choice)| choice)
}

/// The end of a level: "Level N complete" with the time after the
/// landing, then the next level's number and a 3, 2, 1 countdown.
fn draw_level_end(world: &World, result: Option<&FlightResult>) {
    let (w, h) = (screen_width(), screen_height());
    let s = world.bounds.scale();
    let centred = |text: &str, y: f32, size: f32, colour: Color| {
        let size = size * s;
        let dims = measure_text(text, None, size as u16, 1.0);
        draw_text_on_landscape(text, (w - dims.width) / 2.0, y, size, colour);
    };
    let gold = Color::from_rgba(255, 220, 90, 255);
    if let Some(level) = world.level_complete() {
        draw_rectangle(0.0, 0.0, w, h, Color::new(0.08, 0.09, 0.12, 0.55));
        let mut y = h * 0.17;
        centred(&format!("Level {level} complete"), y, 44.0, gold);
        y += 38.0 * s;
        let time = format!("Time {}", best_time_text(world.millis_in_level()));
        centred(&time, y, 30.0, WHITE);
        if let Some(r) = result {
            draw_best_times(r, y, s, &centred);
        }
        for (rect, label, choice) in level_end_buttons(world) {
            let fill = match choice {
                LevelEndChoice::Next => Color::from_rgba(31, 111, 235, 255),
                LevelEndChoice::Menu => Color::new(0.08, 0.09, 0.12, 0.75),
            };
            draw_rectangle(rect.x, rect.y, rect.w, rect.h, fill);
            draw_rectangle_lines(
                rect.x,
                rect.y,
                rect.w,
                rect.h,
                1.5 * s,
                Color::new(1.0, 1.0, 1.0, 0.4),
            );
            let size = 24.0 * s;
            let dims = measure_text(label, None, size as u16, 1.0);
            let x = rect.x + (rect.w - dims.width) / 2.0;
            let y = rect.y + (rect.h + dims.offset_y) / 2.0;
            draw_text(label, x, y, size, WHITE);
        }
        if last_level_done(world) {
            // A quiet box (no button colour, it does nothing) with the note.
            let r = no_more_levels_box(world);
            draw_rectangle(r.x, r.y, r.w, r.h, Color::new(0.08, 0.09, 0.12, 0.6));
            draw_rectangle_lines(
                r.x,
                r.y,
                r.w,
                r.h,
                1.5 * s,
                Color::new(1.0, 0.86, 0.35, 0.5),
            );
            let size = 17.0 * s;
            let line_gap = 17.0 * s;
            for (i, line) in NO_MORE_LEVELS.iter().enumerate() {
                let dims = measure_text(line, None, size as u16, 1.0);
                let x = r.x + (r.w - dims.width) / 2.0;
                let y =
                    r.y + r.h / 2.0 - line_gap / 2.0 + dims.offset_y / 2.0 + i as f32 * line_gap;
                draw_text(line, x, y, size, gold);
            }
        }
    } else if let Some(n) = world.countdown() {
        centred(
            &format!("Level {}", world.levels().level),
            h * 0.3,
            44.0,
            WHITE,
        );
        centred(&n.to_string(), h * 0.3 + 110.0 * s, 120.0, gold);
    }
}

/// The airspeed you set with the throttle, top left: a bar with the
/// stall zone (amber), cruising (green) and overspeed (red), and a marker
/// for the speed now.
fn draw_airspeed(world: &World) {
    if world.is_over() {
        return;
    }
    let w = screen_width();
    let s = world.bounds.scale();
    let (bar_w, bar_h) = (150.0 * s, 8.0 * s);
    // Top left, under the warning lamp (the bottom is runway and huts).
    // Right of the menu button (44 px, top left).
    let left = (w * 0.06).max(16.0) + 52.0;
    let top = 30.0 * s;
    let (lo, hi) = AIRSPEED_RANGE;
    let at = |v: f32| left + (v - lo) / (hi - lo) * bar_w;
    let zone = |from: f32, to: f32, colour: Color| {
        draw_rectangle(at(from), top, at(to) - at(from), bar_h, colour);
    };
    zone(lo, STALL_AT, Color::from_rgba(240, 170, 40, 200));
    zone(STALL_AT, OVERSPEED_AT, Color::from_rgba(70, 180, 90, 200));
    zone(OVERSPEED_AT, hi, Color::from_rgba(220, 50, 40, 220));
    draw_rectangle_lines(
        left,
        top,
        bar_w,
        bar_h,
        1.5 * s,
        Color::new(0.0, 0.0, 0.0, 0.5),
    );
    // The marker: a little white triangle above the bar.
    let x = at(world.player.airspeed);
    draw_triangle(
        vec2(x, top - 1.0 * s),
        vec2(x - 5.0 * s, top - 9.0 * s),
        vec2(x + 5.0 * s, top - 9.0 * s),
        WHITE,
    );
    let label = Color::from_rgba(235, 235, 225, 230);
    draw_text_on_landscape("SPEED", left, top - 12.0 * s, 16.0 * s, label);
}

/// The cockpit warnings: a blinking word in the middle, a warning lamp
/// and the screen's edge glowing. "OVERSPEED" counts down to the break-up.
fn draw_warning(world: &World) {
    let Some(warning) = world.warning() else {
        return;
    };
    let (w, h) = (screen_width(), screen_height());
    let s = world.bounds.scale();
    let blink = ((get_time() * 4.0) as u64).is_multiple_of(2);
    let (text, colour) = match warning {
        Warning::Overspeed { seconds_left } => (
            format!("OVERSPEED  {:.1}", seconds_left.max(0.0)),
            Color::from_rgba(255, 60, 40, 255),
        ),
        Warning::PullUp => ("PULL UP".to_string(), Color::from_rgba(255, 180, 40, 255)),
    };
    // The screen's edge glows in the warning's colour.
    let alpha = if blink { 0.45 } else { 0.15 };
    let glow = Color::new(colour.r, colour.g, colour.b, alpha);
    let edge = 6.0 * s;
    draw_rectangle(0.0, 0.0, w, edge, glow);
    draw_rectangle(0.0, h - edge, w, edge, glow);
    draw_rectangle(0.0, 0.0, edge, h, glow);
    draw_rectangle(w - edge, 0.0, edge, h, glow);
    // The warning lamp, top left, and the word in the middle.
    // Under the menu button (44 px, top left), left of the speed gauge.
    let lamp = vec2((w * 0.06).max(16.0) + 12.0 * s, 70.0 * s);
    draw_circle(lamp.x, lamp.y, 11.0 * s, Color::new(0.1, 0.1, 0.1, 0.7));
    let dim = Color::new(colour.r * 0.4, colour.g * 0.4, colour.b * 0.4, 1.0);
    draw_circle(lamp.x, lamp.y, 8.0 * s, if blink { colour } else { dim });
    if blink {
        let size = 44.0 * s;
        let dims = measure_text(&text, None, size as u16, 1.0);
        draw_text_on_landscape(&text, (w - dims.width) / 2.0, h * 0.3, size, colour);
    }
}

/// The flight time (minutes and seconds), top centre, all game long.
fn draw_time(world: &World) {
    let scale = world.bounds.scale();
    let text = format!("Time {}", clock_text(world.seconds_in_level()));
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

/// How far the level is done, under the clock: a thin track filling up
/// in gold, a small plane flying along it and a chequered flag at the
/// end, where the landing begins (core's `World::level_progress`).
fn draw_progress(world: &World) {
    let s = world.bounds.scale();
    let width = 200.0 * s;
    let left = (screen_width() - width) / 2.0;
    let y = 56.0 * s;
    let done = world.level_progress();
    let thick = 5.0 * s;
    draw_rectangle(
        left,
        y - thick / 2.0,
        width,
        thick,
        Color::new(0.0, 0.0, 0.0, 0.35),
    );
    draw_rectangle(
        left,
        y - thick / 2.0,
        width * done,
        thick,
        Color::from_rgba(255, 220, 90, 230),
    );
    // The flag: a pole and a small chequered cloth.
    let fx = left + width + 6.0 * s;
    let pole = Color::from_rgba(235, 235, 225, 255);
    draw_line(fx, y + 5.0 * s, fx, y - 9.0 * s, 1.5 * s, pole);
    let cell = 3.0 * s;
    for row in 0..2 {
        for col in 0..3 {
            let white = (row + col) % 2 == 0;
            let colour = if white {
                WHITE
            } else {
                Color::from_rgba(30, 30, 34, 255)
            };
            draw_rectangle(
                fx + col as f32 * cell,
                y - 9.0 * s + row as f32 * cell,
                cell,
                cell,
                colour,
            );
        }
    }
    draw_plane(
        left + width * done,
        y,
        s * 0.42,
        0.0,
        WHITE_PLANE,
        BLUE_PLANE,
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
            0.0,
            Color::new(0.0, 0.0, 0.0, 0.3),
            Color::new(0.0, 0.0, 0.0, 0.3),
        );
        draw_plane(x, y, scale, 0.0, body, accent);
    }
}

/// "Game over", the level's best times (a game over counts no time) and
/// how to fly again.
fn draw_game_over(world: &World, result: Option<&FlightResult>) {
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
    centred("Game over", y, 60.0, gold);
    y += 40.0 * s;
    centred(
        "Only completed levels get a time",
        y,
        22.0,
        Color::from_rgba(200, 210, 225, 255),
    );
    if let Some(r) = result {
        draw_best_times(r, y, s, &centred);
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

/// Under the time on "Level complete" and "Game over" (`y`: its line):
/// "New best time!" if it is one, then the level's best times with this
/// one marked. `centred` draws a line of text across the middle.
fn draw_best_times(r: &FlightResult, mut y: f32, s: f32, centred: &dyn Fn(&str, f32, f32, Color)) {
    let gold = Color::from_rgba(255, 220, 90, 255);
    if r.placed.is_some_and(|p| p.record) {
        y += 30.0 * s;
        // Blinks, to celebrate.
        let on = ((get_time() * 3.0) as u64).is_multiple_of(2);
        let colour = if on {
            gold
        } else {
            Color::from_rgba(255, 150, 60, 255)
        };
        centred("New best time!", y, 26.0, colour);
    }
    // This level's best times, this flight highlighted.
    y += 32.0 * s;
    let label = format!("Best times  Level {}", r.level);
    centred(&label, y, 22.0, Color::from_rgba(200, 210, 225, 255));
    let mine = r.placed.map(|p| p.place);
    if r.times.list().is_empty() {
        y += 22.0 * s;
        centred("none yet", y, 22.0, Color::from_rgba(235, 235, 225, 255));
    }
    for (i, &millis) in r.times.list().iter().enumerate() {
        y += 22.0 * s;
        let colour = if mine == Some(i) {
            gold
        } else {
            Color::from_rgba(235, 235, 225, 255)
        };
        let marker = if mine == Some(i) { "  <" } else { "" };
        let text = format!("{}.  {}{}", i + 1, best_time_text(millis), marker);
        centred(&text, y, 22.0, colour);
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

/// A red warning light that blinks once a second (on real time).
fn draw_warning_light(x: f32, y: f32, s: f32, phase: f64) {
    let on = (get_time() + phase).fract() < 0.5;
    let red = Color::from_rgba(255, 60, 50, 255);
    if on {
        draw_circle(x, y, 7.0 * s, Color::new(1.0, 0.25, 0.2, 0.25));
        draw_circle(x, y, 2.5 * s, red);
    } else {
        draw_circle(x, y, 2.0 * s, Color::from_rgba(110, 30, 28, 255));
    }
}

/// A high-rise at night: dark walls with lit windows, a stair house and
/// an antenna with a blinking red light. Positions from core's
/// `HighRise::parts`.
fn draw_high_rise(b: &HighRise, bounds: Bounds) {
    let p = b.parts(bounds);
    let s = bounds.scale();
    let (l, t, w, h) = p.body;
    let wall = match b.style {
        0 => Color::from_rgba(44, 50, 66, 255),
        1 => Color::from_rgba(58, 54, 60, 255),
        _ => Color::from_rgba(36, 46, 58, 255),
    };
    draw_rectangle(l, t, w, h, wall);
    draw_rectangle_lines(l, t, w, h, 1.5 * s, Color::from_rgba(90, 100, 120, 255));
    // Windows in a grid; which are lit follows from the building alone.
    let (cols, rows) = (
        ((w / (11.0 * s)) as u32).max(2),
        ((h / (13.0 * s)) as u32).max(2),
    );
    let (cw, rh) = (w / cols as f32, h / rows as f32);
    let seed = (b.width * 997.0 + b.height * 7919.0) as u32;
    for r in 0..rows {
        for c in 0..cols {
            let n = (seed ^ (r * 73_856_093) ^ (c * 19_349_663)).wrapping_mul(2_654_435_761);
            let lit = (n >> 13) % 10;
            let colour = match lit {
                0..=3 => continue, // dark
                4..=7 => Color::from_rgba(255, 210, 120, 255),
                8 => Color::from_rgba(255, 235, 190, 255),
                _ => Color::from_rgba(170, 200, 255, 255),
            };
            draw_rectangle(
                l + c as f32 * cw + 0.25 * cw,
                t + r as f32 * rh + 0.25 * rh,
                0.5 * cw,
                0.45 * rh,
                colour,
            );
        }
    }
    let (sl, st, sw, sh) = p.stair_house;
    draw_rectangle(sl, st, sw, sh, wall);
    let (a, tip) = p.antenna;
    draw_line(
        a.x,
        a.y,
        tip.x,
        tip.y,
        p.antenna_width,
        Color::from_rgba(120, 125, 135, 255),
    );
    draw_warning_light(tip.x, tip.y, s, b.width as f64 * 0.01);
    // The helipad: a light circle with an "H" on the roof.
    let pad = b.pad(bounds);
    let pad_w = (0.5 * w).min(40.0 * s);
    draw_rectangle(
        pad.x - pad_w / 2.0,
        pad.y - 1.5 * s,
        pad_w,
        1.5 * s,
        Color::from_rgba(230, 230, 210, 255),
    );
    if let (Some(heli), Some(hp)) = (b.heli, b.heli_parts(bounds)) {
        draw_heli(heli.rotor_on(), heli.age, &hp, s);
    }
}

/// A helicopter: body with a window, tail boom, skids and a rotor that
/// turns while it runs (its span seems to shrink and grow), and a
/// blinking red light under it. Positions from core's `HeliParts`;
/// `age` in ticks turns the rotor.
fn draw_heli(rotor_on: bool, age: u32, p: &HeliParts, s: f32) {
    let body = Color::from_rgba(210, 60, 50, 255);
    let dark = Color::from_rgba(40, 40, 46, 255);
    let (c, hl, hh) = p.body;
    let (a, b) = p.skids;
    draw_line(a.x, a.y, b.x, b.y, 2.0 * s, dark);
    draw_line(a.x + 4.0 * s, a.y, a.x + 4.0 * s, c.y, 1.5 * s, dark);
    draw_line(b.x - 4.0 * s, b.y, b.x - 4.0 * s, c.y, 1.5 * s, dark);
    let (t0, t1) = p.tail;
    draw_line(t0.x, t0.y, t1.x, t1.y, p.tail_width, body);
    draw_circle(t1.x, t1.y, 4.0 * s, Color::new(0.75, 0.75, 0.8, 0.5));
    draw_ellipse(c.x, c.y, hl, hh, 0.0, body);
    draw_ellipse(
        c.x - 0.45 * hl,
        c.y - 0.2 * hh,
        0.4 * hl,
        0.5 * hh,
        0.0,
        Color::from_rgba(170, 210, 240, 255),
    );
    let (r0, r1) = p.rotor;
    let mid = (r0.x + r1.x) / 2.0;
    draw_line(mid, r0.y, mid, c.y - hh, 2.0 * s, dark);
    // A turning rotor looks like its span pulsing; a still one is full.
    let span = if rotor_on {
        ((age as f32 * 0.9).sin() * 0.5 + 0.5).max(0.15)
    } else {
        1.0
    };
    let half = (r1.x - r0.x) / 2.0 * span;
    let blade = if rotor_on {
        Color::new(0.85, 0.85, 0.9, 0.7)
    } else {
        dark
    };
    draw_line(mid - half, r0.y, mid + half, r0.y, 2.0 * s, blade);
    if rotor_on {
        draw_warning_light(c.x, c.y + hh, s * 0.7, 0.3);
    }
}

/// The low morning sun (level 4), fixed in the sky ahead where core puts
/// it (`SUN_AT`): a bright disc in a wide warm glow that breathes a little.
fn draw_sun(bounds: Bounds, ticks: u64) {
    let s = bounds.scale();
    let (x, y) = (SUN_AT.0 * bounds.width, SUN_AT.1 * bounds.height);
    let r = SUN_RADIUS * s;
    let breathe = 1.0 + 0.04 * (ticks as f32 * TICK_SECONDS * 1.3).sin();
    // Many faint rings, one inside the other: a soft glow without edges.
    for i in 0..16 {
        let k = 1.2 + 5.0 * (1.0 - i as f32 / 16.0);
        draw_circle(x, y, r * k * breathe, Color::new(1.0, 0.75, 0.35, 0.035));
    }
    draw_circle(x, y, r, Color::from_rgba(255, 236, 170, 255));
    draw_circle(x, y, r * 0.75, Color::from_rgba(255, 250, 225, 255));
}

/// A high-rise at sunrise (core's `FarHighRise::parts`, an obstacle),
/// like the one in the photo: a dark silhouette with a flat roof that
/// overhangs, a stair house and a mast on it, dim windows, and the sun's
/// light along its roof and right edge.
fn draw_far_high_rise(b: &FarHighRise, bounds: Bounds) {
    let s = bounds.scale();
    let p = b.parts(bounds);
    let (l, t, w, h) = p.walls;
    let dark = Color::from_rgba(26, 16, 10, 255);
    let rim = Color::from_rgba(246, 160, 64, 150);
    draw_rectangle(l, t, w, h, dark);
    let (rl, rt, rw, rh) = p.roof;
    draw_rectangle(rl, rt, rw, rh, dark);
    let (hl, ht, hw, hh) = p.stair_house;
    draw_rectangle(hl, ht, hw, hh, dark);
    let (m0, m1) = p.mast;
    draw_line(m0.x, m0.y, m1.x, m1.y, p.mast_width, dark);
    draw_line(rl, rt, rl + rw, rt, 2.0 * s, rim);
    draw_line(
        l + w,
        t,
        l + w,
        t + h,
        2.0 * s,
        Color::new(0.96, 0.63, 0.25, 0.3),
    );
    // Windows: a dim grid, a few lit.
    let (ww, wh) = (12.0 * s, 6.0 * s);
    let cols = ((w / (22.0 * s)) as i32).max(3);
    let rows = ((h / (34.0 * s)) as i32).max(1);
    for row in 0..rows {
        for col in 0..cols {
            let x = l + (col as f32 + 0.5) * w / cols as f32 - ww / 2.0;
            let y = t + 20.0 * s + row as f32 * 34.0 * s;
            let on = (col * 5 + row * 3 + b.width as i32) % 7 == 0;
            let colour = if on {
                Color::from_rgba(255, 190, 110, 200)
            } else {
                Color::from_rgba(58, 36, 20, 255)
            };
            draw_rectangle(x, y, ww, wh, colour);
        }
    }
}

/// The sun's glare (core's `Sunrise::glare`, 0..1): the screen washes out
/// in warm light, brightest around the sun, so what comes is harder to
/// see (but never gone).
fn draw_glare(glare: f32, bounds: Bounds) {
    if glare <= 0.0 {
        return;
    }
    let (w, h) = (bounds.width, bounds.height);
    draw_rectangle(0.0, 0.0, w, h, Color::new(1.0, 0.85, 0.6, 0.42 * glare));
    let (x, y) = (SUN_AT.0 * w, SUN_AT.1 * h);
    for i in 0..20 {
        let k = 0.05 + 0.9 * (1.0 - i as f32 / 20.0);
        draw_circle(x, y, w * k, Color::new(1.0, 0.95, 0.8, 0.03 * glare));
    }
}

/// A column of steam from a chimney in the valley (core's `Steam::rect`):
/// soft white billows that rise and drift, thinning toward the top, with
/// the chimney at its foot.
fn draw_steam(c: &Steam, bounds: Bounds) {
    let s = bounds.scale();
    let (l, t, w, h) = c.rect(bounds);
    let cx = l + w / 2.0;
    let ground = t + h; // the chimney's top
                        // Billows rise through the column, round and round.
    let rise = c.age as f32 * TICK_SECONDS * 40.0 * s;
    let step = 16.0 * s;
    let count = (h / step).ceil() as i32 + 1;
    for i in 0..count {
        let y = ground - ((i as f32 * step + rise) % (h + step));
        if y < t - step / 2.0 {
            continue;
        }
        let up = ((ground - y) / h).clamp(0.0, 1.0); // 0 at the foot, 1 at the top
        let sway = (i as f32 * 1.7 + c.age as f32 * 0.03).sin() * w * 0.12;
        let r = w * (0.32 + 0.22 * up);
        let alpha = 0.5 * (1.0 - up * 0.7);
        draw_circle(cx + sway, y, r, Color::new(1.0, 0.96, 0.92, alpha));
    }
    draw_house(c, bounds);
}

/// The small house under a column of steam (core's `Steam::house_parts`):
/// dark walls in the shade with a few warm windows, a pitched roof lit
/// orange along its edge by the sun, and the brick chimney.
fn draw_house(c: &Steam, bounds: Bounds) {
    let s = bounds.scale();
    let p = c.house_parts(bounds);
    let (l, t, w, h) = p.walls;
    draw_rectangle(l, t, w, h, Color::from_rgba(58, 40, 30, 255));
    // Windows: two rows, some lit.
    let lit = Color::from_rgba(255, 196, 110, 255);
    let dark = Color::from_rgba(36, 26, 22, 255);
    let (ww, wh) = (8.0 * s, 7.0 * s);
    let cols = ((w / (16.0 * s)) as i32).max(2);
    for row in 0..2 {
        let y = t + h * (0.25 + 0.4 * row as f32);
        for col in 0..cols {
            let x = l + (col as f32 + 0.5) * w / cols as f32 - ww / 2.0;
            let on = (col * 7 + row * 3 + (c.house.0 as i32)) % 3 == 0;
            draw_rectangle(x, y, ww, wh, if on { lit } else { dark });
        }
    }
    let (cl, ct, cw, ch) = p.chimney;
    draw_rectangle(cl, ct, cw, ch, Color::from_rgba(120, 58, 40, 255));
    draw_rectangle(
        cl - 1.0 * s,
        ct,
        cw + 2.0 * s,
        3.0 * s,
        Color::from_rgba(80, 40, 30, 255),
    );
    let roof = Color::from_rgba(92, 46, 34, 255);
    draw_triangle(
        vec2(l - 3.0 * s, t),
        vec2(p.ridge.x, p.ridge.y),
        vec2(l + w + 3.0 * s, t),
        roof,
    );
    // The sun catches the roof's edge.
    let edge = Color::from_rgba(255, 170, 80, 200);
    draw_line(l - 3.0 * s, t, p.ridge.x, p.ridge.y, 2.0 * s, edge);
    draw_line(
        p.ridge.x,
        p.ridge.y,
        l + w + 3.0 * s,
        t,
        1.5 * s,
        Color::new(1.0, 0.67, 0.31, 0.45),
    );
    // (The chimney in front of the roof.)
    draw_rectangle(cl, ct, cw, ch * 0.6, Color::from_rgba(120, 58, 40, 255));
}

/// A goose flying left (core's `Goose::shape`): grey-brown body, a long
/// black neck and head with a white chin strap, wings beating slowly; a
/// struck one tumbles.
fn draw_goose(g: &Goose, s: f32) {
    let (sin, cos) = g.tumble().sin_cos();
    let c = g.pos;
    let turn = |p: airplane_scoring_core::Vec2| {
        let (dx, dy) = (p.x - c.x, p.y - c.y);
        vec2(c.x + dx * cos - dy * sin, c.y + dx * sin + dy * cos)
    };
    let body = Color::from_rgba(120, 104, 88, 255);
    let dark = Color::from_rgba(28, 26, 26, 255);
    let wing = Color::from_rgba(96, 82, 70, 255);
    let tip = turn(g.wing_tip(s));
    draw_line(c.x, c.y, tip.x, tip.y, 4.0 * s, wing);
    draw_ellipse(c.x, c.y, 11.0 * s, 4.5 * s, g.tumble().to_degrees(), body);
    let neck = turn(airplane_scoring_core::Vec2::new(c.x - 6.6 * s, c.y));
    let head = turn(g.head(s));
    draw_line(neck.x, neck.y, head.x, head.y, 3.0 * s, dark);
    draw_circle(head.x, head.y, 3.0 * s, dark);
    draw_circle(
        head.x + 1.2 * s,
        head.y + 1.0 * s,
        1.2 * s,
        Color::from_rgba(240, 236, 228, 255),
    );
    let beak = turn(airplane_scoring_core::Vec2::new(
        g.head(s).x - 4.5 * s,
        g.head(s).y,
    ));
    draw_line(head.x, head.y, beak.x, beak.y, 1.6 * s, dark);
    // The white tail under the back.
    let tail = turn(airplane_scoring_core::Vec2::new(
        c.x + 11.0 * s,
        c.y + 1.0 * s,
    ));
    draw_circle(
        tail.x,
        tail.y,
        2.0 * s,
        Color::from_rgba(235, 232, 226, 255),
    );
}

/// A crashed helicopter: dark and tumbling as it falls, inside a
/// fireball that swells and burns out. Positions from core's `Wreck`.
fn draw_wreck(w: &Wreck, s: f32) {
    let t = w.progress();
    let (c, hl, hh) = HeliParts::at(w.pos, s).body;
    let spin = t * 7.0;
    let (sin, cos) = spin.sin_cos();
    let charred = Color::new(0.16, 0.12, 0.11, 1.0 - t * 0.5);
    draw_line(
        c.x - cos * hl * 1.6,
        c.y - sin * hl * 1.6,
        c.x + cos * hl * 1.6,
        c.y + sin * hl * 1.6,
        2.0 * hh,
        charred,
    );
    draw_explosion(c.x, c.y, s * 0.8, t);
}

/// A tower crane standing on the ground: the lattice mast down to its
/// foot, the jib with the cab, the counterweight, the cable and its load,
/// a steel beam. All of it can be hit. Positions from core's
/// `Crane::parts`.
fn draw_crane(k: &Crane, bounds: Bounds) {
    let p = k.parts(bounds);
    let s = bounds.scale();
    let yellow = Color::from_rgba(230, 180, 60, 255);
    // The mast: a yellow lattice like the jib, on a concrete foot.
    let (top, foot) = p.mast;
    let half = p.mast_width / 2.0;
    draw_rectangle(
        foot.x - 2.0 * half,
        foot.y - 6.0 * s,
        4.0 * half,
        6.0 * s,
        Color::from_rgba(120, 122, 128, 255),
    );
    draw_line(top.x - half, top.y, foot.x - half, foot.y, 1.8 * s, yellow);
    draw_line(top.x + half, top.y, foot.x + half, foot.y, 1.8 * s, yellow);
    let mut y = top.y;
    let mut left = true;
    while y < foot.y {
        let next = (y + p.mast_width).min(foot.y);
        let (a, b) = if left { (-half, half) } else { (half, -half) };
        draw_line(top.x + a, y, top.x + b, next, 1.2 * s, yellow);
        (y, left) = (next, !left);
    }
    // The cab under the jib.
    draw_rectangle(
        top.x - 7.0 * s,
        top.y,
        14.0 * s,
        9.0 * s,
        Color::from_rgba(200, 160, 60, 255),
    );
    // The jib: two chords and zigzag braces.
    let (jl, jt, jw, jh) = p.jib;
    draw_rectangle(jl, jt, jw, jh, Color::new(0.9, 0.7, 0.24, 0.18));
    draw_line(jl, jt, jl + jw, jt, 2.0 * s, yellow);
    draw_line(jl, jt + jh, jl + jw, jt + jh, 2.0 * s, yellow);
    let step = jh;
    let mut x = jl;
    let mut up = true;
    while x < jl + jw {
        let next = (x + step).min(jl + jw);
        let (y0, y1) = if up { (jt + jh, jt) } else { (jt, jt + jh) };
        draw_line(x, y0, next, y1, 1.2 * s, yellow);
        (x, up) = (next, !up);
    }
    let (cl, ct, cw, ch) = p.counterweight;
    draw_rectangle(cl, ct, cw, ch, Color::from_rgba(130, 134, 140, 255));
    draw_rectangle_lines(cl, ct, cw, ch, 1.0 * s, Color::from_rgba(80, 84, 90, 255));
    // The cable and the load: a steel beam with its flanges.
    let (a, b) = p.cable;
    draw_line(
        a.x,
        a.y,
        b.x,
        b.y,
        p.cable_width,
        Color::from_rgba(190, 195, 205, 255),
    );
    let (ll, lt, lw, lh) = p.load;
    let steel = Color::from_rgba(196, 82, 46, 255);
    let flange = Color::from_rgba(150, 56, 32, 255);
    draw_rectangle(ll, lt, lw, lh, steel);
    draw_rectangle(ll, lt, lw, lh * 0.25, flange);
    draw_rectangle(ll, lt + lh * 0.75, lw, lh * 0.25, flange);
    draw_warning_light(jl + 3.0 * s, jt + jh / 2.0, s, k.reach as f64 * 0.01);
}

/// The airport the flight starts from: a control tower and a windsock by
/// the start of the runway, and the runway itself, asphalt with threshold
/// stripes and a dashed centre line. Positions from core's `Airport`.
fn draw_airport(airport: &Airport, bounds: Bounds, ticks: u64, lit: bool) {
    let s = bounds.scale();
    let ground = Airport::ground(bounds);
    let top = Airport::runway_top(bounds);
    let (x, len, h) = (airport.x, RUNWAY_LENGTH * s, RUNWAY_HEIGHT * s);
    // Control tower behind the runway's start: a column with a glass cab.
    let tx = x - 30.0 * s;
    let concrete = Color::from_rgba(214, 214, 206, 255);
    draw_rectangle(
        tx - 9.0 * s,
        ground - 78.0 * s,
        18.0 * s,
        78.0 * s,
        concrete,
    );
    draw_rectangle(
        tx - 18.0 * s,
        ground - 96.0 * s,
        36.0 * s,
        18.0 * s,
        concrete,
    );
    draw_rectangle(
        tx - 15.0 * s,
        ground - 93.0 * s,
        30.0 * s,
        11.0 * s,
        Color::from_rgba(70, 110, 140, 255),
    );
    draw_rectangle(
        tx - 20.0 * s,
        ground - 100.0 * s,
        40.0 * s,
        4.0 * s,
        Color::from_rgba(150, 40, 36, 255),
    );
    // Windsock on a pole, fluttering a little.
    let wx = x - 62.0 * s;
    let pole_top = ground - 46.0 * s;
    draw_line(
        wx,
        ground,
        wx,
        pole_top,
        2.0 * s,
        Color::from_rgba(90, 90, 90, 255),
    );
    let flutter = ((ticks as f32) * 0.25).sin() * 2.0 * s;
    for (i, colour) in [
        (0, Color::from_rgba(240, 120, 30, 255)),
        (1, WHITE),
        (2, Color::from_rgba(240, 120, 30, 255)),
    ] {
        let a = i as f32 * 8.0 * s;
        let b = a + 8.0 * s;
        let half = |d: f32| 5.0 * s * (1.0 - d / (26.0 * s));
        draw_triangle(
            vec2(wx - a, pole_top + 1.0 * s - half(a) + 5.0 * s),
            vec2(
                wx - b,
                pole_top + 1.0 * s - half(b) + 5.0 * s + flutter * (i as f32) / 2.0,
            ),
            vec2(wx - a, pole_top + 1.0 * s + half(a) + 5.0 * s),
            colour,
        );
    }
    // The runway.
    draw_rectangle(x, top, len, h, Color::from_rgba(64, 66, 70, 255));
    draw_rectangle(x, top, len, 1.5 * s, Color::from_rgba(120, 122, 126, 255));
    for i in 0..6 {
        let bx = x + (6.0 + i as f32 * 7.0) * s;
        draw_rectangle(bx, top + 2.0 * s, 4.0 * s, h - 4.0 * s, WHITE);
    }
    let mut dash = x + 70.0 * s;
    while dash < x + len - 20.0 * s {
        draw_rectangle(dash, top + h / 2.0 - 1.0 * s, 20.0 * s, 2.0 * s, WHITE);
        dash += 40.0 * s;
    }
    if lit {
        draw_runway_lights(x, top, len, h, s, ticks);
    }
}

/// Runway lights for the dark levels: warm white lights along both edges,
/// white ones on the centre line between the dashes, green at the start,
/// red at the end, each with a soft glow; the centre line's lights
/// sparkle a little.
fn draw_runway_lights(x: f32, top: f32, len: f32, h: f32, s: f32, ticks: u64) {
    let light = |lx: f32, ly: f32, colour: Color, r: f32| {
        draw_circle(
            lx,
            ly,
            r * 3.2 * s,
            Color::new(colour.r, colour.g, colour.b, 0.18),
        );
        draw_circle(lx, ly, r * s, colour);
    };
    let edge = Color::from_rgba(255, 226, 160, 255);
    let mut lx = x + 4.0 * s;
    while lx < x + len {
        light(lx, top + 0.8 * s, edge, 1.6);
        light(lx, top + h - 0.8 * s, edge, 1.6);
        lx += 30.0 * s;
    }
    // Centre line: between the dashes (which start 70 px in, every 40 px).
    let mut cx = x + 60.0 * s;
    let mut i = 0u64;
    while cx < x + len - 10.0 * s {
        let on = !(ticks / 6 + i).is_multiple_of(9);
        if on {
            light(cx, top + h / 2.0, WHITE, 1.3);
        }
        cx += 40.0 * s;
        i += 1;
    }
    // Threshold (green) and end (red), across the runway.
    for k in 0..4 {
        let ly = top + (1.5 + k as f32 * (h - 3.0) / 3.0 / s) * s;
        light(x + 1.0 * s, ly, Color::from_rgba(80, 255, 120, 255), 1.6);
        light(
            x + len - 1.0 * s,
            ly,
            Color::from_rgba(255, 70, 60, 255),
            1.6,
        );
    }
}

/// A puff of tyre smoke where a wheel touched the runway: a few grey
/// clouds that grow, rise a little and fade (core's `TyreSmoke`).
fn draw_tyre_smoke(puff: &TyreSmoke, s: f32) {
    let t = puff.progress();
    let fade = (1.0 - t).powi(2);
    let clouds = [
        (-5.0, -3.0, 9.0),
        (-16.0, -5.0, 7.5),
        (4.0, -2.0, 6.0),
        (-27.0, -8.0, 6.0),
        (-10.0, -10.0, 5.5),
    ];
    for (i, (dx, dy, r)) in clouds.into_iter().enumerate() {
        let grow = 0.6 + 2.4 * t.sqrt();
        let drift = -(10.0 + 5.0 * i as f32) * t; // backwards along the runway
        draw_circle(
            puff.pos.x + (dx + drift) * s,
            puff.pos.y + (dy - 12.0 * t) * s,
            r * grow * s,
            Color::new(0.88, 0.88, 0.9, 0.8 * fade),
        );
    }
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
    // A burst balloon: the envelope burns, shrivels and goes dark while
    // the whole thing falls (core moves it); a fireball at first.
    let t = balloon.burst_seconds();
    let burst = balloon.is_burst();
    let (wide, tall) = if burst {
        ((1.0 - 1.6 * t).max(0.3), (1.0 - 1.2 * t).max(0.45))
    } else {
        (1.0, 1.0)
    };
    let char = if burst { (2.0 * t).min(0.8) } else { 0.0 };
    let burnt = |c: Color| {
        let k = 1.0 - char;
        Color::new(
            c.r * k + 0.12 * char,
            c.g * k + 0.1 * char,
            c.b * k + 0.08 * char,
            1.0,
        )
    };
    let (main, stripe) = (burnt(main), burnt(stripe));
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
    // (A burst one hangs shrivelled from its mouth.)
    let top = neck_y - (neck_y - y) * tall;
    for (w, colour) in [(1.0, main), (0.62, stripe), (0.3, main)] {
        let w = w * wide;
        draw_ellipse(x, top, r * w, r * 1.05 * tall, 0.0, colour);
        draw_triangle(
            vec2(x - r * w * 0.93, top + r * 0.35 * tall),
            vec2(x + r * w * 0.93, top + r * 0.35 * tall),
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
    if burst {
        draw_balloon_fire(x, top, r, t, s);
    }
}

/// The burst: a fireball where the envelope was, white-hot, then orange
/// and red as it swells and fades (`t` seconds after the burst); later
/// just smoke rising from the falling wreck.
fn draw_balloon_fire(x: f32, y: f32, r: f32, t: f32, s: f32) {
    let ball = (t / 0.5).min(1.0);
    if ball < 1.0 {
        let fade = 1.0 - ball;
        let size = r * (0.7 + 1.1 * ball);
        draw_circle(x, y, size, Color::new(0.85, 0.2, 0.08, 0.7 * fade));
        draw_circle(x, y, size * 0.72, Color::new(1.0, 0.55, 0.12, 0.85 * fade));
        draw_circle(x, y, size * 0.4, Color::new(1.0, 0.95, 0.75, fade));
    }
    // Smoke and a few flames on the falling wreck.
    for i in 0..5 {
        let k = i as f32;
        let up = (t * 60.0 + k * 9.0) % 40.0;
        let alpha = 0.5 * (1.0 - up / 40.0) * (t / 0.3).min(1.0);
        let px = x + (k - 2.0) * 5.0 * s;
        draw_circle(
            px,
            y - up * s,
            (4.0 + up * 0.25) * s,
            Color::new(0.2, 0.2, 0.22, alpha),
        );
    }
    let flicker = 0.7 + 0.3 * (t * 40.0).sin();
    draw_circle(x, y, 6.0 * s * flicker, Color::new(1.0, 0.6, 0.15, 0.9));
}

/// A gull seen from the side, centred on its position, facing where it
/// flies. The wings beat; their size matches core's `GULL_RADIUS`, times
/// the screen's `scale`.
fn draw_gull(gull: &Gull, scale: f32) {
    let (x, y) = (gull.pos.x, gull.pos.y);
    let dir = gull.facing(); // -1 = facing left
                             // A struck gull tumbles: everything turns around its centre.
    let (sin, cos) = gull.tumble().sin_cos();
    // A point (dx, dy) design px from the gull's centre.
    let p = |dx: f32, dy: f32| {
        let (dx, dy) = (dx * scale, dy * scale);
        vec2(x + dx * cos - dy * sin, y + dx * sin + dy * cos)
    };
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
        // The dark tip: the wing's last part (turns along when tumbling).
        let from = bend + (end - bend) * 0.7;
        draw_line(from.x, from.y, end.x, end.y, 3.0 * scale, tip);
    }
    // Body, head and a yellow beak towards the flight direction.
    draw_ellipse(
        x,
        y,
        9.0 * scale,
        4.0 * scale,
        gull.tumble().to_degrees(),
        body,
    );
    let head = p(dir * 8.0, -2.0);
    draw_circle(head.x, head.y, 3.5 * scale, body);
    draw_triangle(
        p(dir * 11.0, -3.0),
        p(dir * 16.0, -1.5),
        p(dir * 11.0, -0.5),
        Color::from_rgba(240, 180, 40, 255),
    );
}

/// The damage smoke trailing from the tail: `smoke` 0 (none) .. 1 (thick
/// and dark, with a little flame at the tail). Where each puff is
/// follows from the tick count, so there is no state to keep.
fn draw_damage_smoke(x: f32, y: f32, size: f32, smoke: f32, ticks: u64) {
    if smoke <= 0.0 {
        return;
    }
    let tail = vec2(x - 24.0 * size, y - 2.0 * size);
    // Many puffs in a long trail; more, bigger and darker with each
    // life lost.
    let puffs = (14.0 + 30.0 * smoke) as u64;
    let grey = 0.7 - 0.45 * smoke; // darker when worse
    for i in 0..puffs {
        let period = 60;
        let age = ((ticks + i * period / puffs) % period) as f32 / period as f32;
        // Puffs drift back with the wind and rise a little, growing and fading.
        let wobble = ((i * 37 % 11) as f32 - 5.0) * (1.0 + smoke);
        let px = tail.x - (8.0 + (120.0 + 80.0 * smoke) * age) * size;
        let py = tail.y - (3.0 + (16.0 + 14.0 * smoke) * age + wobble * age) * size;
        let r = (3.5 + 3.0 * smoke + (7.0 + 13.0 * smoke) * age) * size;
        let alpha = (0.35 + 0.4 * smoke) * (1.0 - age);
        draw_circle(px, py, r, Color::new(grey, grey, grey + 0.02, alpha));
    }
    if smoke >= 1.0 {
        // A small flickering flame at the tail.
        let flicker = 0.7 + 0.3 * ((ticks as f32) * 0.9).sin();
        draw_circle(
            tail.x,
            tail.y,
            3.5 * size * flicker,
            Color::from_rgba(255, 150, 40, 220),
        );
        draw_circle(
            tail.x,
            tail.y,
            2.0 * size * flicker,
            Color::from_rgba(255, 230, 120, 240),
        );
    }
}

/// The plane blowing up at (x, y): `t` from 0 (the moment) to 1 (over).
/// A fireball that swells and cools from white to red, debris flying out
/// and falling, and a dark smoke cloud that grows and fades.
fn draw_explosion(x: f32, y: f32, size: f32, t: f32) {
    let t = t.clamp(0.0, 1.0);
    // Smoke cloud, behind everything.
    for i in 0..7 {
        let a = i as f32 * std::f32::consts::TAU / 7.0;
        let d = (8.0 + 30.0 * t) * size;
        let r = (10.0 + 30.0 * t) * size;
        let alpha = 0.55 * (t * 3.0).min(1.0) * (1.0 - t);
        draw_circle(
            x + a.cos() * d,
            y + a.sin() * d * 0.6 - 12.0 * t * size,
            r,
            Color::new(0.22, 0.2, 0.2, alpha),
        );
    }
    // Debris: pieces of the plane flying out, pulled down.
    for i in 0..12 {
        let a = i as f32 * 2.399; // golden angle: spread all round
        let speed = (60.0 + (i * 23 % 40) as f32) * size;
        let dx = a.cos() * speed * t;
        let dy = a.sin() * speed * t + 90.0 * size * t * t;
        let colour = if i % 3 == 0 { BLUE_PLANE } else { WHITE_PLANE };
        let s = (3.0 - 1.5 * t) * size;
        draw_rectangle(
            x + dx - s,
            y + dy - s,
            2.0 * s,
            2.0 * s,
            Color::new(colour.r, colour.g, colour.b, 1.0 - t * 0.6),
        );
    }
    // Fireball: big quickly, then shrinking and cooling.
    let swell = (t * 5.0).min(1.0);
    let fire = (1.0 - t).powf(1.5);
    let r = (12.0 + 26.0 * swell) * size * (0.4 + 0.6 * fire);
    draw_circle(x, y, r, Color::new(0.85, 0.2, 0.08, 0.8 * fire));
    draw_circle(x, y, r * 0.72, Color::new(1.0, 0.55, 0.1, 0.9 * fire));
    draw_circle(x, y, r * 0.42, Color::new(1.0, 0.92, 0.6, fire));
}

/// Side view of a small plane flying right, centred on (x, y), at the
/// screen's `scale`. `crashed`: the tick count while it is out of control.
fn draw_player(x: f32, y: f32, scale: f32, pitch: f32, crashed: Option<u64>, gear: f32) {
    if gear > 0.02 {
        // Landing gear: a main and a nose strut with wheels, their bottoms
        // `WHEELS_BELOW` under the centre (where core puts the runway).
        let (sin, cos) = pitch.sin_cos();
        let p = |dx: f32, dy: f32| {
            let (dx, dy) = (dx * scale, dy * scale);
            vec2(x + dx * cos - dy * sin, y + dx * sin + dy * cos)
        };
        let dark = Color::from_rgba(40, 42, 48, 255);
        // Folding up: the struts get shorter and the wheels smaller.
        let wheel_r = WHEEL_RADIUS * gear.max(0.5);
        let reach = 4.0 + (STRUT_REACH - 4.0) * gear;
        for gx in [MAIN_WHEEL_X, NOSE_WHEEL_X] {
            let (top, hub) = (p(gx, 4.0), p(gx, reach));
            draw_line(top.x, top.y, hub.x, hub.y, 1.5 * scale, dark);
            draw_circle(hub.x, hub.y, wheel_r * scale, dark);
        }
    }
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
    draw_plane(x, y, scale, pitch, body, accent);
}

/// The plane's shape, flying right, centred on (x, y), `scale` times its
/// size on a phone (the player's plane, and the lives top right).
fn draw_plane(x: f32, y: f32, scale: f32, pitch: f32, body: Color, accent: Color) {
    // The same parts core uses for crashes (`Player::shape`), turned by
    // the pitch around the centre.
    let (sin, cos) = pitch.sin_cos();
    let p = |(dx, dy): (f32, f32)| {
        let (dx, dy) = (dx * scale, dy * scale);
        vec2(x + dx * cos - dy * sin, y + dx * sin + dy * cos)
    };
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
