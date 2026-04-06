# CLAUDE.md — Project Operating Guide (Expense Tracker On-chain)

> BẮT BUỘC: Mọi agent (main/sub) phải đọc file này trước khi propose, code, review hoặc chạy lệnh.

## 1) Project identity
- Domain: Expense Tracker cá nhân minh bạch.
- Hybrid architecture: on-chain (truth/audit) + off-chain (query/analytics/enterprise controls).
- Mục tiêu ưu tiên: correctness, security, auditability, maintainability.

## 2) Engineering rules
1. Security-first cho API và smart contract.
2. Không hardcode secret/private key.
3. Mọi endpoint ghi phải có idempotency.
4. Mọi thay đổi domain phải cập nhật docs liên quan.
5. Không merge khi thiếu test plan hoặc CI fail.

## 3) Role-based breakdown

### Backend role
- Rust + Axum + SQLx.
- Chịu trách nhiệm auth (wallet signature + JWT), RBAC, read model, audit logs.

### Frontend role
- Next.js + TypeScript.
- Chịu trách nhiệm wallet UX, form validation, dashboard/reporting.

### Contract role
- Rust + Anchor.
- Chịu trách nhiệm state accounts, instructions, event emission, invariant checks.

### Tester role
- Unit + integration + e2e smoke.
- Security negative tests cho API và contract.

### DevOps role
- Docker, CI/CD, quality gates, release checklist.

## 4) Coding standards
- Ưu tiên code rõ ràng hơn clever code.
- Boundary validation chặt, không duplicate validate ở tầng dưới khi không cần.
- Logging có correlation-id.
- Commit theo conventional commits (nếu có git).

## 5) Required reviews
- Functional review: đúng business flow.
- Security review: authz/authn/input/idempotency/replay.
- Data review: schema migration backward-safe.
- Docs review: README + docs/*.md đồng bộ.

## 6) No-go actions
- Không bypass security checks để pass nhanh.
- Không dùng mock thay thế integration test critical path nếu có hạ tầng thật.
- Không đưa key/secret vào repo.

## 7) Agent execution protocol
1. Đọc CLAUDE.md.
2. Cập nhật todo theo task.
3. Propose plan ngắn gọn nếu task lớn.
4. Implement theo từng role và verify.
5. Update docs/checklist.

## 8) Done criteria
- Code chạy local.
- Test pass theo scope.
- Security checklist pass.
- Docs cập nhật đầy đủ.
