// Tests for web/board.js, the page's side of the leaderboard, against a
// fake server. Node's own test runner, no packages:
//   node --test tests/web/*.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import vm from "node:vm";

// board.js is a plain browser script: run it in a fresh context and take
// its two functions from there.
const source = readFileSync(new URL("../../web/board.js", import.meta.url), "utf8");
const load = () => {
  const context = vm.createContext({});
  vm.runInContext(source, context, { filename: "web/board.js" });
  return context;
};

const API = "https://board.test";

// A fake server: `routes` maps "METHOD /path" to (request) => [status, body].
// Every request is kept in `requests`.
const server = (routes) => {
  const requests = [];
  const fetch = async (url, init = {}) => {
    const method = init.method || "GET";
    const path = url.slice(API.length);
    const request = { method, path, headers: init.headers || {}, body: init.body };
    requests.push(request);
    const bare = path.split("?")[0];
    const route = routes[`${method} ${bare}`] || routes[`${method} ${bare.replace(/\/\d+\//, "/{id}/")}`];
    if (!route) return { ok: false, status: 404, json: async () => ({ error: "no route" }) };
    const [status, body, headers = {}] = route(request);
    return { ok: status >= 200 && status < 300, status, json: async () => body, headers: { get: (k) => headers[k] ?? null } };
  };
  return { fetch, requests };
};

// A server that signs up pilot "tok-N" and hands out runs with seeds.
const friendly = (extra = {}) => {
  let pilots = 0;
  let runs = 0;
  return server({
    "POST /v1/players": () => [200, { token: `tok-${++pilots}`, name: `Pilot ${pilots}` }],
    "POST /v1/runs": (r) => [200, { run_id: ++runs, seed: String(1000 + runs), level: JSON.parse(r.body).level }],
    ...extra,
  });
};

const memoryStorage = (items = {}) => ({
  items,
  getItem: (k) => (k in items ? items[k] : null),
  setItem: (k, v) => { items[k] = v; },
});

const make = ({ api = API, srv = friendly(), storage = memoryStorage(), clock = { t: 1_000_000 } } = {}) => {
  const ctx = load();
  const warnings = [];
  const waits = [];
  const board = ctx.createBoard({
    api,
    fetch: srv.fetch,
    storage,
    now: () => clock.t,
    wait: async (ms) => { waits.push(ms); },
    warn: (...a) => warnings.push(a.join(" ")),
  });
  const memory = { buffer: new ArrayBuffer(64) };
  const game = board.imports(() => memory);
  return { board, game, srv, storage, clock, memory, warnings, waits };
};

// Let the board's pending requests finish.
const settle = () => new Promise((r) => setTimeout(r, 0));

test("no server: nothing is fetched and no flight is ranked", async () => {
  const { board, game, srv } = make({ api: "" });
  assert.equal(board.on, false);
  await board.start(4);
  assert.equal(game.board_ready(1), 0);
  assert.equal(srv.requests.length, 0);
});

test("one sign-up, then a seed ready for every level", async () => {
  const { board, game, srv, storage } = make();
  await board.start(4);
  const signUps = srv.requests.filter((r) => r.path === "/v1/players");
  assert.equal(signUps.length, 1, "four levels share one sign-up");
  assert.equal(srv.requests.filter((r) => r.path === "/v1/runs").length, 4);
  for (const level of [1, 2, 3, 4]) assert.equal(game.board_ready(level), 1);
  assert.equal(JSON.parse(storage.items["airplane-scoring.pilot"]).token, "tok-1");
  assert.equal(srv.requests[1].headers.Authorization, "Bearer tok-1");
});

test("a pilot kept in this browser is used again, but only for the same server", async () => {
  const kept = JSON.stringify({ api: API, token: "old", name: "Ada" });
  const same = make({ storage: memoryStorage({ "airplane-scoring.pilot": kept }) });
  await same.board.start(1);
  assert.equal(same.srv.requests.filter((r) => r.path === "/v1/players").length, 0);
  assert.equal(same.srv.requests[0].headers.Authorization, "Bearer old");
  assert.equal(same.board.name(), "Ada");

  const other = JSON.stringify({ api: "https://elsewhere.test", token: "old", name: "Ada" });
  const moved = make({ storage: memoryStorage({ "airplane-scoring.pilot": other }) });
  await moved.board.start(1);
  assert.equal(moved.srv.requests[0].path, "/v1/players");
});

test("storage that throws (private mode) still lets the pilot fly ranked", async () => {
  const broken = { getItem: () => { throw new Error("denied"); }, setItem: () => { throw new Error("denied"); } };
  const { board, game } = make({ storage: broken });
  await board.start(1);
  assert.equal(game.board_ready(1), 1);
});

test("the seed reaches the game whole, all 64 bits", async () => {
  const srv = server({
    "POST /v1/players": () => [200, { token: "t", name: "P" }],
    "POST /v1/runs": () => [200, { run_id: 1, seed: "18446744073709551615" }],
  });
  const { board, game } = make({ srv });
  await board.start(1);
  assert.equal(game.board_seed_hi(1), 0xffffffff);
  assert.equal(game.board_seed_lo(1), 0xffffffff);
  // Signed 32-bit would be -1; the game reads these as u32.
  assert.ok(game.board_seed_lo(1) > 0);
});

test("taking a seed uses it up and gets the next flight's", async () => {
  const { board, game, srv } = make();
  await board.start(1);
  game.board_take(1);
  assert.equal(game.board_ready(1), 0, "a seed is for one flight");
  await settle();
  assert.equal(game.board_ready(1), 1);
  assert.equal(srv.requests.filter((r) => r.path === "/v1/runs").length, 2);
});

test("an old seed is dropped and a failed request is not retried at once", async () => {
  let fail = false;
  let runs = 0;
  const srv = server({
    "POST /v1/players": () => [200, { token: "t", name: "P" }],
    "POST /v1/runs": () => (fail ? [503, {}] : [200, { run_id: ++runs, seed: "5" }]),
  });
  const { board, game, clock, warnings } = make({ srv });
  await board.start(1);
  clock.t += 12 * 60 * 60 * 1000 + 1; // older than the page keeps them
  fail = true;
  assert.equal(game.board_ready(1), 0);
  await settle();
  assert.equal(warnings.length, 1);
  const tries = () => srv.requests.filter((r) => r.path === "/v1/runs").length;
  const before = tries();
  game.board_ready(1); // every frame asks
  game.board_ready(1);
  assert.equal(tries(), before, "no new request within 10 s");
  clock.t += 10_001;
  fail = false;
  game.board_ready(1);
  await settle();
  assert.equal(tries(), before + 1);
  assert.equal(game.board_ready(1), 1);
});

test("a level complete: waiting, then the rank the server gave", async () => {
  const srv = friendly({
    "POST /v1/runs/{id}/replay": () => [200, { levels: [{ level: 1, millis: 61450, rank: 3, pilots: 12 }] }],
  });
  const { board, game, memory } = make({ srv });
  await board.start(1);
  game.board_take(1);
  assert.equal(game.board_status(1), -1, "nothing sent yet");
  new Uint8Array(memory.buffer).set([1, 2, 3, 4], 8);
  const sent = game.board_submit(8, 4, 1, 61450);
  assert.equal(game.board_status(1), 0);
  new Uint8Array(memory.buffer).fill(9); // the game moves on
  await sent;
  assert.deepEqual([game.board_status(1), game.board_rank(1), game.board_pilots(1)], [1, 3, 12]);
  const replay = srv.requests.find((r) => r.path === "/v1/runs/1/replay?level=1&shown=61450");
  assert.ok(replay, "the time on screen goes along, for the server's log");
  assert.deepEqual([...replay.body], [1, 2, 3, 4], "the bytes as they were at the call");
  assert.equal(replay.headers["Content-Type"], "application/octet-stream");
});

test("a level the server did not count shows as not counted", async () => {
  const srv = friendly({ "POST /v1/runs/{id}/replay": () => [200, { levels: [] }] });
  const { board, game } = make({ srv });
  await board.start(1);
  game.board_take(1);
  await game.board_submit(0, 4, 1);
  assert.equal(game.board_status(1), -1);
});

test("a refused replay is not counted and signs up nobody new", async () => {
  const srv = friendly({ "POST /v1/runs/{id}/replay": () => [401, { error: "who?" }] });
  const { board, game, warnings } = make({ srv });
  await board.start(1);
  game.board_take(1);
  await game.board_submit(0, 4, 1);
  assert.equal(game.board_status(1), -1);
  assert.equal(srv.requests.filter((r) => r.path === "/v1/players").length, 1);
  assert.equal(warnings.length, 1);
});

test("a flight without a seed sends nothing", async () => {
  const { board, game, srv } = make();
  await board.start(1);
  const before = srv.requests.length;
  assert.equal(game.board_submit(0, 4, 1), undefined);
  assert.equal(srv.requests.length, before);
  assert.equal(game.board_status(1), -1);
});

test("a late answer for the last flight does not show in the next one", async () => {
  let answer;
  const srv = friendly({ "POST /v1/runs/{id}/replay": () => answer });
  const { board, game } = make({ srv });
  await board.start(1);
  game.board_take(1);
  answer = [200, { levels: [{ level: 1, millis: 1, rank: 1, pilots: 1 }] }];
  const late = game.board_submit(0, 4, 1);
  await settle();
  game.board_take(1); // a new flight before the answer was read
  await late;
  assert.equal(game.board_status(1), -1);
});

test("an unknown token on a normal call: a new pilot, and the call once more", async () => {
  const kept = JSON.stringify({ api: API, token: "gone", name: "Old" });
  const srv = friendly({
    "PUT /v1/players/me": (r) => (r.headers.Authorization === "Bearer gone" ? [401, {}] : [200, { name: "New" }]),
  });
  const { board } = make({ srv, storage: memoryStorage({ "airplane-scoring.pilot": kept }) });
  const r = await board.call("PUT", "/v1/players/me", { name: "New" });
  assert.equal(r.name, "New");
  assert.deepEqual(srv.requests.map((q) => q.path), ["/v1/players/me", "/v1/players", "/v1/players/me"]);
});

test("a time reads like the game's best times", () => {
  const { boardClock } = load();
  assert.equal(boardClock(61450), "1:01.450");
  assert.equal(boardClock(5), "0:00.005");
  assert.equal(boardClock(600000), "10:00.000");
});

test("a level's list: the top, and your own place under it if you are not in it", () => {
  const { boardRows } = load();
  const top = [
    { rank: 1, name: "Ada", millis: 61450, me: false },
    { rank: 2, name: "Bo", millis: 62000, me: true },
  ];
  const plain = (rows) => JSON.parse(JSON.stringify(rows)); // out of the vm context
  assert.deepEqual(plain(boardRows({ level: 4, pilots: 2, top, me: { rank: 2, name: "Bo", millis: 62000 } })), [
    { left: "1. Ada", right: "1:01.450", me: false },
    { left: "2. Bo", right: "1:02.000", me: true },
  ], "in the top: not twice");
  const below = boardRows({ level: 4, pilots: 30, top: [top[0]], me: { rank: 17, name: "Cy", millis: 90000 } });
  assert.deepEqual(plain(below.at(-1)), { left: "17. Cy (you, of 30)", right: "1:30.000", me: true });
  assert.deepEqual(plain(boardRows({ level: 5, pilots: 0, top: [], me: null })), [
    { left: "No times yet. Be the first!", right: "", me: false },
  ]);
  // Names are other people's text: they stay text, the page never makes HTML of them.
  assert.equal(boardRows({ level: 1, pilots: 1, top: [{ rank: 1, name: "<b>x</b>", millis: 1, me: false }], me: null })[0].left, "1. <b>x</b>");
});

test("the line with all pilots' landed flights", () => {
  const { boardFlights } = load();
  assert.equal(boardFlights({ flights: 0 }), "No flights landed yet");
  assert.equal(boardFlights({ flights: 1 }), "1 flight landed by all pilots");
  assert.equal(boardFlights({ flights: 1234567 }), "1,234,567 flights landed by all pilots");
  // An older server without the count: no line.
  assert.equal(boardFlights({ level: 1, pilots: 0, top: [] }), "");
  assert.equal(boardFlights({ flights: "lots" }), "");
});

test("only the newest request may change the page", () => {
  const { newestOnly } = load();
  const latest = newestOnly();
  const slow = latest.next(); // e.g. a tab's list
  const rename = latest.next(); // then a rename
  assert.equal(latest.current(slow), false, "the older answer is dropped, even for the same level");
  assert.equal(latest.current(rename), true);
  assert.equal(latest.current(latest.next()), true);
  assert.equal(latest.current(rename), false);
});

test("a busy server gets the replay once more, after the time it asks for", async () => {
  let answers = [[429, { error: "slow down" }, { "Retry-After": "3" }],
    [200, { levels: [{ level: 1, millis: 61450, rank: 2, pilots: 5 }] }]];
  const srv = friendly({ "POST /v1/runs/{id}/replay": () => answers.shift() });
  const { board, game, waits } = make({ srv });
  await board.start(1);
  game.board_take(1);
  await game.board_submit(0, 4, 1, 61450);
  assert.deepEqual(waits, [3000]);
  assert.deepEqual([game.board_status(1), game.board_rank(1)], [1, 2]);
  assert.equal(srv.requests.filter((r) => r.path.startsWith("/v1/runs/1/replay")).length, 2);
});

test("only once, and not when the wait is long or the replay was refused", async () => {
  for (const [first, tries] of [
    [[503, { error: "busy" }, { "Retry-After": "5" }], 2], // and busy again: given up
    [[429, { error: "slow down" }, { "Retry-After": "60" }], 1], // too long to wait
    [[429, { error: "slow down" }], 1], // no Retry-After
    [[422, { error: "the replay is of another flight" }], 1],
  ]) {
    const srv = friendly({ "POST /v1/runs/{id}/replay": () => first });
    const { board, game, warnings } = make({ srv });
    await board.start(1);
    game.board_take(1);
    await game.board_submit(0, 4, 1, 61450);
    assert.equal(game.board_status(1), -1, "not counted");
    assert.equal(srv.requests.filter((r) => r.path.startsWith("/v1/runs/1/replay")).length, tries, JSON.stringify(first));
    assert.equal(warnings.length, 1);
  }
});

test("over the seed limit: no new request before the server's Retry-After", async () => {
  let limited = true;
  const srv = friendly({
    "POST /v1/runs": (r) => (limited
      ? [429, { error: "slow down" }, { "Retry-After": "60" }]
      : [200, { run_id: 7, seed: "1007", level: JSON.parse(r.body).level }]),
  });
  const { board, game, clock } = make({ srv });
  await board.start(1);
  const runs = () => srv.requests.filter((r) => r.path === "/v1/runs").length;
  assert.equal(runs(), 1);
  clock.t += 30_000; // past RETRY_MS, before Retry-After
  assert.equal(game.board_ready(1), 0);
  await settle();
  assert.equal(runs(), 1, "waits for the server");
  limited = false;
  clock.t += 31_000;
  game.board_ready(1);
  await settle();
  assert.equal(runs(), 2);
  assert.equal(game.board_ready(1), 1);
});
