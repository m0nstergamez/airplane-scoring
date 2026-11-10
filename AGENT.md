# airplane-scoring — Claude Code guide

## What this is
A 2D browser game in Rust. Two crates:

- `crates/core` (`airplane-scoring-core`): **pure logic**, no engine, no I/O.
  Every rule of the game lives here and is unit-tested.
- `crates/game` (`airplane-scoring`): thin macroquad shell. Reads input,
  calls `World::tick`, draws. No game rules here.

Target: wasm32 in the browser (`web/index.html`), also runs native.

## Pinned versions — do not drift
- **macroquad = 0.4.16** (exact pin in root `Cargo.toml`). Do not use APIs
  from other versions. If an API you expect is missing, check the local docs
  below before assuming it exists.
- `web/mq_js_bundle.js` is pinned to macroquad commit `8d602d3f8ebb8a3a582ee15d92983bbb81ee3ef6`
  (see `scripts/vendor-js.sh`). macroquad has no git tags per release, so
  the JS glue is pinned by SHA. Bump it together with the crate version.

## Local docs (source of truth over memory)
Run `scripts/docs.sh` once. Then read:
- `target/doc/macroquad/index.html` — exact API for the pinned version
- `target/doc/airplane_scoring_core/index.html` — our core API
- `docs/` — project notes

Prefer `cargo doc` output and the vendored crate source in
`~/.cargo/registry/src/*/macroquad-0.4.16/` over anything you remember.

## The verification loop
`scripts/check.sh` runs all of it in order (~5 s when incremental):
1. `cargo test -p airplane-scoring-core` — fast, no engine. Must stay green.
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `scripts/build-web.sh` — release wasm build into `web/`.
4. `scripts/shot.sh` — loads the real web build in headless Chrome, saves
   `target/shots/latest.png` and fails on console errors or Rust panics
   (panics show file:line). **Read the PNG** after any visual change.

A change is not done until `scripts/check.sh` passes.
While iterating, `scripts/check.sh --fast` (steps 1–2) takes ~1 s.

Visual checks:
- `scripts/shot.sh out.png 5000` — custom path, let the game run 5 s first.
  `SIZE=844,390` = phone in landscape (the main target). Known console noise from the vendored
  JS bundle is filtered in the script; add new noise there, not errors.
- For interaction (key presses, waiting for a state), use a browser tool
  against `scripts/dev.sh` (http://localhost:8080); it live-reloads on rebuild.
- The game runs on real time, so screenshots are approximate. Assert exact
  behaviour in `core` tests, use screenshots to confirm it looks right.

## Mobile-first
- Target: phones in **landscape**, published to GitHub Pages by CI on push to `main`.
- Portrait on touch devices shows a "turn your phone" overlay (CSS in `web/index.html`).
- `web/index.html` blocks zoom/scroll/long-press; keep it that way.
- Work goes through branches + PRs; CI must be green.

## Rules for changes
- New gameplay behaviour → add it to `core` **with a test first**, then
  wire it in `game`. Never put rules in `main.rs`.
- `core` must not depend on macroquad or any platform crate. If you need
  randomness or time, inject it (pass a seed / dt in), don't call it.
- Keep `World::tick` deterministic. The `simulation_is_deterministic` test
  guards this.
- Don't add dependencies without saying why; wasm size matters.
- Don't change the pinned macroquad version unless explicitly asked.
  If asked: edit root `Cargo.toml`, this file, run `scripts/vendor-js.sh`,
  `scripts/docs.sh`, then the full loop.

## Layout
```
Cargo.toml            workspace, pinned deps, release profile for small wasm
crates/core/          pure logic + tests
crates/game/          macroquad shell
web/index.html        canvas page; loads mq_js_bundle.js + airplane-scoring.wasm
scripts/              check.sh, shot.sh, dev.sh, watch.sh, build-web.sh, serve.sh,
                      docs.sh, vendor-js.sh
docs/                 project notes
```
