use axum::{extract::State, http::HeaderMap, Json};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    authz::require_user_or_admin,
    error::{AppError, AppResult},
    models::{AuditLog, Expense, ExpenseStatus},
    state::AppState,
};

#[derive(Deserialize)]
pub struct CreateExpenseRequest {
    pub category_id: String,
    pub amount_minor: i64,
    pub currency: String,
    pub occurred_at: Option<String>,
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
        .parse::<Uuid>()
        .map_err(|_| AppError::bad_request("invalid category_id"))?;

    let categories = state.categories.read().await;
    let category = categories
        .get(&category_id)
        .ok_or_else(|| AppError::not_found("category not found"))?;
    if category.owner_user_id != auth.user_id {
        return Err(AppError::forbidden("cannot use category of another user"));
    }
    drop(categories);

    let idempotency_key = headers
        .get("x-idempotency-key")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::bad_request("x-idempotency-key header is required"))?
        .to_string();

    if let Some(cached) = state.idempotency.read().await.get(&idempotency_key).cloned() {
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
        currency: payload.currency,
        status: ExpenseStatus::Pending,
        tx_hash: None,
        occurred_at,
        created_at: Utc::now(),
    };

    state.expenses.write().await.insert(expense.id, expense.clone());

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
        serde_json::to_value(&response).map_err(|_| AppError::internal("idempotency cache failed"))?,
    );

    state.audit_logs.write().await.push(AuditLog {
        id: Uuid::new_v4(),
        actor_wallet: auth.wallet,
        action: "expense.create".to_string(),
        target_id: Some(response.id.clone()),
        tx_hash: None,
        metadata: json!({ "category_id": response.category_id, "amount_minor": response.amount_minor }),
        created_at: Utc::now(),
    });

    Ok(Json(response))
}

pub async fn list(State(state): State<AppState>, auth: AuthUser) -> AppResult<Json<Vec<ExpenseResponse>>> {
    require_user_or_admin(&auth)?;
    let expenses = state.expenses.read().await;
    let rows = expenses
        .values()
        .filter(|e| e.owner_user_id == auth.user_id)
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
