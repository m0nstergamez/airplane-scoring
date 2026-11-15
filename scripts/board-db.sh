#!/usr/bin/env bash
# The leaderboard database on Fly.io (app airplane-scoring-board), from
# this computer: backups, queries and changes by hand.
#
#   scripts/board-db.sh backup        copy the live database to backups/
#                                     (sqlite3 .backup on the machine: safe
#                                     while the server runs)
#   scripts/board-db.sh sql "SQL"     run a query, read-only
#   scripts/board-db.sh write "SQL"   change it, all or nothing: makes a
#                                     backup first
#   scripts/board-db.sh restore FILE  put a backup back (makes a backup of
#                                     what is there first, asks before),
#                                     then restarts the server
#
# Every change by hand goes through `write` or `restore`, so there is
# always a copy of the database from just before; without one they
# change nothing. Backups hold player names and token hashes: they stay
# on this computer (backups/ is gitignored), never in the repo or a CI
# artifact. Fly also keeps a daily snapshot of the volume for 30 days
# (the manual, Developing, Leaderboard).
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
# quotes in it: it goes through sh -c '...'; fails if the command does),
# copy a file to it and back, restart the server.
case "${BOARD_HOST:-fly}" in
  fly)
    command -v fly >/dev/null || { echo "No flyctl found (https://fly.io/docs/flyctl/install/)" >&2; exit 1; }
    app=airplane-scoring-board
    health="https://$app.fly.dev/health"
    remote() { fly ssh console -q -a "$app" -C "sh -c '$1'"; }
    # sftp's shell exits 0 even when a put fails: the caller checks.
    to_remote() { echo "put $1 $2" | fly ssh sftp shell -a "$app" >/dev/null; }
    from_remote() { fly ssh sftp get -a "$app" "$1" "$2" >/dev/null; }
    restart() { fly apps restart "$app" >/dev/null; }
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
    restart() { docker restart stage-board >/dev/null; }
    ;;
  *) echo "BOARD_HOST is fly or stage" >&2; exit 2 ;;
esac

# Files this run put in the machine's /tmp (copies of the database among
# them): removed when the script ends, also when it fails.
leftovers=()
cleanup() {
  [ ${#leftovers[@]} -eq 0 ] || remote "rm -f ${leftovers[*]}" >/dev/null 2>&1 || true
}
trap cleanup EXIT

# The machine sleeps when nobody plays; a request wakes it.
wake() {
  for _ in $(seq 1 30); do
    curl -fsS "$health" >/dev/null 2>&1 && return 0
    sleep 2
  done
  echo "the server does not answer $health" >&2
  exit 1
}

# A leaderboard database: not empty, sound, with the server's tables.
sound() {
  [ -s "$1" ] || return 1
  [ "$(sqlite3 -readonly "$1" 'PRAGMA integrity_check' 2>/dev/null)" = ok ] || return 1
  [ "$(sqlite3 -readonly "$1" "SELECT count(*) FROM sqlite_master WHERE type = 'table'
     AND name IN ('players', 'runs', 'best')" 2>/dev/null)" = 3 ]
}

# Copy a local file to the machine; sets $path to where it got there.
put() {
  path="/tmp/board-db-$(date +%s)-$$-$(basename "$1")"
  leftovers+=("$path")
  to_remote "$1" "$path"
  remote "test -s $path" || { echo "FAIL: could not copy $1 to the machine" >&2; exit 1; }
}

# Copy the live database here; sets $saved to the file. Exits if that
# fails, so nothing goes on without a backup.
backup() {
  wake
  mkdir -p "$backups"
  local name
  name="$(basename "$db" .sqlite)-$(date -u +%Y%m%d-%H%M%S).sqlite"
  path="/tmp/$name"
  leftovers+=("$path")
  saved="$backups/$name"
  # sqlite3 would make an empty database where there is none.
  remote "test -s $db" || { echo "FAIL: no database $db on the machine" >&2; exit 1; }
  remote "sqlite3 $db \".backup $path\"" >/dev/null || { echo "FAIL: .backup on the machine" >&2; exit 1; }
  from_remote "$path" "$saved" || { echo "FAIL: could not download the backup" >&2; exit 1; }
  sound "$saved" || { rm -f "$saved"; echo "FAIL: the backup is no sound leaderboard database" >&2; exit 1; }
}

# Run SQL on the machine from a file (its quotes need no escaping), in
# safe mode: no .system, .shell, ATTACH and the like.
run_sql() {
  local flags=$1 sql=$2 file
  # In the checkout, not $TMPDIR: OrbStack's machine sees this one.
  mkdir -p target
  file=$(mktemp target/board-db.XXXXXX)
  printf '%s\n' "$sql" >"$file"
  put "$file"
  rm -f "$file"
  remote "sqlite3 -safe $flags $db < $path"
}

cmd="${1:-}"
case "$cmd" in
  backup)
    [ $# -eq 1 ] || usage
    backup
    echo "$saved"
    ;;
  sql)
    [ $# -eq 2 ] || usage
    wake
    run_sql "-readonly -header -column" "$2"
    ;;
  write)
    [ $# -eq 2 ] || usage
    backup
    echo "backup first: $saved"
    # One transaction: -bail stops at the first error, before COMMIT,
    # so a failed statement takes the ones before it back too.
    run_sql "-bail -header -column" "BEGIN;
$2
;
COMMIT;"
    ;;
  restore)
    [ $# -eq 2 ] || usage
    file=$2
    sound "$file" || { echo "$file is no sound leaderboard database" >&2; exit 1; }
    printf 'Replace %s on %s with %s? Type yes: ' "$db" "${BOARD_HOST:-fly}" "$file"
    read -r answer
    [ "$answer" = yes ] || { echo "nothing changed"; exit 1; }
    backup
    echo "backup first: $saved"
    put "$file"
    remote "sqlite3 $db \".restore $path\""
    # The server sets up its tables when it opens the database
    # (crates/server: SCHEMA, migrate): an older backup needs that again.
    restart
    wake
    echo "restored $db from $file, server restarted"
    ;;
  *) usage ;;
esac
