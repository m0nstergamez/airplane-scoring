#!/usr/bin/env bash
# One command for browser development: build, serve, rebuild on save.
# The page reloads itself when a new .wasm lands (see web/index.html).
set -euo pipefail
cd "$(dirname "$0")/.."
scripts/build-web.sh
scripts/serve.sh &
server=$!
trap 'kill $server 2>/dev/null' EXIT INT TERM
scripts/watch.sh
