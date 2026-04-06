use std::net::SocketAddr;

use expense_tracker_api::{build_app, config::AppConfig, spawn_cleanup_task, state::AppState};
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt().with_env_filter("info").init();

    let config = AppConfig::from_env();

    let pg_pool = if config.expenses_pg_enabled || config.auth_pg_enabled {
        let database_url = config
            .database_url
            .clone()
            .expect("DATABASE_URL is required when EXPENSES_PG_ENABLED=true or AUTH_PG_ENABLED=true");
        Some(
            PgPoolOptions::new()
                .max_connections(5)
                .connect(&database_url)
                .await
                .expect("failed to connect to postgres"),
        )
    } else {
        None
    };

    let state = AppState::new(config.clone(), pg_pool);
    spawn_cleanup_task(state.clone());

    let app = build_app(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!("API listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind API listener");

    axum::serve(listener, app)
        .await
        .expect("failed to start API server");
}
