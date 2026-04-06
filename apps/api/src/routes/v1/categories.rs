use axum::{extract::State, Json};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::Row;
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

    if state.config.expenses_pg_enabled {
        return create_pg(&state, &auth, name).await;
    }

    create_in_memory(&state, &auth, name).await
}

async fn create_in_memory(state: &AppState, auth: &AuthUser, name: &str) -> AppResult<Json<CategoryResponse>> {
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

async fn create_pg(state: &AppState, auth: &AuthUser, name: &str) -> AppResult<Json<CategoryResponse>> {
    let pool = state
        .pg_pool
        .as_ref()
        .ok_or_else(|| AppError::internal("postgres pool is not initialized"))?;

    let id = Uuid::new_v4();
    let now = Utc::now();

    sqlx::query(
        "INSERT INTO categories (id, owner_user_id, name, created_at) VALUES ($1, $2, $3, $4)",
    )
    .bind(id)
    .bind(auth.user_id)
    .bind(name)
    .bind(now)
    .execute(pool)
    .await
    .map_err(|_| AppError::internal("failed to create category"))?;

    Ok(Json(CategoryResponse {
        id: id.to_string(),
        owner_user_id: auth.user_id.to_string(),
        name: name.to_string(),
        created_at: now.to_rfc3339(),
    }))
}

pub async fn list(
    State(state): State<AppState>,
    auth: AuthUser,
) -> AppResult<Json<Vec<CategoryResponse>>> {
    require_user_or_admin(&auth)?;

    if state.config.expenses_pg_enabled {
        let pool = state
            .pg_pool
            .as_ref()
            .ok_or_else(|| AppError::internal("postgres pool is not initialized"))?;

        let rows = if auth.role == crate::models::Role::Admin {
            sqlx::query("SELECT id, owner_user_id, name, created_at FROM categories ORDER BY created_at DESC")
                .fetch_all(pool)
                .await
                .map_err(|_| AppError::internal("failed to list categories"))?
        } else {
            sqlx::query(
                "SELECT id, owner_user_id, name, created_at FROM categories WHERE owner_user_id = $1 ORDER BY created_at DESC",
            )
            .bind(auth.user_id)
            .fetch_all(pool)
            .await
            .map_err(|_| AppError::internal("failed to list categories"))?
        };

        let rows = rows
            .into_iter()
            .map(|row| {
                Ok(CategoryResponse {
                    id: row
                        .try_get::<Uuid, _>("id")
                        .map_err(|_| AppError::internal("invalid category row"))?
                        .to_string(),
                    owner_user_id: row
                        .try_get::<Uuid, _>("owner_user_id")
                        .map_err(|_| AppError::internal("invalid category row"))?
                        .to_string(),
                    name: row
                        .try_get("name")
                        .map_err(|_| AppError::internal("invalid category row"))?,
                    created_at: row
                        .try_get::<chrono::DateTime<Utc>, _>("created_at")
                        .map_err(|_| AppError::internal("invalid category row"))?
                        .to_rfc3339(),
                })
            })
            .collect::<AppResult<Vec<_>>>()?;

        return Ok(Json(rows));
    }

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
