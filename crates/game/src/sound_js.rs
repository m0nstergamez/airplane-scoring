//! "Sound on/off": two switches, "Sounds" and "Music" (the tunes). In the
//! browser the page owns them: the speaker button, the menu's options,
//! and it remembers the choice (`web/index.html`, "sound" plugin).
//! Native builds have no page, so the setting lives here until the game quits.

#[cfg(target_arch = "wasm32")]
mod imp {
    // Provided by the page at instantiation (same pattern as tilt_js.rs).
    #[link(wasm_import_module = "env")]
    extern "C" {
        fn sound_muted() -> i32;
        fn sound_music_on() -> i32;
        fn sound_toggle_mute();
    }

    /// miniquad checks this against the JS plugin's `version`.
    #[no_mangle]
    pub extern "C" fn sound_crate_version() -> u32 {
        2
    }

    /// The "Music" switch in the menu's options.
    pub fn music_on() -> bool {
        // SAFETY: plain getter provided by the page's sound plugin; no pointers.
        unsafe { sound_music_on() != 0 }
    }

    pub fn muted() -> bool {
        // SAFETY: plain getter provided by the page's sound plugin; no pointers.
        unsafe { sound_muted() != 0 }
    }

    pub fn toggle_mute() {
        // SAFETY: takes and returns nothing, see above.
        unsafe { sound_toggle_mute() }
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    use std::sync::atomic::{AtomicBool, Ordering};

    static MUTED: AtomicBool = AtomicBool::new(false);

    pub fn muted() -> bool {
        MUTED.load(Ordering::Relaxed)
    }

    pub fn toggle_mute() {
        MUTED.fetch_xor(true, Ordering::Relaxed);
    }

    pub fn music_on() -> bool {
        !MUTED.load(Ordering::Relaxed)
    }
}

pub use imp::{music_on, muted, toggle_mute};
