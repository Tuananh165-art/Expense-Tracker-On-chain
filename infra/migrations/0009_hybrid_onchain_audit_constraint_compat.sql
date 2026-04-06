ALTER TABLE tx_audit_logs
  DROP CONSTRAINT IF EXISTS chk_audit_hybrid_actions_require_onchain_fields;

ALTER TABLE tx_audit_logs
  ADD CONSTRAINT chk_audit_hybrid_actions_require_onchain_fields
  CHECK (
    action NOT IN ('category.create', 'expense.create', 'expense.approve', 'expense.reject')
    OR onchain_verified = FALSE
    OR (
      tx_hash IS NOT NULL
      AND onchain_verified = TRUE
      AND onchain_program_id IS NOT NULL
      AND onchain_slot IS NOT NULL
    )
  ) NOT VALID;
