---
name: review-security
description: Rà soát security cho API + smart contract theo checklist OWASP và blockchain-specific controls.
---

# Review Security Skill

## Checklist API
- Authn/authz đúng vai trò?
- Input validation có chặt ở boundary?
- Rate limit/risk endpoints?
- Idempotency + anti-replay?
- Audit logging đủ thông tin?

## Checklist Contract
- Signer checks đủ và đúng actor?
- PDA seeds có collision risk?
- Overflow/underflow đã xử lý?
- Account ownership + mutability constraints đúng?
- Negative tests cho unauthorized path?

## Output format
- Findings (Critical/High/Medium/Low)
- Evidence (file:path:line)
- Recommendation fix ngắn gọn
