//! What the web page tells the game about the start: whether the player
//! has started yet ("Tap to fly" / "Click to fly") and which level to
//! start in (`?level=2` in the URL, for testing and screenshots). Both
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
    }

    /// miniquad checks this against the JS plugin's `version`.
    #[no_mangle]
    pub extern "C" fn page_crate_version() -> u32 {
        1
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
}

pub use imp::{requested_level, started};
