//! Raw tilt data from the browser. The page's "tilt" plugin
//! (`web/index.html`) stores the latest `devicemotion` gravity sample; we
//! read it here. What it means for steering lives in core (`Tilt`).
//! Native builds have no sensors: everything reads as "no sample".

#[cfg(target_arch = "wasm32")]
mod imp {
    // Provided by the page at instantiation (same pattern miniquad uses).
    #[link(wasm_import_module = "env")]
    extern "C" {
        fn tilt_has_sample() -> i32;
        fn tilt_gx() -> f32;
        fn tilt_gy() -> f32;
        fn tilt_epoch() -> u32;
    }

    /// miniquad checks this against the JS plugin's `version`.
    #[no_mangle]
    pub extern "C" fn tilt_crate_version() -> u32 {
        1
    }

    /// Latest gravity in device x/y (m/s²), once the sensor has reported.
    pub fn sample() -> Option<(f32, f32)> {
        // SAFETY: plain getters provided by the page's tilt plugin; no pointers.
        unsafe { (tilt_has_sample() != 0).then(|| (tilt_gx(), tilt_gy())) }
    }

    /// Bumped by the page on "tap to fly" and on orientation changes:
    /// a new value means "recalibrate neutral now".
    pub fn epoch() -> u32 {
        // SAFETY: plain getter, see above.
        unsafe { tilt_epoch() }
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    pub fn sample() -> Option<(f32, f32)> {
        None
    }

    pub fn epoch() -> u32 {
        0
    }
}

pub use imp::{epoch, sample};
