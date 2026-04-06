# Security Baseline

## 1. API security
- Validate input với schema chặt (Zod/serde validation boundary).
- JWT expiry ngắn + rotate secret theo môi trường.
- Rate limit endpoint auth và endpoint ghi.
- Idempotency key chống replay ở tầng API.

## 2. Smart contract security
- PDA seeds deterministic, không mơ hồ.
- Signer check rõ cho mọi instruction thay đổi state.
- Validation amount/range/owner ở entrypoint.
- Negative tests cho unauthorized, invalid account, overflow.

## 3. Data security
- Không lưu private key.
- Mã hóa secrets trong secret manager khi production.
- Principle of least privilege cho DB user.

## 4. Auditability
- Ghi correlation-id cho request chain/API.
- Lưu tx hash + actor + action + timestamp.

## 5. Checklist trước release
- [ ] API lint/test pass
- [ ] Contract tests pass
- [ ] Dependency audit pass
- [ ] Không hardcode secrets
- [ ] Docs cập nhật đầy đủ
