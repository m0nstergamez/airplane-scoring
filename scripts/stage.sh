#!/usr/bin/env bash
# "stage": the leaderboard end to end on this Mac, before Fly.io.
# An OrbStack Linux machine (amd64 Debian, like Fly's machines) runs the
# real server image (Dockerfile) and serves this checkout's web build with
# the "board-api" meta tag pointing at it. The browser's wasm makes the
# replays, an x86-64 Linux server flies them again: the same split as in
# production.
#
#   scripts/stage.sh up       create the machine if needed, build, (re)start
#   scripts/stage.sh e2e      automated check: sign up, fly a level, rank
#   scripts/stage.sh e2e-browser
#                             the real page in headless Chrome, flown by
#                             core's demo pilot (a test build with
#                             --features autopilot), until "World rank N of M"
#   scripts/stage.sh logs     follow the server's log
#   scripts/stage.sh reset    empty the stage database
#   scripts/stage.sh down     stop server and web page
#   scripts/stage.sh delete   remove the machine
#
# Game:  http://localhost:8180   API: http://localhost:8187 (OrbStack
# forwards the machine's ports to this Mac; Chrome does not resolve
# stage.orb.local itself. Other ports than scripts/dev.sh and a local
# server, so all three can run at once.)
# Needs OrbStack (orb); `e2e` also needs node and the wasm32-wasip1
# target, `e2e-browser` Chrome. Plain http: fine on a computer; phones
# need https for tilt.
#
# STAGE_HOST=docker: no OrbStack machine, Docker on this computer itself
# (an amd64 Linux one, like CI's: .github/workflows/server.yml). The web
# page is then served by a background python3 (pid in target/stage/).
set -euo pipefail
cd "$(dirname "$0")/.."
repo=$(pwd)

machine=stage
web_port=8180
api_port=8187
web_url="http://localhost:$web_port"
api_url="http://localhost:$api_port"
image=airplane-scoring-board:stage
web_dir=target/stage/web

step() { printf '\n\033[1m==> %s\033[0m\n' "$*"; }
host="${STAGE_HOST:-orb}"
case "$host" in
  orb) on_stage() { orb -m "$machine" bash -c "$*"; }; docker="sudo docker" ;;
  docker) on_stage() { bash -c "$*"; }; docker="docker" ;;
  *) echo "STAGE_HOST is 'orb' (default) or 'docker'" >&2; exit 2 ;;
esac
web_pid=target/stage/web.pid

stop_web() {
  if [ "$host" = docker ]; then
    [ -f "$web_pid" ] && kill "$(cat "$web_pid")" 2>/dev/null || true
    rm -f "$web_pid"
  else
    on_stage "sudo systemctl stop stage-web >/dev/null 2>&1 || true
      sudo systemctl reset-failed stage-web >/dev/null 2>&1 || true"
  fi
}

ensure_machine() {
  [ "$host" = orb ] || return 0
  if ! orb info "$machine" >/dev/null 2>&1; then
    step "create the OrbStack machine '$machine' (amd64 Debian bookworm)"
    orb create --arch amd64 debian:bookworm "$machine"
  fi
  orb start "$machine" >/dev/null 2>&1 || true
  if ! on_stage 'command -v docker' >/dev/null; then
    step "install Docker in '$machine'"
    on_stage 'sudo apt-get update -qq && sudo DEBIAN_FRONTEND=noninteractive apt-get install -y -qq docker.io >/dev/null && sudo systemctl enable --now docker'
  fi
}

up() {
  ensure_machine

  step "web build, with board-api = $api_url"
  scripts/build-web.sh
  rm -rf "$web_dir"
  mkdir -p "$web_dir"
  cp -R web/. "$web_dir"/
  scripts/set-board-api.sh "$api_url" "$web_dir/index.html" >/dev/null

  step "server image (Dockerfile) in '$machine'"
  on_stage "cd '$repo' && $docker build -q -t $image ."

  step "start the server and the web page"
  on_stage "$docker rm -f stage-board >/dev/null 2>&1 || true
    $docker run -d --name stage-board --restart unless-stopped \
      -p $api_port:8080 -v stage-board-data:/data \
      -e ALLOWED_ORIGINS=$web_url $image >/dev/null"
  stop_web
  if [ "$host" = docker ]; then
    (cd "$web_dir" && nohup python3 -m http.server "$web_port" >/dev/null 2>&1 & echo $! >"$repo/$web_pid")
  else
    on_stage "sudo systemd-run --quiet --unit=stage-web --uid=\$(id -u) \
      --working-directory='$repo/$web_dir' python3 -m http.server $web_port"
  fi

  for _ in $(seq 1 30); do
    curl -fsS "$api_url/health" >/dev/null 2>&1 && curl -fsS "$web_url/" >/dev/null 2>&1 && break
    sleep 1
  done
  curl -fsS "$api_url/health" >/dev/null || { echo "the server does not answer; scripts/stage.sh logs" >&2; exit 1; }
  printf '\n\033[32mStage is up.\033[0m Game: %s  API: %s\n' "$web_url" "$api_url"
}

e2e() {
  command -v node >/dev/null || { echo "e2e needs node (https://nodejs.org)" >&2; exit 1; }
  if command -v rustup >/dev/null; then rustup target add wasm32-wasip1 >/dev/null 2>&1 || true; fi
  tmp=$(mktemp -d)
  trap 'rm -rf "$tmp"' EXIT
  json() { python3 -c "import json,sys; d=json.load(sys.stdin); print($1)"; }
  fail() { printf '\033[31mFAIL:\033[0m %s\n' "$*" >&2; exit 1; }

  step "the page may call the server (CORS for $web_url)"
  allowed=$(curl -fsS -o /dev/null -D - -X OPTIONS "$api_url/v1/runs" \
    -H "Origin: $web_url" -H "Access-Control-Request-Method: POST" \
    -H "Access-Control-Request-Headers: authorization,content-type" |
    tr -d '\r' | awk -F': ' 'tolower($1)=="access-control-allow-origin"{print $2}')
  [ "$allowed" = "$web_url" ] || fail "CORS: allowed origin is '$allowed'"

  step "sign up and name the pilot"
  token=$(curl -fsS -X POST "$api_url/v1/players" | json 'd["token"]')
  name=$(curl -fsS -X PUT "$api_url/v1/players/me" -H "Authorization: Bearer $token" \
    -H 'Content-Type: application/json' -d '{"name":"stage e2e"}' | json 'd["name"]')
  [ "$name" = "stage e2e" ] || fail "rename gave '$name'"

  step "a run: the server's seed"
  run=$(curl -fsS -X POST "$api_url/v1/runs" -H "Authorization: Bearer $token" \
    -H 'Content-Type: application/json' -d '{"level":1}')
  started=$(date +%s)
  run_id=$(echo "$run" | json 'd["run_id"]')
  seed=$(echo "$run" | json 'd["seed"]')

  step "fly level 1 with core built as wasm (like the browser)"
  cargo build -q --release --example demo_replay --target wasm32-wasip1 -p airplane-scoring-core
  # Node's WASI runs it: the replay on stdout, "ticks millis" on stderr.
  read -r steps millis < <(node --no-warnings - target/wasm32-wasip1/release/examples/demo_replay.wasm \
    "$seed" 1 844 390 2>&1 >"$tmp/replay.bin" <<'JS'
const { WASI } = require("node:wasi");
const [file, ...args] = process.argv.slice(2);
const wasi = new WASI({ version: "preview1", args: [file, ...args], env: {}, returnOnExit: true });
WebAssembly.instantiate(require("fs").readFileSync(file), wasi.getImportObject())
  .then(({ instance }) => { process.exitCode = wasi.start(instance); });
JS
)
  [ -n "$millis" ] || fail "the wasm flight gave no time (ticks: '$steps')"
  echo "landed in $steps ticks, level time $millis ms (wasm)"

  step "too soon: a replay sent before its time had passed is refused"
  status=$(curl -sS -o /dev/null -w '%{http_code}' -X POST "$api_url/v1/runs/$run_id/replay" \
    -H "Authorization: Bearer $token" -H 'Content-Type: application/octet-stream' \
    --data-binary "@$tmp/replay.bin")
  [ "$status" = 422 ] || fail "early replay: HTTP $status, expected 422"

  wait=$(( started + steps / 60 + 2 - $(date +%s) ))
  if [ "$wait" -gt 0 ]; then
    step "wait ${wait}s: a flight cannot be faster than real time"
    sleep "$wait"
  fi

  step "send the replay: the server flies it again (x86-64 Linux)"
  answer=$(curl -fsS -X POST "$api_url/v1/runs/$run_id/replay" \
    -H "Authorization: Bearer $token" -H 'Content-Type: application/octet-stream' \
    --data-binary "@$tmp/replay.bin")
  echo "$answer"
  got=$(echo "$answer" | json 'd["levels"][0]["millis"] if d["levels"] else "none"')
  [ "$got" = "$millis" ] || fail "server read $got ms, wasm flew $millis ms: wasm and native differ"
  rank=$(echo "$answer" | json 'd["levels"][0]["rank"]')

  step "the same replay again is refused without flying it"
  status=$(curl -sS -o /dev/null -w '%{http_code}' -X POST "$api_url/v1/runs/$run_id/replay" \
    -H "Authorization: Bearer $token" -H 'Content-Type: application/octet-stream' \
    --data-binary "@$tmp/replay.bin")
  [ "$status" = 409 ] || fail "resent replay: HTTP $status, expected 409"

  step "the board"
  board=$(curl -fsS "$api_url/v1/leaderboard?level=1" -H "Authorization: Bearer $token")
  [ "$(echo "$board" | json 'd["me"]["millis"]')" = "$millis" ] || fail "board: $board"
  pilots=$(echo "$board" | json 'd["pilots"]')

  printf '\n\033[32mEnd to end OK.\033[0m Level 1 in %s ms, world rank %s of %s.\n' "$millis" "$rank" "$pilots"
}

# The real page, the real game (built with the autopilot), the stage
# server: one level flown in real time (about a minute), the line under
# the time read from the console, the time checked on the board.
e2e_browser() {
  fail() { printf '\033[31mFAIL:\033[0m %s\n' "$*" >&2; exit 1; }
  curl -fsS "$api_url/health" >/dev/null 2>&1 || fail "the stage is not up: scripts/stage.sh up"
  chrome="${CHROME:-}"
  if [ -z "$chrome" ]; then
    for c in "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" \
             "/Applications/Chromium.app/Contents/MacOS/Chromium" \
             "$(command -v google-chrome || true)" "$(command -v chromium || true)"; do
      if [ -n "$c" ] && [ -x "$c" ]; then chrome="$c"; break; fi
    done
  fi
  [ -n "$chrome" ] || fail "no Chrome found; set CHROME=/path/to/chrome"
  tmp=$(mktemp -d)
  trap 'rm -rf "$tmp"' EXIT

  step "the game with the autopilot (a test build, never published)"
  # Its own target dir: the normal web build stays cached.
  CARGO_TARGET_DIR=target/autopilot cargo build -q --release \
    --target wasm32-unknown-unknown -p airplane-scoring --features autopilot
  page_dir="$web_dir/autopilot"
  [ -f "$web_dir/index.html" ] || fail "no stage web build: scripts/stage.sh up"
  rm -rf "$page_dir"
  mkdir -p "$page_dir"
  (cd "$web_dir" && find . -mindepth 1 -maxdepth 1 ! -name autopilot -exec cp -R {} autopilot/ \;)
  cp target/autopilot/wasm32-unknown-unknown/release/airplane-scoring.wasm "$page_dir/"

  step "fly level 1 in headless Chrome against the stage (real time, about a minute)"
  # The start screen first, until the page has a seed for every level
  # (each answered POST /v1/runs is a resource timing entry); then
  # "fly(1)" is what tapping level 1 on the map does. A fresh profile: a
  # new pilot.
  set +e
  SHOT_EVAL='new Promise((ok) => {
      const levels = document.querySelectorAll("#start .node[data-level]").length;
      const since = Date.now();
      const look = () => {
        const runs = performance.getEntriesByType("resource").filter((e) => /\/v1\/runs$/.test(e.name)).length;
        if (runs >= levels || Date.now() - since > 20000) setTimeout(() => { fly(1); ok(runs); }, 300);
        else setTimeout(look, 100);
      };
      look();
    })' \
    SHOT_UNTIL='autopilot: (World rank [0-9]+ of [0-9]+|Not counted online)' \
    SHOT_UNTIL_MS=300000 \
    SHOT_PRINT='localStorage.getItem("airplane-scoring.pilot")' \
    python3 scripts/cdp_shot.py "$chrome" "$web_url/autopilot/" target/shots/stage-autopilot.png \
    844,390 500 "$tmp/profile" >"$tmp/console.log"
  rc=$?
  set -e
  grep 'autopilot:' "$tmp/console.log" | sed 's/^.*autopilot: /  /' || true
  echo "screenshot: target/shots/stage-autopilot.png"
  [ $rc -eq 0 ] || { grep -v 'autopilot:' "$tmp/console.log" | sed 's/^/  /' >&2; fail "no \"World rank\" line (cdp_shot.py exit $rc)"; }
  grep -q 'autopilot: Not counted online' "$tmp/console.log" && fail "the flight was not counted online"
  line=$(grep -oE 'World rank [0-9]+ of [0-9]+' "$tmp/console.log" | tail -1 || true)
  millis=$(sed -nE 's/.*autopilot: level 1 complete in ([0-9]+) ms.*/\1/p' "$tmp/console.log" | tail -1)
  [ -n "$millis" ] || fail "no \"level 1 complete\" line on the console"

  step "the board has the time the game showed"
  token=$(sed -n 's/^eval: //p' "$tmp/console.log" |
    python3 -c 'import json,sys; print((json.loads(sys.stdin.read() or "null") or {}).get("token", ""))' 2>/dev/null || true)
  [ -n "$token" ] || fail "no pilot in the page's storage"
  board=$(curl -fsS "$api_url/v1/leaderboard?level=1" -H "Authorization: Bearer $token") || fail "the board does not answer"
  got=$(echo "$board" | python3 -c 'import json,sys; print((json.load(sys.stdin).get("me") or {}).get("millis", "nothing"))' 2>/dev/null || true)
  [ "$got" = "$millis" ] || fail "the game showed $millis ms, the board has ${got:-nothing} for this pilot: $board"

  printf '\n\033[32mBrowser end to end OK.\033[0m Level 1 in %s ms; the game showed "%s".\n' "$millis" "$line"
}

case "${1:-}" in
  up) up ;;
  e2e) e2e ;;
  e2e-browser) e2e_browser ;;
  logs) on_stage "$docker logs -f stage-board" ;;
  reset)
    on_stage "$docker rm -f stage-board >/dev/null && $docker volume rm stage-board-data >/dev/null"
    echo "Database emptied. Start again: scripts/stage.sh up" ;;
  down)
    on_stage "$docker rm -f stage-board >/dev/null 2>&1; true"
    stop_web
    echo "Stage stopped (the database is kept)." ;;
  delete) [ "$host" = orb ] && orb delete -f "$machine" ;;
  *) sed -n '2,31p' "$0" | sed 's/^# \{0,1\}//'; exit 2 ;;
esac
