use axum::{extract::State, Json};
use chrono::{Duration, Utc};
use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::Row;
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

fn role_from_db(role: &str) -> AppResult<Role> {
    match role {
        "admin" => Ok(Role::Admin),
        "auditor" => Ok(Role::Auditor),
        "user" => Ok(Role::User),
        _ => Err(AppError::internal("invalid role in database")),
    }
}

pub async fn challenge(
    State(state): State<AppState>,
    Json(payload): Json<ChallengeRequest>,
) -> AppResult<Json<ChallengeResponse>> {
    if state.config.auth_pg_enabled {
        challenge_pg(state, payload).await
    } else {
        challenge_in_memory(state, payload).await
    }
}

async fn challenge_in_memory(
    state: AppState,
    payload: ChallengeRequest,
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

async fn challenge_pg(state: AppState, payload: ChallengeRequest) -> AppResult<Json<ChallengeResponse>> {
    let pool = state
        .pg_pool
        .as_ref()
        .ok_or_else(|| AppError::internal("postgres pool is not initialized"))?;

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
    let created_at = Utc::now();
    let expires_at = created_at + Duration::seconds(state.config.challenge_ttl_seconds);
    let message = format!(
        "Expense Tracker login\nwallet:{}\nnonce:{}\nchallenge_id:{}\nexpires:{}",
        wallet,
        nonce,
        challenge_id,
        expires_at.to_rfc3339()
    );

    sqlx::query(
        "INSERT INTO auth_challenges (id, wallet_address, nonce, message, expires_at, used, created_at) \
         VALUES ($1, $2, $3, $4, $5, FALSE, $6)",
    )
    .bind(challenge_id)
    .bind(wallet)
    .bind(&nonce)
    .bind(&message)
    .bind(expires_at)
    .bind(created_at)
    .execute(pool)
    .await
    .map_err(|_| AppError::internal("failed to create challenge"))?;

    Ok(Json(ChallengeResponse {
        challenge_id: challenge_id.to_string(),
        wallet_address: wallet.to_string(),
        message,
        nonce,
        expires_at: expires_at.to_rfc3339(),
    }))
}

pub async fn verify(
    State(state): State<AppState>,
    Json(payload): Json<VerifyRequest>,
) -> AppResult<Json<AuthTokensResponse>> {
    if state.config.auth_pg_enabled {
        verify_pg(state, payload).await
    } else {
        verify_in_memory(state, payload).await
    }
}

async fn verify_in_memory(state: AppState, payload: VerifyRequest) -> AppResult<Json<AuthTokensResponse>> {
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
        state
            .users_by_wallet
            .write()
            .await
            .insert(wallet.clone(), id);
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

async fn verify_pg(state: AppState, payload: VerifyRequest) -> AppResult<Json<AuthTokensResponse>> {
    let pool = state
        .pg_pool
        .as_ref()
        .ok_or_else(|| AppError::internal("postgres pool is not initialized"))?;

    let challenge_id = payload
        .challenge_id
        .parse::<Uuid>()
        .map_err(|_| AppError::bad_request("invalid challenge_id"))?;

    let mut tx = pool
        .begin()
        .await
        .map_err(|_| AppError::internal("failed to begin verify transaction"))?;

    let challenge_row = sqlx::query(
        "SELECT wallet_address, message, expires_at, used \
         FROM auth_challenges \
         WHERE id = $1 \
         FOR UPDATE",
    )
    .bind(challenge_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| AppError::internal("failed to read challenge"))?
    .ok_or_else(|| AppError::unauthorized("challenge not found"))?;

    let challenge_wallet: String = challenge_row
        .try_get("wallet_address")
        .map_err(|_| AppError::internal("invalid challenge row"))?;
    let challenge_message: String = challenge_row
        .try_get("message")
        .map_err(|_| AppError::internal("invalid challenge row"))?;
    let challenge_expires_at: chrono::DateTime<Utc> = challenge_row
        .try_get("expires_at")
        .map_err(|_| AppError::internal("invalid challenge row"))?;
    let challenge_used: bool = challenge_row
        .try_get("used")
        .map_err(|_| AppError::internal("invalid challenge row"))?;

    if challenge_used {
        return Err(AppError::unauthorized("challenge already used"));
    }
    if Utc::now() > challenge_expires_at {
        return Err(AppError::unauthorized("challenge expired"));
    }
    if challenge_wallet != payload.wallet_address {
        return Err(AppError::unauthorized("wallet mismatch"));
    }
    if !verify_signature_base58(
        &payload.wallet_address,
        &payload.signature,
        &challenge_message,
    ) {
        return Err(AppError::unauthorized("invalid signature"));
    }

    sqlx::query("UPDATE auth_challenges SET used = TRUE WHERE id = $1")
        .bind(challenge_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| AppError::internal("failed to update challenge"))?;

    let user_row = sqlx::query(
        "INSERT INTO users (id, wallet_address, role, created_at) \
         VALUES ($1, $2, 'user', $3) \
         ON CONFLICT (wallet_address) DO NOTHING \
         RETURNING id, wallet_address, role, created_at",
    )
    .bind(Uuid::new_v4())
    .bind(&payload.wallet_address)
    .bind(Utc::now())
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| AppError::internal("failed to upsert user"))?;

    let user_row = if let Some(row) = user_row {
        row
    } else {
        sqlx::query("SELECT id, wallet_address, role, created_at FROM users WHERE wallet_address = $1")
            .bind(&payload.wallet_address)
            .fetch_one(&mut *tx)
            .await
            .map_err(|_| AppError::internal("failed to load user"))?
    };

    let user_id: Uuid = user_row
        .try_get("id")
        .map_err(|_| AppError::internal("invalid user row"))?;
    let wallet_address: String = user_row
        .try_get("wallet_address")
        .map_err(|_| AppError::internal("invalid user row"))?;
    let role_raw: String = user_row
        .try_get("role")
        .map_err(|_| AppError::internal("invalid user row"))?;
    let role = role_from_db(&role_raw)?;

    let family_id = Uuid::new_v4().to_string();

    let (access_token, access_claims) = encode_access_jwt(
        user_id,
        &wallet_address,
        &role,
        &family_id,
        &state.config.jwt_secret,
        state.config.jwt_expires_in_seconds,
    )
    .map_err(|_| AppError::internal("failed to create access token"))?;

    let (refresh_token, refresh_claims) = encode_refresh_jwt(
        user_id,
        &wallet_address,
        &role,
        &family_id,
        &state.config.jwt_secret,
        state.config.refresh_expires_in_seconds,
    )
    .map_err(|_| AppError::internal("failed to create refresh token"))?;

    let refresh_expires_at = chrono::DateTime::from_timestamp(refresh_claims.exp as i64, 0)
        .ok_or_else(|| AppError::internal("invalid refresh exp"))?;

    sqlx::query(
        "INSERT INTO auth_refresh_sessions (jti, family_id, user_id, expires_at, used, revoked, created_at) \
         VALUES ($1, $2, $3, $4, FALSE, FALSE, $5)",
    )
    .bind(&refresh_claims.jti)
    .bind(&family_id)
    .bind(user_id)
    .bind(refresh_expires_at)
    .bind(Utc::now())
    .execute(&mut *tx)
    .await
    .map_err(|_| AppError::internal("failed to persist refresh session"))?;

    sqlx::query(
        "INSERT INTO tx_audit_logs (id, actor_wallet, action, target_id, tx_hash, metadata, created_at) \
         VALUES ($1, $2, $3, $4, NULL, $5, $6)",
    )
    .bind(Uuid::new_v4())
    .bind(&wallet_address)
    .bind("auth.verify")
    .bind(user_id.to_string())
    .bind(json!({
        "challenge_id": challenge_id.to_string(),
        "family_id": family_id,
        "access_jti": access_claims.jti,
        "refresh_jti": refresh_claims.jti
    }))
    .bind(Utc::now())
    .execute(&mut *tx)
    .await
    .map_err(|_| AppError::internal("failed to write auth audit log"))?;

    tx.commit()
        .await
        .map_err(|_| AppError::internal("failed to commit verify transaction"))?;

    Ok(Json(AuthTokensResponse {
        access_token,
        refresh_token,
        token_type: "Bearer".to_string(),
        expires_in: state.config.jwt_expires_in_seconds,
        refresh_expires_in: state.config.refresh_expires_in_seconds,
        user_id: user_id.to_string(),
        role,
        family_id,
    }))
}

pub async fn refresh(
    State(state): State<AppState>,
    Json(payload): Json<RefreshRequest>,
) -> AppResult<Json<AuthTokensResponse>> {
    if state.config.auth_pg_enabled {
        refresh_pg(state, payload).await
    } else {
        refresh_in_memory(state, payload).await
    }
}

async fn refresh_in_memory(state: AppState, payload: RefreshRequest) -> AppResult<Json<AuthTokensResponse>> {
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

async fn refresh_pg(state: AppState, payload: RefreshRequest) -> AppResult<Json<AuthTokensResponse>> {
    let pool = state
        .pg_pool
        .as_ref()
        .ok_or_else(|| AppError::internal("postgres pool is not initialized"))?;

    let claims = decode_jwt(&payload.refresh_token, &state.config.jwt_secret)
        .map_err(|_| AppError::unauthorized("Invalid refresh token"))?;

    if claims.typ != TokenType::Refresh {
        return Err(AppError::unauthorized("Refresh token required"));
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(|_| AppError::internal("failed to begin refresh transaction"))?;

    let family_revoked = sqlx::query("SELECT 1 FROM revoked_token_families WHERE family_id = $1")
        .bind(&claims.fid)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_| AppError::internal("failed to check revoked family"))?
        .is_some();

    if family_revoked {
        return Err(AppError::unauthorized("Session revoked"));
    }

    let current_row = sqlx::query(
        "SELECT jti, family_id, user_id, expires_at, used, revoked \
         FROM auth_refresh_sessions \
         WHERE jti = $1 \
         FOR UPDATE",
    )
    .bind(&claims.jti)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| AppError::internal("failed to load refresh session"))?
    .ok_or_else(|| AppError::unauthorized("Unknown refresh token"))?;

    let current_jti: String = current_row
        .try_get("jti")
        .map_err(|_| AppError::internal("invalid refresh row"))?;
    let current_family_id: String = current_row
        .try_get("family_id")
        .map_err(|_| AppError::internal("invalid refresh row"))?;
    let current_user_id: Uuid = current_row
        .try_get("user_id")
        .map_err(|_| AppError::internal("invalid refresh row"))?;
    let current_expires_at: chrono::DateTime<Utc> = current_row
        .try_get("expires_at")
        .map_err(|_| AppError::internal("invalid refresh row"))?;
    let current_used: bool = current_row
        .try_get("used")
        .map_err(|_| AppError::internal("invalid refresh row"))?;
    let current_revoked: bool = current_row
        .try_get("revoked")
        .map_err(|_| AppError::internal("invalid refresh row"))?;

    if current_revoked || Utc::now() > current_expires_at {
        return Err(AppError::unauthorized("Refresh token expired/revoked"));
    }

    if current_used {
        sqlx::query(
            "INSERT INTO revoked_token_families (family_id, revoked_at) \
             VALUES ($1, $2) \
             ON CONFLICT (family_id) DO NOTHING",
        )
        .bind(&current_family_id)
        .bind(Utc::now())
        .execute(&mut *tx)
        .await
        .map_err(|_| AppError::internal("failed to revoke token family"))?;

        tx.commit()
            .await
            .map_err(|_| AppError::internal("failed to commit refresh transaction"))?;

        return Err(AppError::unauthorized("Refresh token reuse detected"));
    }

    sqlx::query("UPDATE auth_refresh_sessions SET used = TRUE WHERE jti = $1")
        .bind(&current_jti)
        .execute(&mut *tx)
        .await
        .map_err(|_| AppError::internal("failed to rotate refresh session"))?;

    let user_row = sqlx::query("SELECT id, wallet_address, role FROM users WHERE id = $1")
        .bind(current_user_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| AppError::unauthorized("User not found"))?;

    let user_id: Uuid = user_row
        .try_get("id")
        .map_err(|_| AppError::internal("invalid user row"))?;
    let wallet_address: String = user_row
        .try_get("wallet_address")
        .map_err(|_| AppError::internal("invalid user row"))?;
    let role_raw: String = user_row
        .try_get("role")
        .map_err(|_| AppError::internal("invalid user row"))?;
    let role = role_from_db(&role_raw)?;

    let (access_token, access_claims) = encode_access_jwt(
        user_id,
        &wallet_address,
        &role,
        &current_family_id,
        &state.config.jwt_secret,
        state.config.jwt_expires_in_seconds,
    )
    .map_err(|_| AppError::internal("failed to create access token"))?;

    let (refresh_token, refresh_claims) = encode_refresh_jwt(
        user_id,
        &wallet_address,
        &role,
        &current_family_id,
        &state.config.jwt_secret,
        state.config.refresh_expires_in_seconds,
    )
    .map_err(|_| AppError::internal("failed to create refresh token"))?;

    let refresh_expires_at = chrono::DateTime::from_timestamp(refresh_claims.exp as i64, 0)
        .ok_or_else(|| AppError::internal("invalid refresh exp"))?;

    sqlx::query(
        "INSERT INTO auth_refresh_sessions (jti, family_id, user_id, expires_at, used, revoked, created_at) \
         VALUES ($1, $2, $3, $4, FALSE, FALSE, $5)",
    )
    .bind(&refresh_claims.jti)
    .bind(&current_family_id)
    .bind(user_id)
    .bind(refresh_expires_at)
    .bind(Utc::now())
    .execute(&mut *tx)
    .await
    .map_err(|_| AppError::internal("failed to persist rotated refresh session"))?;

    sqlx::query(
        "INSERT INTO tx_audit_logs (id, actor_wallet, action, target_id, tx_hash, metadata, created_at) \
         VALUES ($1, $2, $3, $4, NULL, $5, $6)",
    )
    .bind(Uuid::new_v4())
    .bind(&wallet_address)
    .bind("auth.refresh")
    .bind(user_id.to_string())
    .bind(json!({
        "family_id": current_family_id,
        "access_jti": access_claims.jti,
        "refresh_jti": refresh_claims.jti
    }))
    .bind(Utc::now())
    .execute(&mut *tx)
    .await
    .map_err(|_| AppError::internal("failed to write auth audit log"))?;

    tx.commit()
        .await
        .map_err(|_| AppError::internal("failed to commit refresh transaction"))?;

    Ok(Json(AuthTokensResponse {
        access_token,
        refresh_token,
        token_type: "Bearer".to_string(),
        expires_in: state.config.jwt_expires_in_seconds,
        refresh_expires_in: state.config.refresh_expires_in_seconds,
        user_id: user_id.to_string(),
        role,
        family_id: claims.fid,
    }))
}

pub async fn logout(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<LogoutRequest>,
) -> AppResult<Json<GenericOkResponse>> {
    if state.config.auth_pg_enabled {
        logout_pg(state, auth, payload).await
    } else {
        logout_in_memory(state, auth, payload).await
    }
}

async fn logout_in_memory(
    state: AppState,
    auth: AuthUser,
    payload: LogoutRequest,
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

async fn logout_pg(
    state: AppState,
    auth: AuthUser,
    payload: LogoutRequest,
) -> AppResult<Json<GenericOkResponse>> {
    let pool = state
        .pg_pool
        .as_ref()
        .ok_or_else(|| AppError::internal("postgres pool is not initialized"))?;

    let claims = decode_jwt(&payload.refresh_token, &state.config.jwt_secret)
        .map_err(|_| AppError::unauthorized("Invalid refresh token"))?;

    if claims.typ != TokenType::Refresh {
        return Err(AppError::unauthorized("Refresh token required"));
    }

    if claims.sub != auth.user_id.to_string() {
        return Err(AppError::forbidden("Cannot logout another session"));
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(|_| AppError::internal("failed to begin logout transaction"))?;

    sqlx::query(
        "INSERT INTO revoked_token_families (family_id, revoked_at) \
         VALUES ($1, $2) \
         ON CONFLICT (family_id) DO NOTHING",
    )
    .bind(&claims.fid)
    .bind(Utc::now())
    .execute(&mut *tx)
    .await
    .map_err(|_| AppError::internal("failed to revoke token family"))?;

    sqlx::query(
        "INSERT INTO revoked_access_tokens (jti, revoked_at) \
         VALUES ($1, $2) \
         ON CONFLICT (jti) DO NOTHING",
    )
    .bind(&auth.claims.jti)
    .bind(Utc::now())
    .execute(&mut *tx)
    .await
    .map_err(|_| AppError::internal("failed to revoke access token"))?;

    sqlx::query("UPDATE auth_refresh_sessions SET revoked = TRUE WHERE family_id = $1")
        .bind(&claims.fid)
        .execute(&mut *tx)
        .await
        .map_err(|_| AppError::internal("failed to revoke refresh sessions"))?;

    sqlx::query(
        "INSERT INTO tx_audit_logs (id, actor_wallet, action, target_id, tx_hash, metadata, created_at) \
         VALUES ($1, $2, $3, $4, NULL, $5, $6)",
    )
    .bind(Uuid::new_v4())
    .bind(&auth.wallet)
    .bind("auth.logout")
    .bind(auth.user_id.to_string())
    .bind(json!({ "family_id": claims.fid }))
    .bind(Utc::now())
    .execute(&mut *tx)
    .await
    .map_err(|_| AppError::internal("failed to write logout audit log"))?;

    tx.commit()
        .await
        .map_err(|_| AppError::internal("failed to commit logout transaction"))?;

    Ok(Json(GenericOkResponse { ok: true }))
}

pub async fn revoke(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<RevokeRequest>,
) -> AppResult<Json<GenericOkResponse>> {
    if state.config.auth_pg_enabled {
        revoke_pg(state, auth, payload).await
    } else {
        revoke_in_memory(state, auth, payload).await
    }
}

async fn revoke_in_memory(
    state: AppState,
    auth: AuthUser,
    payload: RevokeRequest,
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

async fn revoke_pg(
    state: AppState,
    auth: AuthUser,
    payload: RevokeRequest,
) -> AppResult<Json<GenericOkResponse>> {
    let pool = state
        .pg_pool
        .as_ref()
        .ok_or_else(|| AppError::internal("postgres pool is not initialized"))?;

    if let Some(family_id) = payload.family_id {
        if auth.claims.fid != family_id {
            require_admin(&auth)?;
        }

        sqlx::query(
            "INSERT INTO revoked_token_families (family_id, revoked_at) \
             VALUES ($1, $2) \
             ON CONFLICT (family_id) DO NOTHING",
        )
        .bind(&family_id)
        .bind(Utc::now())
        .execute(pool)
        .await
        .map_err(|_| AppError::internal("failed to revoke token family"))?;

        sqlx::query("UPDATE auth_refresh_sessions SET revoked = TRUE WHERE family_id = $1")
            .bind(&family_id)
            .execute(pool)
            .await
            .map_err(|_| AppError::internal("failed to revoke refresh sessions"))?;

        return Ok(Json(GenericOkResponse { ok: true }));
    }

    if let Some(user_id) = payload.user_id {
        require_admin(&auth)?;
        let uid = user_id
            .parse::<Uuid>()
            .map_err(|_| AppError::bad_request("invalid user_id"))?;

        sqlx::query(
            "INSERT INTO revoked_token_families (family_id, revoked_at) \
             SELECT DISTINCT family_id, $2 \
             FROM auth_refresh_sessions \
             WHERE user_id = $1 \
             ON CONFLICT (family_id) DO NOTHING",
        )
        .bind(uid)
        .bind(Utc::now())
        .execute(pool)
        .await
        .map_err(|_| AppError::internal("failed to revoke user token families"))?;

        sqlx::query("UPDATE auth_refresh_sessions SET revoked = TRUE WHERE user_id = $1")
            .bind(uid)
            .execute(pool)
            .await
            .map_err(|_| AppError::internal("failed to revoke user refresh sessions"))?;

        return Ok(Json(GenericOkResponse { ok: true }));
    }

    Err(AppError::bad_request("family_id or user_id is required"))
}
