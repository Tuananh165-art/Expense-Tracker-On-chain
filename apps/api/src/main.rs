use std::net::SocketAddr;

use expense_tracker_api::{build_app, config::AppConfig, spawn_cleanup_task, state::AppState};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_env_filter("info").init();

    let config = AppConfig::from_env();
    let state = AppState::new(config.clone());
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
