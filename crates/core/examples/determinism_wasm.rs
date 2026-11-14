//! The determinism flights (`common/determinism.rs`) built for the
//! browser's own target, wasm32-unknown-unknown, as a module Node can
//! load: `report()` flies them and returns where the text is,
//! `report_len()` how long it is. `scripts/check-determinism.sh`
//! compares it with the native example `determinism`.

#[path = "common/determinism.rs"]
mod flights;

use std::sync::OnceLock;

static REPORT: OnceLock<String> = OnceLock::new();

/// Flies the flights (once) and returns the address of the report.
#[no_mangle]
pub extern "C" fn report() -> *const u8 {
    REPORT.get_or_init(flights::report).as_ptr()
}

/// The report's length in bytes (UTF-8).
#[no_mangle]
pub extern "C" fn report_len() -> usize {
    REPORT.get_or_init(flights::report).len()
}
