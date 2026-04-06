use axum::{extract::{Query, State}, Json};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    auth::AuthUser,
    authz::require_admin_or_auditor,
    error::{AppError, AppResult},
    state::AppState,
};

#[derive(Deserialize)]
pub struct AuditLogsQuery {
    pub action: Option<String>,
    pub actor_wallet: Option<String>,
    pub target_id: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Serialize)]
pub struct AuditLogItem {
    pub id: String,
    pub actor_wallet: String,
    pub action: String,
    pub target_id: Option<String>,
    pub tx_hash: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: String,
}

pub async fn list_logs(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<AuditLogsQuery>,
) -> AppResult<Json<Vec<AuditLogItem>>> {
    require_admin_or_auditor(&auth)?;

    let from = match query.from.as_deref() {
        Some(v) => Some(
            DateTime::parse_from_rfc3339(v)
                .map_err(|_| AppError::bad_request("invalid from datetime"))?
                .with_timezone(&Utc),
        ),
        None => None,
    };

    let to = match query.to.as_deref() {
        Some(v) => Some(
            DateTime::parse_from_rfc3339(v)
                .map_err(|_| AppError::bad_request("invalid to datetime"))?
                .with_timezone(&Utc),
        ),
        None => None,
    };

    let limit = query.limit.unwrap_or(50).min(200);

    let logs = state.audit_logs.read().await;
    let mut rows = logs
        .iter()
        .filter(|x| {
            if let Some(ref a) = query.action {
                if &x.action != a {
                    return false;
                }
            }
            if let Some(ref w) = query.actor_wallet {
                if &x.actor_wallet != w {
                    return false;
                }
            }
            if let Some(ref t) = query.target_id {
                if x.target_id.as_ref() != Some(t) {
                    return false;
                }
            }
            if let Some(f) = from {
                if x.created_at < f {
                    return false;
                }
            }
            if let Some(t) = to {
                if x.created_at > t {
                    return false;
                }
            }
            true
        })
        .map(|x| AuditLogItem {
            id: x.id.to_string(),
            actor_wallet: x.actor_wallet.clone(),
            action: x.action.clone(),
            target_id: x.target_id.clone(),
            tx_hash: x.tx_hash.clone(),
            metadata: x.metadata.clone(),
            created_at: x.created_at.to_rfc3339(),
        })
        .collect::<Vec<_>>();

    rows.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    rows.truncate(limit);

    Ok(Json(rows))
}
