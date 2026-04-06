#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

STRICT_MODE=false
if [[ "${1:-}" == "--strict" ]]; then
  STRICT_MODE=true
fi

if [[ -f ".env" ]]; then
  # shellcheck disable=SC2046
  export $(grep -E '^[A-Za-z_][A-Za-z0-9_]*=' .env | xargs)
fi

API_BASE_URL="${API_BASE_URL:-${NEXT_PUBLIC_API_BASE_URL:-http://localhost:8080}}"
SOLANA_RPC_URL="${SOLANA_RPC_URL:-http://127.0.0.1:8899}"
SOLANA_PROGRAM_ID="${SOLANA_PROGRAM_ID:-}"
SOLANA_COMMITMENT="${SOLANA_COMMITMENT:-finalized}"
ACCESS_TOKEN="${ACCESS_TOKEN:-}"
CATEGORY_TX_HASH="${CATEGORY_TX_HASH:-}"
EXPENSE_CREATE_TX_HASH="${EXPENSE_CREATE_TX_HASH:-}"
EXPENSE_STATUS_TX_HASH="${EXPENSE_STATUS_TX_HASH:-}"
EXPENSE_ID="${EXPENSE_ID:-}"
POSTGRES_DB="${POSTGRES_DB:-expense_tracker}"
POSTGRES_USER="${POSTGRES_USER:-postgres}"

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

ok() { echo -e "${GREEN}[ok]${NC} $*"; }
warn() { echo -e "${YELLOW}[warn]${NC} $*"; }
fail() { echo -e "${RED}[fail]${NC} $*"; exit 1; }

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || fail "missing command: $1"
}

is_base58() {
  [[ "$1" =~ ^[1-9A-HJ-NP-Za-km-z]+$ ]]
}

require_env_if_strict() {
  local name="$1"
  local value="$2"
  if [[ "$STRICT_MODE" == "true" && -z "$value" ]]; then
    fail "--strict requires $name"
  fi
}

require_cmd solana
require_cmd curl
require_cmd docker

if [[ -z "$SOLANA_PROGRAM_ID" ]]; then
  fail "SOLANA_PROGRAM_ID is empty. Set it in .env first"
fi

require_env_if_strict "ACCESS_TOKEN" "$ACCESS_TOKEN"
require_env_if_strict "CATEGORY_TX_HASH" "$CATEGORY_TX_HASH"
require_env_if_strict "EXPENSE_CREATE_TX_HASH" "$EXPENSE_CREATE_TX_HASH"
require_env_if_strict "EXPENSE_STATUS_TX_HASH" "$EXPENSE_STATUS_TX_HASH"

for tx in "$CATEGORY_TX_HASH" "$EXPENSE_CREATE_TX_HASH" "$EXPENSE_STATUS_TX_HASH"; do
  if [[ -n "$tx" ]] && ! is_base58 "$tx"; then
    fail "invalid base58 tx hash: $tx"
  fi
done

PG_CONTAINER="${PG_CONTAINER:-$(docker compose ps -q postgres)}"
if [[ -z "$PG_CONTAINER" ]]; then
  fail "postgres container is not running (docker compose ps -q postgres is empty)"
fi

db_exec() {
  local sql="$1"
  docker exec -i "$PG_CONTAINER" psql \
    -U "$POSTGRES_USER" \
    -d "$POSTGRES_DB" \
    -v ON_ERROR_STOP=1 \
    -P pager=off \
    -c "$sql"
}

db_scalar() {
  local sql="$1"
  docker exec -i "$PG_CONTAINER" psql \
    -U "$POSTGRES_USER" \
    -d "$POSTGRES_DB" \
    -v ON_ERROR_STOP=1 \
    -tA \
    -c "$sql" | tr -d '[:space:]'
}

echo "== Hybrid E2E verification =="
echo "root: $ROOT_DIR"
echo "api:  $API_BASE_URL"
echo "rpc:  $SOLANA_RPC_URL"
echo "program: $SOLANA_PROGRAM_ID"
echo "commitment: $SOLANA_COMMITMENT"
echo "strict: $STRICT_MODE"

echo
ok "Checking Solana CLI config"
solana config get

echo
ok "Checking localnet RPC health"
SOLANA_URL="$SOLANA_RPC_URL" solana slot >/dev/null
SOLANA_URL="$SOLANA_RPC_URL" solana cluster-version >/dev/null
ok "Solana RPC reachable"

echo
ok "Checking deployed program"
SOLANA_URL="$SOLANA_RPC_URL" solana program show "$SOLANA_PROGRAM_ID" >/dev/null
ok "Program exists on current RPC"

echo
ok "Checking API health"
curl -fsS "$API_BASE_URL/health" >/dev/null
ok "API is healthy"

echo
ok "Checking Docker postgres service"
docker compose ps postgres >/dev/null
ok "docker compose can see postgres service"

echo
ok "Checking DB connectivity through docker exec"
db_exec "SELECT 1;" >/dev/null
ok "Database reachable"

echo
ok "Checking schema columns (hybrid rollout)"
CAT_COLS_COUNT="$(db_scalar "SELECT COUNT(*) FROM information_schema.columns WHERE table_name='categories' AND column_name IN ('tx_hash','onchain_category_pda','onchain_slot','onchain_committed_at');")"
EXP_COLS_COUNT="$(db_scalar "SELECT COUNT(*) FROM information_schema.columns WHERE table_name='expenses_read_model' AND column_name IN ('onchain_expense_pda','status_tx_hash','status_onchain_slot','status_committed_at');")"
AUDIT_COLS_COUNT="$(db_scalar "SELECT COUNT(*) FROM information_schema.columns WHERE table_name='tx_audit_logs' AND column_name IN ('onchain_verified','onchain_program_id','onchain_slot');")"

if [[ "$CAT_COLS_COUNT" -ne 4 || "$EXP_COLS_COUNT" -ne 4 || "$AUDIT_COLS_COUNT" -ne 3 ]]; then
  fail "hybrid schema columns missing (categories=$CAT_COLS_COUNT/4 expenses=$EXP_COLS_COUNT/4 audit=$AUDIT_COLS_COUNT/3). Apply migrations 0006+ first"
fi
ok "Hybrid columns detected"

echo
ok "Checking constraints status"
CONSTRAINT_COUNT="$(db_scalar "SELECT COUNT(*) FROM pg_constraint WHERE conname IN ('chk_categories_hybrid_commit_fields','chk_expenses_hybrid_create_fields','chk_expenses_hybrid_status_fields','chk_audit_hybrid_actions_require_onchain_fields');")"
if [[ "$CONSTRAINT_COUNT" -ne 4 ]]; then
  fail "hybrid constraints missing ($CONSTRAINT_COUNT/4). Apply migrations 0007+ first"
fi

db_exec "SELECT conname, convalidated FROM pg_constraint WHERE conname IN ('chk_categories_hybrid_commit_fields','chk_expenses_hybrid_create_fields','chk_expenses_hybrid_status_fields','chk_audit_hybrid_actions_require_onchain_fields') ORDER BY conname;"

if [[ -z "$ACCESS_TOKEN" ]]; then
  warn "ACCESS_TOKEN not set. Skipping authenticated API checks."
else
  echo
  ok "Checking authenticated categories endpoint"
  curl -fsS "$API_BASE_URL/api/v1/categories" \
    -H "Authorization: Bearer $ACCESS_TOKEN" >/dev/null
  ok "Access token accepted by API"
fi

if [[ -n "$CATEGORY_TX_HASH" ]]; then
  echo
  ok "Checking DB + audit sync for CATEGORY_TX_HASH"
  CATEGORY_ROWS="$(db_scalar "SELECT COUNT(*) FROM categories WHERE tx_hash = '$CATEGORY_TX_HASH';")"
  CATEGORY_AUDIT_ROWS="$(db_scalar "SELECT COUNT(*) FROM tx_audit_logs WHERE tx_hash = '$CATEGORY_TX_HASH';")"
  db_exec "SELECT id, owner_user_id, name, tx_hash, onchain_category_pda, onchain_slot, onchain_committed_at FROM categories WHERE tx_hash = '$CATEGORY_TX_HASH';"
  db_exec "SELECT action, actor_wallet, tx_hash, onchain_verified, onchain_program_id, onchain_slot, created_at FROM tx_audit_logs WHERE tx_hash = '$CATEGORY_TX_HASH' ORDER BY created_at DESC;"
  if [[ "$CATEGORY_ROWS" -eq 0 || "$CATEGORY_AUDIT_ROWS" -eq 0 ]]; then
    fail "CATEGORY_TX_HASH is not synced to DB/audit (categories=$CATEGORY_ROWS audit=$CATEGORY_AUDIT_ROWS)"
  fi
else
  warn "CATEGORY_TX_HASH not set. Skipping category sync query."
fi

if [[ -n "$EXPENSE_CREATE_TX_HASH" ]]; then
  echo
  ok "Checking DB + audit sync for EXPENSE_CREATE_TX_HASH"
  EXP_CREATE_ROWS="$(db_scalar "SELECT COUNT(*) FROM expenses_read_model WHERE tx_hash = '$EXPENSE_CREATE_TX_HASH';")"
  EXP_CREATE_AUDIT_ROWS="$(db_scalar "SELECT COUNT(*) FROM tx_audit_logs WHERE tx_hash = '$EXPENSE_CREATE_TX_HASH';")"
  db_exec "SELECT id, owner_user_id, category_id, amount_minor, currency, status, tx_hash, onchain_expense_pda, occurred_at, created_at FROM expenses_read_model WHERE tx_hash = '$EXPENSE_CREATE_TX_HASH';"
  db_exec "SELECT action, actor_wallet, tx_hash, onchain_verified, onchain_program_id, onchain_slot, created_at FROM tx_audit_logs WHERE tx_hash = '$EXPENSE_CREATE_TX_HASH' ORDER BY created_at DESC;"
  if [[ "$EXP_CREATE_ROWS" -eq 0 || "$EXP_CREATE_AUDIT_ROWS" -eq 0 ]]; then
    fail "EXPENSE_CREATE_TX_HASH is not synced to DB/audit (expenses=$EXP_CREATE_ROWS audit=$EXP_CREATE_AUDIT_ROWS)"
  fi
else
  warn "EXPENSE_CREATE_TX_HASH not set. Skipping expense-create sync query."
fi

if [[ -n "$EXPENSE_STATUS_TX_HASH" ]]; then
  echo
  ok "Checking DB + audit sync for EXPENSE_STATUS_TX_HASH"
  EXP_STATUS_ROWS="$(db_scalar "SELECT COUNT(*) FROM expenses_read_model WHERE status_tx_hash = '$EXPENSE_STATUS_TX_HASH';")"
  EXP_STATUS_AUDIT_ROWS="$(db_scalar "SELECT COUNT(*) FROM tx_audit_logs WHERE tx_hash = '$EXPENSE_STATUS_TX_HASH';")"
  db_exec "SELECT id, status, status_tx_hash, status_onchain_slot, status_committed_at FROM expenses_read_model WHERE status_tx_hash = '$EXPENSE_STATUS_TX_HASH';"
  db_exec "SELECT action, actor_wallet, tx_hash, onchain_verified, onchain_program_id, onchain_slot, created_at FROM tx_audit_logs WHERE tx_hash = '$EXPENSE_STATUS_TX_HASH' ORDER BY created_at DESC;"
  if [[ "$EXP_STATUS_ROWS" -eq 0 || "$EXP_STATUS_AUDIT_ROWS" -eq 0 ]]; then
    fail "EXPENSE_STATUS_TX_HASH is not synced to DB/audit (expenses=$EXP_STATUS_ROWS audit=$EXP_STATUS_AUDIT_ROWS)"
  fi
else
  warn "EXPENSE_STATUS_TX_HASH not set. Skipping expense-status sync query."
fi

if [[ -n "$EXPENSE_ID" ]]; then
  echo
  ok "Checking expense history endpoint"
  if [[ -z "$ACCESS_TOKEN" ]]; then
    if [[ "$STRICT_MODE" == "true" ]]; then
      fail "--strict requires ACCESS_TOKEN when EXPENSE_ID is provided"
    fi
    warn "EXPENSE_ID provided but ACCESS_TOKEN missing, cannot call history endpoint"
  else
    curl -fsS "$API_BASE_URL/api/v1/expenses/$EXPENSE_ID/history" \
      -H "Authorization: Bearer $ACCESS_TOKEN" >/dev/null
    ok "Expense history endpoint reachable"
  fi
fi

echo
ok "Hybrid E2E verification finished"