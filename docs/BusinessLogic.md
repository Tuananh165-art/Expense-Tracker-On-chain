# Business Logic

## 1. Aggregate chính
- UserProfile
- Category
- Expense

## 2. Expense lifecycle
`draft/pending -> approved/rejected -> archived(optional)`

## 3. Rules
1. Mỗi expense thuộc về 1 owner wallet.
2. Amount > 0 và không vượt ngưỡng hệ thống.
3. Category phải thuộc owner hoặc global policy cho phép.
4. Chỉ owner/admin được cập nhật trạng thái tùy role policy.
5. Mỗi request ghi phải có idempotency key.

## 4. Transparency model
- Bản ghi on-chain dùng cho chứng minh tính toàn vẹn.
- Read model off-chain tối ưu phân tích và UI.
- Tx hash luôn liên kết với record ở DB.

## 5. Enterprise controls
- RBAC: user/admin/auditor.
- Audit log cho create/update/delete nghiệp vụ trọng yếu.
- Retention policy cho dữ liệu log theo compliance.
