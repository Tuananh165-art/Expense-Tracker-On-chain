use axum::{extract::State, Json};
use chrono::{Duration, Utc};
use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    authz::require_admin,
    error::{AppError, AppResult},
    models::{AuditLog, AuthChallenge, RefreshTokenRecord, Role, User},
    security::{
        jwt::{decode_jwt, encode_access_jwt, encode_refresh_jwt, TokenType},
        solana_signature::verify_signature_base58,
    },
    state::AppState,
};

#[derive(Deserialize)]
pub struct ChallengeRequest {
    pub wallet_address: String,
}

#[derive(Serialize, Deserialize)]
pub struct ChallengeResponse {
    pub challenge_id: String,
    pub wallet_address: String,
    pub message: String,
    pub nonce: String,
    pub expires_at: String,
}

#[derive(Deserialize)]
pub struct VerifyRequest {
    pub challenge_id: String,
    pub wallet_address: String,
    pub signature: String,
}

#[derive(Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Deserialize)]
pub struct LogoutRequest {
    pub refresh_token: String,
}

#[derive(Deserialize)]
pub struct RevokeRequest {
    pub family_id: Option<String>,
    pub user_id: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct AuthTokensResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub refresh_expires_in: i64,
    pub user_id: String,
    pub role: Role,
    pub family_id: String,
}

#[derive(Serialize)]
pub struct GenericOkResponse {
    pub ok: bool,
}

pub async fn challenge(
    State(state): State<AppState>,
    Json(payload): Json<ChallengeRequest>,
) -> AppResult<Json<ChallengeResponse>> {
    state.cleanup_expired().await;

    let wallet = payload.wallet_address.trim();
    if wallet.is_empty() {
        return Err(AppError::bad_request("wallet_address is required"));
    }

    let nonce: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(24)
        .map(char::from)
        .collect();
    let challenge_id = Uuid::new_v4();
    let expires_at = Utc::now() + Duration::seconds(state.config.challenge_ttl_seconds);
    let message = format!(
        "Expense Tracker login\nwallet:{}\nnonce:{}\nchallenge_id:{}\nexpires:{}",
        wallet,
        nonce,
        challenge_id,
        expires_at.to_rfc3339()
    );

    let challenge = AuthChallenge {
        challenge_id,
        wallet_address: wallet.to_string(),
        nonce,
        message: message.clone(),
        expires_at,
        used: false,
    };

    state
        .auth_challenges
        .write()
        .await
        .insert(challenge_id, challenge.clone());

    Ok(Json(ChallengeResponse {
        challenge_id: challenge.challenge_id.to_string(),
        wallet_address: challenge.wallet_address,
        message: challenge.message,
        nonce: challenge.nonce,
        expires_at: challenge.expires_at.to_rfc3339(),
    }))
}

pub async fn verify(
    State(state): State<AppState>,
    Json(payload): Json<VerifyRequest>,
) -> AppResult<Json<AuthTokensResponse>> {
    state.cleanup_expired().await;

    let challenge_id = payload
        .challenge_id
        .parse::<Uuid>()
        .map_err(|_| AppError::bad_request("invalid challenge_id"))?;

    let mut challenges = state.auth_challenges.write().await;
    let challenge = challenges
        .get_mut(&challenge_id)
        .ok_or_else(|| AppError::unauthorized("challenge not found"))?;

    if challenge.used {
        return Err(AppError::unauthorized("challenge already used"));
    }
    if Utc::now() > challenge.expires_at {
        return Err(AppError::unauthorized("challenge expired"));
    }
    if challenge.wallet_address != payload.wallet_address {
        return Err(AppError::unauthorized("wallet mismatch"));
    }
    if !verify_signature_base58(
        &payload.wallet_address,
        &payload.signature,
        &challenge.message,
    ) {
        return Err(AppError::unauthorized("invalid signature"));
    }

    challenge.used = true;

    let wallet = payload.wallet_address.clone();
    let existing_user_id = {
        let users_by_wallet = state.users_by_wallet.read().await;
        users_by_wallet.get(&wallet).copied()
    };

    let user_id = if let Some(id) = existing_user_id {
        id
    } else {
        let id = Uuid::new_v4();
        let user = User {
            id,
            wallet_address: wallet.clone(),
            role: Role::User,
            created_at: Utc::now(),
        };
        state.users.write().await.insert(id, user);
        state.users_by_wallet.write().await.insert(wallet.clone(), id);
        id
    };

    let user = state
        .users
        .read()
        .await
        .get(&user_id)
        .cloned()
        .ok_or_else(|| AppError::internal("user not found after auth"))?;

    let family_id = Uuid::new_v4().to_string();

    let (access_token, access_claims) = encode_access_jwt(
        user.id,
        &user.wallet_address,
        &user.role,
        &family_id,
        &state.config.jwt_secret,
        state.config.jwt_expires_in_seconds,
    )
    .map_err(|_| AppError::internal("failed to create access token"))?;

    let (refresh_token, refresh_claims) = encode_refresh_jwt(
        user.id,
        &user.wallet_address,
        &user.role,
        &family_id,
        &state.config.jwt_secret,
        state.config.refresh_expires_in_seconds,
    )
    .map_err(|_| AppError::internal("failed to create refresh token"))?;

    state.refresh_tokens.write().await.insert(
        refresh_claims.jti.clone(),
        RefreshTokenRecord {
            jti: refresh_claims.jti.clone(),
            family_id: family_id.clone(),
            user_id: user.id,
            expires_at: chrono::DateTime::from_timestamp(refresh_claims.exp as i64, 0)
                .ok_or_else(|| AppError::internal("invalid refresh exp"))?,
            used: false,
            revoked: false,
        },
    );

    state.audit_logs.write().await.push(AuditLog {
        id: Uuid::new_v4(),
        actor_wallet: user.wallet_address.clone(),
        action: "auth.verify".to_string(),
        target_id: Some(user.id.to_string()),
        tx_hash: None,
        metadata: json!({
            "challenge_id": challenge_id.to_string(),
            "family_id": family_id,
            "access_jti": access_claims.jti,
            "refresh_jti": refresh_claims.jti
        }),
        created_at: Utc::now(),
    });

    Ok(Json(AuthTokensResponse {
        access_token,
        refresh_token,
        token_type: "Bearer".to_string(),
        expires_in: state.config.jwt_expires_in_seconds,
        refresh_expires_in: state.config.refresh_expires_in_seconds,
        user_id: user.id.to_string(),
        role: user.role,
        family_id,
    }))
}

pub async fn refresh(
    State(state): State<AppState>,
    Json(payload): Json<RefreshRequest>,
) -> AppResult<Json<AuthTokensResponse>> {
    state.cleanup_expired().await;

    let claims = decode_jwt(&payload.refresh_token, &state.config.jwt_secret)
        .map_err(|_| AppError::unauthorized("Invalid refresh token"))?;

    if claims.typ != TokenType::Refresh {
        return Err(AppError::unauthorized("Refresh token required"));
    }

    if state
        .revoked_token_families
        .read()
        .await
        .contains_key(&claims.fid)
    {
        return Err(AppError::unauthorized("Session revoked"));
    }

    let mut refresh_tokens = state.refresh_tokens.write().await;
    let current = refresh_tokens
        .get_mut(&claims.jti)
        .ok_or_else(|| AppError::unauthorized("Unknown refresh token"))?;

    if current.revoked || Utc::now() > current.expires_at {
        return Err(AppError::unauthorized("Refresh token expired/revoked"));
    }

    if current.used {
        state
            .revoked_token_families
            .write()
            .await
            .insert(current.family_id.clone(), Utc::now());
        return Err(AppError::unauthorized("Refresh token reuse detected"));
    }

    current.used = true;

    let user = state
        .users
        .read()
        .await
        .get(&current.user_id)
        .cloned()
        .ok_or_else(|| AppError::unauthorized("User not found"))?;

    let (access_token, access_claims) = encode_access_jwt(
        user.id,
        &user.wallet_address,
        &user.role,
        &current.family_id,
        &state.config.jwt_secret,
        state.config.jwt_expires_in_seconds,
    )
    .map_err(|_| AppError::internal("failed to create access token"))?;

    let (refresh_token, refresh_claims) = encode_refresh_jwt(
        user.id,
        &user.wallet_address,
        &user.role,
        &current.family_id,
        &state.config.jwt_secret,
        state.config.refresh_expires_in_seconds,
    )
    .map_err(|_| AppError::internal("failed to create refresh token"))?;

    let family_id = current.family_id.clone();

    refresh_tokens.insert(
        refresh_claims.jti.clone(),
        RefreshTokenRecord {
            jti: refresh_claims.jti.clone(),
            family_id: family_id.clone(),
            user_id: user.id,
            expires_at: chrono::DateTime::from_timestamp(refresh_claims.exp as i64, 0)
                .ok_or_else(|| AppError::internal("invalid refresh exp"))?,
            used: false,
            revoked: false,
        },
    );

    drop(refresh_tokens);

    state.audit_logs.write().await.push(AuditLog {
        id: Uuid::new_v4(),
        actor_wallet: user.wallet_address.clone(),
        action: "auth.refresh".to_string(),
        target_id: Some(user.id.to_string()),
        tx_hash: None,
        metadata: json!({
            "family_id": family_id,
            "access_jti": access_claims.jti,
            "refresh_jti": refresh_claims.jti
        }),
        created_at: Utc::now(),
    });

    Ok(Json(AuthTokensResponse {
        access_token,
        refresh_token,
        token_type: "Bearer".to_string(),
        expires_in: state.config.jwt_expires_in_seconds,
        refresh_expires_in: state.config.refresh_expires_in_seconds,
        user_id: user.id.to_string(),
        role: user.role,
        family_id,
    }))
}

pub async fn logout(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<LogoutRequest>,
) -> AppResult<Json<GenericOkResponse>> {
    state.cleanup_expired().await;

    let claims = decode_jwt(&payload.refresh_token, &state.config.jwt_secret)
        .map_err(|_| AppError::unauthorized("Invalid refresh token"))?;

    if claims.typ != TokenType::Refresh {
        return Err(AppError::unauthorized("Refresh token required"));
    }

    if claims.sub != auth.user_id.to_string() {
        return Err(AppError::forbidden("Cannot logout another session"));
    }

    state
        .revoked_token_families
        .write()
        .await
        .insert(claims.fid.clone(), Utc::now());
    state
        .revoked_access_jtis
        .write()
        .await
        .insert(auth.claims.jti.clone(), Utc::now());

    state.audit_logs.write().await.push(AuditLog {
        id: Uuid::new_v4(),
        actor_wallet: auth.wallet,
        action: "auth.logout".to_string(),
        target_id: Some(auth.user_id.to_string()),
        tx_hash: None,
        metadata: json!({ "family_id": claims.fid }),
        created_at: Utc::now(),
    });

    Ok(Json(GenericOkResponse { ok: true }))
}

pub async fn revoke(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<RevokeRequest>,
) -> AppResult<Json<GenericOkResponse>> {
    state.cleanup_expired().await;

    if let Some(family_id) = payload.family_id {
        if auth.claims.fid != family_id {
            require_admin(&auth)?;
        }
        state
            .revoked_token_families
            .write()
            .await
            .insert(family_id, Utc::now());
        return Ok(Json(GenericOkResponse { ok: true }));
    }

    if let Some(user_id) = payload.user_id {
        require_admin(&auth)?;
        let uid = user_id
            .parse::<Uuid>()
            .map_err(|_| AppError::bad_request("invalid user_id"))?;

        let refresh_tokens = state.refresh_tokens.read().await;
        let family_ids = refresh_tokens
            .values()
            .filter(|r| r.user_id == uid)
            .map(|r| r.family_id.clone())
            .collect::<Vec<_>>();
        drop(refresh_tokens);

        let mut revoked = state.revoked_token_families.write().await;
        for fid in family_ids {
            revoked.insert(fid, Utc::now());
        }
        return Ok(Json(GenericOkResponse { ok: true }));
    }

    Err(AppError::bad_request("family_id or user_id is required"))
}
