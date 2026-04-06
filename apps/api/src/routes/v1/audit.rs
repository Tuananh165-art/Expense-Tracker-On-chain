use axum::{
    extract::{Query, State},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;

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

    if state.config.expenses_pg_enabled {
        let pool = state
            .pg_pool
            .as_ref()
            .ok_or_else(|| AppError::internal("postgres pool is not initialized"))?;

        let rows = sqlx::query(
            "SELECT id, actor_wallet, action, target_id, tx_hash, metadata, created_at\
             FROM tx_audit_logs\
             WHERE ($1::text IS NULL OR action = $1)\
               AND ($2::text IS NULL OR actor_wallet = $2)\
               AND ($3::text IS NULL OR target_id = $3)\
               AND ($4::timestamptz IS NULL OR created_at >= $4)\
               AND ($5::timestamptz IS NULL OR created_at <= $5)\
             ORDER BY created_at DESC\
             LIMIT $6",
        )
        .bind(query.action.clone())
        .bind(query.actor_wallet.clone())
        .bind(query.target_id.clone())
        .bind(from)
        .bind(to)
        .bind(limit as i64)
        .fetch_all(pool)
        .await
        .map_err(|_| AppError::internal("failed to list audit logs"))?;

        let logs = rows
            .into_iter()
            .map(|row| {
                Ok(AuditLogItem {
                    id: row
                        .try_get::<uuid::Uuid, _>("id")
                        .map_err(|_| AppError::internal("invalid audit row"))?
                        .to_string(),
                    actor_wallet: row
                        .try_get("actor_wallet")
                        .map_err(|_| AppError::internal("invalid audit row"))?,
                    action: row
                        .try_get("action")
                        .map_err(|_| AppError::internal("invalid audit row"))?,
                    target_id: row
                        .try_get("target_id")
                        .map_err(|_| AppError::internal("invalid audit row"))?,
                    tx_hash: row
                        .try_get("tx_hash")
                        .map_err(|_| AppError::internal("invalid audit row"))?,
                    metadata: row
                        .try_get("metadata")
                        .map_err(|_| AppError::internal("invalid audit row"))?,
                    created_at: row
                        .try_get::<DateTime<Utc>, _>("created_at")
                        .map_err(|_| AppError::internal("invalid audit row"))?
                        .to_rfc3339(),
                })
            })
            .collect::<AppResult<Vec<_>>>()?;

        return Ok(Json(logs));
    }

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
