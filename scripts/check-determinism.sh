#!/usr/bin/env bash
# wasm and native must fly the same: the browser (wasm) records a flight,
# the leaderboard server (native) flies it again and reads its time.
# Flies the same demo flights both ways (crates/core/examples/
# determinism.rs) and fails if anything differs.
#
# Usage: scripts/check-determinism.sh
# Needs wasmtime (https://wasmtime.dev) and the wasm32-wasip1 target
# (added here). Exit code 2 if wasmtime is missing, so check.sh can skip.
set -euo pipefail
cd "$(dirname "$0")/.."
command -v wasmtime >/dev/null || { echo "No wasmtime found (https://wasmtime.dev)" >&2; exit 2; }
rustup target add wasm32-wasip1 >/dev/null 2>&1
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

cargo build -q --release --example determinism -p airplane-scoring-core
cargo build -q --release --example determinism --target wasm32-wasip1 -p airplane-scoring-core
target/release/examples/determinism >"$tmp/native.txt"
wasmtime target/wasm32-wasip1/release/examples/determinism.wasm >"$tmp/wasm.txt"

flights=$(wc -l <"$tmp/native.txt" | tr -d ' ')
[ "$flights" -gt 0 ] || { echo "FAIL: no flights flown" >&2; exit 1; }
if ! diff -u --label native --label wasm "$tmp/native.txt" "$tmp/wasm.txt"; then
  echo "FAIL: wasm and native fly differently. Look for std maths in core" >&2
  echo "(f32::sin and the like; use libm) or anything platform-dependent." >&2
  exit 1
fi
echo "wasm and native fly the same ($flights demo flights)"
