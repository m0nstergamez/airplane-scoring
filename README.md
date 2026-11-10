# airplane-scoring

Fly with your plane through obstacles and survive.

Rust 2D game: pure-logic core with `cargo test`, macroquad renderer,
wasm build, pinned engine version, Claude Code-friendly loop.

## Setup (once)
```
rustup target add wasm32-unknown-unknown
scripts/docs.sh          # optional: local API docs for the pinned macroquad
```
`web/mq_js_bundle.js` is already in the repo. Only run `scripts/vendor-js.sh`
when bumping the macroquad version (see `AGENT.md`).

## Play / develop
| What | Command |
|---|---|
| Play in a native window | `cargo run` |
| Develop in the browser | `scripts/dev.sh` → http://localhost:8080 — rebuilds on save, page reloads itself |
| Run all checks (incl. screenshot) | `scripts/check.sh` |
| Quick checks while coding | `scripts/check.sh --fast` |
| Screenshot of the current build | `scripts/shot.sh` → `target/shots/latest.png` |
| Only the logic tests | `cargo test -p airplane-scoring-core` |

Port 8080 taken? `PORT=9000 scripts/dev.sh`.
The screenshot needs Google Chrome (or set `CHROME=/path/to/chrome`).

See `AGENT.md` for the working agreement with Claude Code.
