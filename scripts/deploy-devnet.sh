#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CONTRACT_DIR="$ROOT_DIR/contracts/expense_program"
PROGRAM_KEYPAIR="$CONTRACT_DIR/target/deploy/expense_program-keypair.json"

if ! command -v anchor >/dev/null 2>&1; then
  echo "[ERROR] anchor CLI not found. Install Anchor (AVM) first." >&2
  exit 1
fi

if ! command -v solana >/dev/null 2>&1; then
  echo "[ERROR] solana CLI not found. Install Solana CLI first." >&2
  exit 1
fi

if [[ ! -f "$HOME/.config/solana/id.json" ]]; then
  echo "[ERROR] Wallet file missing: $HOME/.config/solana/id.json" >&2
  echo "Run: solana-keygen new --no-bip39-passphrase -o ~/.config/solana/id.json" >&2
  exit 1
fi

echo "[INFO] Using wallet: $HOME/.config/solana/id.json"
solana config set --url devnet >/dev/null

echo "[INFO] Building Anchor program..."
(
  cd "$CONTRACT_DIR"
  anchor build --no-idl
)

echo "[INFO] Deploying to Solana Devnet..."
(
  cd "$CONTRACT_DIR"
  anchor deploy --provider.cluster devnet
)

if [[ ! -f "$PROGRAM_KEYPAIR" ]]; then
  echo "[ERROR] Program keypair not found: $PROGRAM_KEYPAIR" >&2
  exit 1
fi

PROGRAM_ID="$(solana address -k "$PROGRAM_KEYPAIR")"

echo "[OK] Deployed program ID: $PROGRAM_ID"
echo "[INFO] Verify on-chain: solana program show $PROGRAM_ID --url devnet"
echo "[INFO] Update .env values:"
echo "  SOLANA_RPC_URL=https://api.devnet.solana.com"
echo "  PROGRAM_ID=$PROGRAM_ID"
echo "  NEXT_PUBLIC_PROGRAM_ID=$PROGRAM_ID"
