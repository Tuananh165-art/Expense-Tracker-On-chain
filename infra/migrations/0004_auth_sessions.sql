CREATE TABLE IF NOT EXISTS auth_refresh_sessions (
  jti TEXT PRIMARY KEY,
  family_id TEXT NOT NULL,
  user_id UUID NOT NULL,
  expires_at TIMESTAMPTZ NOT NULL,
  used BOOLEAN NOT NULL DEFAULT FALSE,
  revoked BOOLEAN NOT NULL DEFAULT FALSE,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_auth_refresh_sessions_family_id ON auth_refresh_sessions(family_id);
CREATE INDEX IF NOT EXISTS idx_auth_refresh_sessions_user_id ON auth_refresh_sessions(user_id);

CREATE TABLE IF NOT EXISTS revoked_token_families (
  family_id TEXT PRIMARY KEY,
  revoked_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS revoked_access_tokens (
  jti TEXT PRIMARY KEY,
  revoked_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
