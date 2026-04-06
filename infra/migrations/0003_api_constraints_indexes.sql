CREATE INDEX IF NOT EXISTS idx_categories_owner_user_id ON categories(owner_user_id);
CREATE INDEX IF NOT EXISTS idx_expenses_owner_user_id ON expenses_read_model(owner_user_id);
CREATE INDEX IF NOT EXISTS idx_expenses_category_id ON expenses_read_model(category_id);
CREATE INDEX IF NOT EXISTS idx_expenses_occurred_at ON expenses_read_model(occurred_at);
CREATE INDEX IF NOT EXISTS idx_expenses_status ON expenses_read_model(status);

ALTER TABLE categories
  ADD CONSTRAINT uq_categories_owner_name UNIQUE (owner_user_id, name);
