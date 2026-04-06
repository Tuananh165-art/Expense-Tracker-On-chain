# Data Flow

## 1) Authentication flow
1. FE yêu cầu nonce/challenge từ API.
2. User ký challenge bằng wallet.
3. FE gửi signature + public key tới API.
4. API verify signature -> cấp JWT ngắn hạn.

## 2) Create expense flow
1. FE gửi request create expense (kèm idempotency key) tới API.
2. API validate input + RBAC.
3. API tạo/submit transaction tới Solana program.
4. Program emit event.
5. API/indexer consume event, cập nhật `expenses_read_model` trong PostgreSQL.
6. FE poll/subscription để cập nhật trạng thái transaction.

## 3) Reporting flow
1. FE gọi API `/reports`.
2. API query read model trong PostgreSQL.
3. Trả về summary theo tháng/category/status.

## 4) Audit flow
1. Mọi thao tác quan trọng ghi `tx_audit_logs`.
2. Admin/Auditor truy xuất lịch sử thao tác + chain tx hash.
