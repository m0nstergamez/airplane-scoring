#!/usr/bin/env bash
# Point a web build at a leaderboard server: the "board-api" meta tag in
# its index.html. The source (web/index.html) leaves it empty, so local
# builds rank nothing; CI sets the Fly.io server when it publishes the
# page, scripts/stage.sh the stage server.
#
# Usage: scripts/set-board-api.sh URL [index.html]   (default: web/index.html)
set -euo pipefail
url=${1:?usage: scripts/set-board-api.sh URL [index.html]}
page=${2:-web/index.html}
sed -i.bak "s|<meta name=\"board-api\" content=\"[^\"]*\">|<meta name=\"board-api\" content=\"$url\">|" "$page"
rm "$page.bak"
grep -q "<meta name=\"board-api\" content=\"$url\">" "$page" || { echo "no board-api meta tag in $page" >&2; exit 1; }
echo "board-api: $url"
