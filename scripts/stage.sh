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
#   scripts/stage.sh logs     follow the server's log
#   scripts/stage.sh reset    empty the stage database
#   scripts/stage.sh down     stop server and web page
#   scripts/stage.sh delete   remove the machine
#
# Game:  http://localhost:8180   API: http://localhost:8187 (OrbStack
# forwards the machine's ports to this Mac; Chrome does not resolve
# stage.orb.local itself. Other ports than scripts/dev.sh and a local
# server, so all three can run at once.)
# Needs OrbStack (orb); `e2e` also needs wasmtime and the wasm32-wasip1
# target. Plain http: fine on a computer; phones need https for tilt.
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
on_stage() { orb -m "$machine" bash -c "$*"; }

ensure_machine() {
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
  # Point the page at the stage server (the published page leaves it empty).
  sed -i.bak "s|<meta name=\"board-api\" content=\"[^\"]*\">|<meta name=\"board-api\" content=\"$api_url\">|" "$web_dir/index.html"
  rm "$web_dir/index.html.bak"
  grep -q "content=\"$api_url\"" "$web_dir/index.html" || { echo "board-api meta tag not found" >&2; exit 1; }

  step "server image (Dockerfile) in '$machine'"
  on_stage "cd '$repo' && sudo docker build -q -t $image ."

  step "start the server and the web page"
  on_stage "sudo docker rm -f stage-board >/dev/null 2>&1 || true
    sudo docker run -d --name stage-board --restart unless-stopped \
      -p $api_port:8080 -v stage-board-data:/data \
      -e ALLOWED_ORIGINS=$web_url $image >/dev/null"
  on_stage "sudo systemctl stop stage-web >/dev/null 2>&1 || true
    sudo systemctl reset-failed stage-web >/dev/null 2>&1 || true
    sudo systemd-run --quiet --unit=stage-web --uid=\$(id -u) \
      --working-directory='$repo/$web_dir' python3 -m http.server $web_port"

  for _ in $(seq 1 30); do
    curl -fsS "$api_url/v1/leaderboard" >/dev/null 2>&1 && curl -fsS "$web_url/" >/dev/null 2>&1 && break
    sleep 1
  done
  curl -fsS "$api_url/v1/leaderboard" >/dev/null || { echo "the server does not answer; scripts/stage.sh logs" >&2; exit 1; }
  printf '\n\033[32mStage is up.\033[0m Game: %s  API: %s\n' "$web_url" "$api_url"
}

e2e() {
  command -v wasmtime >/dev/null || { echo "e2e needs wasmtime (https://wasmtime.dev)" >&2; exit 1; }
  rustup target add wasm32-wasip1 >/dev/null 2>&1
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
  read -r steps millis < <(wasmtime target/wasm32-wasip1/release/examples/demo_replay.wasm \
    "$seed" 1 844 390 2>&1 >"$tmp/replay.bin")
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

  step "the board"
  board=$(curl -fsS "$api_url/v1/leaderboard?level=1" -H "Authorization: Bearer $token")
  [ "$(echo "$board" | json 'd["me"]["millis"]')" = "$millis" ] || fail "board: $board"
  pilots=$(echo "$board" | json 'd["pilots"]')

  printf '\n\033[32mEnd to end OK.\033[0m Level 1 in %s ms, world rank %s of %s.\n' "$millis" "$rank" "$pilots"
}

case "${1:-}" in
  up) up ;;
  e2e) e2e ;;
  logs) on_stage 'sudo docker logs -f stage-board' ;;
  reset)
    on_stage "sudo docker rm -f stage-board >/dev/null && sudo docker volume rm stage-board-data >/dev/null"
    echo "Database emptied. Start again: scripts/stage.sh up" ;;
  down)
    on_stage "sudo docker rm -f stage-board >/dev/null 2>&1; sudo systemctl stop stage-web 2>/dev/null; true"
    echo "Stage stopped (the database is kept)." ;;
  delete) orb delete -f "$machine" ;;
  *) sed -n '2,21p' "$0" | sed 's/^# \{0,1\}//'; exit 2 ;;
esac
