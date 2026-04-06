use async_trait::async_trait;
use axum::{
    extract::{FromRef, FromRequestParts},
    http::{header, request::Parts},
};
use uuid::Uuid;

use crate::{
    error::AppError,
    models::Role,
    security::jwt::{decode_jwt, Claims, TokenType},
    state::AppState,
};

#[derive(Clone)]
pub struct AuthUser {
    pub user_id: Uuid,
    pub wallet: String,
    pub role: Role,
    pub claims: Claims,
}

#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);

        let auth_header = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| AppError::unauthorized("Missing Authorization header"))?;

        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or_else(|| AppError::unauthorized("Invalid auth scheme"))?;

        let claims = decode_jwt(token, &app_state.config.jwt_secret)
            .map_err(|_| AppError::unauthorized("Invalid or expired token"))?;

        if claims.typ != TokenType::Access {
            return Err(AppError::unauthorized("Access token required"));
        }

        if app_state
            .revoked_access_jtis
            .read()
            .await
            .contains_key(&claims.jti)
        {
            return Err(AppError::unauthorized("Token revoked"));
        }

        if app_state
            .revoked_token_families
            .read()
            .await
            .contains_key(&claims.fid)
        {
            return Err(AppError::unauthorized("Session revoked"));
        }

        let user_id = claims
            .sub
            .parse::<Uuid>()
            .map_err(|_| AppError::unauthorized("Invalid subject claim"))?;

        let role = match claims.role.as_str() {
            "admin" => Role::Admin,
            "auditor" => Role::Auditor,
            "user" => Role::User,
            _ => return Err(AppError::unauthorized("Invalid role claim")),
        };

        Ok(Self {
            user_id,
            wallet: claims.wallet.clone(),
            role,
            claims,
        })
    }
}
