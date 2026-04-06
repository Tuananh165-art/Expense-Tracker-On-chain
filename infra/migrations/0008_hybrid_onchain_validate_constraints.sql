ALTER TABLE categories
  VALIDATE CONSTRAINT chk_categories_hybrid_commit_fields;

ALTER TABLE expenses_read_model
  VALIDATE CONSTRAINT chk_expenses_hybrid_create_fields;

ALTER TABLE expenses_read_model
  VALIDATE CONSTRAINT chk_expenses_hybrid_status_fields;

ALTER TABLE tx_audit_logs
  VALIDATE CONSTRAINT chk_audit_hybrid_actions_require_onchain_fields;
