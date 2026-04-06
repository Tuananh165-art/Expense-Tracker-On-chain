# Project TODO (Role-based)

## Phase A — Foundation
- [ ] Khởi tạo monorepo và conventions
- [ ] Dựng local infra (PostgreSQL, Redis)
- [ ] Thiết lập CI cơ bản (lint/test/build)

## Phase B — Core Domain
### Contract role
- [ ] Implement `init_user_profile`
- [ ] Implement `create_category`
- [ ] Implement `create_expense`
- [ ] Implement `update_expense_status`
- [ ] Viết test positive + negative

### Backend role
- [ ] Wallet auth (signature verify -> JWT)
- [ ] API v1 users/categories/expenses/reports
- [ ] Migration + read model + audit logs
- [ ] Idempotency cho endpoint ghi

## Phase C — Frontend
- [ ] Wallet connect + session bootstrap
- [ ] Expense CRUD + filter/search
- [ ] Dashboard theo tháng/category
- [ ] Timeline trạng thái transaction

## Phase D — Hardening
### Tester role
- [ ] Unit + integration + e2e smoke
- [ ] Security test checklist API + contract

### DevOps role
- [ ] Hoàn thiện GitHub Actions
- [ ] Image build + release tagging
- [ ] Runbook deploy và incident handling
