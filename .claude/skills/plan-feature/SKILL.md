---
name: plan-feature
description: Phân rã feature theo role (backend/frontend/contract/tester/devops) với acceptance criteria, risk và test plan.
---

# Plan Feature Skill

## Input
- Mô tả feature
- Ràng buộc business/security

## Output
1. Scope rõ ràng (in/out)
2. Task breakdown theo role
3. Acceptance criteria dạng checklist
4. Risk + mitigation
5. Test matrix (unit/integration/e2e/security)

## Rules
- Ưu tiên tái sử dụng module sẵn có.
- Không mở rộng scope ngoài yêu cầu.
- Mọi task ghi dữ liệu phải nêu idempotency + audit requirements.
