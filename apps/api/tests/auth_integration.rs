use axum::{body::Body, http::{Request, StatusCode}};
use ed25519_dalek::{Signer, SigningKey};
use expense_tracker_api::{
    build_app,
    config::AppConfig,
    models::{RefreshTokenRecord, Role, User},
    routes::auth::{AuthTokensResponse, ChallengeResponse},
    state::AppState,
};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

async fn call_json(app: &axum::Router, req: Request<Body>) -> (StatusCode, Value) {
    let res = app.clone().oneshot(req).await.expect("request failed");
    let status = res.status();
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let value = serde_json::from_slice::<Value>(&body).unwrap_or(json!({}));
    (status, value)
}

fn app_state_for_test() -> AppState {
    let cfg = AppConfig::from_env();
    AppState::new(cfg)
}

#[tokio::test]
async fn refresh_reuse_revokes_family() {
    let state = app_state_for_test();

    let family_id = Uuid::new_v4().to_string();
    let user_id = Uuid::new_v4();
    let app = build_app(state.clone());

    let refresh_token = expense_tracker_api::security::jwt::encode_refresh_jwt(
        user_id,
        "wallet1",
        &Role::User,
        &family_id,
        &state.config.jwt_secret,
        3600,
    ).unwrap().0;

    let refresh_claims = expense_tracker_api::security::jwt::decode_jwt(
        &refresh_token,
        &state.config.jwt_secret,
    ).unwrap();

    state.refresh_tokens.write().await.insert(
        refresh_claims.jti.clone(),
        RefreshTokenRecord {
            jti: refresh_claims.jti,
            family_id: family_id.clone(),
            user_id,
            expires_at: chrono::Utc::now() + chrono::Duration::hours(1),
            used: true,
            revoked: false,
        },
    );

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/refresh")
        .header("content-type", "application/json")
        .body(Body::from(json!({ "refresh_token": refresh_token }).to_string()))
        .unwrap();

    let (status, _) = call_json(&app, req).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let revoked = state.revoked_token_families.read().await;
    assert!(revoked.contains_key(&family_id));
}

#[tokio::test]
async fn cleanup_expired_removes_old_records() {
    let state = app_state_for_test();

    state.auth_challenges.write().await.insert(
        Uuid::new_v4(),
        expense_tracker_api::models::AuthChallenge {
            challenge_id: Uuid::new_v4(),
            wallet_address: "wallet".to_string(),
            nonce: "nonce".to_string(),
            message: "msg".to_string(),
            expires_at: chrono::Utc::now() - chrono::Duration::seconds(10),
            used: false,
        },
    );

    state.refresh_tokens.write().await.insert(
        "expired-jti".to_string(),
        RefreshTokenRecord {
            jti: "expired-jti".to_string(),
            family_id: "family-x".to_string(),
            user_id: Uuid::new_v4(),
            expires_at: chrono::Utc::now() - chrono::Duration::seconds(10),
            used: false,
            revoked: false,
        },
    );

    state.revoked_access_jtis.write().await.insert(
        "access-jti".to_string(),
        chrono::Utc::now() - chrono::Duration::hours(10),
    );

    state.revoked_token_families.write().await.insert(
        "family-x".to_string(),
        chrono::Utc::now() - chrono::Duration::days(10),
    );

    state.cleanup_expired().await;

    assert!(state.auth_challenges.read().await.is_empty());
    assert!(state.refresh_tokens.read().await.is_empty());
    assert!(state.revoked_access_jtis.read().await.is_empty());
    assert!(state.revoked_token_families.read().await.is_empty());
}

#[tokio::test]
async fn audit_logs_requires_admin_or_auditor() {
    let state = app_state_for_test();
    let app = build_app(state.clone());

    let (user_token, _) = expense_tracker_api::security::jwt::encode_access_jwt(
        Uuid::new_v4(),
        "wallet-user",
        &Role::User,
        &Uuid::new_v4().to_string(),
        &state.config.jwt_secret,
        3600,
    ).unwrap();

    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/audit/logs")
        .header("authorization", format!("Bearer {}", user_token))
        .body(Body::empty())
        .unwrap();

    let (status, _) = call_json(&app, req).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn refresh_route_returns_tokens() {
    let state = app_state_for_test();
    let app = build_app(state.clone());

    let user_id = Uuid::new_v4();
    let family_id = Uuid::new_v4().to_string();

    state.users.write().await.insert(user_id, expense_tracker_api::models::User {
        id: user_id,
        wallet_address: "wallet-ok".to_string(),
        role: Role::User,
        created_at: chrono::Utc::now(),
    });

    let (refresh_token, claims) = expense_tracker_api::security::jwt::encode_refresh_jwt(
        user_id,
        "wallet-ok",
        &Role::User,
        &family_id,
        &state.config.jwt_secret,
        3600,
    ).unwrap();

    state.refresh_tokens.write().await.insert(
        claims.jti.clone(),
        RefreshTokenRecord {
            jti: claims.jti,
            family_id: family_id.clone(),
            user_id,
            expires_at: chrono::Utc::now() + chrono::Duration::hours(1),
            used: false,
            revoked: false,
        },
    );

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/refresh")
        .header("content-type", "application/json")
        .body(Body::from(json!({ "refresh_token": refresh_token }).to_string()))
        .unwrap();

    let (status, body) = call_json(&app, req).await;
    assert_eq!(status, StatusCode::OK);

    let parsed: AuthTokensResponse = serde_json::from_value(body).unwrap();
    assert!(!parsed.access_token.is_empty());
    assert!(!parsed.refresh_token.is_empty());
    assert_eq!(parsed.family_id, family_id);
}

#[tokio::test]
async fn challenge_endpoint_works() {
    let state = app_state_for_test();
    let app = build_app(state);

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/challenge")
        .header("content-type", "application/json")
        .body(Body::from(json!({ "wallet_address": "wallet-demo" }).to_string()))
        .unwrap();

    let (status, body) = call_json(&app, req).await;
    assert_eq!(status, StatusCode::OK);

    let challenge: ChallengeResponse = serde_json::from_value(body).unwrap();
    assert!(!challenge.challenge_id.is_empty());
    assert!(!challenge.message.is_empty());
}

#[tokio::test]
async fn full_flow_verify_refresh_logout_then_refresh_fail() {
    let state = app_state_for_test();
    let app = build_app(state.clone());

    let signing_key = SigningKey::from_bytes(&[7u8; 32]);
    let verify_key = signing_key.verifying_key();
    let wallet_address = bs58::encode(verify_key.as_bytes()).into_string();

    let challenge_req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/challenge")
        .header("content-type", "application/json")
        .body(Body::from(json!({ "wallet_address": wallet_address }).to_string()))
        .unwrap();
    let (challenge_status, challenge_body) = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        call_json(&app, challenge_req),
    )
    .await
    .expect("challenge request timed out");
    assert_eq!(challenge_status, StatusCode::OK);

    let challenge: ChallengeResponse = serde_json::from_value(challenge_body).unwrap();
    let signature = signing_key.sign(challenge.message.as_bytes());
    let signature_b58 = bs58::encode(signature.to_bytes()).into_string();

    let verify_req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/verify")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "challenge_id": challenge.challenge_id,
                "wallet_address": challenge.wallet_address,
                "signature": signature_b58
            })
            .to_string(),
        ))
        .unwrap();
    let (verify_status, verify_body) = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        call_json(&app, verify_req),
    )
    .await
    .expect("verify request timed out");
    assert_eq!(verify_status, StatusCode::OK);
    let verify_tokens: AuthTokensResponse = serde_json::from_value(verify_body).unwrap();

    let refresh_req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/refresh")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({ "refresh_token": verify_tokens.refresh_token }).to_string(),
        ))
        .unwrap();
    let (refresh_status, refresh_body) = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        call_json(&app, refresh_req),
    )
    .await
    .expect("refresh request timed out");
    assert_eq!(refresh_status, StatusCode::OK);
    let refreshed_tokens: AuthTokensResponse = serde_json::from_value(refresh_body).unwrap();

    let logout_req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/logout")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", refreshed_tokens.access_token))
        .body(Body::from(
            json!({ "refresh_token": refreshed_tokens.refresh_token }).to_string(),
        ))
        .unwrap();
    let (logout_status, _) = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        call_json(&app, logout_req),
    )
    .await
    .expect("logout request timed out");
    assert_eq!(logout_status, StatusCode::OK);

    let refresh_after_logout_req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/refresh")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({ "refresh_token": refreshed_tokens.refresh_token }).to_string(),
        ))
        .unwrap();
    let (refresh_after_logout_status, _) = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        call_json(&app, refresh_after_logout_req),
    )
    .await
    .expect("refresh-after-logout request timed out");
    assert_eq!(refresh_after_logout_status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn revoke_scope_self_and_admin() {
    let state = app_state_for_test();
    let app = build_app(state.clone());

    let user_id = Uuid::new_v4();
    let admin_id = Uuid::new_v4();
    let target_user_id = Uuid::new_v4();

    state.users.write().await.insert(
        user_id,
        User {
            id: user_id,
            wallet_address: "wallet-user".to_string(),
            role: Role::User,
            created_at: chrono::Utc::now(),
        },
    );

    state.users.write().await.insert(
        admin_id,
        User {
            id: admin_id,
            wallet_address: "wallet-admin".to_string(),
            role: Role::Admin,
            created_at: chrono::Utc::now(),
        },
    );

    state.users.write().await.insert(
        target_user_id,
        User {
            id: target_user_id,
            wallet_address: "wallet-target".to_string(),
            role: Role::User,
            created_at: chrono::Utc::now(),
        },
    );

    let self_family_id = Uuid::new_v4().to_string();
    let another_family_id = Uuid::new_v4().to_string();

    let (user_access, _) = expense_tracker_api::security::jwt::encode_access_jwt(
        user_id,
        "wallet-user",
        &Role::User,
        &self_family_id,
        &state.config.jwt_secret,
        3600,
    )
    .unwrap();

    let (user_access_other, _) = expense_tracker_api::security::jwt::encode_access_jwt(
        user_id,
        "wallet-user",
        &Role::User,
        &Uuid::new_v4().to_string(),
        &state.config.jwt_secret,
        3600,
    )
    .unwrap();

    let (admin_access, _) = expense_tracker_api::security::jwt::encode_access_jwt(
        admin_id,
        "wallet-admin",
        &Role::Admin,
        &Uuid::new_v4().to_string(),
        &state.config.jwt_secret,
        3600,
    )
    .unwrap();

    state.refresh_tokens.write().await.insert(
        "target-refresh-jti".to_string(),
        RefreshTokenRecord {
            jti: "target-refresh-jti".to_string(),
            family_id: another_family_id.clone(),
            user_id: target_user_id,
            expires_at: chrono::Utc::now() + chrono::Duration::hours(1),
            used: false,
            revoked: false,
        },
    );

    let self_revoke_req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/revoke")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", user_access))
        .body(Body::from(
            json!({ "family_id": self_family_id }).to_string(),
        ))
        .unwrap();
    let (self_revoke_status, _) = call_json(&app, self_revoke_req).await;
    assert_eq!(self_revoke_status, StatusCode::OK);

    let user_revoke_other_family_req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/revoke")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", user_access_other))
        .body(Body::from(
            json!({ "family_id": another_family_id }).to_string(),
        ))
        .unwrap();
    let (user_revoke_other_family_status, _) = call_json(&app, user_revoke_other_family_req).await;
    assert_eq!(user_revoke_other_family_status, StatusCode::FORBIDDEN);

    let admin_revoke_user_req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/revoke")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", admin_access))
        .body(Body::from(
            json!({ "user_id": target_user_id.to_string() }).to_string(),
        ))
        .unwrap();
    let (admin_revoke_user_status, _) = call_json(&app, admin_revoke_user_req).await;
    assert_eq!(admin_revoke_user_status, StatusCode::OK);

    let revoked = state.revoked_token_families.read().await;
    assert!(revoked.contains_key(&another_family_id));
}
