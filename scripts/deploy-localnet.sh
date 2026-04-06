#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CONTRACT_DIR="$ROOT_DIR/contracts/expense_program"
PROGRAM_KEYPAIR="$CONTRACT_DIR/target/deploy/expense_program-keypair.json"

export PATH="$HOME/.cargo/bin:$HOME/.avm/bin:$HOME/.local/share/solana/install/active_release/bin:$PATH"

if command -v anchor >/dev/null 2>&1; then
  ANCHOR_BIN="anchor"
elif command -v anchor.exe >/dev/null 2>&1; then
  ANCHOR_BIN="anchor.exe"
else
  echo "[ERROR] anchor CLI not found. Install Anchor first." >&2
  exit 1
fi

if command -v solana >/dev/null 2>&1; then
  SOLANA_BIN="solana"
elif command -v solana.exe >/dev/null 2>&1; then
  SOLANA_BIN="solana.exe"
else
  echo "[ERROR] solana CLI not found. Install Solana CLI first." >&2
  exit 1
fi

if command -v solana-keygen >/dev/null 2>&1; then
  SOLANA_KEYGEN_BIN="solana-keygen"
elif command -v solana-keygen.exe >/dev/null 2>&1; then
  SOLANA_KEYGEN_BIN="solana-keygen.exe"
else
  echo "[ERROR] solana-keygen not found. Install Solana CLI first." >&2
  exit 1
fi

WALLET_PATH=""
for candidate in "$HOME/.config/solana/id.json" /c/Users/*/.config/solana/id.json; do
  if [[ -f "$candidate" ]]; then
    WALLET_PATH="$candidate"
    break
  fi
done

if [[ -z "$WALLET_PATH" ]]; then
  WALLET_PATH="$HOME/.config/solana/id.json"
  mkdir -p "$(dirname "$WALLET_PATH")"
  "$SOLANA_KEYGEN_BIN" new --no-bip39-passphrase -o "$WALLET_PATH" -f >/dev/null
  echo "[INFO] Created local wallet at: $WALLET_PATH"
fi

if ! "$SOLANA_BIN" cluster-version --url http://127.0.0.1:8899 >/dev/null 2>&1; then
  echo "[ERROR] localnet chưa chạy. Start trước bằng (WSL/Linux):" >&2
  echo "  solana-test-validator --ledger \"$HOME/solana-ledger\" --reset" >&2
  echo "[ERROR] hoặc trên Windows PowerShell:" >&2
  echo "  solana-test-validator --ledger C:/solana-ledger --reset" >&2
  exit 1
fi

echo "[INFO] Building Anchor program..."
(
  cd "$CONTRACT_DIR"
  anchor build --no-idl
)

echo "[INFO] Deploying to localnet via Anchor..."
(
  cd "$CONTRACT_DIR"
  "$ANCHOR_BIN" deploy --provider.cluster localnet --provider.wallet "$WALLET_PATH"
)

if [[ ! -f "$PROGRAM_KEYPAIR" ]]; then
  echo "[ERROR] Program keypair not found: $PROGRAM_KEYPAIR" >&2
  exit 1
fi

PROGRAM_ID="$("$SOLANA_BIN" address -k "$PROGRAM_KEYPAIR")"

echo "[OK] Localnet PROGRAM_ID: $PROGRAM_ID"
echo "[INFO] Update .env values:"
echo "  SOLANA_RPC_URL=http://127.0.0.1:8899"
echo "  PROGRAM_ID=$PROGRAM_ID"
echo "  NEXT_PUBLIC_PROGRAM_ID=$PROGRAM_ID"
echo "  NEXT_PUBLIC_SOLANA_CLUSTER=localnet"
