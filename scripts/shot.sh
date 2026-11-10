#!/usr/bin/env bash
# Headless visual check of the *current* code, for humans and agents.
# Builds the web version, loads it in headless Chrome, saves a screenshot and
# fails if the page logs unexpected console errors.
#
# Usage: scripts/shot.sh [out.png] [ms]
#   out.png  default target/shots/latest.png
#   ms       virtual time to let the game run before the shot (default 3000)
# Env: CHROME=/path/to/chrome   SIZE=800,450   SKIP_BUILD=1
set -euo pipefail
cd "$(dirname "$0")/.."
out="${1:-target/shots/latest.png}"
ms="${2:-3000}"
size="${SIZE:-800,450}"
mkdir -p "$(dirname "$out")"
tmpdir=$(mktemp -d)
trap 'rm -rf "$tmpdir"' EXIT

# Console messages that are known and harmless (from the vendored JS bundle).
known='register_plugin is not defined|is present in JS bundle, but is not used|GL Driver Message|favicon'

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

"$chrome" --headless=new --use-angle=swiftshader --enable-unsafe-swiftshader \
  --window-size="$size" --hide-scrollbars --user-data-dir="$tmpdir/profile" \
  --virtual-time-budget="$ms" --enable-logging=stderr --v=0 \
  --screenshot="$out" "http://127.0.0.1:$port/" 2>"$tmpdir/chrome.log" >/dev/null

console=$(grep 'CONSOLE' "$tmpdir/chrome.log" | sed 's/.*CONSOLE[^]]*\] "//; s/", source: .*//' | awk '!seen[$0]++' || true)
unexpected=$(printf '%s\n' "$console" | grep -vE "$known" | grep -v '^$' || true)

echo "screenshot: $out"
if [ -z "$unexpected" ]; then echo "console: clean"; exit 0; fi
echo "console:"; printf '%s\n' "$unexpected" | sed 's/^/  /'
if printf '%s\n' "$unexpected" | grep -qiE 'error|panic|uncaught|exception|failed'; then
  echo "FAIL: console shows errors (known noise is filtered in scripts/shot.sh)" >&2
  exit 1
fi
