ALTER TABLE expenses_read_model
  ADD COLUMN IF NOT EXISTS amount_minor BIGINT;

UPDATE expenses_read_model
SET amount_minor = ROUND(amount * 100)::BIGINT
WHERE amount_minor IS NULL;

ALTER TABLE expenses_read_model
  ALTER COLUMN amount_minor SET NOT NULL;

ALTER TABLE expenses_read_model
  DROP CONSTRAINT IF EXISTS chk_expenses_amount_minor_positive;

ALTER TABLE expenses_read_model
  ADD CONSTRAINT chk_expenses_amount_minor_positive CHECK (amount_minor > 0);

ALTER TABLE expenses_read_model
  DROP CONSTRAINT IF EXISTS chk_expenses_status;

ALTER TABLE expenses_read_model
  ADD CONSTRAINT chk_expenses_status CHECK (status IN ('pending', 'approved', 'rejected'));

ALTER TABLE tx_audit_logs
  ALTER COLUMN metadata SET DEFAULT '{}'::jsonb;

UPDATE tx_audit_logs
SET metadata = '{}'::jsonb
WHERE metadata IS NULL;

ALTER TABLE tx_audit_logs
  ALTER COLUMN metadata SET NOT NULL;

ALTER TABLE idempotency_keys
  ALTER COLUMN request_hash DROP NOT NULL;

CREATE INDEX IF NOT EXISTS idx_expenses_created_at ON expenses_read_model(created_at);
CREATE INDEX IF NOT EXISTS idx_audit_logs_target_id ON tx_audit_logs(target_id);
CREATE INDEX IF NOT EXISTS idx_audit_logs_created_at ON tx_audit_logs(created_at DESC);
