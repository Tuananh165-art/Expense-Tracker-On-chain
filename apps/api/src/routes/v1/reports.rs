use axum::{extract::State, Json};
use serde::Serialize;
use std::collections::HashMap;

use crate::{auth::AuthUser, authz::require_user_admin_or_auditor, error::AppResult, state::AppState};

#[derive(Serialize)]
pub struct ReportByCategoryItem {
    pub category_id: String,
    pub total_amount_minor: i64,
}

#[derive(Serialize)]
pub struct MonthlyReportResponse {
    pub total_amount_minor: i64,
    pub by_category: Vec<ReportByCategoryItem>,
}

pub async fn monthly_summary(
    State(state): State<AppState>,
    auth: AuthUser,
) -> AppResult<Json<MonthlyReportResponse>> {
    require_user_admin_or_auditor(&auth)?;
    let expenses = state.expenses.read().await;

    let mut total = 0_i64;
    let mut by_category_map: HashMap<String, i64> = HashMap::new();

    for e in expenses.values().filter(|e| e.owner_user_id == auth.user_id) {
        total += e.amount_minor;
        *by_category_map.entry(e.category_id.to_string()).or_insert(0) += e.amount_minor;
    }

    let by_category = by_category_map
        .into_iter()
        .map(|(category_id, total_amount_minor)| ReportByCategoryItem {
            category_id,
            total_amount_minor,
        })
        .collect::<Vec<_>>();

    Ok(Json(MonthlyReportResponse {
        total_amount_minor: total,
        by_category,
    }))
}
