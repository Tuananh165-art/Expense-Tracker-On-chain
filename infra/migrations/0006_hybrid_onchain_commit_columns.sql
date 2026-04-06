ALTER TABLE categories
  ADD COLUMN IF NOT EXISTS tx_hash TEXT,
  ADD COLUMN IF NOT EXISTS onchain_category_pda TEXT,
  ADD COLUMN IF NOT EXISTS onchain_slot BIGINT,
  ADD COLUMN IF NOT EXISTS onchain_committed_at TIMESTAMPTZ;

ALTER TABLE expenses_read_model
  ADD COLUMN IF NOT EXISTS onchain_expense_pda TEXT,
  ADD COLUMN IF NOT EXISTS status_tx_hash TEXT,
  ADD COLUMN IF NOT EXISTS status_onchain_slot BIGINT,
  ADD COLUMN IF NOT EXISTS status_committed_at TIMESTAMPTZ;

ALTER TABLE tx_audit_logs
  ADD COLUMN IF NOT EXISTS onchain_verified BOOLEAN NOT NULL DEFAULT FALSE,
  ADD COLUMN IF NOT EXISTS onchain_program_id TEXT,
  ADD COLUMN IF NOT EXISTS onchain_slot BIGINT;

CREATE UNIQUE INDEX IF NOT EXISTS uq_categories_tx_hash
  ON categories(tx_hash)
  WHERE tx_hash IS NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS uq_expenses_create_tx_hash
  ON expenses_read_model(tx_hash)
  WHERE tx_hash IS NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS uq_expenses_status_tx_hash
  ON expenses_read_model(status_tx_hash)
  WHERE status_tx_hash IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_categories_onchain_category_pda
  ON categories(onchain_category_pda);

CREATE INDEX IF NOT EXISTS idx_expenses_onchain_expense_pda
  ON expenses_read_model(onchain_expense_pda);

CREATE INDEX IF NOT EXISTS idx_audit_logs_tx_hash
  ON tx_audit_logs(tx_hash);
