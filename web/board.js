// The online leaderboard, the page's side of it (crates/game/src/board.rs
// is the game's). The server (crates/server) hands out a seed for every
// flight and flies the replay again before a time counts, so the page
// only fetches seeds, sends the replay at every "Level complete" and
// shows the lists. No server, or it cannot be reached: flights are just
// not ranked.
//
// No DOM and no globals here: what it needs comes in as `env`, so
// tests/web/board.test.mjs runs it in Node with a fake server.
// index.html makes one with the real fetch and localStorage.
"use strict";

/**
 * env: {
 *   api:     the server's URL ("" = no leaderboard)
 *   fetch:   like window.fetch
 *   storage: like localStorage (may throw: private mode)
 *   now:     () => milliseconds, like Date.now
 *   warn:    (...args) => void, like console.warn
 * }
 */
function createBoard(env) {
  const API = env.api.replace(/\/+$/, "");

  // Who this browser is on the board: a secret token from the server
  // (no account, no e-mail). Kept in this browser only; with it gone,
  // the pilot starts again as a new one.
  const PILOT_KEY = "airplane-scoring.pilot";
  let pilot = null; // { api, token, name }
  try {
    const stored = JSON.parse(env.storage.getItem(PILOT_KEY) || "null");
    if (stored && stored.api === API && typeof stored.token === "string") pilot = stored;
  } catch (_) { /* nothing stored, or private mode */ }
  const keepPilot = () => {
    try { env.storage.setItem(PILOT_KEY, JSON.stringify(pilot)); } catch (_) { /* private mode: a new pilot next time */ }
  };
  let signingUp = null;
  const signUp = () => {
    if (!signingUp) {
      signingUp = (async () => {
        try {
          const res = await env.fetch(API + "/v1/players", { method: "POST" });
          if (!res.ok) throw new Error(`sign up: ${res.status}`);
          const p = await res.json();
          pilot = { api: API, token: p.token, name: p.name };
          keepPilot();
        } finally { signingUp = null; }
      })();
    }
    return signingUp;
  };
  // One request to the server; `body` is JSON, or bytes (a replay).
  // `again`: on 401 (the server does not know the token) sign up as a
  // new pilot and try once more. Not for a replay: its run belongs to
  // the old pilot, so it could only fail.
  const call = async (method, path, body, again = !(body instanceof Uint8Array)) => {
    const headers = {};
    if (pilot) headers.Authorization = `Bearer ${pilot.token}`;
    let payload;
    if (body instanceof Uint8Array) { headers["Content-Type"] = "application/octet-stream"; payload = body; }
    else if (body) { headers["Content-Type"] = "application/json"; payload = JSON.stringify(body); }
    const res = await env.fetch(API + path, { method, headers, body: payload });
    if (res.status === 401 && pilot && again) {
      // The server does not know this token (any more): a new pilot.
      pilot = null;
      await signUp();
      return call(method, path, body, false);
    }
    if (!res.ok) throw new Error(`${method} ${path}: ${res.status}`);
    return res.json();
  };

  // A seed waits ready for each level, so that "Tap to fly" never
  // waits for the network: { runId, seed (BigInt), at }.
  const SEED_MAX_AGE_MS = 12 * 60 * 60 * 1000; // the server takes them for 24 h
  const RETRY_MS = 10 * 1000;
  const ready = {};
  const asked = {}; // level -> when the last request went out
  const fetching = {};
  const getSeed = async (level) => {
    if (!API || fetching[level] || ready[level]) return;
    fetching[level] = true;
    asked[level] = env.now();
    try {
      if (!pilot) await signUp();
      const run = await call("POST", "/v1/runs", { level });
      ready[level] = { runId: run.run_id, seed: BigInt(run.seed), at: env.now() };
    } catch (e) {
      env.warn("leaderboard: no seed, flights are not ranked for now", e);
    } finally { fetching[level] = false; }
  };
  const seedFor = (level) => {
    const r = ready[level];
    if (r && env.now() - r.at > SEED_MAX_AGE_MS) delete ready[level];
    if (!ready[level] && env.now() - (asked[level] || 0) > RETRY_MS) getSeed(level);
    return ready[level] || null;
  };

  let flying = null; // the run id of the flight in the air
  // This flight's levels: level -> { rank, pilots } once on the board,
  // "waiting" while its replay is out, "failed" if it was not counted.
  let standing = {};
  const ranked = (level) => typeof standing[level] === "object";

  // The functions the game imports (board.rs, mod js). `memory()`: the
  // wasm memory, for the replay's bytes.
  const imports = (memory) => ({
    board_ready: (level) => (seedFor(level) ? 1 : 0),
    board_seed_hi: (level) => Number(ready[level].seed >> 32n),
    board_seed_lo: (level) => Number(ready[level].seed & 0xffffffffn),
    board_take: (level) => {
      flying = ready[level].runId;
      standing = {};
      delete ready[level];
      getSeed(level); // the next flight's
    },
    // At "Level complete": the replay so far. The server counts the
    // levels it lands that it has not counted for this run yet.
    // `level`: the level just completed, whose standing the game shows.
    // (The promise is for tests; wasm ignores what a void import returns.)
    board_submit: (ptr, len, level) => {
      const runId = flying;
      if (!runId) return;
      // A copy: the game's memory moves on while the request runs.
      const replay = new Uint8Array(memory().buffer, ptr, len).slice();
      standing[level] = "waiting";
      const mine = standing; // this flight's; a new flight gets its own
      return call("POST", `/v1/runs/${runId}/replay`, replay)
        .then((r) => {
          for (const l of r.levels) mine[l.level] = { rank: l.rank, pilots: l.pilots };
          if (mine[level] === "waiting") mine[level] = "failed"; // not among them
        })
        .catch((e) => {
          env.warn("leaderboard: this level was not counted", e);
          mine[level] = "failed";
        });
    },
    // 1: on the board; 0: still waiting for the server; -1: not counted.
    board_status: (level) => (ranked(level) ? 1 : standing[level] === "waiting" ? 0 : -1),
    board_rank: (level) => (ranked(level) ? standing[level].rank : 0),
    board_pilots: (level) => (ranked(level) ? standing[level].pilots : 0),
  });

  return {
    on: Boolean(API),
    imports,
    call,
    /** Get a seed ready for each of the levels 1 to `levels`. */
    start: (levels) => Promise.all(Array.from({ length: levels }, (_, i) => getSeed(i + 1))),
    /** The pilot's name as the server knows it, or "". */
    name: () => (pilot ? pilot.name : ""),
    rename: (name) => {
      if (pilot) { pilot.name = name; keepPilot(); }
    },
  };
}

// "1:02.483", like the best times in the game (best_time_text).
function boardClock(millis) {
  const seconds = Math.floor(millis / 1000);
  return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}.${String(millis % 1000).padStart(3, "0")}`;
}
