//! The leaderboard server. Settings come from the environment:
//! - `PORT` (8787)
//! - `DB_PATH` (`leaderboard.sqlite`)
//! - `ALLOWED_ORIGINS`: the pages that may call it, comma separated
//!   (the published game and the local dev server).

use airplane_scoring_server::{app, App};

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

    let state = App::open(&db_path).expect("open the database");
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .expect("bind the port");
    println!("leaderboard on http://localhost:{port} (db: {db_path}, pages: {origins:?})");
    axum::serve(listener, app(state, &origins))
        .await
        .expect("serve");
}
