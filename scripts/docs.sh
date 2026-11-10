#!/usr/bin/env bash
# Build local, version-exact API docs for the workspace AND its dependencies
# (macroquad included) into target/doc. Read these instead of relying on memory.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo doc --workspace
echo "Docs at: target/doc/macroquad/index.html"
echo "         target/doc/airplane_scoring_core/index.html"
