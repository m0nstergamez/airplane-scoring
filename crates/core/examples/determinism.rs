//! Flies a fixed set of demo flights natively and prints one line per
//! flight: what it was, a hash of its replay and what flying the replay
//! again gives. `scripts/check-determinism.sh` compares this with the
//! same flights in the browser's target (`determinism_wasm`, run by
//! Node): the leaderboard server (native) must fly every browser replay
//! (wasm32-unknown-unknown) to the same time.
//!
//! The demo pilot decides from look-ahead copies of the world, so a
//! single different bit of maths changes its inputs, and with them the
//! replay's hash, long before it changes a level's time.

#[path = "common/determinism.rs"]
mod flights;

fn main() {
    print!("{}", flights::report());
}
