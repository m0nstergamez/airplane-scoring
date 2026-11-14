#!/usr/bin/env bash
# Print the path of Chrome (or Chromium) for the headless checks
# (shot.sh, check-mermaid.sh, stage.sh e2e-browser). $CHROME wins.
# Exit code 2 if there is none.
set -euo pipefail
chrome="${CHROME:-}"
if [ -z "$chrome" ]; then
  for c in "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" \
           "/Applications/Chromium.app/Contents/MacOS/Chromium" \
           "$(command -v google-chrome || true)" "$(command -v chromium || true)"; do
    if [ -n "$c" ] && [ -x "$c" ]; then chrome="$c"; break; fi
  done
fi
[ -n "$chrome" ] || { echo "No Chrome found; set CHROME=/path/to/chrome" >&2; exit 2; }
printf '%s\n' "$chrome"
