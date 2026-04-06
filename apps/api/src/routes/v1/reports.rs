use axum::{
    extract::{Query, State},
    Json,
};
use chrono::{Datelike, TimeZone, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::collections::HashMap;
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    authz::require_user_admin_or_auditor,
    error::{AppError, AppResult},
    state::AppState,
};

#[derive(Deserialize)]
pub struct MonthlyReportQuery {
    pub month: Option<u32>,
    pub year: Option<i32>,
    pub timezone: Option<String>,
    pub top_n: Option<usize>,
}

#[derive(Serialize)]
pub struct ReportByCategoryItem {
    pub category_id: String,
    pub total_amount_minor: i64,
}

#[derive(Serialize)]
pub struct ReportByDayItem {
    pub day: String,
    pub total_amount_minor: i64,
}

#[derive(Serialize)]
pub struct TopSpendingItem {
    pub id: String,
    pub category_id: String,
    pub amount_minor: i64,
    pub currency: String,
    pub status: String,
    pub occurred_at: String,
}

#[derive(Serialize)]
pub struct MonthlyReportPeriod {
    pub month: u32,
    pub year: i32,
    pub timezone: String,
    pub from_utc: String,
    pub to_utc: String,
}

#[derive(Serialize)]
pub struct MonthlyReportResponse {
    pub total_amount_minor: i64,
    pub by_category: Vec<ReportByCategoryItem>,
    pub by_day: Vec<ReportByDayItem>,
    pub top_spending: Vec<TopSpendingItem>,
    pub period: MonthlyReportPeriod,
}

fn resolve_period(query: &MonthlyReportQuery) -> AppResult<(u32, i32, Tz, chrono::DateTime<Utc>, chrono::DateTime<Utc>)> {
    let now_utc = Utc::now();
    let month = query.month.unwrap_or(now_utc.month());
    let year = query.year.unwrap_or(now_utc.year());
    if !(1..=12).contains(&month) {
        return Err(AppError::bad_request("month must be between 1 and 12"));
    }

    let timezone_str = query.timezone.clone().unwrap_or_else(|| "UTC".to_string());
    let tz: Tz = timezone_str
        .parse()
        .map_err(|_| AppError::bad_request("invalid timezone"))?;

    let start_local = tz
        .with_ymd_and_hms(year, month, 1, 0, 0, 0)
        .single()
        .ok_or_else(|| AppError::bad_request("invalid report period"))?;

    let (next_year, next_month) = if month == 12 { (year + 1, 1) } else { (year, month + 1) };
    let end_local = tz
        .with_ymd_and_hms(next_year, next_month, 1, 0, 0, 0)
        .single()
        .ok_or_else(|| AppError::bad_request("invalid report period"))?;

    Ok((month, year, tz, start_local.with_timezone(&Utc), end_local.with_timezone(&Utc)))
}

pub async fn monthly_summary(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<MonthlyReportQuery>,
) -> AppResult<Json<MonthlyReportResponse>> {
    require_user_admin_or_auditor(&auth)?;

    let (month, year, tz, from_utc, to_utc) = resolve_period(&query)?;
    let top_n = query.top_n.unwrap_or(5).clamp(1, 20) as i64;

    if state.config.expenses_pg_enabled {
        let pool = state
            .pg_pool
            .as_ref()
            .ok_or_else(|| AppError::internal("postgres pool is not initialized"))?;

        let total_row = sqlx::query(
            "SELECT COALESCE(SUM(amount_minor), 0)::bigint AS total FROM expenses_read_model \
             WHERE owner_user_id = $1 AND occurred_at >= $2 AND occurred_at < $3",
        )
        .bind(auth.user_id)
        .bind(from_utc)
        .bind(to_utc)
        .fetch_one(pool)
        .await
        .map_err(|_| AppError::internal("failed to compute monthly total"))?;

        let total_amount_minor: i64 = total_row
            .try_get("total")
            .map_err(|_| AppError::internal("invalid total row"))?;

        let by_category_rows = sqlx::query(
            "SELECT category_id, COALESCE(SUM(amount_minor), 0)::bigint AS total_amount_minor \
             FROM expenses_read_model \
             WHERE owner_user_id = $1 AND occurred_at >= $2 AND occurred_at < $3 \
             GROUP BY category_id ORDER BY total_amount_minor DESC",
        )
        .bind(auth.user_id)
        .bind(from_utc)
        .bind(to_utc)
        .fetch_all(pool)
        .await
        .map_err(|_| AppError::internal("failed to compute report by category"))?;

        let by_category = by_category_rows
            .into_iter()
            .map(|row| {
                let category_id: Uuid = row
                    .try_get("category_id")
                    .map_err(|_| AppError::internal("invalid category row"))?;
                let total_amount_minor: i64 = row
                    .try_get("total_amount_minor")
                    .map_err(|_| AppError::internal("invalid category row"))?;
                Ok(ReportByCategoryItem {
                    category_id: category_id.to_string(),
                    total_amount_minor,
                })
            })
            .collect::<AppResult<Vec<_>>>()?;

        let by_day_rows = sqlx::query(
            "SELECT DATE(occurred_at AT TIME ZONE $1) AS day, COALESCE(SUM(amount_minor), 0)::bigint AS total_amount_minor \
             FROM expenses_read_model \
             WHERE owner_user_id = $2 AND occurred_at >= $3 AND occurred_at < $4 \
             GROUP BY day ORDER BY day ASC",
        )
        .bind(tz.name())
        .bind(auth.user_id)
        .bind(from_utc)
        .bind(to_utc)
        .fetch_all(pool)
        .await
        .map_err(|_| AppError::internal("failed to compute report by day"))?;

        let by_day = by_day_rows
            .into_iter()
            .map(|row| {
                let day: chrono::NaiveDate = row
                    .try_get("day")
                    .map_err(|_| AppError::internal("invalid day row"))?;
                let total_amount_minor: i64 = row
                    .try_get("total_amount_minor")
                    .map_err(|_| AppError::internal("invalid day row"))?;
                Ok(ReportByDayItem {
                    day: day.to_string(),
                    total_amount_minor,
                })
            })
            .collect::<AppResult<Vec<_>>>()?;

        let top_rows = sqlx::query(
            "SELECT id, category_id, amount_minor, currency, status, occurred_at \
             FROM expenses_read_model \
             WHERE owner_user_id = $1 AND occurred_at >= $2 AND occurred_at < $3 \
             ORDER BY amount_minor DESC, occurred_at DESC \
             LIMIT $4",
        )
        .bind(auth.user_id)
        .bind(from_utc)
        .bind(to_utc)
        .bind(top_n)
        .fetch_all(pool)
        .await
        .map_err(|_| AppError::internal("failed to compute top spending"))?;

        let top_spending = top_rows
            .into_iter()
            .map(|row| {
                Ok(TopSpendingItem {
                    id: row
                        .try_get::<Uuid, _>("id")
                        .map_err(|_| AppError::internal("invalid top row"))?
                        .to_string(),
                    category_id: row
                        .try_get::<Uuid, _>("category_id")
                        .map_err(|_| AppError::internal("invalid top row"))?
                        .to_string(),
                    amount_minor: row
                        .try_get("amount_minor")
                        .map_err(|_| AppError::internal("invalid top row"))?,
                    currency: row
                        .try_get("currency")
                        .map_err(|_| AppError::internal("invalid top row"))?,
                    status: row
                        .try_get("status")
                        .map_err(|_| AppError::internal("invalid top row"))?,
                    occurred_at: row
                        .try_get::<chrono::DateTime<Utc>, _>("occurred_at")
                        .map_err(|_| AppError::internal("invalid top row"))?
                        .to_rfc3339(),
                })
            })
            .collect::<AppResult<Vec<_>>>()?;

        return Ok(Json(MonthlyReportResponse {
            total_amount_minor,
            by_category,
            by_day,
            top_spending,
            period: MonthlyReportPeriod {
                month,
                year,
                timezone: tz.name().to_string(),
                from_utc: from_utc.to_rfc3339(),
                to_utc: to_utc.to_rfc3339(),
            },
        }));
    }

    let expenses = state.expenses.read().await;

    let mut total = 0_i64;
    let mut by_category_map: HashMap<String, i64> = HashMap::new();
    let mut by_day_map: HashMap<String, i64> = HashMap::new();
    let mut top_spending = Vec::<TopSpendingItem>::new();

    for e in expenses.values().filter(|e| e.owner_user_id == auth.user_id) {
        if e.occurred_at < from_utc || e.occurred_at >= to_utc {
            continue;
        }

        total += e.amount_minor;
        *by_category_map.entry(e.category_id.to_string()).or_insert(0) += e.amount_minor;

        let local_day = e.occurred_at.with_timezone(&tz).date_naive().to_string();
        *by_day_map.entry(local_day).or_insert(0) += e.amount_minor;

        top_spending.push(TopSpendingItem {
            id: e.id.to_string(),
            category_id: e.category_id.to_string(),
            amount_minor: e.amount_minor,
            currency: e.currency.clone(),
            status: match e.status {
                crate::models::ExpenseStatus::Pending => "pending".to_string(),
                crate::models::ExpenseStatus::Approved => "approved".to_string(),
                crate::models::ExpenseStatus::Rejected => "rejected".to_string(),
            },
            occurred_at: e.occurred_at.to_rfc3339(),
        });
    }

    let mut by_day = by_day_map
        .into_iter()
        .map(|(day, total_amount_minor)| ReportByDayItem { day, total_amount_minor })
        .collect::<Vec<_>>();
    by_day.sort_by(|a, b| a.day.cmp(&b.day));

    top_spending.sort_by(|a, b| {
        b.amount_minor
            .cmp(&a.amount_minor)
            .then_with(|| b.occurred_at.cmp(&a.occurred_at))
    });
    top_spending.truncate(top_n as usize);

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
        by_day,
        top_spending,
        period: MonthlyReportPeriod {
            month,
            year,
            timezone: tz.name().to_string(),
            from_utc: from_utc.to_rfc3339(),
            to_utc: to_utc.to_rfc3339(),
        },
    }))
}
