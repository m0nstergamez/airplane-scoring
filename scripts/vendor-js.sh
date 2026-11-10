#!/usr/bin/env bash
# Fetch the miniquad JS glue. macroquad does not tag releases on GitHub, so
# this is pinned to a commit SHA instead of the crate version.
# When bumping macroquad: pick the master commit closest to the crate's
# publish date (https://github.com/not-fl3/macroquad/commits/master/js)
# and update JS_SHA here and in AGENT.md.
set -euo pipefail
cd "$(dirname "$0")/.."
JS_SHA="8d602d3f8ebb8a3a582ee15d92983bbb81ee3ef6"
echo "Vendoring mq_js_bundle.js @ $JS_SHA"
curl -fsSL "https://raw.githubusercontent.com/not-fl3/macroquad/$JS_SHA/js/mq_js_bundle.js" -o web/mq_js_bundle.js
echo "ok -> web/mq_js_bundle.js"
