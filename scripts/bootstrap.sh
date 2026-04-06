#!/usr/bin/env bash
set -euo pipefail

echo "[bootstrap] checking toolchain..."
command -v pnpm >/dev/null 2>&1 || { echo "pnpm is required"; exit 1; }
command -v cargo >/dev/null 2>&1 || { echo "cargo is required"; exit 1; }
command -v docker >/dev/null 2>&1 || { echo "docker is required"; exit 1; }

echo "[bootstrap] installing Node dependencies..."
pnpm install

echo "[bootstrap] done"
