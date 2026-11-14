# The leaderboard server (crates/server) as a small image, e.g. for Fly.io
# (see fly.toml). The game itself is not in here: it stays on GitHub Pages.
FROM rust:1-slim-bookworm AS build
# SQLite is compiled into the server, which needs a C compiler.
RUN apt-get update && apt-get install -y --no-install-recommends build-essential \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY .cargo .cargo
COPY crates crates
RUN cargo build --release --locked -p airplane-scoring-server

# The server must compute flights exactly as the browser does (see
# crates/core/src/replay.rs); core's maths comes from the pure-Rust libm
# crate, so the system's maths library plays no part.
FROM debian:bookworm-slim
COPY --from=build /src/target/release/airplane-scoring-server /usr/local/bin/
# The database lives on a volume mounted here, so it survives deploys.
ENV PORT=8080 DB_PATH=/data/leaderboard.sqlite
EXPOSE 8080
CMD ["airplane-scoring-server"]
