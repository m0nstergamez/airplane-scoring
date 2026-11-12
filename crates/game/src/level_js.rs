//! The level a flight starts in. In the browser the page reads it from the
//! URL (`?level=2`, "level" plugin in `web/index.html`), handy for testing
//! and screenshots. Native builds always start in level 1. Core
//! (`start_level`) turns anything invalid into level 1.

#[cfg(target_arch = "wasm32")]
mod imp {
    // Provided by the page at instantiation (same pattern as tilt_js.rs).
    #[link(wasm_import_module = "env")]
    extern "C" {
        fn level_requested() -> i32;
    }

    /// miniquad checks this against the JS plugin's `version`.
    #[no_mangle]
    pub extern "C" fn level_crate_version() -> u32 {
        1
    }

    pub fn requested() -> i32 {
        // SAFETY: plain getter provided by the page's level plugin; no pointers.
        unsafe { level_requested() }
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    pub fn requested() -> i32 {
        1
    }
}

pub use imp::requested;
