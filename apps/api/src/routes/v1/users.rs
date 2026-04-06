use axum::{extract::State, Json};
use serde::Serialize;

use crate::{auth::AuthUser, authz::require_user_admin_or_auditor, error::{AppError, AppResult}, models::Role, state::AppState};

#[derive(Serialize)]
pub struct MeResponse {
    pub id: String,
    pub wallet_address: String,
    pub role: Role,
    pub created_at: String,
}

pub async fn me(
    State(state): State<AppState>,
    auth: AuthUser,
) -> AppResult<Json<MeResponse>> {
    require_user_admin_or_auditor(&auth)?;
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
