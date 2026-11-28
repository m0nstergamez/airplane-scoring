// The page's tilt helper (the tilt plugin in index.html uses it; steering
// maths lives in core, crates/core/src/tilt.rs).
//
// No DOM here, and the only global is `isIOSDevice`; a plain script,
// tests/web/tilt.test.mjs runs it in Node.
"use strict";

/**
 * Whether the page runs on iOS or iPadOS, whose gravity comes negated, so
 * the tilt plugin flips its z (tipping forward must make z grow on every
 * phone, or the tilt throttle is reversed).
 *
 * Detect the device, not DeviceMotionEvent.requestPermission: Chrome has
 * that function too, and Android then got the flip. iPadOS calls itself a
 * Mac ("MacIntel"), hence the touch check; real Macs have no touch points.
 *
 * userAgent, platform, maxTouchPoints: as in `navigator`.
 */
function isIOSDevice(userAgent, platform, maxTouchPoints) {
  return /iPad|iPhone|iPod/.test(userAgent) || (platform === "MacIntel" && maxTouchPoints > 1);
}
