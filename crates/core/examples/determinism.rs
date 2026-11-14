//! Flies a fixed set of demo flights and prints one line per flight: what
//! it was, a hash of its replay and what flying the replay again gives.
//! `scripts/check-determinism.sh` runs it natively and as wasm (wasmtime)
//! and fails if the two print anything different: the leaderboard server
//! (native) must fly every browser replay (wasm) to the same time.
//!
//! The demo pilot decides from look-ahead copies of the world, so a
//! single different bit of maths changes its inputs, and with them the
//! replay's hash, long before it changes a level's time.
//!
//! Usage: determinism [FLIGHTS]   (default: all; fewer for a quick look)

use airplane_scoring_core::replay::{demo_flight, replay, Start};
use airplane_scoring_core::{Bounds, LEVEL_COUNT};

/// Screens: a phone in landscape, a laptop, a narrow window, a tablet.
const SIZES: [(f32, f32); 4] = [
    (844.0, 390.0),
    (1280.0, 720.0),
    (500.0, 700.0),
    (1024.0, 768.0),
];

/// Long enough to land level 1 and fly on into the next.
const STEPS: u64 = 6_000;

/// FNV-1a: a stable hash, no dependency.
fn fnv(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}

fn main() {
    let limit: usize = std::env::args()
        .nth(1)
        .and_then(|a| a.parse().ok())
        .unwrap_or(usize::MAX);
    let mut flights = Vec::new();
    for level in 1..=LEVEL_COUNT {
        for (i, &(width, height)) in SIZES.iter().enumerate() {
            let seed = 1_000 + u64::from(level) * 10 + i as u64;
            flights.push((seed, level, width, height));
        }
    }
    for (n, &(seed, level, width, height)) in flights.iter().take(limit).enumerate() {
        let start = Start {
            seed,
            level,
            bounds: Bounds { width, height },
        };
        let bytes = demo_flight(&start, n as u64, STEPS);
        let outcome = replay(&bytes).expect("a demo flight is a valid replay");
        let times: Vec<String> = outcome
            .completed
            .iter()
            .map(|t| format!("{}:{}", t.level, t.millis))
            .collect();
        println!(
            "seed {seed} level {level} {width}x{height}: replay {:016x}, {} ticks, over {}, landed [{}]",
            fnv(&bytes),
            outcome.steps,
            outcome.over,
            times.join(" ")
        );
    }
}
