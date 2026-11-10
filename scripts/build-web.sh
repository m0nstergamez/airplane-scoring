#!/usr/bin/env bash
# Build the wasm binary and place it next to index.html.
# The copy is atomic (temp file + mv) so a live-reloading browser never
# loads a half-written .wasm.
set -euo pipefail
cd "$(dirname "$0")/.."
rustup target add wasm32-unknown-unknown >/dev/null 2>&1
cargo build --release --target wasm32-unknown-unknown -p airplane-scoring
tmp=web/.airplane-scoring.wasm.tmp
cp target/wasm32-unknown-unknown/release/airplane-scoring.wasm "$tmp"
if command -v wasm-opt >/dev/null; then
  wasm-opt -Os -o "$tmp" "$tmp"
fi
mv "$tmp" web/airplane-scoring.wasm
echo "built web/airplane-scoring.wasm ($(wc -c < web/airplane-scoring.wasm | tr -d ' ') bytes)"
