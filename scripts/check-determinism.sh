#!/usr/bin/env bash
# wasm and native must fly the same: the browser (wasm32-unknown-unknown,
# the target the game ships in) records a flight, the leaderboard server
# (native) flies it again and reads its time. Flies the same demo flights
# both ways (crates/core/examples/common/determinism.rs) and fails if
# anything differs. The wasm side runs in Node, the same V8 as Chrome.
#
# Usage: scripts/check-determinism.sh
# Needs node and the wasm32-unknown-unknown target (as for the web build).
# Exit code 2 if node is missing, so check.sh can skip.
set -euo pipefail
cd "$(dirname "$0")/.."
command -v node >/dev/null || { echo "No node found (https://nodejs.org)" >&2; exit 2; }
if command -v rustup >/dev/null; then
  rustup target add wasm32-unknown-unknown >/dev/null 2>&1 || true
fi
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

cargo build -q --release --example determinism -p airplane-scoring-core
cargo build -q --release --example determinism_wasm --target wasm32-unknown-unknown -p airplane-scoring-core \
  || { echo "FAIL: could not build for wasm32-unknown-unknown (rustup target add wasm32-unknown-unknown)" >&2; exit 1; }
target/release/examples/determinism >"$tmp/native.txt"
node - target/wasm32-unknown-unknown/release/examples/determinism_wasm.wasm >"$tmp/wasm.txt" <<'JS'
const fs = require("fs");
const bytes = fs.readFileSync(process.argv[2]);
WebAssembly.instantiate(bytes, {}).then(({ instance }) => {
  const { report, report_len, memory } = instance.exports;
  const ptr = report();
  const len = report_len();
  process.stdout.write(new TextDecoder().decode(new Uint8Array(memory.buffer, ptr, len)));
});
JS

flights=$(wc -l <"$tmp/native.txt" | tr -d ' ')
[ "$flights" -gt 0 ] || { echo "FAIL: no flights flown" >&2; exit 1; }
if ! diff -u --label native --label wasm "$tmp/native.txt" "$tmp/wasm.txt"; then
  echo "FAIL: wasm and native fly differently. Look for std maths in core" >&2
  echo "(f32::sin and the like; use libm) or anything platform-dependent." >&2
  exit 1
fi
echo "wasm and native fly the same ($flights demo flights)"
