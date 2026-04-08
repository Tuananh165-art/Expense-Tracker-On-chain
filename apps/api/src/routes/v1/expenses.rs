use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::Row;
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    authz::{require_admin, require_user_admin_or_auditor, require_user_or_admin},
    error::{AppError, AppResult},
    models::{AuditLog, Expense, ExpenseStatus, Role},
    state::AppState,
};

#[derive(Deserialize)]
pub struct CreateExpenseRequest {
    pub category_id: String,
    pub amount_minor: i64,
    pub currency: String,
    pub occurred_at: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateExpenseStatusRequest {
    pub status: ExpenseStatus,
    pub reason: Option<String>,
}

#[derive(Deserialize)]
pub struct ExpenseHistoryQuery {
    pub from: Option<String>,
    pub to: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Deserialize)]
pub struct SearchExpensesQuery {
    pub status: Option<String>,
    pub category_id: Option<String>,
    pub currency: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub q: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ExpenseResponse {
    pub id: String,
    pub owner_user_id: String,
    pub category_id: String,
    pub amount_minor: i64,
    pub currency: String,
    pub status: ExpenseStatus,
    pub tx_hash: Option<String>,
    pub occurred_at: String,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct ExpenseHistoryItem {
    pub id: String,
    pub actor_wallet: String,
    pub action: String,
    pub target_id: Option<String>,
    pub tx_hash: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct SearchExpensesResponse {
    pub items: Vec<ExpenseResponse>,
    pub total: i64,
    pub limit: usize,
    pub offset: usize,
    pub has_more: bool,
}

fn status_to_db(status: &ExpenseStatus) -> &'static str {
    match status {
        ExpenseStatus::Pending => "pending",
        ExpenseStatus::Approved => "approved",
        ExpenseStatus::Rejected => "rejected",
    }
}

fn status_from_db(value: &str) -> AppResult<ExpenseStatus> {
    match value {
        "pending" => Ok(ExpenseStatus::Pending),
        "approved" => Ok(ExpenseStatus::Approved),
        "rejected" => Ok(ExpenseStatus::Rejected),
        _ => Err(AppError::internal("invalid expense status in database")),
    }
}

pub async fn create(
    State(state): State<AppState>,
    auth: AuthUser,
    headers: HeaderMap,
    Json(payload): Json<CreateExpenseRequest>,
) -> AppResult<Json<ExpenseResponse>> {
    require_user_or_admin(&auth)?;
    if payload.amount_minor <= 0 {
        return Err(AppError::bad_request("amount_minor must be > 0"));
    }

    let category_id = payload
        .category_id
        .trim()
        .parse::<Uuid>()
        .map_err(|_| AppError::bad_request("invalid category_id"))?;

    if state.config.expenses_pg_enabled {
        return create_pg(&state, &auth, &headers, &payload, category_id).await;
    }

    create_in_memory(&state, &auth, &headers, &payload, category_id).await
}

async fn create_in_memory(
    state: &AppState,
    auth: &AuthUser,
    headers: &HeaderMap,
    payload: &CreateExpenseRequest,
    category_id: Uuid,
) -> AppResult<Json<ExpenseResponse>> {
    let idempotency_key = headers
        .get("x-idempotency-key")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::bad_request("x-idempotency-key header is required"))?
        .to_string();

    if let Some(cached) = state
        .idempotency
        .read()
        .await
        .get(&idempotency_key)
        .cloned()
    {
        let parsed: ExpenseResponse = serde_json::from_value(cached)
            .map_err(|_| AppError::internal("failed to decode cached response"))?;
        return Ok(Json(parsed));
    }

    let occurred_at = payload
        .occurred_at
        .as_deref()
        .and_then(|v| chrono::DateTime::parse_from_rfc3339(v).ok())
        .map(|v| v.with_timezone(&Utc))
        .unwrap_or_else(Utc::now);

    let expense = Expense {
        id: Uuid::new_v4(),
        owner_user_id: auth.user_id,
        category_id,
        amount_minor: payload.amount_minor,
        currency: payload.currency.clone(),
        status: ExpenseStatus::Pending,
        tx_hash: None,
        occurred_at,
        created_at: Utc::now(),
    };

    state
        .expenses
        .write()
        .await
        .insert(expense.id, expense.clone());

    let response = ExpenseResponse {
        id: expense.id.to_string(),
        owner_user_id: expense.owner_user_id.to_string(),
        category_id: expense.category_id.to_string(),
        amount_minor: expense.amount_minor,
        currency: expense.currency,
        status: expense.status,
        tx_hash: expense.tx_hash,
        occurred_at: expense.occurred_at.to_rfc3339(),
        created_at: expense.created_at.to_rfc3339(),
    };

    state.idempotency.write().await.insert(
        idempotency_key,
        serde_json::to_value(&response)
            .map_err(|_| AppError::internal("idempotency cache failed"))?,
    );

    state.audit_logs.write().await.push(AuditLog {
        id: Uuid::new_v4(),
        actor_wallet: auth.wallet.clone(),
        action: "expense.create".to_string(),
        target_id: Some(response.id.clone()),
        tx_hash: None,
        metadata: json!({ "category_id": response.category_id, "amount_minor": response.amount_minor }),
        created_at: Utc::now(),
    });

    Ok(Json(response))
}

async fn create_pg(
    state: &AppState,
    auth: &AuthUser,
    headers: &HeaderMap,
    payload: &CreateExpenseRequest,
    category_id: Uuid,
) -> AppResult<Json<ExpenseResponse>> {
    let pool = state
        .pg_pool
        .as_ref()
        .ok_or_else(|| AppError::internal("postgres pool is not initialized"))?;

    let idempotency_key = headers
        .get("x-idempotency-key")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::bad_request("x-idempotency-key header is required"))?
        .to_string();

    let category_row = sqlx::query("SELECT owner_user_id FROM categories WHERE id = $1")
        .bind(category_id)
        .fetch_optional(pool)
        .await
        .map_err(|_| AppError::internal("failed to load category"))?
        .ok_or_else(|| AppError::not_found("category not found"))?;

    let category_owner_user_id: Uuid = category_row
        .try_get("owner_user_id")
        .map_err(|_| AppError::internal("invalid category row"))?;
    if category_owner_user_id != auth.user_id {
        return Err(AppError::forbidden("cannot use category of another user"));
    }

    let occurred_at = payload
        .occurred_at
        .as_deref()
        .and_then(|v| chrono::DateTime::parse_from_rfc3339(v).ok())
        .map(|v| v.with_timezone(&Utc))
        .unwrap_or_else(Utc::now);

    let now = Utc::now();
    let expense_id = Uuid::new_v4();

    let mut tx = pool
        .begin()
        .await
        .map_err(|_| AppError::internal("failed to start transaction"))?;

    let idempotency_row =
        sqlx::query("SELECT response_payload FROM idempotency_keys WHERE key = $1 FOR UPDATE")
            .bind(&idempotency_key)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|_| AppError::internal("failed to read idempotency key"))?;

    if let Some(row) = idempotency_row {
        let cached_payload: Option<serde_json::Value> =
            row.try_get("response_payload").unwrap_or(None);
        if let Some(cached_payload) = cached_payload {
            let parsed: ExpenseResponse = serde_json::from_value(cached_payload)
                .map_err(|_| AppError::internal("failed to decode cached response"))?;
            tx.rollback()
                .await
                .map_err(|_| AppError::internal("failed to rollback transaction"))?;
            return Ok(Json(parsed));
        }
    } else {
        sqlx::query(
            "INSERT INTO idempotency_keys (id, key, request_hash, response_payload, created_at) VALUES ($1, $2, NULL, NULL, $3)",
        )
        .bind(Uuid::new_v4())
        .bind(&idempotency_key)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(|_| AppError::internal("failed to reserve idempotency key"))?;
    }

    let status = ExpenseStatus::Pending;
    sqlx::query(
        "INSERT INTO expenses_read_model (id, owner_user_id, category_id, amount, amount_minor, currency, status, tx_hash, occurred_at, created_at) \
         VALUES ($1, $2, $3, $4::numeric / 100.0, $4, $5, $6, $7, $8, $9)",
    )
    .bind(expense_id)
    .bind(auth.user_id)
    .bind(category_id)
    .bind(payload.amount_minor)
    .bind(&payload.currency)
    .bind(status_to_db(&status))
    .bind(Option::<String>::None)
    .bind(occurred_at)
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(|_| AppError::internal("failed to create expense"))?;

    let response = ExpenseResponse {
        id: expense_id.to_string(),
        owner_user_id: auth.user_id.to_string(),
        category_id: category_id.to_string(),
        amount_minor: payload.amount_minor,
        currency: payload.currency.clone(),
        status,
        tx_hash: None,
        occurred_at: occurred_at.to_rfc3339(),
        created_at: now.to_rfc3339(),
    };

    sqlx::query(
        "INSERT INTO tx_audit_logs (id, actor_wallet, action, target_id, tx_hash, metadata, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(Uuid::new_v4())
    .bind(&auth.wallet)
    .bind("expense.create")
    .bind(response.id.clone())
    .bind(Option::<String>::None)
    .bind(json!({ "category_id": response.category_id, "amount_minor": response.amount_minor }))
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(|_| AppError::internal("failed to append audit log"))?;

    sqlx::query("UPDATE idempotency_keys SET response_payload = $2 WHERE key = $1")
        .bind(&idempotency_key)
        .bind(
            serde_json::to_value(&response)
                .map_err(|_| AppError::internal("idempotency cache failed"))?,
        )
        .execute(&mut *tx)
        .await
        .map_err(|_| AppError::internal("failed to store idempotency response"))?;

    tx.commit()
        .await
        .map_err(|_| AppError::internal("failed to commit transaction"))?;

    Ok(Json(response))
}

pub async fn update_status(
    State(state): State<AppState>,
    Path(expense_id): Path<String>,
    auth: AuthUser,
    headers: HeaderMap,
    Json(payload): Json<UpdateExpenseStatusRequest>,
) -> AppResult<Json<ExpenseResponse>> {
    require_admin(&auth)?;

    let expense_id = expense_id
        .parse::<Uuid>()
        .map_err(|_| AppError::bad_request("invalid expense_id"))?;

    let to_status = match payload.status.clone() {
        ExpenseStatus::Approved | ExpenseStatus::Rejected => payload.status.clone(),
        ExpenseStatus::Pending => {
            return Err(AppError::bad_request("status must be approved or rejected"));
        }
    };

    if state.config.expenses_pg_enabled {
        return update_status_pg(&state, expense_id, &auth, &headers, &payload, to_status).await;
    }

    update_status_in_memory(&state, expense_id, &auth, &headers, &payload, to_status).await
}

async fn update_status_in_memory(
    state: &AppState,
    expense_id: Uuid,
    auth: &AuthUser,
    headers: &HeaderMap,
    payload: &UpdateExpenseStatusRequest,
    to_status: ExpenseStatus,
) -> AppResult<Json<ExpenseResponse>> {
    let idempotency_key = headers
        .get("x-idempotency-key")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::bad_request("x-idempotency-key header is required"))?
        .to_string();

    let cache_key = format!("expense.status:{expense_id}:{idempotency_key}");
    if let Some(cached) = state.idempotency.read().await.get(&cache_key).cloned() {
        let parsed: ExpenseResponse = serde_json::from_value(cached)
            .map_err(|_| AppError::internal("failed to decode cached response"))?;
        return Ok(Json(parsed));
    }

    let mut expenses = state.expenses.write().await;
    let expense = expenses
        .get_mut(&expense_id)
        .ok_or_else(|| AppError::not_found("expense not found"))?;

    let from_status = expense.status.clone();
    if from_status != ExpenseStatus::Pending {
        return Err(AppError::bad_request("expense status is final"));
    }

    expense.status = to_status.clone();

    let response = ExpenseResponse {
        id: expense.id.to_string(),
        owner_user_id: expense.owner_user_id.to_string(),
        category_id: expense.category_id.to_string(),
        amount_minor: expense.amount_minor,
        currency: expense.currency.clone(),
        status: expense.status.clone(),
        tx_hash: expense.tx_hash.clone(),
        occurred_at: expense.occurred_at.to_rfc3339(),
        created_at: expense.created_at.to_rfc3339(),
    };
    drop(expenses);

    state.idempotency.write().await.insert(
        cache_key,
        serde_json::to_value(&response)
            .map_err(|_| AppError::internal("idempotency cache failed"))?,
    );

    let action = match to_status {
        ExpenseStatus::Approved => "expense.approve",
        ExpenseStatus::Rejected => "expense.reject",
        ExpenseStatus::Pending => "expense.status",
    };

    state.audit_logs.write().await.push(AuditLog {
        id: Uuid::new_v4(),
        actor_wallet: auth.wallet.clone(),
        action: action.to_string(),
        target_id: Some(response.id.clone()),
        tx_hash: response.tx_hash.clone(),
        metadata: json!({
            "from_status": from_status,
            "to_status": to_status,
            "reason": payload.reason,
            "idempotency_key": idempotency_key
        }),
        created_at: Utc::now(),
    });

    Ok(Json(response))
}

async fn update_status_pg(
    state: &AppState,
    expense_id: Uuid,
    auth: &AuthUser,
    headers: &HeaderMap,
    payload: &UpdateExpenseStatusRequest,
    to_status: ExpenseStatus,
) -> AppResult<Json<ExpenseResponse>> {
    let pool = state
        .pg_pool
        .as_ref()
        .ok_or_else(|| AppError::internal("postgres pool is not initialized"))?;

    let idempotency_key = headers
        .get("x-idempotency-key")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::bad_request("x-idempotency-key header is required"))?
        .to_string();
    let cache_key = format!("expense.status:{expense_id}:{idempotency_key}");

    let now = Utc::now();
    let mut tx = pool
        .begin()
        .await
        .map_err(|_| AppError::internal("failed to start transaction"))?;

    let idempotency_row =
        sqlx::query("SELECT response_payload FROM idempotency_keys WHERE key = $1 FOR UPDATE")
            .bind(&cache_key)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|_| AppError::internal("failed to read idempotency key"))?;

    if let Some(row) = idempotency_row {
        let cached_payload: Option<serde_json::Value> =
            row.try_get("response_payload").unwrap_or(None);
        if let Some(cached_payload) = cached_payload {
            let parsed: ExpenseResponse = serde_json::from_value(cached_payload)
                .map_err(|_| AppError::internal("failed to decode cached response"))?;
            tx.rollback()
                .await
                .map_err(|_| AppError::internal("failed to rollback transaction"))?;
            return Ok(Json(parsed));
        }
    } else {
        sqlx::query(
            "INSERT INTO idempotency_keys (id, key, request_hash, response_payload, created_at) VALUES ($1, $2, NULL, NULL, $3)",
        )
        .bind(Uuid::new_v4())
        .bind(&cache_key)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(|_| AppError::internal("failed to reserve idempotency key"))?;
    }

    let row = sqlx::query(
        "SELECT id, owner_user_id, category_id, amount_minor, currency, status, tx_hash, occurred_at, created_at \
         FROM expenses_read_model WHERE id = $1 FOR UPDATE",
    )
    .bind(expense_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| AppError::internal("failed to read expense"))?
    .ok_or_else(|| AppError::not_found("expense not found"))?;

    let from_status = status_from_db(
        row.try_get::<String, _>("status")
            .map_err(|_| AppError::internal("invalid expense row"))?
            .as_str(),
    )?;
    if from_status != ExpenseStatus::Pending {
        return Err(AppError::bad_request("expense status is final"));
    }

    sqlx::query("UPDATE expenses_read_model SET status = $2 WHERE id = $1")
        .bind(expense_id)
        .bind(status_to_db(&to_status))
        .execute(&mut *tx)
        .await
        .map_err(|_| AppError::internal("failed to update expense status"))?;

    let response = ExpenseResponse {
        id: row
            .try_get::<Uuid, _>("id")
            .map_err(|_| AppError::internal("invalid expense row"))?
            .to_string(),
        owner_user_id: row
            .try_get::<Uuid, _>("owner_user_id")
            .map_err(|_| AppError::internal("invalid expense row"))?
            .to_string(),
        category_id: row
            .try_get::<Uuid, _>("category_id")
            .map_err(|_| AppError::internal("invalid expense row"))?
            .to_string(),
        amount_minor: row
            .try_get::<i64, _>("amount_minor")
            .map_err(|_| AppError::internal("invalid expense row"))?,
        currency: row
            .try_get::<String, _>("currency")
            .map_err(|_| AppError::internal("invalid expense row"))?,
        status: to_status.clone(),
        tx_hash: row
            .try_get::<Option<String>, _>("tx_hash")
            .map_err(|_| AppError::internal("invalid expense row"))?,
        occurred_at: row
            .try_get::<chrono::DateTime<Utc>, _>("occurred_at")
            .map_err(|_| AppError::internal("invalid expense row"))?
            .to_rfc3339(),
        created_at: row
            .try_get::<chrono::DateTime<Utc>, _>("created_at")
            .map_err(|_| AppError::internal("invalid expense row"))?
            .to_rfc3339(),
    };

    let action = match to_status {
        ExpenseStatus::Approved => "expense.approve",
        ExpenseStatus::Rejected => "expense.reject",
        ExpenseStatus::Pending => "expense.status",
    };

    sqlx::query(
        "INSERT INTO tx_audit_logs (id, actor_wallet, action, target_id, tx_hash, metadata, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(Uuid::new_v4())
    .bind(&auth.wallet)
    .bind(action)
    .bind(response.id.clone())
    .bind(response.tx_hash.clone())
    .bind(json!({
        "from_status": from_status,
        "to_status": to_status,
        "reason": payload.reason,
        "idempotency_key": idempotency_key
    }))
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(|_| AppError::internal("failed to append audit log"))?;

    sqlx::query("UPDATE idempotency_keys SET response_payload = $2 WHERE key = $1")
        .bind(&cache_key)
        .bind(
            serde_json::to_value(&response)
                .map_err(|_| AppError::internal("idempotency cache failed"))?,
        )
        .execute(&mut *tx)
        .await
        .map_err(|_| AppError::internal("failed to store idempotency response"))?;

    tx.commit()
        .await
        .map_err(|_| AppError::internal("failed to commit transaction"))?;

    Ok(Json(response))
}

pub async fn history(
    State(state): State<AppState>,
    Path(expense_id): Path<String>,
    auth: AuthUser,
    Query(query): Query<ExpenseHistoryQuery>,
) -> AppResult<Json<Vec<ExpenseHistoryItem>>> {
    require_user_admin_or_auditor(&auth)?;

    let expense_id = expense_id
        .parse::<Uuid>()
        .map_err(|_| AppError::bad_request("invalid expense_id"))?;

    if state.config.expenses_pg_enabled && auth.role == Role::User {
        let pool = state
            .pg_pool
            .as_ref()
            .ok_or_else(|| AppError::internal("postgres pool is not initialized"))?;

        let owner_row = sqlx::query("SELECT owner_user_id FROM expenses_read_model WHERE id = $1")
            .bind(expense_id)
            .fetch_optional(pool)
            .await
            .map_err(|_| AppError::internal("failed to load expense owner"))?
            .ok_or_else(|| AppError::not_found("expense not found"))?;

        let owner_user_id: Uuid = owner_row
            .try_get("owner_user_id")
            .map_err(|_| AppError::internal("invalid expense row"))?;

        if owner_user_id != auth.user_id {
            return Err(AppError::forbidden("cannot access history of another user's expense"));
        }
    }

    let from = match query.from.as_deref() {
        Some(v) => Some(
            chrono::DateTime::parse_from_rfc3339(v)
                .map_err(|_| AppError::bad_request("invalid from datetime"))?
                .with_timezone(&Utc),
        ),
        None => None,
    };

    let to = match query.to.as_deref() {
        Some(v) => Some(
            chrono::DateTime::parse_from_rfc3339(v)
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
            "SELECT id, actor_wallet, action, target_id, tx_hash, metadata, created_at \
             FROM tx_audit_logs \
             WHERE target_id = $1 \
               AND ($2::timestamptz IS NULL OR created_at >= $2) \
               AND ($3::timestamptz IS NULL OR created_at <= $3) \
             ORDER BY created_at DESC \
             LIMIT $4",
        )
        .bind(expense_id.to_string())
        .bind(from)
        .bind(to)
        .bind(limit as i64)
        .fetch_all(pool)
        .await
        .map_err(|_| AppError::internal("failed to list expense history"))?;

        let out = rows
            .into_iter()
            .map(|row| {
                Ok(ExpenseHistoryItem {
                    id: row
                        .try_get::<Uuid, _>("id")
                        .map_err(|_| AppError::internal("invalid history row"))?
                        .to_string(),
                    actor_wallet: row
                        .try_get("actor_wallet")
                        .map_err(|_| AppError::internal("invalid history row"))?,
                    action: row
                        .try_get("action")
                        .map_err(|_| AppError::internal("invalid history row"))?,
                    target_id: row
                        .try_get("target_id")
                        .map_err(|_| AppError::internal("invalid history row"))?,
                    tx_hash: row
                        .try_get("tx_hash")
                        .map_err(|_| AppError::internal("invalid history row"))?,
                    metadata: row
                        .try_get("metadata")
                        .map_err(|_| AppError::internal("invalid history row"))?,
                    created_at: row
                        .try_get::<chrono::DateTime<Utc>, _>("created_at")
                        .map_err(|_| AppError::internal("invalid history row"))?
                        .to_rfc3339(),
                })
            })
            .collect::<AppResult<Vec<_>>>()?;

        return Ok(Json(out));
    }

    let logs = state.audit_logs.read().await;
    let mut rows = logs
        .iter()
        .filter(|x| {
            if x.target_id.as_deref() != Some(&expense_id.to_string()) {
                return false;
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
        .map(|x| ExpenseHistoryItem {
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

pub async fn search(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<SearchExpensesQuery>,
) -> AppResult<Json<SearchExpensesResponse>> {
    require_user_or_admin(&auth)?;

    let status = query.status.clone();
    let category_id = match query.category_id.as_deref() {
        Some(v) => Some(
            v.parse::<Uuid>()
                .map_err(|_| AppError::bad_request("invalid category_id"))?,
        ),
        None => None,
    };
    let currency = query.currency.clone();
    let from = match query.from.as_deref() {
        Some(v) => Some(
            chrono::DateTime::parse_from_rfc3339(v)
                .map_err(|_| AppError::bad_request("invalid from datetime"))?
                .with_timezone(&Utc),
        ),
        None => None,
    };
    let to = match query.to.as_deref() {
        Some(v) => Some(
            chrono::DateTime::parse_from_rfc3339(v)
                .map_err(|_| AppError::bad_request("invalid to datetime"))?
                .with_timezone(&Utc),
        ),
        None => None,
    };
    let q = query.q.clone();
    let limit = query.limit.unwrap_or(20).clamp(1, 100);
    let offset = query.offset.unwrap_or(0);

    if state.config.expenses_pg_enabled {
        let pool = state
            .pg_pool
            .as_ref()
            .ok_or_else(|| AppError::internal("postgres pool is not initialized"))?;

        let owner_filter = if auth.role == Role::Admin {
            None
        } else {
            Some(auth.user_id)
        };

        let total_row = sqlx::query(
            "SELECT COUNT(*)::bigint AS total \
             FROM expenses_read_model \
             WHERE ($1::uuid IS NULL OR owner_user_id = $1) \
               AND ($2::text IS NULL OR status = $2) \
               AND ($3::uuid IS NULL OR category_id = $3) \
               AND ($4::text IS NULL OR currency = $4) \
               AND ($5::timestamptz IS NULL OR occurred_at >= $5) \
               AND ($6::timestamptz IS NULL OR occurred_at <= $6) \
               AND ($7::text IS NULL OR CAST(amount_minor AS text) ILIKE '%' || $7::text || '%' OR currency ILIKE '%' || $7::text || '%')",
        )
        .bind(owner_filter)
        .bind(status.clone())
        .bind(category_id)
        .bind(currency.clone())
        .bind(from)
        .bind(to)
        .bind(q.clone())
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::internal(format!("failed to count expenses: {e}")))?;

        let total: i64 = total_row
            .try_get("total")
            .map_err(|_| AppError::internal("invalid count row"))?;

        let rows = sqlx::query(
            "SELECT id, owner_user_id, category_id, amount_minor, currency, status, tx_hash, occurred_at, created_at \
             FROM expenses_read_model \
             WHERE ($1::uuid IS NULL OR owner_user_id = $1) \
               AND ($2::text IS NULL OR status = $2) \
               AND ($3::uuid IS NULL OR category_id = $3) \
               AND ($4::text IS NULL OR currency = $4) \
               AND ($5::timestamptz IS NULL OR occurred_at >= $5) \
               AND ($6::timestamptz IS NULL OR occurred_at <= $6) \
               AND ($7::text IS NULL OR CAST(amount_minor AS text) ILIKE '%' || $7::text || '%' OR currency ILIKE '%' || $7::text || '%') \
             ORDER BY created_at DESC \
             LIMIT $8 OFFSET $9",
        )
        .bind(owner_filter)
        .bind(status)
        .bind(category_id)
        .bind(currency)
        .bind(from)
        .bind(to)
        .bind(q)
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(pool)
        .await
        .map_err(|_| AppError::internal("failed to search expenses"))?;

        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            let status_text = row
                .try_get::<String, _>("status")
                .map_err(|_| AppError::internal("invalid expense row"))?;
            items.push(ExpenseResponse {
                id: row
                    .try_get::<Uuid, _>("id")
                    .map_err(|_| AppError::internal("invalid expense row"))?
                    .to_string(),
                owner_user_id: row
                    .try_get::<Uuid, _>("owner_user_id")
                    .map_err(|_| AppError::internal("invalid expense row"))?
                    .to_string(),
                category_id: row
                    .try_get::<Uuid, _>("category_id")
                    .map_err(|_| AppError::internal("invalid expense row"))?
                    .to_string(),
                amount_minor: row
                    .try_get::<i64, _>("amount_minor")
                    .map_err(|_| AppError::internal("invalid expense row"))?,
                currency: row
                    .try_get::<String, _>("currency")
                    .map_err(|_| AppError::internal("invalid expense row"))?,
                status: status_from_db(&status_text)?,
                tx_hash: row
                    .try_get::<Option<String>, _>("tx_hash")
                    .map_err(|_| AppError::internal("invalid expense row"))?,
                occurred_at: row
                    .try_get::<chrono::DateTime<Utc>, _>("occurred_at")
                    .map_err(|_| AppError::internal("invalid expense row"))?
                    .to_rfc3339(),
                created_at: row
                    .try_get::<chrono::DateTime<Utc>, _>("created_at")
                    .map_err(|_| AppError::internal("invalid expense row"))?
                    .to_rfc3339(),
            });
        }

        return Ok(Json(SearchExpensesResponse {
            has_more: (offset as i64 + items.len() as i64) < total,
            items,
            total,
            limit,
            offset,
        }));
    }

    let expenses = state.expenses.read().await;
    let mut rows = expenses
        .values()
        .filter(|e| auth.role == Role::Admin || e.owner_user_id == auth.user_id)
        .filter(|e| {
            if let Some(ref s) = query.status {
                let current = match e.status {
                    ExpenseStatus::Pending => "pending",
                    ExpenseStatus::Approved => "approved",
                    ExpenseStatus::Rejected => "rejected",
                };
                if current != s {
                    return false;
                }
            }
            if let Some(cid) = category_id {
                if e.category_id != cid {
                    return false;
                }
            }
            if let Some(ref cur) = query.currency {
                if e.currency != *cur {
                    return false;
                }
            }
            if let Some(f) = from {
                if e.occurred_at < f {
                    return false;
                }
            }
            if let Some(t) = to {
                if e.occurred_at > t {
                    return false;
                }
            }
            if let Some(ref keyword) = query.q {
                let hay = format!("{} {}", e.amount_minor, e.currency).to_lowercase();
                if !hay.contains(&keyword.to_lowercase()) {
                    return false;
                }
            }
            true
        })
        .map(|e| ExpenseResponse {
            id: e.id.to_string(),
            owner_user_id: e.owner_user_id.to_string(),
            category_id: e.category_id.to_string(),
            amount_minor: e.amount_minor,
            currency: e.currency.clone(),
            status: e.status.clone(),
            tx_hash: e.tx_hash.clone(),
            occurred_at: e.occurred_at.to_rfc3339(),
            created_at: e.created_at.to_rfc3339(),
        })
        .collect::<Vec<_>>();

    rows.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    let total = rows.len() as i64;
    let items = rows.into_iter().skip(offset).take(limit).collect::<Vec<_>>();

    Ok(Json(SearchExpensesResponse {
        has_more: (offset + items.len()) < total as usize,
        items,
        total,
        limit,
        offset,
    }))
}

pub async fn list(
    State(state): State<AppState>,
    auth: AuthUser,
) -> AppResult<Json<Vec<ExpenseResponse>>> {
    require_user_or_admin(&auth)?;

    if state.config.expenses_pg_enabled {
        let pool = state
            .pg_pool
            .as_ref()
            .ok_or_else(|| AppError::internal("postgres pool is not initialized"))?;

        let rows = if auth.role == Role::Admin {
            sqlx::query(
                "SELECT id, owner_user_id, category_id, amount_minor, currency, status, tx_hash, occurred_at, created_at \
                 FROM expenses_read_model ORDER BY created_at DESC",
            )
            .fetch_all(pool)
            .await
            .map_err(|_| AppError::internal("failed to list expenses"))?
        } else {
            sqlx::query(
                "SELECT id, owner_user_id, category_id, amount_minor, currency, status, tx_hash, occurred_at, created_at \
                 FROM expenses_read_model WHERE owner_user_id = $1 ORDER BY created_at DESC",
            )
            .bind(auth.user_id)
            .fetch_all(pool)
            .await
            .map_err(|_| AppError::internal("failed to list expenses"))?
        };

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let status_text = row
                .try_get::<String, _>("status")
                .map_err(|_| AppError::internal("invalid expense row"))?;
            out.push(ExpenseResponse {
                id: row
                    .try_get::<Uuid, _>("id")
                    .map_err(|_| AppError::internal("invalid expense row"))?
                    .to_string(),
                owner_user_id: row
                    .try_get::<Uuid, _>("owner_user_id")
                    .map_err(|_| AppError::internal("invalid expense row"))?
                    .to_string(),
                category_id: row
                    .try_get::<Uuid, _>("category_id")
                    .map_err(|_| AppError::internal("invalid expense row"))?
                    .to_string(),
                amount_minor: row
                    .try_get::<i64, _>("amount_minor")
                    .map_err(|_| AppError::internal("invalid expense row"))?,
                currency: row
                    .try_get::<String, _>("currency")
                    .map_err(|_| AppError::internal("invalid expense row"))?,
                status: status_from_db(&status_text)?,
                tx_hash: row
                    .try_get::<Option<String>, _>("tx_hash")
                    .map_err(|_| AppError::internal("invalid expense row"))?,
                occurred_at: row
                    .try_get::<chrono::DateTime<Utc>, _>("occurred_at")
                    .map_err(|_| AppError::internal("invalid expense row"))?
                    .to_rfc3339(),
                created_at: row
                    .try_get::<chrono::DateTime<Utc>, _>("created_at")
                    .map_err(|_| AppError::internal("invalid expense row"))?
                    .to_rfc3339(),
            });
        }
        return Ok(Json(out));
    }

    let expenses = state.expenses.read().await;
    let rows = expenses
        .values()
        .filter(|e| auth.role == Role::Admin || e.owner_user_id == auth.user_id)
        .map(|e| ExpenseResponse {
            id: e.id.to_string(),
            owner_user_id: e.owner_user_id.to_string(),
            category_id: e.category_id.to_string(),
            amount_minor: e.amount_minor,
            currency: e.currency.clone(),
            status: e.status.clone(),
            tx_hash: e.tx_hash.clone(),
            occurred_at: e.occurred_at.to_rfc3339(),
            created_at: e.created_at.to_rfc3339(),
        })
        .collect::<Vec<_>>();

    Ok(Json(rows))
}
