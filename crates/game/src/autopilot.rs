//! Test builds only: core's demo pilot (`replay::DemoPilot`) flies the
//! real game in the browser, so a script can watch a whole ranked flight
//! end to end (`scripts/stage.sh e2e-browser`). On with
//! `--features autopilot`; without it `ON` is false, `new_flight` makes no
//! pilot, so every call here does nothing and the release build leaves
//! the pilot out.
//!
//! The pilot flies until "Level complete" and stays there. After a game
//! over it starts a new flight. It tells the console what happens, as
//! `autopilot: ...` lines ("flight 2", "level 1 complete in 61450 ms",
//! then the online line the game draws, e.g. "World rank 3 of 12").

use airplane_scoring_core::replay::DemoPilot;
use airplane_scoring_core::World;

/// Built with the autopilot.
pub const ON: bool = cfg!(feature = "autopilot");

#[derive(Default)]
pub struct Autopilot {
    pilot: Option<DemoPilot>,
    /// Flights started: each gets the next pilot, so a restart differs.
    flights: u64,
    /// The last line told, so each is told once.
    told: Option<String>,
}

impl Autopilot {
    /// A new world: a new pilot for it.
    pub fn new_flight(&mut self) {
        if ON {
            self.pilot = Some(DemoPilot::new(self.flights));
            self.flights += 1;
            self.told = None;
            macroquad::logging::info!("autopilot: flight {}", self.flights);
        }
    }

    /// The pilot's steering and throttle, if it flies. At "Level
    /// complete" it only waits (no look-ahead: nothing to steer).
    pub fn controls(&mut self, world: &World) -> Option<(f32, f32)> {
        // Checked here too, so the compiler drops the pilot from builds
        // without it (6 KB of wasm otherwise).
        if !ON {
            return None;
        }
        if world.level_complete().is_some() {
            return self.pilot.as_ref().map(|_| (0.0, 0.0));
        }
        Some(self.pilot.as_mut()?.controls(world))
    }

    /// Game over: the pilot starts a new flight (a new seed, if ranked).
    pub fn wants_restart(&self, world: &World) -> bool {
        ON && world.can_restart()
    }

    /// Say the line on the console, once. A closure, so builds without
    /// the autopilot do not even make the text.
    pub fn tell(&mut self, line: impl FnOnce() -> String) {
        if !ON {
            return;
        }
        let line = line();
        if self.told.as_ref() != Some(&line) {
            macroquad::logging::info!("autopilot: {}", line);
            self.told = Some(line);
        }
    }
}
