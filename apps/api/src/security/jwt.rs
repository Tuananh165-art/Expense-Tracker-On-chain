use crate::models::Role;
use chrono::Utc;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TokenType {
    Access,
    Refresh,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub wallet: String,
    pub role: String,
    pub jti: String,
    pub fid: String,
    pub typ: TokenType,
    pub exp: usize,
    pub iat: usize,
}

fn role_to_str(role: &Role) -> &'static str {
    match role {
        Role::User => "user",
        Role::Admin => "admin",
        Role::Auditor => "auditor",
    }
}

fn encode_token(claims: Claims, secret: &str) -> Result<String, jsonwebtoken::errors::Error> {
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
}

pub fn encode_access_jwt(
    user_id: Uuid,
    wallet: &str,
    role: &Role,
    family_id: &str,
    secret: &str,
    expires_in_seconds: i64,
) -> Result<(String, Claims), jsonwebtoken::errors::Error> {
    let now = Utc::now().timestamp() as usize;
    let claims = Claims {
        sub: user_id.to_string(),
        wallet: wallet.to_string(),
        role: role_to_str(role).to_string(),
        jti: Uuid::new_v4().to_string(),
        fid: family_id.to_string(),
        typ: TokenType::Access,
        exp: (Utc::now().timestamp() + expires_in_seconds) as usize,
        iat: now,
    };
    let token = encode_token(claims.clone(), secret)?;
    Ok((token, claims))
}

pub fn encode_refresh_jwt(
    user_id: Uuid,
    wallet: &str,
    role: &Role,
    family_id: &str,
    secret: &str,
    expires_in_seconds: i64,
) -> Result<(String, Claims), jsonwebtoken::errors::Error> {
    let now = Utc::now().timestamp() as usize;
    let claims = Claims {
        sub: user_id.to_string(),
        wallet: wallet.to_string(),
        role: role_to_str(role).to_string(),
        jti: Uuid::new_v4().to_string(),
        fid: family_id.to_string(),
        typ: TokenType::Refresh,
        exp: (Utc::now().timestamp() + expires_in_seconds) as usize,
        iat: now,
    };
    let token = encode_token(claims.clone(), secret)?;
    Ok((token, claims))
}

pub fn decode_jwt(token: &str, secret: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )?;
    Ok(data.claims)
}
