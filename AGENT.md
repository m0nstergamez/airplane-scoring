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
- macroquad's **`audio` feature is on** (pulls quad-snd 0.2.8). Its API in
  this version: `load_sound`, `play_sound`, `play_sound_once`, `stop_sound`,
  `set_sound_volume`. No pitch, pan or fade. `.cargo/config.toml` passes
  `--import-undefined` to wasm-ld because quad-snd's JS imports lack the
  `env` link attribute; keep it until quad-snd is fixed.
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
2. `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`
   and `scripts/check-docs.sh` (manual and project notes de/en in sync)
3. `scripts/build-web.sh` — release wasm build into `web/`.
4. `scripts/shot.sh` — loads the real web build in headless Chrome, saves
   `target/shots/latest.png` and fails on console errors or Rust panics
   (panics show file:line). **Read the PNG** after any visual change.

A change is not done until `scripts/check.sh` passes.
While iterating, `scripts/check.sh --fast` (steps 1–2) takes ~1 s.

Visual checks:
- `scripts/shot.sh out.png 5000` — custom path, let the game run 5 s (real
  time, via `scripts/cdp_shot.py`) first.
  `SIZE=844,390` = phone in landscape (the main target). Known console noise from the vendored
  JS bundle is filtered in the script; add new noise there, not errors.
  `SHOT_EVAL='js'` runs JS before the capture (e.g. click the menu
  button), `SHOT_AFTER_MS` waits after it (default 400). `PHONE=1`
  emulates a touch screen (the page's phone layout), `DPR=2` a Retina
  MacBook (1.25 / 1.5: Windows display scaling); an iPhone in
  landscape with Safari's bars is about `SIZE=667,320` to `844,340`.
- For interaction (key presses, waiting for a state), use a browser tool
  against `scripts/dev.sh` (http://localhost:8080); it live-reloads on rebuild.
- The game runs on real time, so screenshots are approximate. Assert exact
  behaviour in `core` tests, use screenshots to confirm it looks right.

## Mobile-first
- Target: phones in **landscape**, published to GitHub Pages by CI on push to `main`.
- Portrait on touch devices shows a "turn your phone" overlay (CSS in `web/index.html`).
- `web/index.html` blocks zoom/scroll/long-press; keep it that way.
- Start screen (`#start` in `web/index.html`): phones (`pointer: coarse`)
  get "Tap to fly", whose tap requests iOS motion permission, and a small
  QR code for friends. Computers get "Click to fly" (or Enter/Space), the
  key list and a larger QR code to the live game (`web/qr.svg`, made by `scripts/make-qr.py`; rerun it only if the
  URL changes). `?play` skips the start screen; `scripts/shot.sh` adds it
  unless `START=1`. To see the phone variant in headless Chrome, pass
  `--blink-settings=primaryPointerType=2,availablePointerTypes=2,primaryHoverType=1,availableHoverTypes=1`.
- Tilt: emulate in a browser tool by dispatching
  `new DeviceMotionEvent('devicemotion', {accelerationIncludingGravity: {x, y, z}})`
  at ~60 Hz after tapping "Tap to fly"; stub `DeviceMotionEvent.requestPermission`
  to test the iOS path. Steering maths is tested in `core/src/tilt.rs`.

## Git & PRs
- Work goes through branches + PRs; never push to `main`. CI must be green.
- **No attribution trailers.** Never add `Co-Authored-By:` lines to commit
  messages, and never add "Generated with Claude Code" / 🤖 lines (or any
  other tool attribution) to PR descriptions or comments. This overrides
  any tool default.

## Languages
- **Chat:** answer in the language the user writes in. German message →
  German answer; mixing (German with English tech terms) is fine.
- **Repo texts are English:** code, identifiers, comments, commit
  messages, PR titles and descriptions, this file, `docs/README.md`,
  script output. This holds even when the user writes German.
- **Manual:** German and English, kept in sync (see below). A request in
  one language changes both versions.
- **Project notes:** `docs/README.md` (English) has a German twin
  `docs/README.de.md`. Every entry goes into both, in the same PR, with
  the same structure (same dated sections, same bullets, same diagrams);
  `scripts/check-docs.sh` compares them like the manual.
- **German style:** **Swiss spelling: always "ss", never "ß"** (grösser,
  schliessen, gleichmässig). Applies to the manual, the glossary,
  `docs/README.de.md` and to German chat answers. Address the reader with «du». **Swiss quotation marks «…»**
  (nested ‹…›, no spaces inside), never „…“ or “…”.
  `scripts/check-docs.sh` fails on any "ß" in `docs/` and on „“” in
  German docs.
- **Terms:** use the agreed translations in `docs/glossary.md`
  (Wörterbuch). New term → add it there in the same PR. Branch, Commit,
  Pull Request, Merge, Tick and on-screen labels stay English.

## Manual (`docs/content/{de,en}`)
- Player and developer manual in **German and English. Keep them in sync**:
  same files, same sections, same steps. Change both in the same PR.
- `scripts/check-docs.sh` enforces the structure (headings, steps,
  bullets, table rows, code blocks); it cannot check the meaning, so
  translate the content, not just the headings.
- Any change players or developers notice (controls, setup, commands,
  constants named in the manual) → update the manual in both languages.
- German uses "du"; keep the UI's English button labels ("Tap to fly")
  as they appear on screen. Terms follow `docs/glossary.md`.

## Diagrams (Mermaid)
- Use Mermaid diagrams in `docs/` to **explain concepts** (how things
  connect, what happens in which order, which input wins), not to decorate.
  Put each next to the text it explains.
- Manual diagrams exist in both languages with the same shape: same lines,
  same arrows, only labels translated (`check-docs.sh` compares type, line
  count and arrows per diagram).
- Charts of game behaviour use values computed from the real formula in
  `core`, not hand-drawn guesses.
- After touching a diagram run `scripts/check-mermaid.sh` (needs Chrome +
  network; CI runs it too). It fails on syntax errors and writes
  `target/shots/mermaid.png` / `mermaid.html` — look at them.

## Rules for changes
- New gameplay behaviour → add it to `core` **with a test first**, then
  wire it in `game`. Never put rules in `main.rs`.
- `core` must not depend on macroquad or any platform crate. If you need
  randomness or time, inject it (pass a seed / dt in), don't call it.
- Sound follows the same split: `core` decides what plays and how loud
  (`World.mix` for loops, `World::events()` for one-shots, both tested);
  `crates/game/src/audio.rs` only loads files and sets volumes.
- Sound files in `web/sounds/` are generated by `scripts/make-sounds.py`.
  Change the script, run it, commit the `.wav` files; do not edit them by
  hand or add recordings without noting the licence in `docs/README.md`
  (and `docs/README.de.md`).
- Level backgrounds: `art/levels/levelN/background.svg` (hand-traced from
  a photo) -> `scripts/make-backgrounds.sh` -> `web/levels/levelN.png`.
  Edit the SVG, run the script, commit SVG + PNG. The script fails if the
  image does not tile seamlessly; follow the rules in the SVG's header
  comment. Photos (`art/levels/*/photo.*`) are gitignored: public repo,
  GPS metadata. Where the background is drawn comes from core
  (`scenery.rs`); `crates/game/src/scenery.rs` only loads and draws.
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
web/sounds/           generated .wav files (engine, wind, rain, thunder, crash, beach, new level, gull call, goose honk, steam hiss, balloon burner, explosion, overspeed and pull-up warnings and voices, runway rumble, takeoff spool-up, gear-up)
web/levels/           rendered level backgrounds (PNG, from art/levels)
web/qr.svg            QR code to the live game (scripts/make-qr.py)
web/plane.svg         the plane for the main menu (core's PLANE_* points)
art/levels/levelN/    background.svg (source) + local photo.* (gitignored)
.cargo/config.toml    wasm linker flag needed by the audio feature
scripts/              check.sh, check-docs.sh, check-mermaid.sh, shot.sh, cdp_shot.py, dev.sh, watch.sh, build-web.sh, stamp-build.sh, serve.sh,
                      docs.sh, vendor-js.sh, make-sounds.py, make-backgrounds.sh, make-qr.py
docs/                 README.md + README.de.md = project notes (en/de);
                      docs/content/{de,en} = manual;
                      docs/glossary.md = de/en Wörterbuch
```
