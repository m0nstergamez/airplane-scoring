#!/usr/bin/env bash
# The full verification loop from AGENT.md. A change is done when this passes.
#   scripts/check.sh          everything below
#   scripts/check.sh --fast   steps 1-3 only (seconds; for tight loops)
set -euo pipefail
cd "$(dirname "$0")/.."
step() { printf '\n\033[1m==> %s\033[0m\n' "$*"; }

step "1/7 core tests"
cargo test -q -p airplane-scoring-core

step "2/7 page script tests (web/board.js, Node)"
if command -v node >/dev/null; then
  node --test tests/web/*.test.mjs
else
  echo "(skipped: no node found)"
fi

step "3/7 format + clippy + manual sync"
scripts/check-docs.sh
cargo fmt --all --check || { echo "Run: cargo fmt --all" >&2; exit 1; }
cargo clippy -q --workspace --all-targets -- -D warnings
cargo clippy -q --target wasm32-unknown-unknown -p airplane-scoring -- -D warnings
# The test build with the autopilot (scripts/stage.sh e2e-browser).
cargo clippy -q --target wasm32-unknown-unknown -p airplane-scoring --features autopilot -- -D warnings

if [ "${1:-}" = "--fast" ]; then
  printf '\n\033[32mFast checks passed\033[0m (run without --fast before calling it done)\n'
  exit 0
fi

step "4/7 leaderboard server tests"
cargo test -q -p airplane-scoring-server

step "5/7 wasm and native fly the same"
set +e
scripts/check-determinism.sh
rc=$?
set -e
if [ $rc -eq 2 ]; then
  echo "(skipped: no node found; CI runs it)"
elif [ $rc -ne 0 ]; then
  exit $rc
fi

step "6/7 wasm build"
scripts/build-web.sh

step "7/7 visual: headless screenshot + console"
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
