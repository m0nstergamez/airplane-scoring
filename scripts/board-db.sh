#!/usr/bin/env bash
# The leaderboard database on Fly.io (app airplane-scoring-board), from
# this computer: backups, queries and changes by hand.
#
#   scripts/board-db.sh backup        copy the live database to backups/
#                                     (sqlite3 .backup on the machine: safe
#                                     while the server runs)
#   scripts/board-db.sh sql "SQL"     run a query, read-only
#   scripts/board-db.sh write "SQL"   change it: makes a backup first
#   scripts/board-db.sh restore FILE  put a backup back (makes a backup of
#                                     what is there first, asks before)
#
# Every change by hand goes through `write` or `restore`, so there is
# always a copy of the database from just before. Backups hold player
# names and token hashes: they stay on this computer (backups/ is
# gitignored), never in the repo or a CI artifact. Fly also keeps a
# daily snapshot of the volume for 30 days (the manual, Developing,
# Leaderboard).
#
# Needs flyctl, logged in (fly auth login), and sqlite3 here (to check
# each backup). BOARD_HOST=stage works on the stage's server instead
# (scripts/stage.sh, with its STAGE_HOST), to try it out;
# BOARD_DB=/data/other.sqlite on another file there, BOARD_BACKUPS
# saves to another local folder.
set -euo pipefail
cd "$(dirname "$0")/.."

db="${BOARD_DB:-/data/leaderboard.sqlite}"
backups="${BOARD_BACKUPS:-backups}"

usage() {
  awk 'NR > 1 && !/^#/ {exit} NR > 1 {sub(/^# ?/, ""); print}' "$0" >&2
  exit 2
}

case "$db" in
  /data/*.sqlite) ;;
  *) echo "BOARD_DB must be a .sqlite file in /data: $db" >&2; exit 2 ;;
esac
command -v sqlite3 >/dev/null || { echo "No sqlite3 found" >&2; exit 1; }

# How to reach the server's machine: run a command there (no single
# quotes in it: it goes through sh -c '...'), copy a file to it and back.
case "${BOARD_HOST:-fly}" in
  fly)
    command -v fly >/dev/null || { echo "No flyctl found (https://fly.io/docs/flyctl/install/)" >&2; exit 1; }
    app=airplane-scoring-board
    health="https://$app.fly.dev/health"
    remote() { fly ssh console -q -a "$app" -C "sh -c '$1'"; }
    to_remote() { echo "put $1 $2" | fly ssh sftp shell -a "$app" >/dev/null; }
    from_remote() { fly ssh sftp get -a "$app" "$1" "$2" >/dev/null; }
    ;;
  stage)
    # As scripts/stage.sh runs it: the container stage-board.
    health="http://localhost:8187/health"
    case "${STAGE_HOST:-orb}" in
      orb) docker() { orb -m stage sudo docker "$@"; } ;;
      docker) docker() { command docker "$@"; } ;;
      *) echo "STAGE_HOST is orb or docker" >&2; exit 2 ;;
    esac
    # OrbStack shares this Mac's files with the machine at the same paths.
    remote() { docker exec stage-board sh -c "$1"; }
    to_remote() { docker cp "$(cd "$(dirname "$1")" && pwd)/$(basename "$1")" "stage-board:$2" >/dev/null; }
    from_remote() { docker cp "stage-board:$1" "$(cd "$(dirname "$2")" && pwd)/$(basename "$2")" >/dev/null; }
    ;;
  *) echo "BOARD_HOST is fly or stage" >&2; exit 2 ;;
esac

# The machine sleeps when nobody plays; a request wakes it.
wake() {
  for _ in $(seq 1 30); do
    curl -fsS "$health" >/dev/null 2>&1 && return 0
    sleep 2
  done
  echo "the server does not answer $health" >&2
  exit 1
}

# Copy a local file to the machine: prints the path it got there.
put() {
  local path
  path="/tmp/board-db-$(date +%s)-$$-$(basename "$1")"
  to_remote "$1" "$path" || return 1
  echo "$path"
}

backup() {
  wake
  mkdir -p "$backups"
  local name path
  name="$(basename "$db" .sqlite)-$(date -u +%Y%m%d-%H%M%S).sqlite"
  path="/tmp/$name"
  remote "sqlite3 $db \".backup $path\"" >/dev/null
  from_remote "$path" "$backups/$name"
  remote "rm -f $path" >/dev/null
  # A copy that does not open as a database is no backup.
  [ "$(sqlite3 "$backups/$name" 'PRAGMA integrity_check')" = ok ] \
    || { echo "FAIL: $backups/$name is not a sound database" >&2; exit 1; }
  echo "$backups/$name"
}

# Run SQL on the machine: through a file, so its quotes need no escaping.
run_sql() {
  local flags=$1 sql=$2 file path
  # In the checkout, not $TMPDIR: OrbStack's machine sees this one.
  mkdir -p target
  file=$(mktemp target/board-db.XXXXXX)
  printf '%s\n' "$sql" >"$file"
  path=$(put "$file") || { rm -f "$file"; exit 1; }
  rm -f "$file"
  remote "sqlite3 $flags $db \".read $path\"; status=\$?; rm -f $path; exit \$status"
}

cmd="${1:-}"
case "$cmd" in
  backup)
    [ $# -eq 1 ] || usage
    backup
    ;;
  sql)
    [ $# -eq 2 ] || usage
    wake
    run_sql "-readonly -header -column" "$2"
    ;;
  write)
    [ $# -eq 2 ] || usage
    echo "backup first: $(backup)"
    run_sql "-bail -header -column" "$2"
    ;;
  restore)
    [ $# -eq 2 ] || usage
    file=$2
    [ -f "$file" ] || { echo "no such file: $file" >&2; exit 1; }
    [ "$(sqlite3 "$file" 'PRAGMA integrity_check')" = ok ] \
      || { echo "$file is not a sound database" >&2; exit 1; }
    printf 'Replace %s on %s with %s? Type yes: ' "$db" "${BOARD_HOST:-fly}" "$file"
    read -r answer
    [ "$answer" = yes ] || { echo "nothing changed"; exit 1; }
    echo "backup first: $(backup)"
    path=$(put "$file") || exit 1
    # .restore goes through SQLite's lock: the running server sees the
    # restored data with its next request, no restart needed.
    remote "sqlite3 $db \".restore $path\"; status=\$?; rm -f $path; exit \$status"
    echo "restored $db from $file"
    ;;
  *) usage ;;
esac
