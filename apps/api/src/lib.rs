pub mod auth;
pub mod authz;
pub mod config;
pub mod error;
pub mod models;
pub mod routes;
pub mod security;
pub mod state;

use axum::{
    http::{header::HeaderName, HeaderValue, Method},
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use tower_http::cors::CorsLayer;

use crate::{routes::{auth as auth_routes, v1}, state::AppState};

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: "expense-tracker-api",
    })
}

pub fn build_app(state: AppState) -> Router {
    let auth_router = Router::new()
        .route("/challenge", post(auth_routes::challenge))
        .route("/verify", post(auth_routes::verify))
        .route("/refresh", post(auth_routes::refresh))
        .route("/logout", post(auth_routes::logout))
        .route("/revoke", post(auth_routes::revoke));

    let v1_router = Router::new()
        .route("/users/me", get(v1::users::me))
        .route("/categories", post(v1::categories::create).get(v1::categories::list))
        .route("/expenses", post(v1::expenses::create).get(v1::expenses::list))
        .route("/reports/monthly", get(v1::reports::monthly_summary))
        .route("/audit/logs", get(v1::audit::list_logs));

    let cors = CorsLayer::new()
        .allow_origin([
            HeaderValue::from_static("http://localhost:3000"),
            HeaderValue::from_static("http://127.0.0.1:3000"),
        ])
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([
            axum::http::header::AUTHORIZATION,
            axum::http::header::CONTENT_TYPE,
            HeaderName::from_static("x-idempotency-key"),
        ]);

    Router::new()
        .route("/health", get(health))
        .nest("/api/v1/auth", auth_router)
        .nest("/api/v1", v1_router)
        .layer(cors)
        .with_state(state)
}

pub fn spawn_cleanup_task(state: AppState) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(
            state.config.cleanup_interval_seconds.max(5) as u64,
        ));
        loop {
            interval.tick().await;
            state.cleanup_expired().await;
        }
    });
}
