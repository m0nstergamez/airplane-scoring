#!/usr/bin/env bash
# Rebuild the web build whenever Rust sources or manifests change.
# No dependencies: polls file mtimes twice a second (cheap for a repo this size).
set -uo pipefail
cd "$(dirname "$0")/.."
stamp=target/.watch-stamp
mkdir -p target
touch "$stamp"
echo "Watching crates/ and Cargo.toml for changes (Ctrl+C to stop)"
while true; do
  changed=$(find crates Cargo.toml -newer "$stamp" \( -name '*.rs' -o -name 'Cargo.toml' \) -print -quit 2>/dev/null)
  if [ -n "$changed" ]; then
    touch "$stamp"
    printf '\n\033[1m==> change in %s, rebuilding\033[0m\n' "$changed"
    if scripts/build-web.sh >target/.watch-build.log 2>&1; then
      tail -1 target/.watch-build.log
      printf '\033[32m✓ rebuilt %s - browser reloads itself\033[0m\n' "$(date +%H:%M:%S)"
    else
      grep -vE '^\s+(Compiling|Checking)' target/.watch-build.log
      printf '\033[31m✗ build failed - browser keeps the last good build\033[0m\n'
    fi
  fi
  sleep 0.5
done
