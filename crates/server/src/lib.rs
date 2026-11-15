//! The leaderboard's HTTP API. The server trusts nothing the game
//! claims: a time counts only after its replay was flown again here,
//! with the same core, on a seed this server handed out.
//!
//! The score is the game's own: the time of a completed level
//! (`World::millis_in_level` at "Level complete"), fastest first, one
//! board per level. Each pilot is on a board once, with their fastest
//! time. A game over or a flight left early counts no time.
//!
//! - `POST /v1/players`: a new anonymous pilot. Returns the token that
//!   proves who they are from now on (only its hash is kept here).
//! - `PUT /v1/players/me`: `{ "name": .. }`, the name on the board.
//! - `POST /v1/runs`: `{ "level": 1 }` (the level the flight starts in).
//!   Returns a run id and the seed to fly with.
//! - `POST /v1/runs/{id}/replay?level=1&shown=61450`: the replay bytes
//!   so far, sent at every "Level complete". Returns the levels it
//!   landed that were not counted for this run before, each with the
//!   pilot's rank. `level` and `shown` (optional): the level just
//!   completed and the time the game showed, only for the log line each
//!   replay gets (`replay_line`); what counts is the time flown here.
//! - `GET /v1/leaderboard?level=1&limit=10`: the fastest pilots of a
//!   level, where the asking pilot stands (if a token came along), and
//!   `flights`: how many levels all pilots together have landed, flown
//!   again here (every level, ever).
//! - `GET /health`: "ok", without touching the database (for Fly's
//!   health check, which runs all the time).
//!
//! All but the first and last need `Authorization: Bearer <token>`.

use std::net::SocketAddr;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::atomic::{AtomicI64, AtomicU64};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use airplane_scoring_core::replay::{replay, start_of, steps_of};
use airplane_scoring_core::{start_level, TICK_SECONDS};
use axum::extract::{
    ConnectInfo, DefaultBodyLimit, FromRequestParts, Path, Query, RawQuery, State,
};
use axum::http::header::{AUTHORIZATION, CONTENT_TYPE};
use axum::http::request::Parts;
use axum::http::{HeaderValue, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::{body::Bytes, Json, Router};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tower_http::cors::{AllowOrigin, CorsLayer};

mod limits;
use limits::Limiter;

/// A replay may be this big: more than the busiest steering for
/// `MAX_REPLAY_TICKS` (4 bytes a tick, about 0.9 MB).
const MAX_REPLAY_BYTES: usize = 1024 * 1024;
/// A seed is good for this long after it was handed out.
const RUN_MAX_AGE_MS: i64 = 24 * 60 * 60 * 1000;
/// How much sooner than real time a flight may seem to end (clocks and
/// the network are not exact).
const CLOCK_SLACK_MS: i64 = 2_000;
/// How long a replay may wait for its turn to be flown again before the
/// server says it is busy.
const REPLAY_WAIT: std::time::Duration = std::time::Duration::from_secs(5);
const NAME_MAX_CHARS: usize = 16;
const BOARD_MAX: u32 = 100;

// Rate limits (`limits.rs`): (burst, per hour). Over a limit: 429 with
// `Retry-After`. Sized from the real board: a pilot made up to 12 runs in
// 10 minutes (each page load asks for a seed per level, every flight
// for one more) and a level with takeoff and landing takes about 80 s.
/// Sign-ups from one IP address: a school class on one Wi-Fi.
pub const SIGNUPS_PER_IP: (u32, u32) = (40, 40);
/// Sign-ups from everywhere together: a second line if addresses rotate.
pub const SIGNUPS_ALL: (u32, u32) = (300, 300);
/// New runs (seeds) per pilot: a few page loads at once, one a minute.
pub const RUNS_PER_PILOT: (u32, u32) = (15, 60);
/// Replays per pilot: one per "Level complete", with retries.
pub const REPLAYS_PER_PILOT: (u32, u32) = (10, 60);

const HOUR_MS: i64 = 60 * 60 * 1000;
/// A pilot with no best time, not seen for this long, is deleted.
const PILOT_IDLE_MS: i64 = 90 * 24 * 60 * 60 * 1000;
/// `players.last_seen_ms` is written at most this often per pilot.
const SEEN_EVERY_MS: i64 = HOUR_MS;
/// "The database is full" goes into the log at most this often.
const FULL_LOG_EVERY_MS: i64 = HOUR_MS;

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS players (
    id           TEXT PRIMARY KEY,
    token_hash   BLOB NOT NULL UNIQUE,
    name         TEXT NOT NULL,
    created_ms   INTEGER NOT NULL,
    last_seen_ms INTEGER NOT NULL DEFAULT 0
);
-- One flight: its seed, and how many of its landed levels are counted.
CREATE TABLE IF NOT EXISTS runs (
    id         TEXT PRIMARY KEY,
    player_id  TEXT NOT NULL REFERENCES players(id),
    level      INTEGER NOT NULL,
    seed       INTEGER NOT NULL,
    started_ms INTEGER NOT NULL,
    counted    INTEGER NOT NULL DEFAULT 0,
    last_ms    INTEGER,
    last_steps INTEGER NOT NULL DEFAULT 0
);
-- Each pilot's fastest time per level.
CREATE TABLE IF NOT EXISTS best (
    player_id TEXT NOT NULL REFERENCES players(id),
    level     INTEGER NOT NULL,
    millis    INTEGER NOT NULL,
    at_ms     INTEGER NOT NULL,
    PRIMARY KEY (player_id, level)
);
CREATE INDEX IF NOT EXISTS best_by_level ON best (level, millis, at_ms);
-- Counts that outlive the rows behind them (the cleanup deletes runs).
-- One row; `migrate` writes it.
CREATE TABLE IF NOT EXISTS stats (
    id      INTEGER PRIMARY KEY CHECK (id = 1),
    flights INTEGER NOT NULL
);
";

/// What the server is told by its environment (`main.rs`).
pub struct Settings {
    /// The header with the client's IP address, set by a proxy in front
    /// (`Fly-Client-IP` on Fly.io). `None`: the socket's address. Only
    /// trusted when set, so nobody can fake it where no proxy sets it.
    pub trusted_ip_header: Option<String>,
    /// Over this size the database takes no new pilots (503); the ones
    /// there keep playing.
    pub db_max_bytes: u64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            trusted_ip_header: None,
            db_max_bytes: 512 * 1024 * 1024, // half the 1 GB volume
        }
    }
}

struct Limits {
    signups_per_ip: Limiter,
    signups_all: Limiter,
    runs: Limiter,
    replays: Limiter,
}

/// Counted since the last hourly log line (`App::cleanup`).
#[derive(Default)]
struct Counts {
    signups: AtomicU64,
    runs: AtomicU64,
    replays_accepted: AtomicU64,
    replays_refused: AtomicU64,
    limited_signups: AtomicU64,
    limited_runs: AtomicU64,
    limited_replays: AtomicU64,
    full: AtomicU64,
}

pub struct App {
    pub db: Mutex<Connection>,
    settings: Settings,
    /// Replays flown again at the same time: as many as there are CPUs.
    /// Each is real work; more at once would only slow them all.
    flying: tokio::sync::Semaphore,
    /// Replays flown again so far (for tests and logs).
    pub replays_flown: AtomicU64,
    limits: Limits,
    counts: Counts,
    /// Added to the real time: tests let time pass (`time_passes`).
    offset_ms: AtomicI64,
    /// When "the database is full" was logged last.
    full_logged_ms: AtomicI64,
}

type Shared = Arc<App>;

impl App {
    pub fn open(path: &str, settings: Settings) -> rusqlite::Result<Shared> {
        Self::with(Connection::open(path)?, settings)
    }

    pub fn in_memory() -> rusqlite::Result<Shared> {
        Self::in_memory_with(Settings::default())
    }

    pub fn in_memory_with(settings: Settings) -> rusqlite::Result<Shared> {
        Self::with(Connection::open_in_memory()?, settings)
    }

    fn with(db: Connection, settings: Settings) -> rusqlite::Result<Shared> {
        db.execute_batch(SCHEMA)?;
        migrate(&db)?;
        let cpus = std::thread::available_parallelism().map_or(1, |n| n.get());
        let limiter = |(burst, per_hour)| Limiter::new(burst, per_hour);
        Ok(Arc::new(App {
            db: Mutex::new(db),
            settings,
            flying: tokio::sync::Semaphore::new(cpus),
            replays_flown: Default::default(),
            limits: Limits {
                signups_per_ip: limiter(SIGNUPS_PER_IP),
                signups_all: limiter(SIGNUPS_ALL),
                runs: limiter(RUNS_PER_PILOT),
                replays: limiter(REPLAYS_PER_PILOT),
            },
            counts: Counts::default(),
            offset_ms: AtomicI64::new(0),
            full_logged_ms: AtomicI64::new(i64::MIN),
        }))
    }

    /// The time now, in ms since 1970.
    fn now(&self) -> i64 {
        now_ms() + self.offset_ms.load(Relaxed)
    }

    /// For tests: as if `ms` more had passed (runs get older, limits
    /// refill, pilots go idle).
    pub fn time_passes(&self, ms: i64) {
        self.offset_ms.fetch_add(ms, Relaxed);
    }

    /// Housekeeping, at start and then every hour (`main.rs`): delete
    /// what can never be used again, forget idle limits, and return the
    /// hourly log line (counts since the last one, no IP addresses, no
    /// tokens).
    pub fn cleanup(&self) -> rusqlite::Result<String> {
        let now = self.now();
        let (runs, pilots) = {
            let mut db = self.db();
            let tx = db.transaction()?;
            // A run that counted nothing can never count once too old;
            // runs that counted stay, the history behind `best`.
            // An hour more than a seed is good for: never one that is
            // being flown right now.
            let runs = tx.execute(
                "DELETE FROM runs WHERE counted = 0 AND started_ms < ?1",
                [now - RUN_MAX_AGE_MS - HOUR_MS],
            )?;
            // Pilots not on any board and gone for long: their browser
            // signs up again (a new name) if it ever comes back.
            let idle = "SELECT id FROM players WHERE last_seen_ms < ?1
                        AND id NOT IN (SELECT player_id FROM best)";
            tx.execute(
                &format!("DELETE FROM runs WHERE player_id IN ({idle})"),
                [now - PILOT_IDLE_MS],
            )?;
            let pilots = tx.execute(
                &format!("DELETE FROM players WHERE id IN ({idle})"),
                [now - PILOT_IDLE_MS],
            )?;
            tx.commit()?;
            db.execute_batch("PRAGMA optimize")?;
            (runs, pilots)
        };
        let l = &self.limits;
        for limiter in [&l.signups_per_ip, &l.signups_all, &l.runs, &l.replays] {
            limiter.forget_idle(now);
        }
        let c = &self.counts;
        let n = |a: &AtomicU64| a.swap(0, Relaxed);
        Ok(format!(
            "hourly: signups {} runs {} replays accepted {} refused {} | \
             429 signups {} runs {} replays {} | full {} | \
             deleted runs {runs} pilots {pilots} | limit keys {}",
            n(&c.signups),
            n(&c.runs),
            n(&c.replays_accepted),
            n(&c.replays_refused),
            n(&c.limited_signups),
            n(&c.limited_runs),
            n(&c.limited_replays),
            n(&c.full),
            l.signups_per_ip.keys() + l.runs.keys() + l.replays.keys(),
        ))
    }

    /// How big the database is now, in bytes.
    fn db_bytes(&self) -> rusqlite::Result<u64> {
        self.db().query_row(
            "SELECT page_count * page_size FROM pragma_page_count, pragma_page_size",
            [],
            |r| r.get(0),
        )
    }

    fn db(&self) -> std::sync::MutexGuard<'_, Connection> {
        // A panic while holding the lock leaves the connection usable.
        self.db.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// Columns added after a database was first made (`CREATE TABLE IF NOT
/// EXISTS` leaves existing tables as they are).
fn migrate(db: &Connection) -> rusqlite::Result<()> {
    let has_last_steps: bool = db.query_row(
        "SELECT COUNT(*) > 0 FROM pragma_table_info('runs') WHERE name = 'last_steps'",
        [],
        |r| r.get(0),
    )?;
    if !has_last_steps {
        db.execute_batch("ALTER TABLE runs ADD COLUMN last_steps INTEGER NOT NULL DEFAULT 0")?;
    }
    let has_last_seen: bool = db.query_row(
        "SELECT COUNT(*) > 0 FROM pragma_table_info('players') WHERE name = 'last_seen_ms'",
        [],
        |r| r.get(0),
    )?;
    if !has_last_seen {
        // Pilots from before: seen when they signed up, so the cleanup
        // gives them their full 90 days.
        db.execute_batch(
            "ALTER TABLE players ADD COLUMN last_seen_ms INTEGER NOT NULL DEFAULT 0;
             UPDATE players SET last_seen_ms = created_ms;",
        )?;
    }
    // The flight count starts from the landed levels the runs still
    // know (a new database: 0); from then on each one adds itself.
    db.execute_batch(
        "INSERT OR IGNORE INTO stats (id, flights)
         SELECT 1, COALESCE(SUM(counted), 0) FROM runs",
    )?;
    Ok(())
}

/// The API, callable from the given pages (origins).
pub fn app(state: Shared, origins: &[&str]) -> Router {
    let origins: Vec<HeaderValue> = origins.iter().filter_map(|o| o.parse().ok()).collect();
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods([Method::GET, Method::POST, Method::PUT])
        .allow_headers([AUTHORIZATION, CONTENT_TYPE])
        // The page on another origin may read it only when exposed: it
        // waits that long before sending a replay once more.
        .expose_headers([axum::http::header::RETRY_AFTER]);
    Router::new()
        .route("/v1/players", post(new_player))
        .route("/v1/players/me", put(rename))
        .route("/v1/runs", post(start_run))
        .route("/v1/runs/{id}/replay", post(send_replay))
        .route("/v1/leaderboard", get(leaderboard))
        .route("/health", get(|| async { "ok" }))
        .layer(DefaultBodyLimit::max(MAX_REPLAY_BYTES))
        .layer(cors)
        .with_state(state)
}

// ---- Errors -------------------------------------------------------------

struct ApiError(StatusCode, String, Option<u64>);

impl ApiError {
    fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self(status, message.into(), None)
    }

    /// Over a rate limit: try again in `seconds` (`Retry-After`).
    fn slow_down(seconds: u64) -> Self {
        Self(
            StatusCode::TOO_MANY_REQUESTS,
            "slow down".into(),
            Some(seconds),
        )
    }

    /// Busy now: try again in `seconds` (the `Retry-After` header).
    fn busy(seconds: u64) -> Self {
        Self(
            StatusCode::SERVICE_UNAVAILABLE,
            "busy, try again".into(),
            Some(seconds),
        )
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        #[derive(Serialize)]
        struct Body {
            error: String,
        }
        let mut response = (self.0, Json(Body { error: self.1 })).into_response();
        if let Some(seconds) = self.2 {
            response
                .headers_mut()
                .insert(axum::http::header::RETRY_AFTER, seconds.into());
        }
        response
    }
}

impl From<rusqlite::Error> for ApiError {
    fn from(e: rusqlite::Error) -> Self {
        eprintln!("database: {e}");
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "database error")
    }
}

type Api<T> = Result<Json<T>, ApiError>;

// ---- Who is asking ------------------------------------------------------

/// The pilot a request's token belongs to.
struct Pilot {
    id: String,
}

fn bearer(parts: &Parts) -> Option<&str> {
    let value = parts.headers.get(AUTHORIZATION)?.to_str().ok()?;
    value.strip_prefix("Bearer ")
}

fn pilot_of(state: &App, token: &str) -> Result<Option<Pilot>, rusqlite::Error> {
    let hash = Sha256::digest(token.as_bytes());
    let now = state.now();
    let db = state.db();
    let found: Option<(String, i64)> = db
        .query_row(
            "SELECT id, last_seen_ms FROM players WHERE token_hash = ?1",
            [&hash[..]],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let Some((id, seen)) = found else {
        return Ok(None);
    };
    // Seen now (for the cleanup); written at most once an hour.
    if seen < now - SEEN_EVERY_MS {
        db.execute(
            "UPDATE players SET last_seen_ms = ?1 WHERE id = ?2",
            params![now, id],
        )?;
    }
    Ok(Some(Pilot { id }))
}

/// The client's IP address, for the sign-up limit only: from the
/// trusted proxy header if one is set (`Settings::trusted_ip_header`),
/// else from the socket. Never stored or logged.
struct ClientIp(String);

impl FromRequestParts<Shared> for ClientIp {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &Shared) -> Result<Self, ApiError> {
        let from_header = state
            .settings
            .trusted_ip_header
            .as_deref()
            .and_then(|name| {
                let value = parts.headers.get(name)?.to_str().ok()?;
                Some(value.split(',').next()?.trim().to_string())
            });
        let from_socket = || {
            parts
                .extensions
                .get::<ConnectInfo<SocketAddr>>()
                .map(|c| c.0.ip().to_string())
        };
        Ok(Self(
            from_header
                .or_else(from_socket)
                .unwrap_or_else(|| "unknown".into()),
        ))
    }
}

impl FromRequestParts<Shared> for Pilot {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &Shared) -> Result<Self, ApiError> {
        let unknown = || ApiError::new(StatusCode::UNAUTHORIZED, "unknown pilot");
        let token = bearer(parts).ok_or_else(unknown)?;
        pilot_of(state, token)?.ok_or_else(unknown)
    }
}

/// A token may come along or not (the board is public).
struct MaybePilot(Option<Pilot>);

impl FromRequestParts<Shared> for MaybePilot {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &Shared) -> Result<Self, ApiError> {
        Ok(Self(match bearer(parts) {
            Some(token) => pilot_of(state, token)?,
            None => None,
        }))
    }
}

// ---- Small helpers ------------------------------------------------------

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// `n` random bytes as hex. Tokens and ids must not be guessable.
fn random_hex(n: usize) -> Result<String, ApiError> {
    let mut bytes = vec![0_u8; n];
    getrandom::fill(&mut bytes)
        .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "no randomness"))?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

/// A name for the board: 1 to 16 letters, digits, spaces, `-` or `_`.
fn clean_name(name: &str) -> Option<String> {
    let name = name.split_whitespace().collect::<Vec<_>>().join(" ");
    let ok = |c: char| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_');
    let len = name.chars().count();
    ((1..=NAME_MAX_CHARS).contains(&len) && name.chars().all(ok)).then_some(name)
}

// ---- Pilots -------------------------------------------------------------

#[derive(Serialize)]
struct NewPlayer {
    player_id: String,
    token: String,
    name: String,
}

async fn new_player(
    State(state): State<Shared>,
    ClientIp(ip): ClientIp,
) -> Result<(StatusCode, Json<NewPlayer>), ApiError> {
    let now = state.now();
    let limited = |seconds| {
        state.counts.limited_signups.fetch_add(1, Relaxed);
        ApiError::slow_down(seconds)
    };
    // A sign-up refused further on gives its address's token back, so
    // retries while the server is full or busy do not use it up.
    let (per_ip, all) = (&state.limits.signups_per_ip, &state.limits.signups_all);
    per_ip.take(&ip, now).map_err(limited)?;
    if let Err(seconds) = all.take("all", now) {
        per_ip.give_back(&ip);
        return Err(limited(seconds));
    }
    if state.db_bytes()? > state.settings.db_max_bytes {
        per_ip.give_back(&ip);
        all.give_back("all");
        state.counts.full.fetch_add(1, Relaxed);
        let last = state.full_logged_ms.load(Relaxed);
        if now.saturating_sub(last) >= FULL_LOG_EVERY_MS {
            state.full_logged_ms.store(now, Relaxed);
            eprintln!(
                "database over {} bytes: no new pilots",
                state.settings.db_max_bytes
            );
        }
        return Err(ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "the leaderboard is full",
        ));
    }
    let player_id = random_hex(8)?;
    let token = random_hex(32)?;
    let name = format!("Pilot-{}", &player_id[..4]);
    let hash = Sha256::digest(token.as_bytes());
    state.db().execute(
        "INSERT INTO players (id, token_hash, name, created_ms, last_seen_ms)
         VALUES (?1, ?2, ?3, ?4, ?4)",
        params![player_id, &hash[..], name, now],
    )?;
    state.counts.signups.fetch_add(1, Relaxed);
    Ok((
        StatusCode::CREATED,
        Json(NewPlayer {
            player_id,
            token,
            name,
        }),
    ))
}

#[derive(Deserialize, Serialize)]
struct Name {
    name: String,
}

async fn rename(State(state): State<Shared>, pilot: Pilot, Json(body): Json<Name>) -> Api<Name> {
    let name = clean_name(&body.name).ok_or_else(|| {
        ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "a name has 1 to 16 letters, digits, spaces, - or _",
        )
    })?;
    state.db().execute(
        "UPDATE players SET name = ?1 WHERE id = ?2",
        params![name, pilot.id],
    )?;
    Ok(Json(Name { name }))
}

// ---- Runs ---------------------------------------------------------------

#[derive(Deserialize)]
struct StartRun {
    level: i32,
}

#[derive(Serialize)]
struct Run {
    run_id: String,
    /// As text: JSON numbers cannot hold every 64-bit seed.
    seed: String,
    level: u32,
}

async fn start_run(
    State(state): State<Shared>,
    pilot: Pilot,
    Json(body): Json<StartRun>,
) -> Api<Run> {
    state
        .limits
        .runs
        .take(&pilot.id, state.now())
        .map_err(|seconds| {
            state.counts.limited_runs.fetch_add(1, Relaxed);
            ApiError::slow_down(seconds)
        })?;
    let level = start_level(body.level);
    let run_id = random_hex(16)?;
    let mut seed = [0_u8; 8];
    getrandom::fill(&mut seed)
        .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "no randomness"))?;
    let seed = u64::from_le_bytes(seed);
    state.db().execute(
        "INSERT INTO runs (id, player_id, level, seed, started_ms) VALUES (?1, ?2, ?3, ?4, ?5)",
        // SQLite keeps signed numbers: the seed's bits, as they are.
        params![run_id, pilot.id, level, seed as i64, state.now()],
    )?;
    state.counts.runs.fetch_add(1, Relaxed);
    Ok(Json(Run {
        run_id,
        seed: seed.to_string(),
        level,
    }))
}

/// A level landed on this run, now on its board.
#[derive(Serialize)]
struct Counted {
    level: u32,
    millis: u64,
    /// This is the pilot's fastest time in this level so far.
    new_best: bool,
    best_millis: i64,
    /// The pilot's place in this level, 1 = the fastest.
    rank: i64,
    pilots: i64,
}

#[derive(Serialize)]
struct Replayed {
    /// The levels this replay landed that were not counted before, in
    /// the order they were flown. Empty: nothing new.
    levels: Vec<Counted>,
    /// The flight has ended (last life gone).
    over: bool,
}

/// What the game showed at "Level complete": the level and its time,
/// from `?level=5&shown=58533`. Only for the log line.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Shown {
    level: u32,
    millis: u64,
}

/// Read by hand, not with `Query`: a missing, doubled or wrong value
/// only means no `Shown`, never a refused replay.
fn shown_of(query: Option<&str>) -> Option<Shown> {
    fn value<T: std::str::FromStr>(query: &str, key: &str) -> Option<T> {
        query
            .split('&')
            .find_map(|pair| pair.strip_prefix(key)?.strip_prefix('='))?
            .parse()
            .ok()
    }
    let query = query?;
    Some(Shown {
        level: value(query, "level")?,
        millis: value(query, "shown")?,
    })
}

/// The run's start level and the pilot's name, as far as they were
/// found: for the log line, also when the replay is refused.
#[derive(Default)]
struct Who {
    start: Option<u32>,
    name: Option<String>,
}

async fn send_replay(
    State(state): State<Shared>,
    pilot: Pilot,
    Path(run_id): Path<String>,
    RawQuery(query): RawQuery,
    body: Bytes,
) -> Api<Replayed> {
    let mut who = Who::default();
    let result = match state.limits.replays.take(&pilot.id, state.now()) {
        Ok(()) => fly_replay(&state, &pilot, &run_id, body, &mut who).await,
        Err(seconds) => {
            state.counts.limited_replays.fetch_add(1, Relaxed);
            Err(ApiError::slow_down(seconds))
        }
    };
    let count = match &result {
        Ok(_) => Some(&state.counts.replays_accepted),
        // A 429 is counted as such, not as a refused replay.
        Err(e) if e.0 == StatusCode::TOO_MANY_REQUESTS => None,
        Err(_) => Some(&state.counts.replays_refused),
    };
    if let Some(count) = count {
        count.fetch_add(1, Relaxed);
    }
    let outcome = match &result {
        Ok(Json(r)) => Ok(&r.levels[..]),
        Err(e) => Err(e.1.as_str()),
    };
    let shown = shown_of(query.as_deref());
    println!("{}", replay_line(&run_id, &who, shown, outcome));
    result
}

/// One line in the server's log per replay (`fly logs`), so a time a
/// pilot reports can be checked: who, which run and start level, the
/// level and time the game showed against what was flown here, or why
/// it was refused. A difference ends the line in `DIFFERS: ...`: the
/// game and the server should always fly the same.
fn replay_line(
    run_id: &str,
    who: &Who,
    shown: Option<Shown>,
    outcome: Result<&[Counted], &str>,
) -> String {
    // From the URL: only its first letters and digits, so nobody can
    // write lines of their own into the log.
    let run: String = run_id
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .take(8)
        .collect();
    let name = who.name.as_deref().unwrap_or("?");
    let start = who.start.map_or("?".into(), |l| l.to_string());
    let shown_text = shown.map_or("?".into(), |s| format!("L{} {}", s.level, s.millis));
    let head = format!("replay run={run} pilot={name:?} start={start} shown={shown_text}");
    let levels = match outcome {
        Err(why) => return format!("{head} refused: {why}"),
        Ok(levels) => levels,
    };
    let counted: Vec<String> = levels
        .iter()
        .map(|c| {
            let best = if c.new_best { " best" } else { "" };
            format!(
                "L{} {} ms (rank {} of {}{best})",
                c.level, c.millis, c.rank, c.pilots
            )
        })
        .collect();
    let counted = if counted.is_empty() {
        "counted nothing".to_string()
    } else {
        format!("counted {}", counted.join(", "))
    };
    // The level the game showed must be among those counted, with the
    // same time.
    let differs = shown.and_then(|s| match levels.iter().find(|c| c.level == s.level) {
        Some(c) if c.millis == s.millis => None,
        Some(c) => Some(format!(
            " DIFFERS: L{} shown {}, flown here {}",
            s.level, s.millis, c.millis
        )),
        None => Some(format!(
            " DIFFERS: L{} shown {}, not counted here",
            s.level, s.millis
        )),
    });
    format!("{head} {counted}{}", differs.unwrap_or_default())
}

async fn fly_replay(
    state: &Shared,
    pilot: &Pilot,
    run_id: &str,
    body: Bytes,
    who: &mut Who,
) -> Api<Replayed> {
    let refuse = |why: &str| ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, why);
    let now = state.now();
    let run: Option<(u32, i64, i64, usize, u64, String)> = state
        .db()
        .query_row(
            "SELECT r.level, r.seed, r.started_ms, r.counted, r.last_steps, p.name
             FROM runs r JOIN players p ON p.id = r.player_id
             WHERE r.id = ?1 AND r.player_id = ?2",
            params![run_id, pilot.id],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                ))
            },
        )
        .optional()?;
    let (level, seed, started_ms, counted, last_steps, name) =
        run.ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "no such run"))?;
    *who = Who {
        start: Some(level),
        name: Some(name),
    };
    if now - started_ms > RUN_MAX_AGE_MS {
        return Err(refuse("this run is too old"));
    }

    // The replay must be of the flight this server asked for...
    let start = start_of(&body).map_err(|e| refuse(&e.to_string()))?;
    if start.seed != seed as u64 || start.level != level {
        return Err(refuse("the replay is of another flight"));
    }
    // A run only moves forward: each replay is the flight so far, so it
    // is longer than the last one. Checked without flying it, so sending
    // the same replay again and again costs next to nothing.
    let steps = steps_of(&body).map_err(|e| refuse(&e.to_string()))?;
    if steps <= last_steps {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "this replay is not longer than the last one",
        ));
    }
    // ...and is flown again here, one per CPU at a time, off the async
    // threads: it is real work.
    let _turn = tokio::time::timeout(REPLAY_WAIT, state.flying.acquire())
        .await
        .map_err(|_| ApiError::busy(REPLAY_WAIT.as_secs()))?
        .map_err(|_| ApiError::busy(REPLAY_WAIT.as_secs()))?;
    state
        .replays_flown
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let outcome = tokio::task::spawn_blocking(move || replay(&body))
        .await
        .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "the replay crashed"))?
        .map_err(|e| refuse(&e.to_string()))?;
    // The game never runs faster than real time, so a flight cannot be
    // longer than the time since its seed was handed out.
    let took_ms = (outcome.steps as f64 * f64::from(TICK_SECONDS) * 1000.0) as i64;
    if took_ms > now - started_ms + CLOCK_SLACK_MS {
        return Err(refuse("the flight is longer than the time it had"));
    }

    let new = outcome.completed.get(counted..).unwrap_or_default();
    let mut levels = Vec::new();
    let mut db = state.db();
    let tx = db.transaction()?;
    let updated = tx.execute(
        "UPDATE runs SET counted = ?1, last_ms = ?2, last_steps = ?3
         WHERE id = ?4 AND counted = ?5 AND last_steps = ?6",
        params![
            counted + new.len(),
            now,
            outcome.steps,
            run_id,
            counted,
            last_steps
        ],
    )?;
    if updated == 0 {
        // Sent twice at the same moment: the other one counted.
        return Err(ApiError::new(StatusCode::CONFLICT, "already counted"));
    }
    if !new.is_empty() {
        tx.execute(
            "UPDATE stats SET flights = flights + ?1 WHERE id = 1",
            [new.len()],
        )?;
    }
    for done in new {
        let millis = done.millis as i64;
        let before: Option<i64> = tx
            .query_row(
                "SELECT millis FROM best WHERE player_id = ?1 AND level = ?2",
                params![pilot.id, done.level],
                |r| r.get(0),
            )
            .optional()?;
        let new_best = millis > 0 && before.is_none_or(|b| millis < b);
        if new_best {
            tx.execute(
                "INSERT INTO best (player_id, level, millis, at_ms) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT (player_id, level) DO UPDATE SET millis = ?3, at_ms = ?4",
                params![pilot.id, done.level, millis, now],
            )?;
        }
        let best_millis = if new_best {
            millis
        } else {
            before.unwrap_or(millis)
        };
        let (rank, pilots) = standing(&tx, done.level, best_millis)?;
        levels.push(Counted {
            level: done.level,
            millis: done.millis,
            new_best,
            best_millis,
            rank,
            pilots,
        });
    }
    tx.commit()?;
    Ok(Json(Replayed {
        levels,
        over: outcome.over,
    }))
}

/// The place of a best time of `millis` in a level (1 = the fastest;
/// equal times share a place) and how many pilots are on that board.
fn standing(db: &Connection, level: u32, millis: i64) -> rusqlite::Result<(i64, i64)> {
    db.query_row(
        "SELECT COUNT(*) FILTER (WHERE millis < ?2) + 1, COUNT(*) FROM best WHERE level = ?1",
        params![level, millis],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
}

// ---- The board ----------------------------------------------------------

#[derive(Deserialize)]
struct BoardQuery {
    level: Option<i32>,
    limit: Option<u32>,
}

#[derive(Serialize)]
struct Entry {
    rank: i64,
    name: String,
    millis: i64,
    /// This is the pilot who asked.
    me: bool,
}

#[derive(Serialize)]
struct Me {
    name: String,
    /// `None`: no time on this board yet.
    rank: Option<i64>,
    millis: Option<i64>,
}

#[derive(Serialize)]
struct Board {
    level: u32,
    pilots: i64,
    /// Levels landed by all pilots together, in every level, ever.
    flights: i64,
    top: Vec<Entry>,
    me: Option<Me>,
}

async fn leaderboard(
    State(state): State<Shared>,
    MaybePilot(pilot): MaybePilot,
    Query(query): Query<BoardQuery>,
) -> Api<Board> {
    let level = start_level(query.level.unwrap_or(1));
    let limit = query.limit.unwrap_or(10).clamp(1, BOARD_MAX);
    let db = state.db();
    let my_id = pilot.as_ref().map(|p| p.id.as_str());
    let mut top = Vec::new();
    {
        let mut rows = db.prepare(
            "SELECT RANK() OVER (ORDER BY b.millis), p.name, b.millis, p.id
             FROM best b JOIN players p ON p.id = b.player_id
             WHERE b.level = ?1 ORDER BY b.millis, b.at_ms LIMIT ?2",
        )?;
        let rows = rows.query_map(params![level, limit], |r| {
            let id: String = r.get(3)?;
            Ok(Entry {
                rank: r.get(0)?,
                name: r.get(1)?,
                millis: r.get(2)?,
                me: Some(id.as_str()) == my_id,
            })
        })?;
        for row in rows {
            top.push(row?);
        }
    }
    let me = match &pilot {
        None => None,
        Some(p) => {
            let name: String =
                db.query_row("SELECT name FROM players WHERE id = ?1", [&p.id], |r| {
                    r.get(0)
                })?;
            let millis: Option<i64> = db
                .query_row(
                    "SELECT millis FROM best WHERE player_id = ?1 AND level = ?2",
                    params![p.id, level],
                    |r| r.get(0),
                )
                .optional()?;
            let rank = match millis {
                Some(m) => Some(standing(&db, level, m)?.0),
                None => None,
            };
            Some(Me { name, rank, millis })
        }
    };
    let pilots: i64 = db.query_row("SELECT COUNT(*) FROM best WHERE level = ?1", [level], |r| {
        r.get(0)
    })?;
    let flights: i64 = db.query_row("SELECT flights FROM stats WHERE id = 1", [], |r| r.get(0))?;
    Ok(Json(Board {
        level,
        pilots,
        flights,
        top,
        me,
    }))
}

#[cfg(test)]
mod tests {
    fn counted(level: u32, millis: u64) -> Counted {
        Counted {
            level,
            millis,
            new_best: true,
            best_millis: millis as i64,
            rank: 2,
            pilots: 9,
        }
    }

    fn who(start: u32, name: &str) -> Who {
        Who {
            start: Some(start),
            name: Some(name.into()),
        }
    }

    fn shown(level: u32, millis: u64) -> Option<Shown> {
        Some(Shown { level, millis })
    }

    #[test]
    fn a_replay_line_says_what_was_shown_and_what_counted() {
        let levels = [counted(5, 58533)];
        let line = replay_line(
            "abcdef1234567890",
            &who(5, "Geoport"),
            shown(5, 58533),
            Ok(&levels),
        );
        assert_eq!(
            line,
            "replay run=abcdef12 pilot=\"Geoport\" start=5 shown=L5 58533 counted L5 58533 ms (rank 2 of 9 best)"
        );
        let line = replay_line("abc", &Who::default(), None, Ok(&levels));
        assert!(
            line.contains("pilot=\"?\" start=? shown=?") && !line.contains("DIFFERS"),
            "{line}"
        );
    }

    #[test]
    fn a_replay_line_marks_a_time_the_server_did_not_fly() {
        // Another time for the level the game showed.
        let levels = [counted(4, 60000), counted(5, 63067)];
        let line = replay_line("r", &who(4, "A"), shown(5, 56000), Ok(&levels));
        assert!(
            line.ends_with(" DIFFERS: L5 shown 56000, flown here 63067"),
            "{line}"
        );
        // The right time, though not of the last level counted.
        let line = replay_line("r", &who(4, "A"), shown(4, 60000), Ok(&levels));
        assert!(!line.contains("DIFFERS"), "{line}");
        // The level the game showed did not land here.
        let line = replay_line("r", &who(4, "A"), shown(5, 56000), Ok(&[]));
        assert!(
            line.ends_with("counted nothing DIFFERS: L5 shown 56000, not counted here"),
            "{line}"
        );
    }

    #[test]
    fn a_replay_line_says_why_it_was_refused_and_cannot_be_split() {
        assert_eq!(
            replay_line(
                "r1",
                &who(1, "A"),
                shown(1, 1),
                Err("the replay is of another flight")
            ),
            "replay run=r1 pilot=\"A\" start=1 shown=L1 1 refused: the replay is of another flight"
        );
        let line = replay_line("x\nreplay fake", &who(1, "a\nb"), None, Ok(&[]));
        assert!(!line.contains('\n'), "{line}");
    }

    #[test]
    fn what_the_game_showed_is_read_leniently() {
        assert_eq!(shown_of(Some("level=5&shown=58533")), shown(5, 58533));
        assert_eq!(shown_of(Some("shown=58533&level=5")), shown(5, 58533));
        assert_eq!(
            shown_of(Some("level=5&shown=1&shown=2")),
            shown(5, 1),
            "the first one"
        );
        for bad in [
            None,
            Some(""),
            Some("shown=58533"),
            Some("level=x&shown=1"),
            Some("levels=5&shown=1"),
        ] {
            assert_eq!(shown_of(bad), None, "{bad:?}");
        }
    }

    use super::*;

    #[test]
    fn an_older_database_gets_the_new_columns() {
        let db = Connection::open_in_memory().expect("a database");
        db.execute_batch(
            "CREATE TABLE players (id TEXT PRIMARY KEY, token_hash BLOB NOT NULL UNIQUE,
             name TEXT NOT NULL, created_ms INTEGER NOT NULL);
             INSERT INTO players (id, token_hash, name, created_ms) VALUES ('p', x'00', 'Old', 1234);
             CREATE TABLE runs (id TEXT PRIMARY KEY, player_id TEXT NOT NULL, level INTEGER NOT NULL,
             seed INTEGER NOT NULL, started_ms INTEGER NOT NULL, counted INTEGER NOT NULL DEFAULT 0,
             last_ms INTEGER);
             INSERT INTO runs (id, player_id, level, seed, started_ms) VALUES ('r', 'p', 1, 7, 0);
             INSERT INTO runs (id, player_id, level, seed, started_ms, counted)
             VALUES ('r2', 'p', 1, 8, 0, 2), ('r3', 'p', 4, 9, 0, 1);",
        )
        .expect("the old schema");
        let app = App::with(db, Settings::default()).expect("opens");
        let db = app.db();
        let steps: i64 = db
            .query_row("SELECT last_steps FROM runs WHERE id = 'r'", [], |r| {
                r.get(0)
            })
            .expect("the column is there");
        assert_eq!(steps, 0);
        let (name, seen): (String, i64) = db
            .query_row("SELECT name, last_seen_ms FROM players", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .expect("the pilot is still there");
        assert_eq!((name.as_str(), seen), ("Old", 1234), "seen when signed up");
        let flights: i64 = db
            .query_row("SELECT flights FROM stats", [], |r| r.get(0))
            .expect("the flight count");
        assert_eq!(flights, 3, "the landed levels the runs know");
        drop(db);
        // Opened again: counted once, not twice.
        migrate(&app.db()).expect("migrates again");
        let flights: i64 = app
            .db()
            .query_row("SELECT flights FROM stats", [], |r| r.get(0))
            .expect("the flight count");
        assert_eq!(flights, 3);
    }

    #[test]
    fn names_are_tidied_or_refused() {
        assert_eq!(clean_name("  Andi   B "), Some("Andi B".into()));
        assert_eq!(clean_name("Zoë_1"), Some("Zoë_1".into()));
        assert_eq!(clean_name(""), None);
        assert_eq!(clean_name("<script>"), None);
        assert_eq!(clean_name("a name that is far too long"), None);
    }
}
