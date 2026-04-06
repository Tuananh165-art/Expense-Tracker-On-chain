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

## 4. Definition of done
- Code + test + docs + security checklist đều pass.
- Có evidence chạy local và CI.
