//! A replay of a flight that lands its first level, flown by core's demo
//! pilot: for testing a leaderboard server from the outside
//! (`scripts/stage.sh e2e` builds this for wasm32-wasip1, so the replay
//! is made by the same wasm maths as in the browser).
//!
//! Usage: demo_replay SEED LEVEL WIDTH HEIGHT > replay.bin
//! Prints "steps millis" to stderr: the ticks the flight ran (the real
//! time it must have taken, 60 a second) and the first level's time.

use airplane_scoring_core::replay::{demo_flight, replay, Start};
use airplane_scoring_core::Bounds;
use std::io::Write;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let number = |i: usize| -> f64 {
        args.get(i).and_then(|a| a.parse().ok()).unwrap_or_else(|| {
            eprintln!("usage: demo_replay SEED LEVEL WIDTH HEIGHT > replay.bin");
            std::process::exit(2)
        })
    };
    let start = Start {
        seed: args[0].parse().unwrap_or_else(|_| {
            eprintln!("SEED must be a whole number");
            std::process::exit(2)
        }),
        level: number(1) as u32,
        bounds: Bounds {
            width: number(2) as f32,
            height: number(3) as f32,
        },
    };
    // The shortest flight that lands: less real time to wait out.
    for pilot in 0..20 {
        for max_steps in (3_000..=8_000).step_by(200) {
            let bytes = demo_flight(&start, pilot, max_steps);
            let Ok(outcome) = replay(&bytes) else {
                continue;
            };
            if let Some(first) = outcome.completed.first() {
                eprintln!("{} {}", outcome.steps, first.millis);
                std::io::stdout()
                    .write_all(&bytes)
                    .expect("write the replay");
                return;
            }
        }
    }
    eprintln!("no demo pilot landed a level on this seed");
    std::process::exit(1);
}
