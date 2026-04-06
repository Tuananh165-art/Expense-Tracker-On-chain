#!/usr/bin/env bash
set -euo pipefail

echo "[test] frontend type/lint/test"
pnpm --filter web lint
pnpm --filter web test

echo "[test] backend"
cargo test --manifest-path apps/api/Cargo.toml

echo "[test] done"
