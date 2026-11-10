#!/usr/bin/env bash
# The full verification loop from AGENT.md. A change is done when this passes.
#   scripts/check.sh          tests, clippy, wasm build, headless screenshot
#   scripts/check.sh --fast   tests, format and clippy only (seconds; for tight loops)
set -euo pipefail
cd "$(dirname "$0")/.."
step() { printf '\n\033[1m==> %s\033[0m\n' "$*"; }

step "1/4 core tests"
cargo test -q -p airplane-scoring-core

step "2/4 format + clippy"
cargo fmt --all --check || { echo "Run: cargo fmt --all" >&2; exit 1; }
cargo clippy -q --workspace --all-targets -- -D warnings
cargo clippy -q --target wasm32-unknown-unknown -p airplane-scoring -- -D warnings

if [ "${1:-}" = "--fast" ]; then
  printf '\n\033[32mFast checks passed\033[0m (run without --fast before calling it done)\n'
  exit 0
fi

step "3/4 wasm build"
scripts/build-web.sh

step "4/4 visual: headless screenshot + console"
set +e
SKIP_BUILD=1 scripts/shot.sh
rc=$?
set -e
if [ $rc -eq 2 ]; then
  echo "(skipped: no Chrome found)"
elif [ $rc -ne 0 ]; then
  exit $rc
fi

printf '\n\033[32mAll checks passed.\033[0m Look at target/shots/latest.png\n'
