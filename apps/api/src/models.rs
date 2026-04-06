use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Admin,
    Auditor,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub wallet_address: String,
    pub role: Role,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: Uuid,
    pub owner_user_id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExpenseStatus {
    Pending,
    Approved,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Expense {
    pub id: Uuid,
    pub owner_user_id: Uuid,
    pub category_id: Uuid,
    pub amount_minor: i64,
    pub currency: String,
    pub status: ExpenseStatus,
    pub tx_hash: Option<String>,
    pub occurred_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    pub id: Uuid,
    pub actor_wallet: String,
    pub action: String,
    pub target_id: Option<String>,
    pub tx_hash: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct AuthChallenge {
    pub challenge_id: Uuid,
    pub wallet_address: String,
    pub nonce: String,
    pub message: String,
    pub expires_at: DateTime<Utc>,
    pub used: bool,
}

#[derive(Debug, Clone)]
pub struct RefreshTokenRecord {
    pub jti: String,
    pub family_id: String,
    pub user_id: Uuid,
    pub expires_at: DateTime<Utc>,
    pub used: bool,
    pub revoked: bool,
}
