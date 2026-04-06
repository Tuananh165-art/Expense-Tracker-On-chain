---
name: generate-test-matrix
description: Sinh ma trận test chuẩn enterprise cho feature: unit, integration, e2e, security, regression.
---

# Generate Test Matrix Skill

## Output sections
1. Unit tests
2. Integration tests
3. E2E smoke tests
4. Security negative tests
5. Regression tests

## Rules
- Mỗi test case có: precondition, input, expected, cleanup.
- Bắt buộc include edge cases amount/status/authorization.
- Mapping test case -> requirement ID nếu có.
