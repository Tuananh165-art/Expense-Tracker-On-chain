# Expense Tracker On-chain

> Tài liệu đầy đủ (keyword, thuật ngữ, tech stack, blockchain, database schema, setup/run chi tiết): **[docs/Project-Guide.md](docs/Project-Guide.md)**

Ứng dụng quản lý chi tiêu theo mô hình **hybrid on-chain + off-chain**:
- **On-chain (Solana/Anchor):** ghi nhận transaction có thể kiểm toán.
- **Off-chain (Rust/Axum + Postgres):** auth, RBAC, read model, reporting.
- **Frontend (Next.js):** dashboard và wallet UX.

---

## 1) Mục tiêu

- Minh bạch lịch sử thay đổi expense/category.
- Có audit trail rõ ràng.
- Giữ UX dễ dùng như app web thông thường.

---

## 2) Kiến trúc repo

- `contracts/expense_program` — Anchor program
- `apps/api` — Axum API
- `apps/web` — Next.js frontend
- `packages/shared` — shared DTO/types
- `packages/sdk` — API client SDK
- `infra/migrations` — SQL migrations
- `scripts` — local/dev verification scripts

---

## 3) Yêu cầu môi trường

- Node.js >= 20
- pnpm >= 9
- Rust stable + cargo
- Solana CLI
- Anchor CLI
- Docker + Docker Compose (Postgres)

---

## 4) Cài dependencies

```bash
pnpm install
```

---

## 5) Cấu hình env

## 5.1 Root `.env` (API/contract runtime)

Tạo `.env` từ `.env.example`, bảo đảm các biến sau đúng:

```env
EXPENSES_PG_ENABLED=true
AUTH_PG_ENABLED=true
HYBRID_ONCHAIN_ENABLED=true

SOLANA_RPC_URL=http://127.0.0.1:8899
SOLANA_COMMITMENT=confirmed
SOLANA_PROGRAM_ID=<PROGRAM_ID_LOCALNET>

NEXT_PUBLIC_API_BASE_URL=http://localhost:8080
NEXT_PUBLIC_PROGRAM_ID=<PROGRAM_ID_LOCALNET>
```

> `SOLANA_PROGRAM_ID` và `NEXT_PUBLIC_PROGRAM_ID` phải trùng nhau.

## 5.2 Frontend env `apps/web/.env.local` (bắt buộc cho hybrid UI)

```env
NEXT_PUBLIC_API_BASE_URL=http://localhost:8080
NEXT_PUBLIC_HYBRID_ONCHAIN_ENABLED=true
NEXT_PUBLIC_SOLANA_RPC_URL=http://127.0.0.1:8899
NEXT_PUBLIC_SOLANA_COMMITMENT=confirmed
NEXT_PUBLIC_PROGRAM_ID=<PROGRAM_ID_LOCALNET>
```

> Nếu thiếu `NEXT_PUBLIC_HYBRID_ONCHAIN_ENABLED=true`, frontend sẽ đi flow off-chain cũ (`/api/v1/categories`, `/api/v1/expenses`, `/api/v1/expenses/:id/status`).

---

## 6) Chạy local chuẩn (4 terminal)

## Terminal #1 — Solana local validator

```bash
pkill -f solana-test-validator || true

solana-test-validator \
  --ledger "$HOME/solana-ledger" \
  --reset \
  --rpc-port 8899 \
  --faucet-port 9900 \
  --gossip-port 18000 \
  --dynamic-port-range 18001-18100
```

## Terminal #2 — Deploy program localnet

```bash
pnpm run deploy:localnet
```

Hoặc thủ công:

```bash
cd contracts/expense_program
anchor build
anchor deploy --provider.cluster localnet
```

Lấy Program ID:

```bash
solana address -k target/deploy/expense_program-keypair.json
```

Cập nhật lại `.env` + `apps/web/.env.local` bằng Program ID vừa deploy.

## Terminal #3 — API

```bash
cargo run --manifest-path apps/api/Cargo.toml
```

Expected: `API listening on 0.0.0.0:8080`

## Terminal #4 — Web

```bash
pnpm --filter web dev
```

Expected: `http://localhost:3000`

> Mỗi lần đổi `NEXT_PUBLIC_*`, bắt buộc restart `pnpm --filter web dev`.

---

## 7) Phantom setup cho localnet

Trong Phantom:

1. `Developer Settings` bật `Testnet Mode`
2. Chọn **Solana Localnet** (không chọn Devnet/Testnet nếu backend đang local RPC)
3. Dùng đúng account đang active để test

Airdrop SOL cho account đang active:

```bash
export SOLANA_RPC_URL=http://127.0.0.1:8899
export WALLET=<PHANTOM_ACTIVE_PUBLIC_KEY>

SOLANA_URL="$SOLANA_RPC_URL" solana airdrop 5 "$WALLET"
SOLANA_URL="$SOLANA_RPC_URL" solana balance "$WALLET"
```

---

## 8) Flow hybrid trên UI (manual checklist)

1. Mở `http://localhost:3000`, sign in bằng wallet.
2. Tạo category từ UI.
3. Tạo expense từ UI.
4. Approve expense (admin).
5. Kiểm tra Network tab phải có:
   - `POST /api/v1/onchain/categories/commit`
   - `POST /api/v1/onchain/expenses/commit-create`
   - `POST /api/v1/onchain/expenses/:id/commit-status`
6. Kiểm tra DB `categories` / `expenses_read_model` có cột onchain (`tx_hash`, `onchain_*`, `status_tx_hash`) khác `NULL`.

---

## 9) Verify E2E strict

Sau khi có tx hash/token thật:

```bash
export ACCESS_TOKEN='<JWT_ACCESS_TOKEN>'
export CATEGORY_TX_HASH='<CATEGORY_TX_HASH>'
export EXPENSE_CREATE_TX_HASH='<EXPENSE_CREATE_TX_HASH>'
export EXPENSE_STATUS_TX_HASH='<EXPENSE_STATUS_TX_HASH>'
export EXPENSE_ID='<EXPENSE_UUID>'

./scripts/verify_hybrid_e2e.sh --strict
```

Script sẽ fail nếu thiếu env hoặc DB/audit chưa sync.

---

## 10) Scripts hữu ích

```bash
pnpm run dev:web
pnpm run build:web
pnpm run lint:web
pnpm run test:web
pnpm run deploy:localnet
pnpm run deploy:devnet
bash scripts/test-all.sh
./scripts/verify_hybrid_e2e.sh
./scripts/verify_hybrid_e2e.sh --strict
```

---

## 11) Troubleshooting nhanh

- **UI vẫn gọi `/api/v1/categories` thay vì `/api/v1/onchain/...`**
  - Thiếu `NEXT_PUBLIC_HYBRID_ONCHAIN_ENABLED=true` trong `apps/web/.env.local`
  - Chưa restart web dev server

- **Phantom báo thiếu SOL**
  - Airdrop sai ví (không phải account active)
  - Phantom đang Devnet/Testnet thay vì Localnet

- **`transaction not found at selected commitment`**
  - RPC/commitment mismatch
  - Dùng `SOLANA_COMMITMENT=confirmed` cho local dev

- **`jwt wallet is not a transaction signer`**
  - Token wallet khác ví ký transaction

- **`--strict requires ...`**
  - Chưa export đủ env bắt buộc (`ACCESS_TOKEN`, 3 tx hash, `EXPENSE_ID`)

---

## 12) Security baseline

- Verify wallet signature ở backend
- JWT + refresh/session revoke
- RBAC (`user/admin/auditor`)
- Idempotency cho API ghi
- Audit logs cho action quan trọng
- Hybrid constraints ở DB để đảm bảo dữ liệu onchain/offchain nhất quán

Xem thêm:
- [docs/Architecture.md](docs/Architecture.md)
- [docs/Security.md](docs/Security.md)
