#!/usr/bin/env bash
# Headless visual check of the *current* code, for humans and agents.
# Builds the web version, loads it in headless Chrome, saves a screenshot and
# fails if the page logs unexpected console errors.
#
# Usage: scripts/shot.sh [out.png] [ms]
#   out.png  default target/shots/latest.png
#   ms       real time to let the game run after page load (default 3000)
# Env: CHROME=/path/to/chrome   SIZE=800,450   SKIP_BUILD=1
#      START=1   show the start screen ("Click to fly"); by default the page
#                gets "?play" and skips it, so the shot shows the game
#      LEVEL=2   start the flight in level 2 ("&level=2")
#      PHONE=1   emulate a touch screen (the page's phone layout)
set -euo pipefail
cd "$(dirname "$0")/.."
out="${1:-target/shots/latest.png}"
ms="${2:-3000}"
size="${SIZE:-800,450}"
mkdir -p "$(dirname "$out")"
tmpdir=$(mktemp -d)
trap 'rm -rf "$tmpdir"' EXIT

# Console messages that are known and harmless (from the vendored JS bundle).
known='register_plugin is not defined|AudioContext was not allowed to start|is present in JS bundle, but is not used|GL Driver Message|favicon'

chrome="${CHROME:-}"
if [ -z "$chrome" ]; then
  for c in "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" \
           "/Applications/Chromium.app/Contents/MacOS/Chromium" \
           "$(command -v google-chrome || true)" "$(command -v chromium || true)"; do
    if [ -n "$c" ] && [ -x "$c" ]; then chrome="$c"; break; fi
  done
fi
[ -n "$chrome" ] || { echo "No Chrome found; set CHROME=/path/to/chrome" >&2; exit 2; }

[ -n "${SKIP_BUILD:-}" ] || scripts/build-web.sh >/dev/null 2>"$tmpdir/build.log" || { cat "$tmpdir/build.log" >&2; exit 1; }

port=$(python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1])')
PORT=$port scripts/serve.sh >/dev/null 2>&1 &
server=$!
trap 'kill $server 2>/dev/null; rm -rf "$tmpdir"' EXIT
for _ in $(seq 50); do curl -sf "http://127.0.0.1:$port/" >/dev/null && break; sleep 0.1; done

page="?play"
[ -z "${START:-}" ] || page="?"
[ -z "${LEVEL:-}" ] || page="$page&level=$LEVEL"

# Real time, driven over DevTools: --virtual-time-budget does not advance the
# game loop (requestAnimationFrame), so shots always showed the first ~0.3 s.
python3 scripts/cdp_shot.py "$chrome" "http://127.0.0.1:$port/$page" "$out" "$size" "$ms" \
  "$tmpdir/profile" >"$tmpdir/console.log"

console=$(awk '!seen[$0]++' "$tmpdir/console.log")
unexpected=$(printf '%s\n' "$console" | grep -vE "$known" | grep -v '^$' || true)

echo "screenshot: $out"
if [ -z "$unexpected" ]; then echo "console: clean"; exit 0; fi
echo "console:"; printf '%s\n' "$unexpected" | sed 's/^/  /'
if printf '%s\n' "$unexpected" | grep -qiE 'error|panic|uncaught|exception|failed'; then
  echo "FAIL: console shows errors (known noise is filtered in scripts/shot.sh)" >&2
  exit 1
fi
