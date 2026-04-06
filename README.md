# Expense Tracker On-chain

Ứng dụng quản lý chi tiêu cá nhân theo hướng **minh bạch & kiểm toán được**:
- **On-chain (Solana/Anchor)** cho phần dữ liệu/trạng thái cần tính bất biến.
- **Off-chain (Rust/Axum)** cho API, auth, RBAC, báo cáo, tốc độ truy vấn.
- **Frontend (Next.js)** cho trải nghiệm dashboard hiện đại.

---

## 1) Bài toán dự án giải quyết

### Vấn đề
- App chi tiêu truyền thống dễ thiếu minh bạch lịch sử thay đổi.
- Khó audit khi cần đối soát giao dịch quan trọng.
- UX blockchain thường khó dùng với người dùng phổ thông.

### Solution của project
- Dùng mô hình **hybrid on-chain + off-chain**:
  - On-chain lưu/đối soát các trạng thái quan trọng.
  - Off-chain phục vụ read model, tổng hợp báo cáo, phân quyền.
- Đăng nhập ví Solana 1-click (challenge/sign/verify) để cấp JWT phiên làm việc.
- Có audit log + idempotency cho API ghi.

---

## 2) Tính năng hiện có

- Wallet auth (Solana): connect + sign message + verify JWT.
- Categories: tạo và liệt kê danh mục chi tiêu.
- Expenses: tạo expense, hỗ trợ `x-idempotency-key`.
- Monthly Report: tổng chi tiêu và tổng theo category.
- Role-based access: `user`, `admin`, `auditor`.

---

## 3) Kiến trúc & cấu trúc repo

- `contracts/expense_program`: Anchor smart contract (Rust)
- `apps/api`: Backend API (Axum)
- `apps/web`: Frontend dashboard (Next.js)
- `scripts/`: script chạy local/dev/test/deploy
- `docs/`: tài liệu kiến trúc, bảo mật, workflow

Xem thêm: [docs/Architecture.md](docs/Architecture.md)

---

## 4) Tech stack

- **Smart contract:** Rust, Anchor, Solana localnet/devnet
- **Backend:** Rust, Axum, Tokio, JWT, tower-http CORS
- **Frontend:** Next.js 15, React 18, TypeScript, TanStack Query
- **UI/UX:** TailwindCSS, shadcn-style components, Framer Motion
- **Tooling:** pnpm workspace, bash scripts

---

## 5) Yêu cầu môi trường

- Node.js >= 20
- pnpm >= 9
- Rust stable + cargo
- Solana CLI + Anchor CLI (khi chạy contract)
- Khuyến nghị chạy validator trên **WSL** nếu Windows gặp lỗi quyền truy cập

---

## 6) Cài đặt nhanh

```bash
pnpm install
```

Copy `.env.example` -> `.env`, rồi điền các biến chính:
- `SOLANA_RPC_URL`
- `PROGRAM_ID`
- `NEXT_PUBLIC_PROGRAM_ID`
- `NEXT_PUBLIC_SOLANA_CLUSTER`
- `JWT_SECRET`

---

## 7) Hướng dẫn chạy local (chuẩn đang dùng)

### Bước 1 — Chạy Solana local validator (terminal WSL #1)

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

Khi thấy:
- `JSON RPC URL: http://127.0.0.1:8899`
- slot tăng liên tục (`Processed/Confirmed/Finalized`)
=> local blockchain đang chạy OK.

### Bước 2 — Deploy contract localnet (terminal WSL #2)

```bash
pnpm run deploy:localnet
```

Hoặc thủ công:

```bash
cd contracts/expense_program
anchor build --no-idl
anchor deploy --provider.cluster localnet
```

Lấy program id:

```bash
solana address -k contracts/expense_program/target/deploy/expense_program-keypair.json
```

Update `.env`:

```env
SOLANA_RPC_URL=http://127.0.0.1:8899
PROGRAM_ID=<PROGRAM_ID_LOCALNET>
NEXT_PUBLIC_PROGRAM_ID=<PROGRAM_ID_LOCALNET>
NEXT_PUBLIC_SOLANA_CLUSTER=localnet
```

### Bước 3 — Chạy API (terminal WSL #3)

```bash
cargo run --manifest-path apps/api/Cargo.toml
```

Expected log:

```text
API listening on 0.0.0.0:8080
```

### Bước 4 — Chạy Web (terminal WSL #4)

```bash
pnpm --filter web dev
```

Expected log:

```text
Local: http://localhost:3000
```

---

## 8) Quy trình sử dụng nhanh

1. Mở `http://localhost:3000`
2. Connect wallet -> Sign In With Wallet
3. Tạo category
4. Tạo expense (đúng category UUID)
5. Kiểm tra Monthly Report cập nhật tổng

---

## 9) Các vấn đề đã gặp & cách xử lý

- **Devnet faucet rate-limit / thiếu SOL**
  - Chuyển sang **localnet** để dev miễn phí, ổn định.
- **Windows validator lỗi quyền truy cập (Access denied)**
  - Chạy validator qua **WSL**.
- **`Failed to fetch` khi create expense**
  - Bổ sung CORS header `x-idempotency-key` ở API.
- **Hydration mismatch (`Authenticated: No/Yes`)**
  - Đồng bộ `isAuthed` sau mount bằng `useEffect`, tránh lệch SSR/client.

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
```

---

## 11) Security baseline

- Verify wallet signature ở backend
- JWT access token theo session
- RBAC theo role
- Idempotency cho endpoint ghi
- Audit log cho thao tác quan trọng

Xem thêm: [docs/Security.md](docs/Security.md)
