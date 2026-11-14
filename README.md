# airplane-scoring

Fly with your plane through obstacles and survive.

**Play:** https://m0nstergamez.github.io/airplane-scoring/ (phone in landscape; "Add to Home Screen" for full screen)

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

## Leaderboard (prototype)

An online board of the fastest completed levels, one per level (the same
score as the local best times). The game writes each flight down (seed and
inputs, `crates/core/src/replay.rs`) and sends it at every "Level
complete"; the server (`crates/server`) hands out the seed, flies the
replay again with the same core and only then counts the time. Off unless
a server is set: the `board-api` meta tag in `web/index.html`. Deploying
the server: `fly.toml` and the manual (Developing).

| What | Command |
|---|---|
| Run the server | `cargo run -p airplane-scoring-server` (port 8787, `leaderboard.sqlite`) |
| Play against it locally | `scripts/dev.sh`, then <http://localhost:8080/?board=http://localhost:8787> |
| Its tests | `cargo test -p airplane-scoring-server` |

Settings (environment): `PORT`, `DB_PATH`, `ALLOWED_ORIGINS` (the pages
that may call it, comma separated). GitHub Pages only serves files, so the
server runs elsewhere; the game stays on Pages.

## Publishing
Every push to `main` runs `.github/workflows/pages.yml`: tests, clippy, wasm
build, then deploys `web/` to GitHub Pages. Pull requests run the same checks
without deploying.

## Manual
- Deutsch: [docs/content/de/index.md](docs/content/de/index.md)
- English: [docs/content/en/index.md](docs/content/en/index.md)
- Wörterbuch / glossary (de ↔ en): [docs/glossary.md](docs/glossary.md)

See `AGENT.md` for the working agreement with Claude Code.
