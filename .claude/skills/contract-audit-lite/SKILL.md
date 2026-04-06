---
name: contract-audit-lite
description: Audit nhanh smart contract Anchor tập trung access control, signer, PDA, replay/overflow.
---

# Contract Audit Lite Skill

## Steps
1. Kiểm tra instruction handlers và constraints.
2. Kiểm tra ownership/signer/mut constraints.
3. Kiểm tra arithmetic bounds + state transitions.
4. Đối chiếu test coverage cho unauthorized/invalid account.

## Output
- Risk summary
- Issue list + impacted instruction
- Suggested patch strategy
