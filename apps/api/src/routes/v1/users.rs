use axum::{extract::State, Json};
use serde::Serialize;
use sqlx::Row;

use crate::{
    auth::AuthUser,
    authz::require_user_admin_or_auditor,
    error::{AppError, AppResult},
    models::Role,
    state::AppState,
};

#[derive(Serialize)]
pub struct MeResponse {
    pub id: String,
    pub wallet_address: String,
    pub role: Role,
    pub created_at: String,
}

fn role_from_db(role: &str) -> AppResult<Role> {
    match role {
        "admin" => Ok(Role::Admin),
        "auditor" => Ok(Role::Auditor),
        "user" => Ok(Role::User),
        _ => Err(AppError::internal("invalid role in database")),
    }
}

pub async fn me(State(state): State<AppState>, auth: AuthUser) -> AppResult<Json<MeResponse>> {
    require_user_admin_or_auditor(&auth)?;

    if state.config.auth_pg_enabled {
        me_pg(state, auth).await
    } else {
        me_in_memory(state, auth).await
    }
}

async fn me_in_memory(state: AppState, auth: AuthUser) -> AppResult<Json<MeResponse>> {
    let users = state.users.read().await;
    let user = users
        .get(&auth.user_id)
        .ok_or_else(|| AppError::not_found("user not found"))?;

    Ok(Json(MeResponse {
        id: user.id.to_string(),
        wallet_address: user.wallet_address.clone(),
        role: user.role.clone(),
        created_at: user.created_at.to_rfc3339(),
    }))
}

async fn me_pg(state: AppState, auth: AuthUser) -> AppResult<Json<MeResponse>> {
    let pool = state
        .pg_pool
        .as_ref()
        .ok_or_else(|| AppError::internal("postgres pool is not initialized"))?;

    let row = sqlx::query("SELECT id, wallet_address, role, created_at FROM users WHERE id = $1")
        .bind(auth.user_id)
        .fetch_optional(pool)
        .await
        .map_err(|_| AppError::internal("failed to load user"))?
        .ok_or_else(|| AppError::not_found("user not found"))?;

    let id: uuid::Uuid = row
        .try_get("id")
        .map_err(|_| AppError::internal("invalid user row"))?;
    let wallet_address: String = row
        .try_get("wallet_address")
        .map_err(|_| AppError::internal("invalid user row"))?;
    let role_raw: String = row
        .try_get("role")
        .map_err(|_| AppError::internal("invalid user row"))?;
    let created_at: chrono::DateTime<chrono::Utc> = row
        .try_get("created_at")
        .map_err(|_| AppError::internal("invalid user row"))?;

    Ok(Json(MeResponse {
        id: id.to_string(),
        wallet_address,
        role: role_from_db(&role_raw)?,
        created_at: created_at.to_rfc3339(),
    }))
}
