use std::env;

#[derive(Clone)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
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
        let jwt_secret = env::var("JWT_SECRET").unwrap_or_else(|_| "change_me_in_production".to_string());
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
