//! The API from the outside: a pilot signs up, lands a level, and is on
//! that level's board; whatever is not an honest flight is refused.

use airplane_scoring_core::replay::{demo_flight, replay, Recorder, Start};
use airplane_scoring_core::Bounds;
use airplane_scoring_server::{
    app, App, Settings, REPLAYS_PER_PILOT, RUNS_PER_PILOT, SIGNUPS_ALL, SIGNUPS_PER_IP,
};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tower::ServiceExt;

const PHONE: Bounds = Bounds {
    width: 844.0,
    height: 390.0,
};

const HOUR: i64 = 60 * 60 * 1000;
const DAY: i64 = 24 * HOUR;

/// Long enough for the demo pilot to land level 1 (about 70 s).
const ONE_LEVEL: u64 = 6_000;

/// A flight on `start` that lands level 1, by the first demo pilot (from
/// `pilot`) that manages it. The seed comes from the server, so which
/// pilot that is differs from run to run.
fn landed(start: &Start, pilot: u64) -> (Vec<u8>, u64) {
    (pilot..pilot + 30)
        .map(|p| demo_flight(start, p, ONE_LEVEL))
        .find_map(|bytes| {
            let millis = replay(&bytes).ok()?.completed.first()?.millis;
            Some((bytes, millis))
        })
        .expect("some demo pilot lands level 1")
}

/// A flight on `start` that flies into the ground until it is over.
fn crashed(start: &Start) -> Vec<u8> {
    let mut world = start.world();
    let mut rec = Recorder::new(start);
    while !world.is_over() {
        world.tick(&rec.tick(1.0, 0.0));
    }
    rec.bytes()
}

struct Server(Arc<App>);

impl Server {
    fn new() -> Self {
        Self(App::in_memory().expect("a database"))
    }

    fn with(settings: Settings) -> Self {
        Self(App::in_memory_with(settings).expect("a database"))
    }

    async fn call(
        &self,
        method: &str,
        path: &str,
        token: Option<&str>,
        body: Body,
    ) -> (StatusCode, Value) {
        let (status, _, body) = self.call_from(None, method, path, token, body).await;
        (status, body)
    }

    /// A call from the IP address `ip` (in the `x-client-ip` header);
    /// also returns `Retry-After`, if any.
    async fn call_from(
        &self,
        ip: Option<&str>,
        method: &str,
        path: &str,
        token: Option<&str>,
        body: Body,
    ) -> (StatusCode, Option<u64>, Value) {
        let mut request = Request::builder().method(method).uri(path);
        if let Some(token) = token {
            request = request.header("authorization", format!("Bearer {token}"));
        }
        if let Some(ip) = ip {
            request = request.header("x-client-ip", ip);
        }
        let request = request
            .header("content-type", "application/json")
            .body(body)
            .expect("a request");
        let response = app(self.0.clone(), &[])
            .oneshot(request)
            .await
            .expect("a response");
        let status = response.status();
        let retry_after = response
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok()?.parse().ok());
        let bytes = response
            .into_body()
            .collect()
            .await
            .expect("a body")
            .to_bytes();
        (
            status,
            retry_after,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    /// Sign up from `ip`: the status and `Retry-After`.
    async fn sign_up_from(&self, ip: &str) -> (StatusCode, Option<u64>) {
        let (status, retry, _) = self
            .call_from(Some(ip), "POST", "/v1/players", None, Body::empty())
            .await;
        (status, retry)
    }

    async fn json(
        &self,
        method: &str,
        path: &str,
        token: Option<&str>,
        body: Value,
    ) -> (StatusCode, Value) {
        self.call(method, path, token, Body::from(body.to_string()))
            .await
    }

    /// A new pilot's token.
    async fn pilot(&self) -> String {
        let (status, body) = self.json("POST", "/v1/players", None, json!({})).await;
        assert_eq!(status, StatusCode::CREATED);
        body["token"].as_str().expect("a token").to_string()
    }

    /// Start a run: its id and the start to fly from.
    async fn run(&self, token: &str, level: u32) -> (String, Start) {
        let (status, body) = self
            .json("POST", "/v1/runs", Some(token), json!({ "level": level }))
            .await;
        assert_eq!(status, StatusCode::OK);
        let seed = body["seed"]
            .as_str()
            .expect("a seed")
            .parse()
            .expect("a number");
        let start = Start {
            seed,
            level,
            bounds: PHONE,
        };
        (body["run_id"].as_str().expect("an id").to_string(), start)
    }

    /// An hour later: runs get older, limits refill.
    fn an_hour_passes(&self) {
        self.0.time_passes(HOUR);
    }

    fn count(&self, table: &str) -> i64 {
        let db = self.0.db.lock().expect("the database");
        db.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
            .expect("a count")
    }

    async fn send(&self, token: &str, run_id: &str, replay: Vec<u8>) -> (StatusCode, Value) {
        let path = format!("/v1/runs/{run_id}/replay");
        self.call("POST", &path, Some(token), Body::from(replay))
            .await
    }

    async fn board(&self, level: u32, token: Option<&str>) -> Value {
        let path = format!("/v1/leaderboard?level={level}");
        let (status, board) = self.json("GET", &path, token, json!({})).await;
        assert_eq!(status, StatusCode::OK);
        board
    }
}

#[tokio::test]
async fn a_landed_level_is_on_its_board() {
    let server = Server::new();
    let token = server.pilot().await;
    let (status, body) = server
        .json(
            "PUT",
            "/v1/players/me",
            Some(&token),
            json!({ "name": "Andi" }),
        )
        .await;
    assert_eq!(
        (status, body["name"].as_str()),
        (StatusCode::OK, Some("Andi"))
    );

    let (run_id, start) = server.run(&token, 1).await;
    let (flight, millis) = landed(&start, 0);
    server.an_hour_passes();
    let (status, body) = server.send(&token, &run_id, flight).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let counted = &body["levels"][0];
    assert_eq!(counted["level"], 1);
    assert_eq!(counted["millis"], millis);
    assert_eq!(counted["new_best"], true);
    assert_eq!(
        (counted["rank"].as_i64(), counted["pilots"].as_i64()),
        (Some(1), Some(1))
    );

    let board = server.board(1, Some(&token)).await;
    assert_eq!(board["top"][0]["name"], "Andi");
    assert_eq!(board["top"][0]["millis"], millis);
    assert_eq!(board["top"][0]["me"], true);
    assert_eq!(board["me"]["rank"], 1);
    // Level 2 has its own board.
    assert_eq!(server.board(2, None).await["pilots"], 0);
}

#[tokio::test]
async fn the_fastest_ranks_first_and_only_a_pilots_best_counts() {
    let server = Server::new();
    let (a, b) = (server.pilot().await, server.pilot().await);
    let mut times = Vec::new();
    for (token, pilot) in [(&a, 0), (&b, 10), (&a, 20)] {
        let (run_id, start) = server.run(token, 1).await;
        let (flight, millis) = landed(&start, pilot);
        server.an_hour_passes();
        let (status, body) = server.send(token, &run_id, flight).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        times.push(millis as i64);
    }
    let (best_a, best_b) = (times[0].min(times[2]), times[1]);
    let board = server.board(1, Some(&a)).await;
    assert_eq!(board["pilots"], 2, "one line per pilot");
    assert_eq!(board["top"][0]["millis"], best_a.min(best_b));
    assert_eq!(board["me"]["millis"], best_a);
}

#[tokio::test]
async fn the_time_the_game_showed_is_only_for_the_log() {
    // `?level=&shown=` goes into the log line; a wrong one changes nothing.
    let server = Server::new();
    for query in [
        "level=1&shown=61450",
        "level=1&shown=not-a-number",
        "shown=1&shown=2",
        "",
    ] {
        let token = server.pilot().await;
        let (run_id, start) = server.run(&token, 1).await;
        let (flight, millis) = landed(&start, 0);
        server.an_hour_passes();
        let path = format!("/v1/runs/{run_id}/replay?{query}");
        let (status, body) = server
            .call("POST", &path, Some(&token), Body::from(flight))
            .await;
        assert_eq!(status, StatusCode::OK, "{query:?}: {body}");
        assert_eq!(body["levels"][0]["millis"], millis, "the server's own time");
    }
}

#[tokio::test]
async fn a_game_over_counts_no_time() {
    let server = Server::new();
    let token = server.pilot().await;
    let (run_id, start) = server.run(&token, 1).await;
    server.an_hour_passes();
    let (status, body) = server.send(&token, &run_id, crashed(&start)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["levels"], json!([]));
    assert_eq!(body["over"], true);
    assert_eq!(server.board(1, None).await["pilots"], 0);
}

#[tokio::test]
async fn a_replay_of_another_seed_is_refused() {
    let server = Server::new();
    let token = server.pilot().await;
    let (run_id, start) = server.run(&token, 1).await;
    let other = Start {
        seed: start.seed ^ 1,
        ..start
    };
    server.an_hour_passes();
    let (status, _) = server.send(&token, &run_id, landed(&other, 0).0).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let (status, _) = server.send(&token, &run_id, b"made up".to_vec()).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn a_flight_cannot_be_longer_than_the_time_it_had() {
    let server = Server::new();
    let token = server.pilot().await;
    let (run_id, start) = server.run(&token, 1).await;
    // Flown in no time at all: computed, not played.
    let (status, body) = server.send(&token, &run_id, landed(&start, 0).0).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
}

#[tokio::test]
async fn a_level_counts_once_and_only_for_its_pilot() {
    let server = Server::new();
    let (owner, thief) = (server.pilot().await, server.pilot().await);
    let (run_id, start) = server.run(&owner, 1).await;
    let (flight, _) = landed(&start, 0);
    server.an_hour_passes();
    let (status, _) = server.send(&thief, &run_id, flight.clone()).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, body) = server.send(&owner, &run_id, flight.clone()).await;
    assert_eq!(
        (status, body["levels"].as_array().map(Vec::len)),
        (StatusCode::OK, Some(1))
    );
    // The same replay again: refused before it is flown again, so
    // sending it over and over costs the server next to nothing.
    let flown = server.0.replays_flown.load(Ordering::Relaxed);
    let (status, _) = server.send(&owner, &run_id, flight).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(server.0.replays_flown.load(Ordering::Relaxed), flown);
    assert_eq!(server.board(1, None).await["pilots"], 1);
}

#[tokio::test]
async fn the_health_check_is_cheap_and_answers() {
    let server = Server::new();
    let (status, _) = server.call("GET", "/health", None, Body::empty()).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn nobody_flies_without_a_token() {
    let server = Server::new();
    let (status, _) = server
        .json("POST", "/v1/runs", None, json!({ "level": 1 }))
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = server
        .json("POST", "/v1/runs", Some("guessed"), json!({ "level": 1 }))
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = server
        .json(
            "PUT",
            "/v1/players/me",
            Some(&server.pilot().await),
            json!({ "name": "<b>" }),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

// ---- Limits and cleanup ---------------------------------------------------

fn behind_a_proxy() -> Settings {
    Settings {
        trusted_ip_header: Some("x-client-ip".into()),
        ..Settings::default()
    }
}

#[tokio::test]
async fn sign_ups_from_one_address_are_limited_others_still_work() {
    let server = Server::with(behind_a_proxy());
    for _ in 0..SIGNUPS_PER_IP.0 {
        assert_eq!(server.sign_up_from("10.0.0.1").await.0, StatusCode::CREATED);
    }
    let (status, retry) = server.sign_up_from("10.0.0.1").await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert!(retry.is_some_and(|s| (1..=90).contains(&s)), "{retry:?}");
    assert_eq!(server.sign_up_from("10.0.0.2").await.0, StatusCode::CREATED);
    server.an_hour_passes();
    assert_eq!(server.sign_up_from("10.0.0.1").await.0, StatusCode::CREATED);
}

#[tokio::test]
async fn the_address_header_counts_only_when_a_proxy_is_trusted() {
    // No proxy configured: a made-up header gives no fresh bucket.
    let server = Server::new();
    for i in 0..SIGNUPS_PER_IP.0 {
        let ip = format!("10.0.0.{i}");
        assert_eq!(server.sign_up_from(&ip).await.0, StatusCode::CREATED);
    }
    let status = server.sign_up_from("10.9.9.9").await.0;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn sign_ups_from_everywhere_together_are_limited_too() {
    let server = Server::with(behind_a_proxy());
    for i in 0..SIGNUPS_ALL.0 {
        let ip = format!("10.{}.{}.1", i / 200, i % 200);
        assert_eq!(server.sign_up_from(&ip).await.0, StatusCode::CREATED);
    }
    let status = server.sign_up_from("192.168.1.1").await.0;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn runs_and_replays_are_limited_per_pilot() {
    let server = Server::new();
    let (a, b) = (server.pilot().await, server.pilot().await);
    for _ in 0..RUNS_PER_PILOT.0 {
        server.run(&a, 1).await;
    }
    let (status, _) = server
        .json("POST", "/v1/runs", Some(&a), json!({ "level": 1 }))
        .await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    let (run_id, _) = server.run(&b, 1).await;
    // Replays: each takes a token, also one that is refused.
    for _ in 0..REPLAYS_PER_PILOT.0 {
        let (status, _) = server.send(&b, &run_id, b"made up".to_vec()).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    }
    let flown = server.0.replays_flown.load(Ordering::Relaxed);
    let (status, _) = server.send(&b, &run_id, b"made up".to_vec()).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(server.0.replays_flown.load(Ordering::Relaxed), flown);
}

#[tokio::test]
async fn the_cleanup_deletes_what_can_never_count_and_keeps_the_board() {
    let server = Server::new();
    let (on_board, idle) = (server.pilot().await, server.pilot().await);
    let (run_id, start) = server.run(&on_board, 1).await;
    server.run(&on_board, 2).await; // never flown
    server.run(&idle, 1).await; // never flown
    server.an_hour_passes();
    let (status, _) = server.send(&on_board, &run_id, landed(&start, 0).0).await;
    assert_eq!(status, StatusCode::OK);

    server.0.cleanup().expect("cleanup");
    assert_eq!(server.count("runs"), 3, "all still young enough to fly");
    server.0.time_passes(DAY - HOUR / 2); // 24 and a half hours old
    server.0.cleanup().expect("cleanup");
    assert_eq!(server.count("runs"), 3, "an hour to spare: none in flight");
    server.0.time_passes(DAY);
    let line = server.0.cleanup().expect("cleanup");
    assert_eq!(server.count("runs"), 1, "only the one that counted");
    assert!(line.contains("deleted runs 2 pilots 0"), "{line}");

    server.0.time_passes(87 * DAY); // 89 days and half an hour since seen
    server.0.cleanup().expect("cleanup");
    assert_eq!(server.count("players"), 2, "not yet 90 days");
    server.0.time_passes(DAY);
    server.0.cleanup().expect("cleanup");
    assert_eq!(server.count("players"), 1, "the idle pilot is gone");
    assert_eq!(server.board(1, Some(&on_board)).await["me"]["rank"], 1);
    // The idle pilot's browser comes back: unknown, it signs up again.
    let (status, _) = server
        .json("POST", "/v1/runs", Some(&idle), json!({ "level": 1 }))
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn a_pilot_who_plays_is_not_idle() {
    let server = Server::new();
    let token = server.pilot().await;
    server.0.time_passes(80 * DAY);
    server.board(1, Some(&token)).await; // any call with the token
    server.0.time_passes(80 * DAY);
    server.0.cleanup().expect("cleanup");
    assert_eq!(server.count("players"), 1);
}

#[tokio::test]
async fn a_full_database_takes_no_new_pilots() {
    let server = Server::with(Settings {
        db_max_bytes: 1,
        ..Settings::default()
    });
    let (status, body) = server.json("POST", "/v1/players", None, json!({})).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{body}");
    let line = server.0.cleanup().expect("cleanup");
    assert!(line.contains("full 1"), "{line}");
    // Retrying while full uses up no sign-ups of the address.
    for _ in 0..2 * SIGNUPS_PER_IP.0 {
        let (status, _) = server.json("POST", "/v1/players", None, json!({})).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    }
}

#[tokio::test]
async fn the_page_may_read_retry_after() {
    // The page is on another origin: without this it reads no
    // Retry-After and never sends a replay once more.
    let server = Server::new();
    let request = Request::builder()
        .method("POST")
        .uri("/v1/players")
        .header("origin", "https://game.test")
        .body(Body::empty())
        .expect("a request");
    let response = app(server.0.clone(), &["https://game.test"])
        .oneshot(request)
        .await
        .expect("a response");
    let exposed = response
        .headers()
        .get("access-control-expose-headers")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(
        exposed.to_ascii_lowercase().contains("retry-after"),
        "{exposed:?}"
    );
}

#[tokio::test]
async fn a_limited_replay_counts_once_in_the_hourly_line() {
    let server = Server::new();
    let token = server.pilot().await;
    let (run_id, _) = server.run(&token, 1).await;
    for _ in 0..REPLAYS_PER_PILOT.0 + 3 {
        server.send(&token, &run_id, b"made up".to_vec()).await;
    }
    let line = server.0.cleanup().expect("cleanup");
    let refused = format!("refused {}", REPLAYS_PER_PILOT.0);
    assert!(
        line.contains(&refused) && line.contains("replays 3 |"),
        "{line}"
    );
}
