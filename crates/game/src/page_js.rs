//! What the web page tells the game about the start: whether the player
//! has started yet ("Tap to fly" / "Click to fly") and which level to
//! start in (the level map, or `?level=2` in the URL). The page also
//! keeps each level's best times in the browser's local storage. All of
//! it comes from the "page" plugin in `web/index.html`. Native builds have no
//! start screen: they fly right away, in level 1. Core decides what the
//! values mean (`Clock`, `start_level`).

#[cfg(target_arch = "wasm32")]
mod imp {
    // Provided by the page at instantiation (same pattern as tilt_js.rs).
    #[link(wasm_import_module = "env")]
    extern "C" {
        fn page_started() -> i32;
        fn page_level() -> i32;
        fn page_new_flight() -> u32;
        fn page_show_menu();
        fn page_level_complete(level: u32);
        fn page_best_time(level: u32, i: u32) -> f64;
        fn page_set_best_time(level: u32, i: u32, millis: f64);
    }

    /// miniquad checks this against the JS plugin's `version`.
    #[no_mangle]
    pub extern "C" fn page_crate_version() -> u32 {
        6
    }

    /// A level's stored best times, in milliseconds (0 = none).
    pub fn load_best_times(level: u32, count: usize) -> Vec<u64> {
        // SAFETY: plain getter provided by the page's plugin; no pointers.
        (0..count as u32)
            .map(|i| unsafe { page_best_time(level, i) }.max(0.0) as u64)
            .collect()
    }

    /// Store a level's best times (missing places as 0).
    pub fn save_best_times(level: u32, count: usize, millis: &[u64]) {
        for i in 0..count {
            let s = millis.get(i).copied().unwrap_or(0) as f64;
            // SAFETY: plain setter provided by the page's plugin; no pointers.
            unsafe { page_set_best_time(level, i as u32, s) }
        }
    }

    /// Back to the main menu (the start page).
    pub fn show_menu() {
        // SAFETY: takes and returns nothing.
        unsafe { page_show_menu() }
    }

    /// A level was landed: the page unlocks the next one on its map.
    pub fn level_complete(level: u32) {
        // SAFETY: plain value, no pointers.
        unsafe { page_level_complete(level) }
    }

    /// Bumped by the menu's "Menu" button: set up a fresh flight (the
    /// start screen is showing again).
    pub fn new_flight() -> u32 {
        // SAFETY: plain getter provided by the page's plugin; no pointers.
        unsafe { page_new_flight() }
    }

    pub fn started() -> bool {
        // SAFETY: plain getter provided by the page's plugin; no pointers.
        unsafe { page_started() != 0 }
    }

    pub fn requested_level() -> i32 {
        // SAFETY: see above.
        unsafe { page_level() }
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    pub fn started() -> bool {
        true
    }

    pub fn requested_level() -> i32 {
        1
    }

    pub fn new_flight() -> u32 {
        0
    }

    pub fn show_menu() {}

    pub fn level_complete(_level: u32) {}

    // Native: best times last until the window closes.
    static BEST: std::sync::Mutex<Vec<(u32, Vec<u64>)>> = std::sync::Mutex::new(Vec::new());

    pub fn load_best_times(level: u32, _count: usize) -> Vec<u64> {
        let best = BEST.lock().map(|b| b.clone()).unwrap_or_default();
        (best.into_iter().find(|(l, _)| *l == level))
            .map(|(_, t)| t)
            .unwrap_or_default()
    }

    pub fn save_best_times(level: u32, _count: usize, millis: &[u64]) {
        if let Ok(mut best) = BEST.lock() {
            best.retain(|(l, _)| *l != level);
            best.push((level, millis.to_vec()));
        }
    }
}

pub use imp::{
    level_complete, load_best_times, new_flight, requested_level, save_best_times, show_menu,
    started,
};
