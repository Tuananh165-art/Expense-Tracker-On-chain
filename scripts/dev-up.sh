#!/usr/bin/env bash
set -euo pipefail

echo "[dev-up] starting postgres + redis..."
docker compose up -d postgres redis

echo "[dev-up] services status"
docker compose ps
