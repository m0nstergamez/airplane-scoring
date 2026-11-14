//! The API from the outside: a pilot signs up, lands a level, and is on
//! that level's board; whatever is not an honest flight is refused.

use airplane_scoring_core::replay::{demo_flight, replay, Recorder, Start};
use airplane_scoring_core::Bounds;
use airplane_scoring_server::{app, App};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;

const PHONE: Bounds = Bounds {
    width: 844.0,
    height: 390.0,
};

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

    async fn call(
        &self,
        method: &str,
        path: &str,
        token: Option<&str>,
        body: Body,
    ) -> (StatusCode, Value) {
        let mut request = Request::builder().method(method).uri(path);
        if let Some(token) = token {
            request = request.header("authorization", format!("Bearer {token}"));
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
        let bytes = response
            .into_body()
            .collect()
            .await
            .expect("a body")
            .to_bytes();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
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

    /// As if every run had started an hour ago.
    fn an_hour_passes(&self) {
        let db = self.0.db.lock().expect("the database");
        db.execute("UPDATE runs SET started_ms = started_ms - 3600000", [])
            .expect("update");
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
    // The same replay again (the page retried): nothing new.
    let (status, body) = server.send(&owner, &run_id, flight).await;
    assert_eq!(
        (status, body["levels"].clone()),
        (StatusCode::OK, json!([]))
    );
    assert_eq!(server.board(1, None).await["pilots"], 1);
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
