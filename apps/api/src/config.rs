use std::env;

#[derive(Clone)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub database_url: Option<String>,
    pub expenses_pg_enabled: bool,
    pub auth_pg_enabled: bool,
    pub hybrid_onchain_enabled: bool,
    pub solana_rpc_url: String,
    pub solana_program_id: String,
    pub solana_commitment: String,
    pub jwt_secret: String,
    pub jwt_expires_in_seconds: i64,
    pub refresh_expires_in_seconds: i64,
    pub challenge_ttl_seconds: i64,
    pub cleanup_interval_seconds: i64,
    pub used_challenge_retention_seconds: i64,
    pub used_refresh_retention_seconds: i64,
    pub revoked_access_retention_seconds: i64,
    pub revoked_family_retention_seconds: i64,
}

impl AppConfig {
    pub fn from_env() -> Self {
        let host = env::var("API_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
        let port = env::var("API_PORT")
            .ok()
            .and_then(|v| v.parse::<u16>().ok())
            .unwrap_or(8080);
        let database_url = env::var("DATABASE_URL")
            .ok()
            .filter(|v| !v.trim().is_empty());
        let expenses_pg_enabled = env::var("EXPENSES_PG_ENABLED")
            .ok()
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("yes"))
            .unwrap_or(false);
        let auth_pg_enabled = env::var("AUTH_PG_ENABLED")
            .ok()
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("yes"))
            .unwrap_or(false);
        let hybrid_onchain_enabled = env::var("HYBRID_ONCHAIN_ENABLED")
            .ok()
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("yes"))
            .unwrap_or(false);
        let solana_rpc_url = env::var("SOLANA_RPC_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:8899".to_string());
        let solana_program_id = env::var("SOLANA_PROGRAM_ID")
            .unwrap_or_else(|_| "rzMxNuut6R34aFgt8NY9hj3SoRB37iszrsSqZR2DSnB".to_string());
        let solana_commitment = env::var("SOLANA_COMMITMENT")
            .unwrap_or_else(|_| "finalized".to_string());

        let jwt_secret =
            env::var("JWT_SECRET").unwrap_or_else(|_| "change_me_in_production".to_string());
        let jwt_expires_in_seconds = env::var("JWT_EXPIRES_IN_SECONDS")
            .ok()
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(3600);
        let refresh_expires_in_seconds = env::var("REFRESH_EXPIRES_IN_SECONDS")
            .ok()
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(60 * 60 * 24 * 7);

        let cleanup_interval_seconds = env::var("CLEANUP_INTERVAL_SECONDS")
            .ok()
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(30);

        Self {
            host,
            port,
            database_url,
            expenses_pg_enabled,
            auth_pg_enabled,
            hybrid_onchain_enabled,
            solana_rpc_url,
            solana_program_id,
            solana_commitment,
            jwt_secret,
            jwt_expires_in_seconds,
            refresh_expires_in_seconds,
            challenge_ttl_seconds: 300,
            cleanup_interval_seconds,
            used_challenge_retention_seconds: 300,
            used_refresh_retention_seconds: 3600,
            revoked_access_retention_seconds: 7200,
            revoked_family_retention_seconds: 604800,
        }
    }
}
