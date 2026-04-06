# Architecture

## 1. Mục tiêu
Xây dựng hệ thống Expense Tracker minh bạch:
- Transaction trọng yếu ghi on-chain để đảm bảo bất biến và khả năng kiểm toán.
- Truy vấn nhanh và báo cáo doanh nghiệp chạy off-chain.

## 2. C4-lite

### Context
- **User**: cá nhân theo dõi chi tiêu
- **Auditor/Admin**: kiểm tra dữ liệu và tuân thủ
- **System**: Expense Tracker On-chain

### Containers
1. **Web App (Next.js)**
2. **API Service (Rust/Axum)**
3. **Solana Program (Anchor/Rust)**
4. **PostgreSQL** (read model/reporting)
5. **Redis** (cache/rate-limit/session hỗ trợ)

## 3. Trust boundaries
- Boundary A: Client browser <-> API
- Boundary B: API <-> Blockchain RPC
- Boundary C: API <-> DB/Redis

## 4. Data ownership
- On-chain: immutable transaction footprint, status nghiệp vụ quan trọng.
- Off-chain: profile, indexing, analytics, query optimized, RBAC metadata.

## 5. NFR
- Security-first: signature verify, RBAC, audit logging.
- Observability: structured logs, healthcheck, metrics-ready.
- Scalability: tách read/write model, có khả năng scale ngang API.

## 6. Recommended technology
- FE: Next.js + TypeScript + TanStack Query + RHF + Zod
- BE: Rust + Axum + SQLx + Tracing + JWT
- Contract: Rust + Anchor
- Data: PostgreSQL + Redis
- CI/CD: GitHub Actions + Docker
