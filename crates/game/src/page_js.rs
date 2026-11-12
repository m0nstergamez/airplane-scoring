//! What the web page tells the game about the start: whether the player
//! has started yet ("Tap to fly" / "Click to fly") and which level to
//! start in (`?level=2` in the URL, for testing and screenshots). The
//! page also keeps the highscores in the browser's local storage. Both
//! come from the "page" plugin in `web/index.html`. Native builds have no
//! start screen: they fly right away, in level 1. Core decides what the
//! values mean (`Clock`, `start_level`).

#[cfg(target_arch = "wasm32")]
mod imp {
    // Provided by the page at instantiation (same pattern as tilt_js.rs).
    #[link(wasm_import_module = "env")]
    extern "C" {
        fn page_started() -> i32;
        fn page_level() -> i32;
        fn page_highscore(i: i32) -> f64;
        fn page_set_highscore(i: i32, score: f64);
    }

    /// miniquad checks this against the JS plugin's `version`.
    #[no_mangle]
    pub extern "C" fn page_crate_version() -> u32 {
        2
    }

    pub fn load_highscores(count: usize) -> Vec<u64> {
        // SAFETY: plain getter provided by the page's plugin; no pointers.
        (0..count as i32)
            .map(|i| unsafe { page_highscore(i) }.max(0.0) as u64)
            .collect()
    }

    pub fn save_highscores(count: usize, scores: &[u64]) {
        for i in 0..count {
            let score = scores.get(i).copied().unwrap_or(0) as f64;
            // SAFETY: plain setter provided by the page's plugin; no pointers.
            unsafe { page_set_highscore(i as i32, score) }
        }
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

    // Native: highscores last until the window closes.
    static HIGHSCORES: std::sync::Mutex<Vec<u64>> = std::sync::Mutex::new(Vec::new());

    pub fn load_highscores(_count: usize) -> Vec<u64> {
        HIGHSCORES.lock().map(|h| h.clone()).unwrap_or_default()
    }

    pub fn save_highscores(_count: usize, scores: &[u64]) {
        if let Ok(mut h) = HIGHSCORES.lock() {
            *h = scores.to_vec();
        }
    }
}

pub use imp::{load_highscores, requested_level, save_highscores, started};
