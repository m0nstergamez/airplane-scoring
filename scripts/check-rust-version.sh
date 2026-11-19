#!/usr/bin/env bash
# One Rust everywhere: the server image (Dockerfile) must build with the
# version in rust-toolchain.toml. Run by scripts/check.sh and CI.
set -euo pipefail
cd "$(dirname "$0")/.."
rust=$(sed -nE "s/^[[:space:]]*channel[[:space:]]*=[[:space:]]*[\"']([^\"']+)[\"'].*/\1/p" rust-toolchain.toml)
[ -n "$rust" ] || { echo "FAIL: no channel = \"...\" in rust-toolchain.toml" >&2; exit 1; }
tag=$(sed -nE 's/^FROM rust:([^ ]+)-slim.*/\1/p' Dockerfile)
[ "$tag" = "$rust" ] \
  || { echo "FAIL: Dockerfile builds with rust:${tag:-?}, rust-toolchain.toml says $rust" >&2; exit 1; }
echo "Rust $rust in rust-toolchain.toml and Dockerfile"
