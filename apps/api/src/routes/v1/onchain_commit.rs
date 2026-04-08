use axum::{
    extract::{Path, State},
    Json,
};
use bs58;
use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use solana_pubkey::Pubkey;
use sqlx::Row;
use std::str::FromStr;
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    authz::{require_admin, require_user_or_admin},
    error::{AppError, AppResult},
    models::ExpenseStatus,
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct CommitCategoryRequest {
    pub tx_hash: String,
    pub category_name: String,
    pub client_ref_id: Option<String>,
    pub rpc_url_override: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CommitExpenseCreateRequest {
    pub tx_hash: String,
    pub expense_id_onchain: u64,
    pub category_pda: String,
    pub amount_minor: i64,
    pub currency: String,
    pub occurred_at: Option<String>,
    pub client_ref_id: Option<String>,
    pub rpc_url_override: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CommitExpenseStatusRequest {
    pub tx_hash: String,
    pub to_status: String,
    pub reason: Option<String>,
    pub client_ref_id: Option<String>,
    pub rpc_url_override: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OnchainCommitResponse {
    pub ok: bool,
    pub tx_hash: String,
    pub commitment: String,
    pub slot: u64,
    pub program_id: String,
    pub action: String,
    pub target_id: String,
    pub audit_log_id: String,
    pub metadata: Value,
}

#[derive(Debug)]
struct VerifiedInstruction {
    slot: u64,
    signer_wallet: String,
    account_keys: Vec<String>,
    account_indices: Vec<usize>,
    data_raw: Vec<u8>,
}

pub async fn commit_category(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<CommitCategoryRequest>,
) -> AppResult<Json<OnchainCommitResponse>> {
    require_user_or_admin(&auth)?;
    ensure_hybrid_onchain_enabled(&state)?;
    ensure_pg_mode(&state)?;

    let tx_hash = validate_signature_base58(&payload.tx_hash)?;
    let category_name = payload.category_name.trim();
    if category_name.is_empty() {
        return Err(AppError::bad_request("category_name is required"));
    }

    let program_id = parse_program_id(&state.config.solana_program_id)?;
    let owner = parse_wallet_pubkey(&auth.wallet)?;
    let rpc_url = resolve_rpc_url(&state, payload.rpc_url_override.as_deref())?;

    let verified = verify_instruction_from_tx(
        &state,
        &rpc_url,
        &tx_hash,
        &program_id,
        "create_category",
        &auth.wallet,
    )
    .await?;

    let decoded_name = decode_create_category_name(&verified.data_raw)?;
    if decoded_name != category_name {
        return Err(AppError::bad_request("category_name does not match onchain instruction"));
    }

    let user_profile_pda = Pubkey::find_program_address(&[b"user_profile", owner.as_ref()], &program_id).0;
    let category_pda = Pubkey::find_program_address(
        &[b"category", owner.as_ref(), category_name.as_bytes()],
        &program_id,
    )
    .0;

    ensure_account_at(&verified, 0, &auth.wallet)?;
    ensure_account_at(&verified, 1, &user_profile_pda.to_string())?;
    ensure_account_at(&verified, 2, &category_pda.to_string())?;

    let pool = state
        .pg_pool
        .as_ref()
        .ok_or_else(|| AppError::internal("postgres pool is not initialized"))?;

    let now = Utc::now();
    let category_id = Uuid::new_v4();
    let audit_log_id = Uuid::new_v4();

    let mut tx = pool
        .begin()
        .await
        .map_err(|_| AppError::internal("failed to start transaction"))?;

    let existing = sqlx::query(
        "SELECT id, owner_user_id, name, created_at, onchain_slot FROM categories WHERE tx_hash = $1",
    )
    .bind(&tx_hash)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| AppError::internal("failed to check duplicate category tx"))?;

    if let Some(row) = existing {
        let existing_id = row
            .try_get::<Uuid, _>("id")
            .map_err(|_| AppError::internal("invalid category row"))?;
        let existing_owner = row
            .try_get::<Uuid, _>("owner_user_id")
            .map_err(|_| AppError::internal("invalid category row"))?;
        let existing_name: String = row
            .try_get("name")
            .map_err(|_| AppError::internal("invalid category row"))?;
        let slot = row
            .try_get::<Option<i64>, _>("onchain_slot")
            .map_err(|_| AppError::internal("invalid category row"))?
            .unwrap_or(verified.slot as i64) as u64;

        let metadata = json!({
            "replayed": true,
            "owner_user_id": existing_owner.to_string(),
            "category_name": existing_name,
            "onchain_category_pda": category_pda.to_string(),
            "signer_wallet": verified.signer_wallet,
            "client_ref_id": payload.client_ref_id,
        });

        tx.rollback()
            .await
            .map_err(|_| AppError::internal("failed to rollback transaction"))?;

        return Ok(Json(OnchainCommitResponse {
            ok: true,
            tx_hash,
            commitment: state.config.solana_commitment.clone(),
            slot,
            program_id: state.config.solana_program_id.clone(),
            action: "category.create".to_string(),
            target_id: existing_id.to_string(),
            audit_log_id: Uuid::nil().to_string(),
            metadata,
        }));
    }

    sqlx::query(
        "INSERT INTO categories (id, owner_user_id, name, tx_hash, onchain_category_pda, onchain_slot, onchain_committed_at, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(category_id)
    .bind(auth.user_id)
    .bind(category_name)
    .bind(&tx_hash)
    .bind(category_pda.to_string())
    .bind(verified.slot as i64)
    .bind(now)
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(|_| AppError::internal("failed to insert category commit"))?;

    let metadata = json!({
        "owner_user_id": auth.user_id.to_string(),
        "category_name": category_name,
        "onchain_category_pda": category_pda.to_string(),
        "signer_wallet": verified.signer_wallet,
        "client_ref_id": payload.client_ref_id,
    });

    sqlx::query(
        "INSERT INTO tx_audit_logs (id, actor_wallet, action, target_id, tx_hash, metadata, onchain_verified, onchain_program_id, onchain_slot, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
    )
    .bind(audit_log_id)
    .bind(&auth.wallet)
    .bind("category.create")
    .bind(category_id.to_string())
    .bind(&tx_hash)
    .bind(metadata.clone())
    .bind(true)
    .bind(&state.config.solana_program_id)
    .bind(verified.slot as i64)
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(|_| AppError::internal("failed to append category audit log"))?;

    tx.commit()
        .await
        .map_err(|_| AppError::internal("failed to commit category tx"))?;

    Ok(Json(OnchainCommitResponse {
        ok: true,
        tx_hash,
        commitment: state.config.solana_commitment.clone(),
        slot: verified.slot,
        program_id: state.config.solana_program_id.clone(),
        action: "category.create".to_string(),
        target_id: category_id.to_string(),
        audit_log_id: audit_log_id.to_string(),
        metadata,
    }))
}

pub async fn commit_expense_create(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<CommitExpenseCreateRequest>,
) -> AppResult<Json<OnchainCommitResponse>> {
    require_user_or_admin(&auth)?;
    ensure_hybrid_onchain_enabled(&state)?;
    ensure_pg_mode(&state)?;

    let tx_hash = validate_signature_base58(&payload.tx_hash)?;
    let program_id = parse_program_id(&state.config.solana_program_id)?;
    let owner = parse_wallet_pubkey(&auth.wallet)?;
    let category_pda = parse_wallet_pubkey(&payload.category_pda)?;
    let rpc_url = resolve_rpc_url(&state, payload.rpc_url_override.as_deref())?;

    if payload.amount_minor <= 0 {
        return Err(AppError::bad_request("amount_minor must be > 0"));
    }
    let amount_onchain = u64::try_from(payload.amount_minor)
        .map_err(|_| AppError::bad_request("amount_minor out of range"))?;
    let currency = payload.currency.trim();
    if currency.is_empty() {
        return Err(AppError::bad_request("currency is required"));
    }

    let verified = verify_instruction_from_tx(
        &state,
        &rpc_url,
        &tx_hash,
        &program_id,
        "create_expense",
        &auth.wallet,
    )
    .await?;

    let (decoded_expense_id, decoded_amount) = decode_create_expense_args(&verified.data_raw)?;
    if decoded_expense_id != payload.expense_id_onchain {
        return Err(AppError::bad_request("expense_id_onchain does not match instruction"));
    }
    if decoded_amount != amount_onchain {
        return Err(AppError::bad_request("amount_minor does not match instruction amount"));
    }

    let user_profile_pda = Pubkey::find_program_address(&[b"user_profile", owner.as_ref()], &program_id).0;
    let expense_pda = Pubkey::find_program_address(
        &[b"expense", owner.as_ref(), &payload.expense_id_onchain.to_le_bytes()],
        &program_id,
    )
    .0;

    ensure_account_at(&verified, 0, &auth.wallet)?;
    ensure_account_at(&verified, 1, &user_profile_pda.to_string())?;
    ensure_account_at(&verified, 2, &category_pda.to_string())?;
    ensure_account_at(&verified, 3, &expense_pda.to_string())?;

    let pool = state
        .pg_pool
        .as_ref()
        .ok_or_else(|| AppError::internal("postgres pool is not initialized"))?;

    let now = Utc::now();
    let expense_id = Uuid::new_v4();
    let audit_log_id = Uuid::new_v4();

    let mut tx = pool
        .begin()
        .await
        .map_err(|_| AppError::internal("failed to start transaction"))?;

    let existing = sqlx::query(
        "SELECT id, owner_user_id, category_id, amount_minor, currency, status, tx_hash, occurred_at, created_at \
         FROM expenses_read_model WHERE tx_hash = $1",
    )
    .bind(&tx_hash)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| AppError::internal("failed to check duplicate expense tx"))?;

    if let Some(row) = existing {
        let existing_id = row
            .try_get::<Uuid, _>("id")
            .map_err(|_| AppError::internal("invalid expense row"))?;
        let existing_slot = verified.slot;
        let metadata = json!({
            "replayed": true,
            "onchain_expense_pda": expense_pda.to_string(),
            "category_pda": category_pda.to_string(),
            "signer_wallet": verified.signer_wallet,
            "client_ref_id": payload.client_ref_id,
        });

        tx.rollback()
            .await
            .map_err(|_| AppError::internal("failed to rollback transaction"))?;

        return Ok(Json(OnchainCommitResponse {
            ok: true,
            tx_hash,
            commitment: state.config.solana_commitment.clone(),
            slot: existing_slot,
            program_id: state.config.solana_program_id.clone(),
            action: "expense.create".to_string(),
            target_id: existing_id.to_string(),
            audit_log_id: Uuid::nil().to_string(),
            metadata,
        }));
    }

    let category_row = sqlx::query(
        "SELECT id, owner_user_id FROM categories WHERE onchain_category_pda = $1",
    )
    .bind(category_pda.to_string())
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| AppError::internal("failed to load category by onchain pda"))?
    .ok_or_else(|| AppError::bad_request("category commit not found for category_pda"))?;

    let category_id = category_row
        .try_get::<Uuid, _>("id")
        .map_err(|_| AppError::internal("invalid category row"))?;
    let category_owner_user_id = category_row
        .try_get::<Uuid, _>("owner_user_id")
        .map_err(|_| AppError::internal("invalid category row"))?;

    if category_owner_user_id != auth.user_id {
        return Err(AppError::forbidden("cannot use category of another user"));
    }

    let occurred_at = payload
        .occurred_at
        .as_deref()
        .and_then(|v| chrono::DateTime::parse_from_rfc3339(v).ok())
        .map(|v| v.with_timezone(&Utc))
        .unwrap_or(now);

    sqlx::query(
        "INSERT INTO expenses_read_model \
         (id, owner_user_id, category_id, amount, amount_minor, currency, status, tx_hash, onchain_expense_pda, occurred_at, created_at) \
         VALUES ($1, $2, $3, $4::numeric / 100.0, $4, $5, $6, $7, $8, $9, $10)",
    )
    .bind(expense_id)
    .bind(auth.user_id)
    .bind(category_id)
    .bind(payload.amount_minor)
    .bind(currency)
    .bind("pending")
    .bind(&tx_hash)
    .bind(expense_pda.to_string())
    .bind(occurred_at)
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(|_| AppError::internal("failed to insert expense commit"))?;

    let metadata = json!({
        "owner_user_id": auth.user_id.to_string(),
        "category_id": category_id.to_string(),
        "category_pda": category_pda.to_string(),
        "onchain_expense_pda": expense_pda.to_string(),
        "expense_id_onchain": payload.expense_id_onchain,
        "amount_minor": payload.amount_minor,
        "currency": currency,
        "signer_wallet": verified.signer_wallet,
        "client_ref_id": payload.client_ref_id,
    });

    sqlx::query(
        "INSERT INTO tx_audit_logs (id, actor_wallet, action, target_id, tx_hash, metadata, onchain_verified, onchain_program_id, onchain_slot, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
    )
    .bind(audit_log_id)
    .bind(&auth.wallet)
    .bind("expense.create")
    .bind(expense_id.to_string())
    .bind(&tx_hash)
    .bind(metadata.clone())
    .bind(true)
    .bind(&state.config.solana_program_id)
    .bind(verified.slot as i64)
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(|_| AppError::internal("failed to append expense create audit log"))?;

    tx.commit()
        .await
        .map_err(|_| AppError::internal("failed to commit expense create tx"))?;

    Ok(Json(OnchainCommitResponse {
        ok: true,
        tx_hash,
        commitment: state.config.solana_commitment.clone(),
        slot: verified.slot,
        program_id: state.config.solana_program_id.clone(),
        action: "expense.create".to_string(),
        target_id: expense_id.to_string(),
        audit_log_id: audit_log_id.to_string(),
        metadata,
    }))
}

pub async fn commit_expense_status(
    State(state): State<AppState>,
    Path(expense_id): Path<String>,
    auth: AuthUser,
    Json(payload): Json<CommitExpenseStatusRequest>,
) -> AppResult<Json<OnchainCommitResponse>> {
    require_admin(&auth)?;
    ensure_hybrid_onchain_enabled(&state)?;
    ensure_pg_mode(&state)?;

    let expense_id = expense_id
        .parse::<Uuid>()
        .map_err(|_| AppError::bad_request("invalid expense_id"))?;

    let tx_hash = validate_signature_base58(&payload.tx_hash)?;
    let program_id = parse_program_id(&state.config.solana_program_id)?;
    let rpc_url = resolve_rpc_url(&state, payload.rpc_url_override.as_deref())?;

    let to_status = parse_status_text(&payload.to_status)?;
    if to_status == ExpenseStatus::Pending {
        return Err(AppError::bad_request("to_status must be approved or rejected"));
    }

    let verified = verify_instruction_from_tx(
        &state,
        &rpc_url,
        &tx_hash,
        &program_id,
        "update_expense_status",
        &auth.wallet,
    )
    .await?;

    let program_config_pda = derive_program_config_pda(&program_id);

    let onchain_status = decode_update_status_arg(&verified.data_raw)?;
    if onchain_status != to_status {
        return Err(AppError::bad_request("to_status does not match onchain instruction"));
    }

    let pool = state
        .pg_pool
        .as_ref()
        .ok_or_else(|| AppError::internal("postgres pool is not initialized"))?;

    let now = Utc::now();
    let audit_log_id = Uuid::new_v4();

    let mut tx = pool
        .begin()
        .await
        .map_err(|_| AppError::internal("failed to start transaction"))?;

    let existing = sqlx::query("SELECT id, status, status_tx_hash FROM expenses_read_model WHERE status_tx_hash = $1")
        .bind(&tx_hash)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_| AppError::internal("failed to check duplicate status tx"))?;

    if let Some(row) = existing {
        let existing_id = row
            .try_get::<Uuid, _>("id")
            .map_err(|_| AppError::internal("invalid expense row"))?;

        let metadata = json!({
            "replayed": true,
            "to_status": status_to_db(&to_status),
            "reason": payload.reason,
            "signer_wallet": verified.signer_wallet,
            "client_ref_id": payload.client_ref_id,
        });

        tx.rollback()
            .await
            .map_err(|_| AppError::internal("failed to rollback transaction"))?;

        return Ok(Json(OnchainCommitResponse {
            ok: true,
            tx_hash,
            commitment: state.config.solana_commitment.clone(),
            slot: verified.slot,
            program_id: state.config.solana_program_id.clone(),
            action: match to_status {
                ExpenseStatus::Approved => "expense.approve".to_string(),
                ExpenseStatus::Rejected => "expense.reject".to_string(),
                ExpenseStatus::Pending => "expense.status".to_string(),
            },
            target_id: existing_id.to_string(),
            audit_log_id: Uuid::nil().to_string(),
            metadata,
        }));
    }

    let row = sqlx::query(
        "SELECT id, status, onchain_expense_pda FROM expenses_read_model WHERE id = $1 FOR UPDATE",
    )
    .bind(expense_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| AppError::internal("failed to load expense"))?
    .ok_or_else(|| AppError::not_found("expense not found"))?;

    let from_status = status_from_db(
        row.try_get::<String, _>("status")
            .map_err(|_| AppError::internal("invalid expense row"))?
            .as_str(),
    )?;
    if from_status != ExpenseStatus::Pending {
        return Err(AppError::bad_request("expense status is final"));
    }

    let expense_pda = row
        .try_get::<Option<String>, _>("onchain_expense_pda")
        .map_err(|_| AppError::internal("invalid expense row"))?;

    ensure_account_at(&verified, 0, &auth.wallet)?;
    ensure_account_at(&verified, 1, &program_config_pda.to_string())?;
    if let Some(expense_pda) = expense_pda {
        ensure_account_at(&verified, 2, &expense_pda)?;
    }

    sqlx::query(
        "UPDATE expenses_read_model \
         SET status = $2, status_tx_hash = $3, status_onchain_slot = $4, status_committed_at = $5 \
         WHERE id = $1",
    )
    .bind(expense_id)
    .bind(status_to_db(&to_status))
    .bind(&tx_hash)
    .bind(verified.slot as i64)
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(|_| AppError::internal("failed to update expense status commit"))?;

    let action = match to_status {
        ExpenseStatus::Approved => "expense.approve",
        ExpenseStatus::Rejected => "expense.reject",
        ExpenseStatus::Pending => "expense.status",
    };

    let metadata = json!({
        "from_status": status_to_db(&from_status),
        "to_status": status_to_db(&to_status),
        "reason": payload.reason,
        "signer_wallet": verified.signer_wallet,
        "client_ref_id": payload.client_ref_id,
    });

    sqlx::query(
        "INSERT INTO tx_audit_logs (id, actor_wallet, action, target_id, tx_hash, metadata, onchain_verified, onchain_program_id, onchain_slot, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
    )
    .bind(audit_log_id)
    .bind(&auth.wallet)
    .bind(action)
    .bind(expense_id.to_string())
    .bind(&tx_hash)
    .bind(metadata.clone())
    .bind(true)
    .bind(&state.config.solana_program_id)
    .bind(verified.slot as i64)
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(|_| AppError::internal("failed to append expense status audit log"))?;

    tx.commit()
        .await
        .map_err(|_| AppError::internal("failed to commit expense status tx"))?;

    Ok(Json(OnchainCommitResponse {
        ok: true,
        tx_hash,
        commitment: state.config.solana_commitment.clone(),
        slot: verified.slot,
        program_id: state.config.solana_program_id.clone(),
        action: action.to_string(),
        target_id: expense_id.to_string(),
        audit_log_id: audit_log_id.to_string(),
        metadata,
    }))
}

fn ensure_hybrid_onchain_enabled(state: &AppState) -> AppResult<()> {
    if !state.config.hybrid_onchain_enabled {
        return Err(AppError::bad_request("hybrid onchain flow is disabled"));
    }
    Ok(())
}

fn ensure_pg_mode(state: &AppState) -> AppResult<()> {
    if !state.config.expenses_pg_enabled {
        return Err(AppError::bad_request("expenses postgres mode is required for hybrid commits"));
    }
    Ok(())
}

fn parse_program_id(value: &str) -> AppResult<Pubkey> {
    Pubkey::from_str(value).map_err(|_| AppError::internal("invalid SOLANA_PROGRAM_ID"))
}

fn parse_wallet_pubkey(value: &str) -> AppResult<Pubkey> {
    Pubkey::from_str(value).map_err(|_| AppError::bad_request("invalid wallet pubkey"))
}

fn derive_program_config_pda(program_id: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[b"program_config"], program_id).0
}

fn resolve_rpc_url(state: &AppState, rpc_url_override: Option<&str>) -> AppResult<String> {
    if let Some(url) = rpc_url_override {
        if !cfg!(debug_assertions) {
            return Err(AppError::forbidden("rpc_url_override is allowed only in debug build"));
        }
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(AppError::bad_request("rpc_url_override must be http(s) url"));
        }
        return Ok(url.to_string());
    }
    Ok(state.config.solana_rpc_url.clone())
}

fn validate_signature_base58(value: &str) -> AppResult<String> {
    let tx_hash = value.trim();
    if tx_hash.is_empty() {
        return Err(AppError::bad_request("tx_hash is required"));
    }
    let raw = bs58::decode(tx_hash)
        .into_vec()
        .map_err(|_| AppError::bad_request("invalid tx_hash base58"))?;
    if raw.len() != 64 {
        return Err(AppError::bad_request("invalid tx_hash length"));
    }
    Ok(tx_hash.to_string())
}

fn instruction_discriminator(name: &str) -> [u8; 8] {
    let mut hasher = Sha256::new();
    hasher.update(format!("global:{name}").as_bytes());
    let out = hasher.finalize();
    let mut disc = [0u8; 8];
    disc.copy_from_slice(&out[..8]);
    disc
}

async fn verify_instruction_from_tx(
    state: &AppState,
    rpc_url: &str,
    tx_hash: &str,
    program_id: &Pubkey,
    instruction_name: &str,
    expected_signer_wallet: &str,
) -> AppResult<VerifiedInstruction> {
    let payload = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getTransaction",
        "params": [
            tx_hash,
            {
                "encoding": "json",
                "commitment": state.config.solana_commitment,
                "maxSupportedTransactionVersion": 0
            }
        ]
    });

    let client = Client::new();
    let rpc_resp = client
        .post(rpc_url)
        .json(&payload)
        .send()
        .await
        .map_err(|_| AppError::internal("failed to call solana rpc"))?;

    if !rpc_resp.status().is_success() {
        return Err(AppError::internal("solana rpc returned non-success status"));
    }

    let value: Value = rpc_resp
        .json()
        .await
        .map_err(|_| AppError::internal("failed to decode solana rpc response"))?;

    if value.get("error").is_some() {
        return Err(AppError::bad_request("solana rpc returned error for tx_hash"));
    }

    let result = value
        .get("result")
        .ok_or_else(|| AppError::bad_request("transaction not found at selected commitment"))?;

    if result.is_null() {
        return Err(AppError::bad_request(
            "transaction not found at selected commitment",
        ));
    }

    let meta = result
        .get("meta")
        .ok_or_else(|| AppError::bad_request("missing transaction meta"))?;
    if !meta.get("err").unwrap_or(&Value::Null).is_null() {
        return Err(AppError::bad_request("transaction failed onchain"));
    }

    let slot = result
        .get("slot")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| AppError::internal("missing slot in transaction result"))?;

    let message = result
        .get("transaction")
        .and_then(|v| {
            v.get("message")
                .or_else(|| v.get("transaction").and_then(|tx| tx.get("message")))
        })
        .ok_or_else(|| AppError::internal("missing transaction.message"))?;

    let account_keys = parse_account_keys(message)?;
    let required_signatures = message
        .get("header")
        .and_then(|v| v.get("numRequiredSignatures"))
        .and_then(|v| v.as_u64())
        .ok_or_else(|| AppError::internal("missing numRequiredSignatures"))? as usize;

    let signer_wallet = account_keys
        .iter()
        .take(required_signatures)
        .find(|k| *k == expected_signer_wallet)
        .cloned()
        .ok_or_else(|| AppError::forbidden("jwt wallet is not a transaction signer"))?;

    let instruction_disc = instruction_discriminator(instruction_name);

    let instructions = message
        .get("instructions")
        .and_then(|v| v.as_array())
        .ok_or_else(|| AppError::internal("missing message.instructions"))?;

    let mut matched: Option<VerifiedInstruction> = None;

    for ins in instructions {
        let program_index = ins
            .get("programIdIndex")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| AppError::internal("missing instruction programIdIndex"))?
            as usize;

        let ins_program = account_keys
            .get(program_index)
            .ok_or_else(|| AppError::internal("programIdIndex out of range"))?;

        if ins_program != &program_id.to_string() {
            continue;
        }

        let data_b58 = ins
            .get("data")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::internal("missing instruction data"))?;
        let data_raw = bs58::decode(data_b58)
            .into_vec()
            .map_err(|_| AppError::internal("invalid base58 instruction data"))?;

        if data_raw.len() < 8 || data_raw[..8] != instruction_disc {
            continue;
        }

        let account_indices = ins
            .get("accounts")
            .and_then(|v| v.as_array())
            .ok_or_else(|| AppError::internal("missing instruction accounts"))?
            .iter()
            .map(|v| {
                v.as_u64()
                    .map(|x| x as usize)
                    .ok_or_else(|| AppError::internal("invalid account index"))
            })
            .collect::<AppResult<Vec<_>>>()?;

        if matched.is_some() {
            return Err(AppError::bad_request("multiple matching instructions found in one tx"));
        }

        matched = Some(VerifiedInstruction {
            slot,
            signer_wallet: signer_wallet.clone(),
            account_keys: account_keys.clone(),
            account_indices,
            data_raw,
        });
    }

    matched.ok_or_else(|| AppError::bad_request("matching instruction not found in transaction"))
}

fn parse_account_keys(message: &Value) -> AppResult<Vec<String>> {
    let keys = message
        .get("accountKeys")
        .and_then(|v| v.as_array())
        .ok_or_else(|| AppError::internal("missing message.accountKeys"))?;

    keys.iter()
        .map(|v| {
            if let Some(s) = v.as_str() {
                Ok(s.to_string())
            } else if let Some(obj) = v.as_object() {
                obj.get("pubkey")
                    .and_then(|x| x.as_str())
                    .map(|x| x.to_string())
                    .ok_or_else(|| AppError::internal("invalid parsed account key object"))
            } else {
                Err(AppError::internal("invalid account key format"))
            }
        })
        .collect::<AppResult<Vec<_>>>()
}

fn ensure_account_at(verified: &VerifiedInstruction, pos: usize, expected_pubkey: &str) -> AppResult<()> {
    let account_idx = *verified
        .account_indices
        .get(pos)
        .ok_or_else(|| AppError::bad_request("instruction account index missing"))?;

    let actual = verified
        .account_keys
        .get(account_idx)
        .ok_or_else(|| AppError::bad_request("instruction account index out of range"))?;

    if actual != expected_pubkey {
        return Err(AppError::bad_request("instruction account mismatch"));
    }

    Ok(())
}

fn decode_create_category_name(data: &[u8]) -> AppResult<String> {
    if data.len() < 12 {
        return Err(AppError::bad_request("invalid create_category instruction data"));
    }
    let len = u32::from_le_bytes(
        data[8..12]
            .try_into()
            .map_err(|_| AppError::bad_request("invalid category name length prefix"))?,
    ) as usize;

    if data.len() < 12 + len {
        return Err(AppError::bad_request("category name bytes truncated"));
    }

    String::from_utf8(data[12..12 + len].to_vec())
        .map_err(|_| AppError::bad_request("category name is not utf8"))
}

fn decode_create_expense_args(data: &[u8]) -> AppResult<(u64, u64)> {
    if data.len() < 8 + 8 + 8 + 32 {
        return Err(AppError::bad_request("invalid create_expense instruction data"));
    }

    let expense_id = u64::from_le_bytes(
        data[8..16]
            .try_into()
            .map_err(|_| AppError::bad_request("invalid expense_id bytes"))?,
    );
    let amount = u64::from_le_bytes(
        data[16..24]
            .try_into()
            .map_err(|_| AppError::bad_request("invalid amount bytes"))?,
    );

    Ok((expense_id, amount))
}

fn decode_update_status_arg(data: &[u8]) -> AppResult<ExpenseStatus> {
    if data.len() < 9 {
        return Err(AppError::bad_request("invalid update_expense_status instruction data"));
    }

    match data[8] {
        0 => Ok(ExpenseStatus::Pending),
        1 => Ok(ExpenseStatus::Approved),
        2 => Ok(ExpenseStatus::Rejected),
        _ => Err(AppError::bad_request("invalid onchain status enum value")),
    }
}

fn parse_status_text(v: &str) -> AppResult<ExpenseStatus> {
    match v.trim().to_ascii_lowercase().as_str() {
        "pending" => Ok(ExpenseStatus::Pending),
        "approved" => Ok(ExpenseStatus::Approved),
        "rejected" => Ok(ExpenseStatus::Rejected),
        _ => Err(AppError::bad_request("to_status must be one of: pending, approved, rejected")),
    }
}

fn status_to_db(status: &ExpenseStatus) -> &'static str {
    match status {
        ExpenseStatus::Pending => "pending",
        ExpenseStatus::Approved => "approved",
        ExpenseStatus::Rejected => "rejected",
    }
}

fn status_from_db(value: &str) -> AppResult<ExpenseStatus> {
    match value {
        "pending" => Ok(ExpenseStatus::Pending),
        "approved" => Ok(ExpenseStatus::Approved),
        "rejected" => Ok(ExpenseStatus::Rejected),
        _ => Err(AppError::internal("invalid expense status in database")),
    }
}
