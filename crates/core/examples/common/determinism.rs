//! The flights behind the determinism check, shared by the native
//! example (`determinism`) and the browser-target one
//! (`determinism_wasm`). See `scripts/check-determinism.sh`.

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

/// One line per flight: what it was, a hash of its replay and what flying
/// the replay again gives.
pub fn report() -> String {
    let mut flights = Vec::new();
    for level in 1..=LEVEL_COUNT {
        for (i, &(width, height)) in SIZES.iter().enumerate() {
            let seed = 1_000 + u64::from(level) * 10 + i as u64;
            flights.push((seed, level, width, height));
        }
    }
    let mut out = String::new();
    for (n, &(seed, level, width, height)) in flights.iter().enumerate() {
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
        out += &format!(
            "seed {seed} level {level} {width}x{height}: replay {:016x}, {} ticks, over {}, landed [{}]\n",
            fnv(&bytes),
            outcome.steps,
            outcome.over,
            times.join(" ")
        );
    }
    out
}
