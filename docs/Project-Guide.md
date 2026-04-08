# Project Guide — Expense Tracker On-chain

Tài liệu này là bản hướng dẫn đầy đủ cho dự án: thuật ngữ, kiến trúc, luồng nghiệp vụ, blockchain, database, lệnh chạy, setup local, kiểm thử, và troubleshooting.

---

## 1) Dự án giải quyết bài toán gì?

### Bài toán
Các app quản lý chi tiêu truyền thống thường gặp:
- Khó kiểm toán ai đã tạo/sửa/duyệt dữ liệu.
- Dữ liệu có thể bị chỉnh sửa mà khó truy vết.
- Reporting cần nhanh nhưng vẫn phải đảm bảo tính minh bạch.

### Cách giải quyết của dự án
Dự án dùng mô hình **hybrid on-chain + off-chain**:
- **On-chain (Solana/Anchor):** ghi nhận các giao dịch nghiệp vụ quan trọng (category/expense/status), tạo dấu vết bất biến.
- **Off-chain (Rust/Axum + PostgreSQL):** auth, RBAC, query nhanh, reporting, audit index.
- **Frontend (Next.js):** UX thân thiện như web app thông thường, tích hợp wallet.

### Giá trị thực tế
- Truy vết đầy đủ theo `tx_hash`.
- Dễ đối soát nội bộ (admin/auditor).
- Vẫn giữ hiệu năng truy vấn tốt cho dashboard/report.

---

## 2) Thuật ngữ và keyword quan trọng

- **Hybrid mode**: chế độ kết hợp on-chain và off-chain.
- **PDA (Program Derived Address)**: địa chỉ account xác định theo seed + program id.
- **Program ID**: định danh smart contract Anchor đã deploy.
- **Commit on-chain**: API verify transaction on-chain rồi ghi nhận vào DB read model.
- **Read model**: bảng tối ưu truy vấn/reporting, không phải nguồn bất biến gốc.
- **Idempotency key**: khóa chống ghi trùng khi retry request.
- **RBAC**: phân quyền theo vai trò `user/admin/auditor`.
- **Audit log**: nhật ký hành động có actor, target, tx hash, metadata.
- **Commitment**: mức xác nhận RPC Solana (`processed/confirmed/finalized`).

---

## 3) Tech stack

### Frontend
- Next.js 15
- React 18
- TypeScript
- TanStack Query
- @solana/web3.js

### Backend API
- Rust
- Axum
- SQLx
- PostgreSQL
- JWT (jsonwebtoken)
- Tracing/logging

### Smart contract
- Rust
- Anchor
- Solana localnet/devnet

### Data/Infra
- PostgreSQL 16
- Redis 7
- Docker Compose
- pnpm workspace (monorepo)

---

## 4) Cấu trúc repo

- `apps/web` — frontend dashboard + wallet UX
- `apps/api` — API (auth, RBAC, read model, commit verify)
- `contracts/expense_program` — Anchor smart contract + tests
- `packages/shared` — DTO/type shared
- `packages/sdk` — API client SDK cho frontend
- `infra/migrations` — database migrations
- `scripts` — script deploy/verify/test

---

## 5) Blockchain model (Anchor/Solana)

### Account/PDA chính
- `program_config` PDA
  - Lưu `admin_authority`, dùng cho approve/reject hợp lệ on-chain theo admin model.
- `user_profile` PDA
  - Hồ sơ owner wallet trong program.
- `category` PDA
  - Danh mục chi tiêu của owner.
- `expense` PDA
  - Bản ghi expense on-chain (amount, status, owner, liên kết category).

### Instruction chính
- `init_program_config`
- `init_user_profile`
- `create_category`
- `create_expense`
- `update_expense_status` (owner hoặc admin authority)

### Quy tắc chính
- PDA seed xác định, tránh account giả.
- Chỉ signer hợp lệ mới đổi trạng thái expense.
- `amount > 0`.
- Trạng thái nghiệp vụ: `pending | approved | rejected`.

---

## 6) Data model (PostgreSQL)

Dưới đây là các bảng theo migrations hiện có trong `infra/migrations/*.sql`.

### `users`
| Column | Type | Mô tả |
|---|---|---|
| `id` | UUID PK | User id |
| `wallet_address` | TEXT UNIQUE | Địa chỉ ví |
| `role` | TEXT | `user/admin/auditor` (mặc định `user`) |
| `created_at` | TIMESTAMPTZ | Thời điểm tạo |

### `categories`
| Column | Type | Mô tả |
|---|---|---|
| `id` | UUID PK | Category id |
| `owner_user_id` | UUID FK -> users(id) | Chủ sở hữu |
| `name` | TEXT | Tên category |
| `created_at` | TIMESTAMPTZ | Thời điểm tạo |
| `tx_hash` | TEXT NULL | Tx hash tạo category (hybrid) |
| `onchain_category_pda` | TEXT NULL | PDA category |
| `onchain_slot` | BIGINT NULL | Slot commit |
| `onchain_committed_at` | TIMESTAMPTZ NULL | Thời điểm commit |

Chỉ mục/ràng buộc nổi bật:
- `UNIQUE(owner_user_id, name)`
- unique index `tx_hash` có điều kiện `WHERE tx_hash IS NOT NULL`

### `expenses_read_model`
| Column | Type | Mô tả |
|---|---|---|
| `id` | UUID PK | Expense id |
| `owner_user_id` | UUID FK -> users(id) | Chủ expense |
| `category_id` | UUID FK -> categories(id) | Category |
| `amount` | NUMERIC(18,2) | Giá trị hiển thị/report |
| `amount_minor` | BIGINT | Giá trị minor unit (ví dụ cents) |
| `currency` | TEXT | Mã tiền tệ |
| `status` | TEXT | `pending/approved/rejected` |
| `tx_hash` | TEXT NULL | Tx hash create expense |
| `onchain_expense_pda` | TEXT NULL | PDA expense |
| `status_tx_hash` | TEXT NULL | Tx hash update status |
| `status_onchain_slot` | BIGINT NULL | Slot update status |
| `status_committed_at` | TIMESTAMPTZ NULL | Thời điểm commit status |
| `occurred_at` | TIMESTAMPTZ | Thời điểm nghiệp vụ |
| `created_at` | TIMESTAMPTZ | Thời điểm tạo read model |

Chỉ mục/ràng buộc nổi bật:
- Check `amount_minor > 0`
- Check `status IN ('pending','approved','rejected')`
- Unique index `tx_hash` (create) và `status_tx_hash` (status) khi khác NULL

### `tx_audit_logs`
| Column | Type | Mô tả |
|---|---|---|
| `id` | UUID PK | Audit id |
| `actor_wallet` | TEXT | Ví thực hiện hành động |
| `action` | TEXT | `category.create`, `expense.create`, `expense.approve`, ... |
| `target_id` | TEXT NULL | ID đối tượng bị tác động |
| `tx_hash` | TEXT NULL | Tx liên quan |
| `metadata` | JSONB NOT NULL DEFAULT `{}` | Metadata mở rộng |
| `onchain_verified` | BOOLEAN NOT NULL DEFAULT FALSE | Đã verify on-chain chưa |
| `onchain_program_id` | TEXT NULL | Program id verify |
| `onchain_slot` | BIGINT NULL | Slot verify |
| `created_at` | TIMESTAMPTZ | Thời điểm log |

### `idempotency_keys`
| Column | Type | Mô tả |
|---|---|---|
| `id` | UUID PK | Record id |
| `key` | TEXT UNIQUE | Idempotency key |
| `request_hash` | TEXT NULL | Hash request (tùy flow) |
| `response_payload` | JSONB NULL | Cached response |
| `created_at` | TIMESTAMPTZ | Thời điểm tạo |

### Auth/session tables
- `auth_challenges`: nonce/challenge đăng nhập wallet.
- `auth_refresh_sessions`: phiên refresh token.
- `revoked_token_families`: revoke theo family.
- `revoked_access_tokens`: blacklist access token theo `jti`.

---

## 7) Luồng nghiệp vụ chính

### 7.1 Đăng nhập wallet
1. FE xin challenge từ API.
2. User ký message bằng wallet.
3. FE gửi signature + public key.
4. API verify signature, cấp JWT.

### 7.2 Tạo category hybrid
1. FE gửi tx tạo category lên chain (wallet ký).
2. FE/API gọi endpoint commit category.
3. API verify instruction/accounts/signer/tx.
4. API ghi `categories` + `tx_audit_logs`.

### 7.3 Tạo expense hybrid
1. FE tạo tx `create_expense` on-chain.
2. API commit create, verify tx hợp lệ.
3. API upsert `expenses_read_model` và audit log.

### 7.4 Approve/reject expense
1. Admin (hoặc owner theo policy contract) ký tx update status.
2. API commit status, verify signer + account layout.
3. API cập nhật `status`, `status_tx_hash`, `status_onchain_slot`, `status_committed_at`.

---

## 8) Các lệnh chạy thường dùng

### 8.1 Cài dependencies
```bash
pnpm install
```

### 8.2 Chạy hạ tầng DB/Redis
```bash
docker compose up -d postgres redis
```

### 8.3 Chạy web/api
```bash
pnpm run dev:web
cargo run --manifest-path apps/api/Cargo.toml
```

### 8.4 Build/lint/test frontend
```bash
pnpm run lint:web
pnpm run test:web
pnpm run build:web
```

### 8.5 Deploy contract localnet/devnet
```bash
pnpm run deploy:localnet
pnpm run deploy:devnet
```

### 8.6 Script verify end-to-end hybrid
```bash
./scripts/verify_hybrid_e2e.sh
./scripts/verify_hybrid_e2e.sh --strict
```

---

## 9) Setup local đầy đủ (khuyến nghị)

### Bước 1: Chuẩn bị môi trường
Cần có:
- Node.js >= 20
- pnpm >= 9
- Rust stable + cargo
- Solana CLI
- Anchor CLI
- Docker + Docker Compose

### Bước 2: Tạo env
- Root `.env` (API/runtime)
- `apps/web/.env.local` (frontend)

Giá trị quan trọng:
```env
EXPENSES_PG_ENABLED=true
AUTH_PG_ENABLED=true
HYBRID_ONCHAIN_ENABLED=true

SOLANA_RPC_URL=http://127.0.0.1:8899
SOLANA_COMMITMENT=confirmed
SOLANA_PROGRAM_ID=<PROGRAM_ID_LOCALNET>

NEXT_PUBLIC_API_BASE_URL=http://localhost:8080
NEXT_PUBLIC_HYBRID_ONCHAIN_ENABLED=true
NEXT_PUBLIC_SOLANA_RPC_URL=http://127.0.0.1:8899
NEXT_PUBLIC_SOLANA_COMMITMENT=confirmed
NEXT_PUBLIC_PROGRAM_ID=<PROGRAM_ID_LOCALNET>
```

`SOLANA_PROGRAM_ID` và `NEXT_PUBLIC_PROGRAM_ID` phải trùng nhau.

### Bước 3: Chạy Postgres/Redis
```bash
docker compose up -d postgres redis
```

Theo `docker-compose.yml`, Postgres map `5434 -> 5432`.

### Bước 4: Chạy Solana validator local
```bash
solana-test-validator \
  --ledger "$HOME/solana-ledger" \
  --reset \
  --rpc-port 8899 \
  --faucet-port 9900 \
  --gossip-port 18000 \
  --dynamic-port-range 18001-18100
```

### Bước 5: Build/deploy program
```bash
pnpm run deploy:localnet
```

Hoặc:
```bash
cd contracts/expense_program
anchor build
anchor deploy --provider.cluster localnet
```

Lấy Program ID rồi cập nhật env:
```bash
solana address -k target/deploy/expense_program-keypair.json
```

### Bước 6: Chạy API
```bash
cargo run --manifest-path apps/api/Cargo.toml
```

### Bước 7: Chạy Web
```bash
pnpm --filter web dev
```

### Bước 8: Cấu hình Phantom
- Bật Testnet mode.
- Chọn **Solana Localnet**.
- Dùng đúng account active.

Airdrop:
```bash
export SOLANA_RPC_URL=http://127.0.0.1:8899
export WALLET=<PHANTOM_ACTIVE_PUBLIC_KEY>
SOLANA_URL="$SOLANA_RPC_URL" solana airdrop 5 "$WALLET"
SOLANA_URL="$SOLANA_RPC_URL" solana balance "$WALLET"
```

---

## 10) Troubleshooting thường gặp

- **`Wallet has 0 SOL`**
  - Ví chưa được nạp trên đúng cluster (localnet/devnet).

- **`Program ... is not deployed`**
  - `NEXT_PUBLIC_PROGRAM_ID` không khớp program vừa deploy.

- **`transaction not found at selected commitment`**
  - RPC/commitment chưa đồng bộ, thử `confirmed` trên local.

- **`AccountNotInitialized` với expense/program_config**
  - Dữ liệu localnet cũ hoặc DB stale metadata sau reset/redeploy.
  - Tạo lại dữ liệu category/expense trên state hiện tại.

- **`Category ID not found` khi create expense hybrid**
  - ID category nhập không tồn tại trong danh sách category đã commit.

- **Admin không approve/reject được**
  - Kiểm tra role trong DB + signer wallet + program config PDA đã init đúng.

---

## 11) Security & compliance checklist

- Không hardcode private key/secret.
- Endpoint ghi có idempotency.
- Verify signer tx trùng auth wallet theo flow yêu cầu.
- RBAC đúng vai trò (`user/admin/auditor`).
- Lưu audit logs đầy đủ với tx hash và metadata cần thiết.

---

## 12) Tài liệu liên quan

- [Architecture.md](./Architecture.md)
- [DataFlow.md](./DataFlow.md)
- [BusinessLogic.md](./BusinessLogic.md)
- [Security.md](./Security.md)
- [Workflow.md](./Workflow.md)
- [Diagrams.md](./Diagrams.md)
