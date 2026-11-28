// Tests for web/tilt.js: which devices get gravity's z flipped. A wrong
// answer reverses the tilt throttle (tipping forward brakes).
//   node --test tests/web/*.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import vm from "node:vm";

const source = readFileSync(new URL("../../web/tilt.js", import.meta.url), "utf8");
const context = vm.createContext({});
vm.runInContext(source, context, { filename: "web/tilt.js" });
const { isIOSDevice } = context;

const ANDROID_CHROME =
  "Mozilla/5.0 (Linux; Android 15; Pixel 9) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/158.0.0.0 Mobile Safari/537.36";
const IPHONE_SAFARI =
  "Mozilla/5.0 (iPhone; CPU iPhone OS 26_1 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/26.1 Mobile/15E148 Safari/604.1";
const IPHONE_CHROME =
  "Mozilla/5.0 (iPhone; CPU iPhone OS 26_1 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) CriOS/158.0.0.0 Mobile/15E148 Safari/604.1";
const MAC_SAFARI =
  "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/26.1 Safari/605.1.15";

test("Android is not iOS, so its gravity is not flipped", () => {
  assert.equal(isIOSDevice(ANDROID_CHROME, "Linux armv8l", 5), false);
});

test("iPhones are iOS, in Safari and in Chrome", () => {
  assert.equal(isIOSDevice(IPHONE_SAFARI, "iPhone", 5), true);
  assert.equal(isIOSDevice(IPHONE_CHROME, "iPhone", 5), true);
});

test("an iPad that calls itself a Mac is iOS by its touch points", () => {
  assert.equal(isIOSDevice(MAC_SAFARI, "MacIntel", 5), true);
});

test("a Mac (no touch points) is not iOS", () => {
  assert.equal(isIOSDevice(MAC_SAFARI, "MacIntel", 0), false);
});
