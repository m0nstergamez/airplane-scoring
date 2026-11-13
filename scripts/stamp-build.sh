#!/usr/bin/env bash
# Stamp the web build with its commit, for the published site (CI runs
# this after build-web.sh). index.html gets it in <meta name="build"> and
# in the ?v= of the bundle and the wasm (so no old cached copy is used),
# and version.txt holds it, so an app installed on the home screen sees
# when a newer build is out and loads it. Local builds stay "__BUILD__".
set -euo pipefail
cd "$(dirname "$0")/.."
sha="${1:?usage: scripts/stamp-build.sh <commit sha>}"
[[ "$sha" =~ ^[0-9a-f]{7,40}$ ]] || { echo "not a commit sha: $sha" >&2; exit 1; }
sed -i.bak "s/__BUILD__/$sha/g" web/index.html && rm web/index.html.bak
printf '%s\n' "$sha" > web/version.txt
grep -q "__BUILD__" web/index.html && { echo "index.html still has __BUILD__" >&2; exit 1; }
echo "stamped build $sha"
