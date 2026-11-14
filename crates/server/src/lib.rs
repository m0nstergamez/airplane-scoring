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
//! - `POST /v1/runs/{id}/replay`: the replay bytes so far, sent at every
//!   "Level complete". Returns the levels it landed that were not
//!   counted for this run before, each with the pilot's rank.
//! - `GET /v1/leaderboard?level=1&limit=10`: the fastest pilots of a
//!   level, and where the asking pilot stands (if a token came along).
//! - `GET /health`: "ok", without touching the database (for Fly's
//!   health check, which runs all the time).
//!
//! All but the first and last need `Authorization: Bearer <token>`.

use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use airplane_scoring_core::replay::{replay, start_of, steps_of};
use airplane_scoring_core::{start_level, TICK_SECONDS};
use axum::extract::{DefaultBodyLimit, FromRequestParts, Path, Query, State};
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

/// A replay may be this big: about two hours of the busiest steering.
const MAX_REPLAY_BYTES: usize = 2 * 1024 * 1024;
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

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS players (
    id         TEXT PRIMARY KEY,
    token_hash BLOB NOT NULL UNIQUE,
    name       TEXT NOT NULL,
    created_ms INTEGER NOT NULL
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
";

pub struct App {
    pub db: Mutex<Connection>,
    /// Replays flown again at the same time: as many as there are CPUs.
    /// Each is real work; more at once would only slow them all.
    flying: tokio::sync::Semaphore,
    /// Replays flown again so far (for tests and logs).
    pub replays_flown: std::sync::atomic::AtomicU64,
}

type Shared = Arc<App>;

impl App {
    pub fn open(path: &str) -> rusqlite::Result<Shared> {
        Self::with(Connection::open(path)?)
    }

    pub fn in_memory() -> rusqlite::Result<Shared> {
        Self::with(Connection::open_in_memory()?)
    }

    fn with(db: Connection) -> rusqlite::Result<Shared> {
        db.execute_batch(SCHEMA)?;
        migrate(&db)?;
        let cpus = std::thread::available_parallelism().map_or(1, |n| n.get());
        Ok(Arc::new(App {
            db: Mutex::new(db),
            flying: tokio::sync::Semaphore::new(cpus),
            replays_flown: Default::default(),
        }))
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
    Ok(())
}

/// The API, callable from the given pages (origins).
pub fn app(state: Shared, origins: &[&str]) -> Router {
    let origins: Vec<HeaderValue> = origins.iter().filter_map(|o| o.parse().ok()).collect();
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods([Method::GET, Method::POST, Method::PUT])
        .allow_headers([AUTHORIZATION, CONTENT_TYPE]);
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
    state
        .db()
        .query_row(
            "SELECT id FROM players WHERE token_hash = ?1",
            [&hash[..]],
            |row| row.get(0),
        )
        .optional()
        .map(|id| id.map(|id| Pilot { id }))
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
) -> Result<(StatusCode, Json<NewPlayer>), ApiError> {
    let player_id = random_hex(8)?;
    let token = random_hex(32)?;
    let name = format!("Pilot-{}", &player_id[..4]);
    let hash = Sha256::digest(token.as_bytes());
    state.db().execute(
        "INSERT INTO players (id, token_hash, name, created_ms) VALUES (?1, ?2, ?3, ?4)",
        params![player_id, &hash[..], name, now_ms()],
    )?;
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
    let level = start_level(body.level);
    let run_id = random_hex(16)?;
    let mut seed = [0_u8; 8];
    getrandom::fill(&mut seed)
        .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "no randomness"))?;
    let seed = u64::from_le_bytes(seed);
    state.db().execute(
        "INSERT INTO runs (id, player_id, level, seed, started_ms) VALUES (?1, ?2, ?3, ?4, ?5)",
        // SQLite keeps signed numbers: the seed's bits, as they are.
        params![run_id, pilot.id, level, seed as i64, now_ms()],
    )?;
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

async fn send_replay(
    State(state): State<Shared>,
    pilot: Pilot,
    Path(run_id): Path<String>,
    body: Bytes,
) -> Api<Replayed> {
    let refuse = |why: &str| ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, why);
    let now = now_ms();
    let run: Option<(u32, i64, i64, usize, u64)> = state
        .db()
        .query_row(
            "SELECT level, seed, started_ms, counted, last_steps FROM runs
             WHERE id = ?1 AND player_id = ?2",
            params![run_id, pilot.id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .optional()?;
    let (level, seed, started_ms, counted, last_steps) =
        run.ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "no such run"))?;
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
    Ok(Json(Board {
        level,
        pilots,
        top,
        me,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_older_database_gets_the_new_columns() {
        let db = Connection::open_in_memory().expect("a database");
        db.execute_batch(
            "CREATE TABLE runs (id TEXT PRIMARY KEY, player_id TEXT NOT NULL, level INTEGER NOT NULL,
             seed INTEGER NOT NULL, started_ms INTEGER NOT NULL, counted INTEGER NOT NULL DEFAULT 0,
             last_ms INTEGER);
             INSERT INTO runs (id, player_id, level, seed, started_ms) VALUES ('r', 'p', 1, 7, 0);",
        )
        .expect("the old schema");
        let app = App::with(db).expect("opens");
        let steps: i64 = app
            .db()
            .query_row("SELECT last_steps FROM runs WHERE id = 'r'", [], |r| {
                r.get(0)
            })
            .expect("the column is there");
        assert_eq!(steps, 0);
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
