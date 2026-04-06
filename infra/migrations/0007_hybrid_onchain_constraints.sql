ALTER TABLE categories
  DROP CONSTRAINT IF EXISTS chk_categories_hybrid_commit_fields;

ALTER TABLE categories
  ADD CONSTRAINT chk_categories_hybrid_commit_fields
  CHECK (
    tx_hash IS NULL
    OR (
      tx_hash IS NOT NULL
      AND onchain_category_pda IS NOT NULL
      AND onchain_slot IS NOT NULL
      AND onchain_committed_at IS NOT NULL
    )
  ) NOT VALID;

ALTER TABLE expenses_read_model
  DROP CONSTRAINT IF EXISTS chk_expenses_hybrid_create_fields;

ALTER TABLE expenses_read_model
  ADD CONSTRAINT chk_expenses_hybrid_create_fields
  CHECK (
    tx_hash IS NULL
    OR (
      tx_hash IS NOT NULL
      AND onchain_expense_pda IS NOT NULL
    )
  ) NOT VALID;

ALTER TABLE expenses_read_model
  DROP CONSTRAINT IF EXISTS chk_expenses_hybrid_status_fields;

ALTER TABLE expenses_read_model
  ADD CONSTRAINT chk_expenses_hybrid_status_fields
  CHECK (
    status_tx_hash IS NULL
    OR (
      status_tx_hash IS NOT NULL
      AND status IN ('approved', 'rejected')
      AND status_onchain_slot IS NOT NULL
      AND status_committed_at IS NOT NULL
    )
  ) NOT VALID;

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
