# Spec: leaderboard rate limits and data cleanup

Status: proposed, 14 November 2026. Review finding 8 on PR #45 (and 7,
the repeated replay, which the same mechanism bounds).

## Problem

The leaderboard server (`crates/server`) has no limits and never deletes
anything:

- `POST /v1/players` needs no token and has no limit. A script can create
  pilots in a loop and fill the 1 GB Fly volume.
- Every page load creates one run per level (`getSeed` for each level in
  `web/board.js`). Most of these runs are never flown.
  Nothing deletes them.
- `POST /v1/runs/{id}/replay` flies the whole replay again on every call
  (`spawn_blocking`, no bound). The same replay can be sent again and
  again: `counted` does not change, so the update always matches. One
  free account can keep a `shared-cpu-1x` machine busy for everyone.
- `players` rows stay forever, also for pilots whose token is long gone
  (cleared browser data, iOS storage purges).

## Goals

- Bound the work and the storage one client can cause, without an honest
  player ever noticing.
- Keep it simple: one machine, one SQLite file (the documented
  constraint in `fly.toml`), no new service.
- Do not store anything new about people. IP addresses stay in memory
  only, for the length of a limit's window.

## Non-goals

- CAPTCHAs, accounts, e-mail.
- Limits across several machines (there is one, by design).
- Stopping a determined attacker with many IP addresses: this bounds
  cost and noise, it does not prove identity.
- The seed being known before the flight: see
  [leaderboard-seed-reveal.md](leaderboard-seed-reveal.md).

## Design

### 1. Who is asking

- **Pilot:** the token, as now. Limits on runs and replays count per
  pilot.
- **IP address:** only for sign-up, the one call without a token. Behind
  Fly's proxy the socket address is the proxy, so the server reads the
  client address from a header named in `TRUSTED_IP_HEADER`
  (`Fly-Client-IP` in `fly.toml`). Unset (local, stage): the socket
  address. The header is trusted only when that variable is set, so it
  cannot be faked where no proxy sets it.

Schools and families share one address: the sign-up limit per address
must allow a class of children starting at once (see the numbers below).

### 2. Token buckets in memory

A small `Limiter` in `crates/server` (no new dependency): a map from key
to `(tokens, last_refill_ms)`, refilled by time, behind the existing
mutex style. Entries not touched for one window are dropped by the
cleanup task (below), so the map cannot grow without bound.

| What | Key | Burst | Refill | Why these numbers |
|---|---|---|---|---|
| Sign-up | IP address | 40 | 40 per hour | a school class on one Wi-Fi |
| Sign-up, all together | (global) | 300 | 300 per hour | a second line if addresses rotate |
| New run | pilot | `LEVEL_COUNT` + 4 | 60 per hour | one per flight; a flight lasts at least about a minute |
| Replay | pilot | 6 | 30 per hour | one per "Level complete", a few retries |

Over the limit: `429 Too Many Requests` with `Retry-After` (seconds) and
`{ "error": "slow down" }`. The numbers are constants in `lib.rs`, named
like the others (`SIGNUP_PER_HOUR`, ...), so the manual can name them.

### 3. Replays: a run only moves forward

- `runs` gets `last_steps INTEGER NOT NULL DEFAULT 0`: the length of the
  last replay accepted for this run.
- A replay whose header and length show it is not longer than
  `last_steps` is refused with `409 Conflict` **before** it is flown
  again. The length in ticks is counted from the records without
  simulating (cheap: sum of the run counts); `replay::steps_of(bytes)`
  in core, with a test that it matches `Outcome::steps`.
- Then the replay is flown; on success `last_steps` is updated together
  with `counted`.
- One replay at a time for the whole server: a `tokio::sync::Semaphore`
  with `available_parallelism()` permits (one on `shared-cpu-1x`).
  Waiting longer than 5 s for a permit: `503` with `Retry-After: 5`.
  Measured on an Apple Silicon Mac (release build): about 1.5 µs per
  tick, so a level-1 replay (about 4,000 ticks) takes about 6 ms and the
  longest allowed one (`MAX_REPLAY_TICKS`, three hours) about 1 s. A
  `shared-cpu-1x` machine is likely several times slower: measure on the
  stage and on Fly before fixing the timeout. Honest replays are short,
  so a queue only forms under abuse.
- Consider lowering `MAX_REPLAY_TICKS`: a flight of all levels lasts
  minutes, not hours. Three hours is about 650,000 ticks of work per
  request; 30 minutes would cut that by six.

The page already treats any failure as "Not counted online". On `429`
and `503` it retries the replay once after `Retry-After` (at most 10 s),
since a landed level is worth one more try.

### 4. Fewer runs from the page

Most runs today are made on page load and never flown. With the seed
taken at "Tap to fly" (finding 9 of the review, and
[leaderboard-seed-reveal.md](leaderboard-seed-reveal.md)), the page
creates a run only when a flight starts. Until that lands, the run
limit above is sized for today's prefetching (`LEVEL_COUNT` + 4 burst).

### 5. Cleanup

A task started in `main.rs` runs every hour (and once at start):

- Delete runs that never counted a level (`counted = 0`) and are older
  than `RUN_MAX_AGE_MS` (24 h): they can no longer be used.
- Keep runs that counted a level: they are the history behind `best`
  (and the place replays would go, review gap "keep replays").
- `players` gets `last_seen_ms`, updated on any call with the token (at
  most once an hour per pilot, to keep writes rare). Delete pilots with
  no row in `best` and not seen for 90 days, with their runs. Pilots on
  the board stay.
- Drop limiter entries idle for longer than their window.
- `PRAGMA optimize` after the deletes. No `VACUUM` (it rewrites the whole
  file); SQLite reuses the free pages.

A size guard: before `INSERT INTO players`, if the database file is
larger than `DB_MAX_BYTES` (default 512 MB, half the volume), sign-ups
answer `503` and the server logs it once an hour. Existing pilots keep
playing.

### 6. Schema change

`CREATE TABLE IF NOT EXISTS` does not add columns to existing tables.
Introduce `PRAGMA user_version` migrations in `App::with`:
version 1 = today's schema, version 2 adds `runs.last_steps` and
`players.last_seen_ms`. Each step runs once in a transaction.

### 7. Logging

Per hour, one line with counts: sign-ups, runs, replays accepted and
refused, `429`s by kind, rows deleted. No IP addresses and no tokens in
logs.

## Privacy

IP addresses are personal data. They are used only as limiter keys, in
memory, and dropped after the window (at most an hour). Nothing about
them is written to the database or logs. Say so in the manual's
leaderboard section (both languages). The legal note in the project
notes still applies; this design was chosen so that it does not add a
tracking-like signal.

## Tests

- `Limiter` unit tests with an injected clock: burst, refill, `Retry-After`.
- `App` gets an injectable clock (`now_ms` from the state, real time in
  `main.rs`), so API tests stop rewriting `started_ms` in the database.
- API tests:
  - the 41st sign-up from one address within an hour gets `429`; one
    from another address still works;
  - the same replay twice: the second gets `409` and is not flown again
    (a counter on `App` of replays flown);
  - replays over the pilot limit get `429`;
  - cleanup deletes an unused run older than 24 h and keeps a counted
    one; deletes an idle pilot without a best time after 90 days and
    keeps one on the board;
  - migration from a version-1 database file keeps its rows.
- `scripts/stage.sh e2e`: unchanged flow still passes; add a sign-up
  burst that must hit `429` (the stage sees all requests from one
  address, which suits this test).

## Rollout

1. Server first (`fly deploy`): the page treats `429` like any failure,
   so old pages keep working.
2. Then the page's retry on `429` / `503`.
3. Watch the hourly counts for a week; tune the constants.

## Open questions

- Are 40 sign-ups per address per hour enough for a school event? A
  higher limit only for the first hour after a deploy is not worth the
  complexity; better to raise the constant if it happens.
- Should the global sign-up ceiling page someone (e-mail, Fly alert)?
  Today nobody would notice except in the logs.
