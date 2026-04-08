# Workflow

## 1. Branching
- `main`: stable
- `feature/*`: phát triển tính năng
- `hotfix/*`: vá khẩn cấp

## 2. Pull request
- PR nhỏ, mục tiêu rõ.
- Bắt buộc điền test plan và impact.
- Không merge khi CI fail.

## 3. Role responsibilities
- Backend: API, DB, auth, indexer.
- Frontend: UX, data fetching, form validation.
- Contract: chương trình Anchor và test.
- Tester: test matrix + regression.
- DevOps: pipeline, container, release.

## 4. Frontend workflow demo (Account 2 user -> Account 1 admin)

Tài liệu này mô tả đúng flow đã demo trong folder `asset/`:
- Account 2 đăng nhập và tạo dữ liệu (category/expense)
- Account 1 (admin) đăng nhập để approve expense

### 4.1 Điều kiện trước khi demo
- Local stack chạy đủ: Solana validator, program đã deploy, API, Web.
- `NEXT_PUBLIC_HYBRID_ONCHAIN_ENABLED=true` trong `apps/web/.env.local`.
- 2 ví Phantom:
  - Account 2: user tạo dữ liệu.
  - Account 1: admin duyệt expense.
- Cả 2 ví có SOL trên localnet để ký tx.

### 4.2 Flow chi tiết theo UI
1. Account 2 mở web, bấm **Sign in with Wallet**.
2. Phantom hiện popup ký message login -> bấm **Confirm**.
3. Vào **Categories** -> tạo category mới.
4. Vào **Expenses** -> tạo expense thuộc category vừa tạo.
5. Xác nhận tx trong Phantom để commit create expense.
6. Kiểm tra expense mới ở trạng thái `pending`.
7. Logout Account 2.
8. Đăng nhập Account 1 (role admin).
9. Vào **Expenses** -> bấm **Approve** cho expense `pending`.
10. Kiểm tra expense đổi sang `approved` và dữ liệu tổng hợp cập nhật ở Dashboard/Reports.

### 4.3 API/DB evidence cần kiểm tra
- API requests phải xuất hiện trong Network tab:
  - `POST /api/v1/onchain/categories/commit`
  - `POST /api/v1/onchain/expenses/commit-create`
  - `POST /api/v1/onchain/expenses/:id/commit-status`
- DB columns phải được fill:
  - `categories.tx_hash`, `categories.onchain_category_pda`, `categories.onchain_slot`
  - `expenses_read_model.tx_hash`, `expenses_read_model.onchain_expense_pda`
  - `expenses_read_model.status_tx_hash`, `expenses_read_model.status_onchain_slot`

### 4.4 Lưu ý khi dùng Phantom trên localnet
- Cảnh báo `This feature is not supported when Solana Localnet is enabled` có thể xuất hiện trong Phantom, không ảnh hưởng flow ký/submit tx.
- Cảnh báo `Failed to simulate the results of this request` có thể xuất hiện ở popup sign/confirm trên localnet; chỉ xác nhận khi đúng domain `localhost:3000` và đúng hành động vừa thao tác.

## 5. Definition of done
- Code + test + docs + security checklist đều pass.
- Có evidence chạy local và CI.
