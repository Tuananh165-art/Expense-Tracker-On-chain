use axum::{extract::State, Json};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    authz::require_user_or_admin,
    error::{AppError, AppResult},
    models::Category,
    state::AppState,
};

#[derive(Deserialize)]
pub struct CreateCategoryRequest {
    pub name: String,
}

#[derive(Serialize)]
pub struct CategoryResponse {
    pub id: String,
    pub owner_user_id: String,
    pub name: String,
    pub created_at: String,
}

pub async fn create(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<CreateCategoryRequest>,
) -> AppResult<Json<CategoryResponse>> {
    require_user_or_admin(&auth)?;
    let name = payload.name.trim();
    if name.is_empty() {
        return Err(AppError::bad_request("name is required"));
    }

    let category = Category {
        id: Uuid::new_v4(),
        owner_user_id: auth.user_id,
        name: name.to_string(),
        created_at: Utc::now(),
    };

    state
        .categories
        .write()
        .await
        .insert(category.id, category.clone());

    Ok(Json(CategoryResponse {
        id: category.id.to_string(),
        owner_user_id: category.owner_user_id.to_string(),
        name: category.name,
        created_at: category.created_at.to_rfc3339(),
    }))
}

pub async fn list(State(state): State<AppState>, auth: AuthUser) -> AppResult<Json<Vec<CategoryResponse>>> {
    require_user_or_admin(&auth)?;
    let categories = state.categories.read().await;
    let rows = categories
        .values()
        .filter(|c| c.owner_user_id == auth.user_id)
        .map(|c| CategoryResponse {
            id: c.id.to_string(),
            owner_user_id: c.owner_user_id.to_string(),
            name: c.name.clone(),
            created_at: c.created_at.to_rfc3339(),
        })
        .collect::<Vec<_>>();

    Ok(Json(rows))
}
