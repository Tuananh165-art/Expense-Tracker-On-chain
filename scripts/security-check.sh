#!/usr/bin/env bash
set -euo pipefail

echo "[security] Rust dependency audit"
if command -v cargo-audit >/dev/null 2>&1; then
  cargo audit
else
  echo "cargo-audit not installed, skipping cargo audit"
fi

echo "[security] JS dependency audit"
pnpm audit --prod --audit-level high || true

echo "[security] Secret scan"
if command -v gitleaks >/dev/null 2>&1; then
  gitleaks detect --no-banner --redact
else
  echo "gitleaks not installed, skipping secret scan"
fi

echo "[security] done"
