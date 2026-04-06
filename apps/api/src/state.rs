use crate::{config::AppConfig, models::*};
use chrono::{Duration, Utc};
use serde_json::Value;
use sqlx::PgPool;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub pg_pool: Option<PgPool>,
    pub users: Arc<RwLock<HashMap<Uuid, User>>>,
    pub users_by_wallet: Arc<RwLock<HashMap<String, Uuid>>>,
    pub categories: Arc<RwLock<HashMap<Uuid, Category>>>,
    pub expenses: Arc<RwLock<HashMap<Uuid, Expense>>>,
    pub auth_challenges: Arc<RwLock<HashMap<Uuid, AuthChallenge>>>,
    pub refresh_tokens: Arc<RwLock<HashMap<String, RefreshTokenRecord>>>,
    pub revoked_token_families: Arc<RwLock<HashMap<String, chrono::DateTime<Utc>>>>,
    pub revoked_access_jtis: Arc<RwLock<HashMap<String, chrono::DateTime<Utc>>>>,
    pub idempotency: Arc<RwLock<HashMap<String, Value>>>,
    pub audit_logs: Arc<RwLock<Vec<AuditLog>>>,
}

impl AppState {
    pub fn new(config: AppConfig, pg_pool: Option<PgPool>) -> Self {
        Self {
            config,
            pg_pool,
            users: Arc::new(RwLock::new(HashMap::new())),
            users_by_wallet: Arc::new(RwLock::new(HashMap::new())),
            categories: Arc::new(RwLock::new(HashMap::new())),
            expenses: Arc::new(RwLock::new(HashMap::new())),
            auth_challenges: Arc::new(RwLock::new(HashMap::new())),
            refresh_tokens: Arc::new(RwLock::new(HashMap::new())),
            revoked_token_families: Arc::new(RwLock::new(HashMap::new())),
            revoked_access_jtis: Arc::new(RwLock::new(HashMap::new())),
            idempotency: Arc::new(RwLock::new(HashMap::new())),
            audit_logs: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn cleanup_expired(&self) {
        let now = Utc::now();

        {
            let mut challenges = self.auth_challenges.write().await;
            let used_retention = Duration::seconds(self.config.used_challenge_retention_seconds);
            challenges.retain(|_, c| {
                if c.expires_at < now {
                    return false;
                }
                if c.used && c.expires_at + used_retention < now {
                    return false;
                }
                true
            });
        }

        {
            let mut refresh = self.refresh_tokens.write().await;
            let used_retention = Duration::seconds(self.config.used_refresh_retention_seconds);
            refresh.retain(|_, r| {
                if r.expires_at < now {
                    return false;
                }
                if (r.used || r.revoked) && r.expires_at + used_retention < now {
                    return false;
                }
                true
            });
        }

        {
            let mut revoked_access = self.revoked_access_jtis.write().await;
            let ttl = Duration::seconds(self.config.revoked_access_retention_seconds);
            revoked_access.retain(|_, at| *at + ttl >= now);
        }

        {
            let mut revoked_family = self.revoked_token_families.write().await;
            let ttl = Duration::seconds(self.config.revoked_family_retention_seconds);
            revoked_family.retain(|_, at| *at + ttl >= now);
        }
    }
}
