//! The leaderboard server. Settings come from the environment:
//! - `PORT` (8787)
//! - `DB_PATH` (`leaderboard.sqlite`)
//! - `ALLOWED_ORIGINS`: the pages that may call it, comma separated
//!   (the published game and the local dev server).
//! - `TRUSTED_IP_HEADER`: the header a proxy in front puts the client's
//!   address in (`Fly-Client-IP` on Fly.io), for the sign-up limit.
//!   Unset: the socket's address.
//! - `DB_MAX_BYTES`: no new pilots over this size (512 MB).

use std::net::SocketAddr;
use std::time::Duration;

use airplane_scoring_server::{app, App, Settings};

/// Housekeeping and the log line with the counts (`App::cleanup`).
const CLEANUP_EVERY: Duration = Duration::from_secs(60 * 60);

#[tokio::main]
async fn main() {
    let var = |name: &str, default: &str| std::env::var(name).unwrap_or_else(|_| default.into());
    let port = var("PORT", "8787");
    let db_path = var("DB_PATH", "leaderboard.sqlite");
    let origins = var(
        "ALLOWED_ORIGINS",
        "https://m0nstergamez.github.io,http://localhost:8080",
    );
    let origins: Vec<&str> = origins.split(',').map(str::trim).collect();
    let defaults = Settings::default();
    let settings = Settings {
        trusted_ip_header: std::env::var("TRUSTED_IP_HEADER")
            .ok()
            .filter(|h| !h.trim().is_empty()),
        db_max_bytes: std::env::var("DB_MAX_BYTES").map_or(defaults.db_max_bytes, |bytes| {
            bytes.parse().expect("DB_MAX_BYTES is a number of bytes")
        }),
    };
    let ip_from = settings
        .trusted_ip_header
        .clone()
        .unwrap_or_else(|| "the socket".into());

    let state = App::open(&db_path, settings).expect("open the database");
    // At start too: Fly stops the machine when nobody plays, so it may
    // seldom run a whole hour.
    let housekeeping = state.clone();
    tokio::spawn(async move {
        let mut every = tokio::time::interval(CLEANUP_EVERY);
        loop {
            every.tick().await;
            match housekeeping.cleanup() {
                Ok(line) => println!("{line}"),
                Err(e) => eprintln!("cleanup: {e}"),
            }
        }
    });
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .expect("bind the port");
    println!(
        "leaderboard on http://localhost:{port} (db: {db_path}, pages: {origins:?}, client address from {ip_from})"
    );
    axum::serve(
        listener,
        app(state, &origins).into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .expect("serve");
}
