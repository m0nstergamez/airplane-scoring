//! The online leaderboard, from the game's side. Its score is the
//! game's own: the time of a completed level. A time counts there only
//! if the server can fly it again, so a ranked flight starts from a seed
//! the server handed out and every input is written down
//! (`core::replay`). At every "Level complete" the replay so far goes to
//! the server; a game over or a flight left early sends nothing, since
//! it has no time.
//!
//! The page's "board" plugin (`web/index.html`) does the talking: it
//! keeps a seed ready for each start level, sends the replay and
//! remembers the rank that comes back. No seed ready (offline, no
//! server): the flight is flown as ever and simply not ranked. Native
//! builds are never ranked.

use airplane_scoring_core::replay::{playable_bounds, rounded_input, Recorder, Start};
use airplane_scoring_core::{Bounds, Input, World};

#[cfg(target_arch = "wasm32")]
mod js {
    // Provided by the page at instantiation (same pattern as tilt_js.rs).
    #[link(wasm_import_module = "env")]
    extern "C" {
        fn board_ready(level: u32) -> i32;
        fn board_seed_hi(level: u32) -> u32;
        fn board_seed_lo(level: u32) -> u32;
        fn board_take(level: u32);
        fn board_submit(ptr: *const u8, len: u32, level: u32);
        fn board_status(level: u32) -> i32;
        fn board_rank(level: u32) -> u32;
        fn board_pilots(level: u32) -> u32;
    }

    /// miniquad checks this against the JS plugin's `version`.
    #[no_mangle]
    pub extern "C" fn board_crate_version() -> u32 {
        1
    }

    /// Take the seed that is waiting for a flight in `level`, if any.
    pub fn take_seed(level: u32) -> Option<u64> {
        // SAFETY: plain getters and a notification; no pointers.
        unsafe {
            (board_ready(level) != 0).then(|| {
                let seed = u64::from(board_seed_hi(level)) << 32 | u64::from(board_seed_lo(level));
                board_take(level);
                seed
            })
        }
    }

    /// Send the replay so far of the flight whose seed was taken last.
    pub fn submit(replay: &[u8], level: u32) {
        // SAFETY: the page copies the bytes before this call returns.
        unsafe { board_submit(replay.as_ptr(), replay.len() as u32, level) }
    }

    /// What the server said about `level` in the last replay sent:
    /// status (0 waiting, 1 ranked, else not counted), rank, pilots.
    pub fn status(level: u32) -> (i32, u32, u32) {
        // SAFETY: plain getters; no pointers.
        unsafe { (board_status(level), board_rank(level), board_pilots(level)) }
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod js {
    pub fn take_seed(_level: u32) -> Option<u64> {
        None
    }

    pub fn submit(_replay: &[u8], _level: u32) {}

    pub fn status(_level: u32) -> (i32, u32, u32) {
        (-1, 0, 0)
    }
}

/// Where a completed level stands online.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Standing {
    /// Sent; the server has not answered yet.
    Waiting,
    /// Flown again and counted: the pilot's place in this level (1 = the
    /// fastest) and how many pilots are on its board.
    Ranked { rank: u32, pilots: u32 },
    /// The server refused it or could not be reached.
    NotCounted,
}

impl Standing {
    /// The line "Level complete" shows under the time.
    pub fn line(self) -> String {
        match self {
            Standing::Waiting => "World rank ...".to_owned(),
            Standing::Ranked { rank, pilots } => format!("World rank {rank} of {pilots}"),
            Standing::NotCounted => "Not counted online".to_owned(),
        }
    }
}

/// One flight's leaderboard state. Make a new one with every new world.
#[derive(Default)]
pub struct Board {
    /// Writes the flight down, if it is a ranked one.
    recorder: Option<Recorder>,
    /// The world has ticked: too late to change its seed.
    flying: bool,
    /// The level whose "Level complete" sent the replay last.
    sent: Option<u32>,
}

impl Board {
    /// Call every frame right before the ticks (after any new world was
    /// made), with the screen's size now and whether the flight has
    /// started ("Tap to fly", not paused). Keeps the world's bounds in
    /// sync, moved into the sizes a replay accepts
    /// (`replay::playable_bounds`; for every flight, so all fly alike).
    /// Once started and before the first tick, it swaps in the server's
    /// seed if one is there (the flight has not begun, so nothing is
    /// lost) and starts recording. Not before: a seed taken in the menu
    /// would be wasted when the player picks another level.
    pub fn frame(&mut self, world: &mut World, bounds: Bounds, started: bool) {
        let bounds = playable_bounds(bounds);
        world.bounds = bounds;
        if started && !self.flying && self.recorder.is_none() {
            if let Some(seed) = js::take_seed(world.start_level) {
                let start = Start {
                    seed,
                    level: world.start_level,
                    bounds,
                };
                *world = start.world();
                self.recorder = Some(Recorder::new(&start));
            }
        }
        if let Some(recorder) = &mut self.recorder {
            recorder.resize(bounds);
        }
    }

    /// The player tapped "Next level": call with `World::go_on`.
    pub fn go_on(&mut self) {
        if let Some(recorder) = &mut self.recorder {
            recorder.go_on();
        }
    }

    /// The input for the next tick. Rounded the same way whether the
    /// flight is ranked or not, so the plane always feels the same.
    pub fn input(&mut self, steer: f32, throttle: f32) -> Input {
        self.flying = true;
        match &mut self.recorder {
            Some(recorder) => recorder.tick(steer, throttle),
            None => rounded_input(steer, throttle),
        }
    }

    /// "Level complete" for `level`: send the replay so far, for the
    /// server to count the level's time.
    pub fn level_complete(&mut self, level: u32) {
        if let Some(recorder) = &self.recorder {
            js::submit(&recorder.bytes(), level);
            self.sent = Some(level);
        }
    }

    /// Where the level just completed stands online; `None` for a
    /// flight that is not ranked.
    pub fn standing(&self, level: u32) -> Option<Standing> {
        (self.sent == Some(level)).then(|| match js::status(level) {
            (0, _, _) => Standing::Waiting,
            (1, rank, pilots) => Standing::Ranked { rank, pilots },
            _ => Standing::NotCounted,
        })
    }
}
