# API Examples — Categories, Expenses, Monthly Report

## 0) Prerequisites

```bash
# API base (localnet default)
export BASE_URL="http://127.0.0.1:8080"

# Access token lấy từ flow /api/v1/auth/challenge -> /api/v1/auth/verify
export ACCESS_TOKEN="<your_jwt_access_token>"
```

> Tất cả endpoint bên dưới cần header:
>
> `Authorization: Bearer $ACCESS_TOKEN`

---

## 1) Categories

### 1.1 Create Category

### cURL
```bash
curl -sS -X POST "$BASE_URL/api/v1/categories" \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Food"
  }'
```

### Input
```json
{
  "name": "Food"
}
```

### Output (200)
```json
{
  "id": "6bbffdb9-29fd-47f2-96ea-ae7b806c6af7",
  "owner_user_id": "dc6b1028-9d73-4ee6-ba17-63f2e73e45aa",
  "name": "Food",
  "created_at": "2026-04-06T09:20:11.998196+00:00"
}
```

### Negative output (400 - name rỗng)
```json
{
  "code": "bad_request",
  "message": "name is required"
}
```

### 1.2 List Categories

### cURL
```bash
curl -sS "$BASE_URL/api/v1/categories" \
  -H "Authorization: Bearer $ACCESS_TOKEN"
```

### Output (200)
```json
[
  {
    "id": "6bbffdb9-29fd-47f2-96ea-ae7b806c6af7",
    "owner_user_id": "dc6b1028-9d73-4ee6-ba17-63f2e73e45aa",
    "name": "Food",
    "created_at": "2026-04-06T09:20:11.998196+00:00"
  }
]
```

### Use case tests (Categories)

- Happy path:
  1. Create category `Food`.
  2. List categories.
  3. Verify category vừa tạo xuất hiện.
- Validation:
  - Create với `name="   "` -> expect `400 bad_request`.
- Auth:
  - Missing `Authorization` -> expect `401 unauthorized`.

---

## 2) Expenses

## Lưu ý quan trọng
`POST /api/v1/expenses` **bắt buộc** header `x-idempotency-key`.

### 2.1 Create Expense

### cURL
```bash
# Dùng category_id lấy từ bước create/list categories
export CATEGORY_ID="6bbffdb9-29fd-47f2-96ea-ae7b806c6af7"
export IDEMPOTENCY_KEY="expense-20260406-001"

curl -sS -X POST "$BASE_URL/api/v1/expenses" \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -H "x-idempotency-key: $IDEMPOTENCY_KEY" \
  -d '{
    "category_id": "'"$CATEGORY_ID"'",
    "amount_minor": 120000,
    "currency": "VND",
    "occurred_at": "2026-04-06T09:30:00Z"
  }'
```

### Input
```json
{
  "category_id": "6bbffdb9-29fd-47f2-96ea-ae7b806c6af7",
  "amount_minor": 120000,
  "currency": "VND",
  "occurred_at": "2026-04-06T09:30:00Z"
}
```

### Output (200)
```json
{
  "id": "9aeae7a5-f62f-42f2-af85-d98b460f96a6",
  "owner_user_id": "dc6b1028-9d73-4ee6-ba17-63f2e73e45aa",
  "category_id": "6bbffdb9-29fd-47f2-96ea-ae7b806c6af7",
  "amount_minor": 120000,
  "currency": "VND",
  "status": "pending",
  "tx_hash": null,
  "occurred_at": "2026-04-06T09:30:00+00:00",
  "created_at": "2026-04-06T09:30:12.211123+00:00"
}
```

### 2.2 Idempotency check (gọi lại cùng key)

Gọi lại đúng request trên với cùng `x-idempotency-key`, server trả lại **cùng response** (đặc biệt `id` giữ nguyên).

### 2.3 List Expenses

### cURL
```bash
curl -sS "$BASE_URL/api/v1/expenses" \
  -H "Authorization: Bearer $ACCESS_TOKEN"
```

### Output (200)
```json
[
  {
    "id": "9aeae7a5-f62f-42f2-af85-d98b460f96a6",
    "owner_user_id": "dc6b1028-9d73-4ee6-ba17-63f2e73e45aa",
    "category_id": "6bbffdb9-29fd-47f2-96ea-ae7b806c6af7",
    "amount_minor": 120000,
    "currency": "VND",
    "status": "pending",
    "tx_hash": null,
    "occurred_at": "2026-04-06T09:30:00+00:00",
    "created_at": "2026-04-06T09:30:12.211123+00:00"
  }
]
```

### Negative outputs

#### Missing idempotency key (400)
```json
{
  "code": "bad_request",
  "message": "x-idempotency-key header is required"
}
```

#### amount_minor <= 0 (400)
```json
{
  "code": "bad_request",
  "message": "amount_minor must be > 0"
}
```

#### invalid category_id (400)
```json
{
  "code": "bad_request",
  "message": "invalid category_id"
}
```

#### category không tồn tại (404)
```json
{
  "code": "not_found",
  "message": "category not found"
}
```

### Use case tests (Expenses)

- Happy path:
  1. Create expense với category hợp lệ.
  2. List expenses.
  3. Verify expense mới có trong list.
- Idempotency:
  1. POST 2 lần cùng body + cùng `x-idempotency-key`.
  2. Verify `id` response lần 1 == lần 2.
- Validation:
  - `amount_minor=0` -> `400`.
  - thiếu `x-idempotency-key` -> `400`.
- Auth:
  - thiếu JWT -> `401`.

---

## 3) Monthly Report

### 3.1 Get Monthly Report

### cURL
```bash
curl -sS "$BASE_URL/api/v1/reports/monthly" \
  -H "Authorization: Bearer $ACCESS_TOKEN"
```

### Output (200)
```json
{
  "total_amount_minor": 120000,
  "by_category": [
    {
      "category_id": "6bbffdb9-29fd-47f2-96ea-ae7b806c6af7",
      "total_amount_minor": 120000
    }
  ]
}
```

### Use case tests (Monthly Report)

- Happy path:
  1. Tạo nhiều expenses ở các category khác nhau.
  2. Gọi monthly report.
  3. Verify `total_amount_minor` = tổng toàn bộ expenses user.
  4. Verify `by_category` aggregate đúng theo từng `category_id`.
- Empty data:
  - User chưa có expense -> `total_amount_minor=0`, `by_category=[]`.
- Auth/Role:
  - thiếu JWT -> `401 unauthorized`.

---

## 4) SDK script example (TypeScript)

```ts
import { ExpenseApiClient } from "@expense/sdk";

const baseUrl = "http://127.0.0.1:8080";
const accessToken = process.env.ACCESS_TOKEN!;

const api = new ExpenseApiClient(baseUrl, () => accessToken);

async function main() {
  const category = await api.createCategory({ name: "Food" });

  const expense = await api.createExpense(
    {
      category_id: category.id,
      amount_minor: 120000,
      currency: "VND",
      occurred_at: new Date().toISOString(),
    },
    "expense-20260406-sdk-001"
  );

  const categories = await api.listCategories();
  const expenses = await api.listExpenses();
  const report = await api.monthlyReport();

  console.log({ category, expense, categories, expenses, report });
}

main().catch(console.error);
```

---

## 5) Quick smoke script (bash)

```bash
set -euo pipefail

BASE_URL="${BASE_URL:-http://127.0.0.1:8080}"
ACCESS_TOKEN="${ACCESS_TOKEN:?ACCESS_TOKEN is required}"
IDEMPOTENCY_KEY="smoke-expense-001"

CATEGORY_ID=$(curl -sS -X POST "$BASE_URL/api/v1/categories" \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name":"Smoke"}' | python -c 'import sys,json; print(json.load(sys.stdin)["id"])')

echo "CATEGORY_ID=$CATEGORY_ID"

curl -sS -X POST "$BASE_URL/api/v1/expenses" \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -H "x-idempotency-key: $IDEMPOTENCY_KEY" \
  -d "{\"category_id\":\"$CATEGORY_ID\",\"amount_minor\":50000,\"currency\":\"VND\"}" >/dev/null

curl -sS "$BASE_URL/api/v1/reports/monthly" \
  -H "Authorization: Bearer $ACCESS_TOKEN"

echo "\nSmoke OK"
```
