# Spec: reveal the seed at "Tap to fly", and keep the flight live

Status: proposed, 14 November 2026. Review finding 9 on PR #45 (with
finding 4, seeds used up in the menu).

## Problem

A flight is decided by its seed and the inputs. Today the seed is known
long before the flight:

- The page fetches a run (and with it the seed) for every level when it
  loads, and keeps it up to 12 hours (`SEED_MAX_AGE_MS` in
  `web/board.js`). The server accepts it for 24 hours
  (`RUN_MAX_AGE_MS`, `crates/server/src/lib.rs`).
- The only time check is the lower bound: a replay may not be longer
  than the real time since the run was handed out. There is no upper
  bound.
- `core` is in a public repository, and `demo_flight` in
  `crates/core/src/replay.rs` already shows how to search a level with
  look-ahead.

So a cheater can take a seed, search offline for the fastest inputs
(hours of computing, any number of tries), wait until enough real time
has passed, and send a replay that is a real, legal flight. It tops the
board. Variants:

- **Seed shopping:** fetch many runs, measure offline which seed gives
  the easiest level, fly only that one.
- **Pausing:** fly a little, pause, search the next seconds offline,
  continue. Today a pause of any length is allowed.

Also, `Board::frame` takes a seed while the start screen shows, so every
new world in the menu uses up a run (finding 4).

## Goals

- The seed is unknown until the player starts the flight.
- A ranked flight is flown roughly live: from the reveal to each "Level
  complete", the real time is close to the time flown.
- "Tap to fly" stays instant when the server answers quickly, and never
  waits more than a moment when it does not.
- Seed shopping costs a run each time, and runs are limited
  ([leaderboard-rate-limits.md](leaderboard-rate-limits.md)).

## Non-goals

- A bot that plays live, searching ahead while it flies (as `demo_flight`
  does). It gets a seed, flies in real time and passes. Against that only
  detection helps (keeping replays, review gap "bot detection"), not
  timing.
- Hiding the obstacles from the client (the server streaming the world
  ahead). The game draws what is ahead, so the client must know it; this
  would mean online-only play and a different architecture.
- A daily board with one shared seed (separate idea, gap 8 of the plan):
  it would make seed shopping pointless for that board.

## Design

### 1. Reserve early, reveal at the start

The run is split in two steps.

- `POST /v1/runs` `{ "level": 1 }` creates a **reservation**: a run id,
  no seed in the answer. The page may hold one per level, as today, so
  the reveal is a single short request.
- `POST /v1/runs/{id}/start` **reveals** the seed: `{ "seed": "..." }`.
  The server sets `revealed_ms = now`. A run can be revealed once
  (`409` after that).
- Revealing a run closes any other revealed, still open run of the same
  pilot: one live flight per pilot. A replay for a closed run gets `409`.
- An unrevealed reservation expires after 24 h (cleanup in the rate
  limit spec); a revealed run after `MAX_REPLAY_TICKS` of real time.

The seed is drawn when the run is created, as now, but only sent at the
reveal.

### 2. When the page reveals

In `fly()` (`web/index.html`), which is already `async` (it waits for the
iOS motion permission):

1. Take this level's reservation, call `/start`.
2. Wait for the answer at most `REVEAL_WAIT_MS` (1500 ms), in parallel
   with the motion permission. The button shows "Ready..." meanwhile.
3. Answer in time: `ready[level]` gets the seed, then `flying = true`.
   The game's `Board::frame` takes it before the first tick, as now.
4. No answer in time (offline, server asleep): fly unranked, as today.
   A late answer is ignored for this flight.

To keep the reveal fast after the machine slept (`auto_stop_machines`),
the page sends a cheap `GET /health` on load, which wakes it (see the
review finding about the health check).

The game takes a seed only once the flight has started, not while the
start screen shows: `board_ready` is true only for a revealed seed, so
the menu no longer uses up runs (finding 4). The restart paths (game
over, pause menu "Restart") go through the same reveal; the page calls
`/start` on a fresh reservation before the new flight ticks
(finding 1 must be fixed first, so the game waits for it).

### 3. The flight must be live

The server keeps today's lower bound and adds an upper bound, both from
`revealed_ms`:

```
flown_ms = steps × TICK_SECONDS × 1000
flown_ms ≤ now − revealed_ms + CLOCK_SLACK_MS              (not faster than real time, as now)
now − revealed_ms ≤ flown_ms + MAX_IDLE_MS + network slack (not much slower than real time)
```

`MAX_IDLE_MS` = 90 s for the whole flight. What eats into it: time in
the pause menu, the 3-2-1 countdown after a pause, the page hidden or
the phone locked, the time from the reveal to the first tick. Waiting at
"Level complete" does not: the world keeps ticking there, so those ticks
are in `steps`.

A replay over the limit: `422` `{ "error": "paused too long" }`. Levels
already counted on this run stay counted.

For players: pause as long as you like, but a flight paused for more
than about a minute and a half in all is not counted online from then
on. The page knows its own pause time; when it passes `MAX_IDLE_MS` it
marks the run as given up (no more replays sent), and the game shows
"Not counted online: paused too long" at the next "Level complete"
(a new `board_status` value, `-2`). Manual: both languages, Playing →
Leaderboard.

### 4. What this leaves an attacker

- Offline search before the flight: gone, the seed is unknown.
- Offline search during a pause: at most 90 s per flight in all, instead
  of hours.
- Seed shopping: each look at a seed costs a revealed run and closes the
  previous one. With the run limit (60 per hour) and every reveal
  logged in the hourly counts, a pilot revealing far more runs than
  they finish stands out. A further step, if needed: refuse a reveal
  while the pilot has more than N revealed runs without a single replay
  in the last hour.
- A live bot: not stopped (non-goal).

## Data

`runs` gets `revealed_ms INTEGER` (NULL = reservation only) and
`closed INTEGER NOT NULL DEFAULT 0`. Migration with `PRAGMA
user_version`, as in the rate limit spec (same release or after it).

## Tests

With the injectable clock from the rate limit spec:

- `POST /v1/runs` returns no seed; `/start` returns it; a second
  `/start` gets `409`.
- A replay for a reservation that was never revealed gets `409`.
- Revealing a second run closes the first: a replay for the first gets
  `409`.
- A replay at `revealed_ms + flown + 60 s` is counted; at
  `revealed_ms + flown + 100 s` it gets `422 paused too long`.
- The lower bound still holds (sent too early: `422`), now measured from
  `revealed_ms`.
- Page: with the server stopped, "Tap to fly" starts within
  `REVEAL_WAIT_MS` and the flight is unranked (headless check with
  `scripts/shot.sh` and `SHOT_EVAL`, or a browser tool against the stage).
- `scripts/stage.sh e2e`: reserve, reveal, fly with the wasm demo pilot,
  send in time (counted); a second run sent after the allowed idle
  (refused). The second case waits about two minutes, so make it
  optional (`e2e --slow`).

## Rollout

The API changes shape (`POST /v1/runs` no longer returns a seed), so
page and server must change together:

1. Server accepts both for one release: `POST /v1/runs` still returns a
   seed for old pages (marked "revealed at creation", upper bound not
   applied), `/start` exists for new ones.
2. Deploy the page that uses `/start`. GitHub Pages and installed home
   screen apps pick it up within minutes (`scripts/stamp-build.sh`).
3. A day later, the server stops returning seeds from `POST /v1/runs`.

## Open questions

- Is 90 s of pause per flight right? Long enough for a phone call
  ending or a quick look at another app; short enough that searching
  offline gives little. Could be per level instead of per flight.
- Should a flight that went over the idle limit tell the player right
  away (a note in the pause menu: "Leaderboard: this flight no longer
  counts") rather than at "Level complete"? Kinder, and cheap: the page
  knows.
- `REVEAL_WAIT_MS` 1500: measure the real round trip from a phone to Fly
  `fra` (warm and cold) on the stage-to-Fly path before fixing it.
