//! Dashboard service — urgency classification, filtering, and response assembly.
//!
//! Pure business logic: no SQL, no Tauri, no Svelte.
//! Uses `domain::expiry_status` for urgency classification and `domain::alerts`
//! for day calculations.

use chrono::NaiveDate;

use crate::db::DbPool;
use crate::domain::expiry_status::{classify_urgency_with_alert, Urgency};
use crate::dto::dashboard::{
    DashboardFilters, DashboardLotRow, DashboardPreset, DashboardResponse, UrgencyCounts,
};

/// Returns today's date in UTC (stable within a single request).
fn today() -> NaiveDate {
    chrono::Utc::now().date_naive()
}

// ─── Urgency helpers ───────────────────────────────────────────────────────────

/// Classifies a single row into an urgency variant string and computes
/// `days_remaining`. Returns an enriched `DashboardLotRow`.
fn enrich_row(row: DashboardLotRow) -> DashboardLotRow {
    let Ok(expiry) = NaiveDate::parse_from_str(&row.expiry_date, "%Y-%m-%d") else {
        // Parsing failed: treat as far-future to avoid false urgency; log internally.
        tracing::warn!(date = %row.expiry_date, lot_id = %row.lot_id, "Could not parse expiry date; treating as far-future");
        return DashboardLotRow {
            urgency: "future".to_string(),
            days_remaining: i64::MAX,
            ..row
        };
    };

    let urgency = classify_to_string_with_alert(today(), expiry, row.alert_days_before);
    let days_remaining = (expiry - today()).num_days();

    DashboardLotRow {
        urgency,
        days_remaining,
        ..row
    }
}

/// Maps `Urgency` enum to the snake_case string used in DTOs and UI.
fn classify_to_string(today: NaiveDate, expiry: NaiveDate) -> String {
    use crate::domain::expiry_status::classify_urgency;

    match classify_urgency(today, expiry) {
        Urgency::Expired => "expired".to_string(),
        Urgency::Today => "today".to_string(),
        Urgency::AlertWindow => "alert_window".to_string(),
        Urgency::Next30Days => "next_30_days".to_string(),
        Urgency::Future => "future".to_string(),
    }
}

/// Maps Urgency (computed with alert_days_before) to snake_case string.
fn classify_to_string_with_alert(
    today: NaiveDate,
    expiry: NaiveDate,
    alert_days_before: i32,
) -> String {
    match classify_urgency_with_alert(today, expiry, alert_days_before) {
        Urgency::Expired => "expired".to_string(),
        Urgency::Today => "today".to_string(),
        Urgency::AlertWindow => "alert_window".to_string(),
        Urgency::Next30Days => "next_30_days".to_string(),
        Urgency::Future => "future".to_string(),
    }
}

/// Returns `true` when the row's urgency matches the given preset filter.
/// `All` preset returns `true` for all rows.
fn matches_preset(urgency: &str, preset: Option<DashboardPreset>) -> bool {
    match preset {
        None | Some(DashboardPreset::All) => true,
        Some(DashboardPreset::Expired) => urgency == "expired",
        Some(DashboardPreset::Today) => urgency == "today",
        Some(DashboardPreset::AlertWindow) => urgency == "alert_window",
        Some(DashboardPreset::Next7Days) => {
            // "next 7 days" includes today and Next30Days rows where days_remaining <= 7.
            urgency == "today" || urgency == "next_30_days"
        }
        Some(DashboardPreset::Next30Days) => urgency == "next_30_days",
    }
}

// ─── Service entry point ──────────────────────────────────────────────────────

/// Fetches active lots, classifies urgency, counts the buckets, and returns
/// the full dashboard bundle.
///
/// Filtering:
///   - `store_id` / `location_id` → repository-level SQL filter
///   - `preset` → client-side urgency filter after enrichment
///
/// Sorting: urgency (most urgent first: expired → today → alert_window →
///          next_30_days → future), then by expiry_date ASC within each group.
pub async fn get_dashboard(
    pool: &DbPool,
    filters: DashboardFilters,
) -> Result<DashboardResponse, crate::error::AppError> {
    use crate::db::repositories::dashboard as repo;

    // 1. Fetch raw rows from repository (no urgency set yet).
    let raw_rows = repo::list_dashboard_lots(pool, &filters)
        .await
        .map_err(crate::error::AppError::from)?;

    // 2. Enrich each row: set urgency string + days_remaining.
    let enriched: Vec<DashboardLotRow> = raw_rows.into_iter().map(enrich_row).collect();

    // 3. Count all rows into urgency buckets (before applying client filter).
    let counts = count_by_urgency(&enriched);

    // 4. Apply urgency preset filter if specified.
    let preset = filters.preset.or(filters.urgency.and_then(|u| {
        // Accept snake_case string equivalents as a fallback.
        match u.as_str() {
            "expired" => Some(DashboardPreset::Expired),
            "today" => Some(DashboardPreset::Today),
            "alert_window" => Some(DashboardPreset::AlertWindow),
            "next_7_days" => Some(DashboardPreset::Next7Days),
            "next_30_days" => Some(DashboardPreset::Next30Days),
            _ => None,
        }
    }));

    let filtered: Vec<DashboardLotRow> = enriched
        .into_iter()
        .filter(|row| matches_preset(&row.urgency, preset))
        .collect();

    // 5. Sort: most urgent first, then by expiry_date ASC within group.
    let mut lots = filtered;
    lots.sort_by(|a, b| {
        let urgency_order = urgency_rank(&a.urgency).cmp(&urgency_rank(&b.urgency));
        if urgency_order == std::cmp::Ordering::Equal {
            a.expiry_date.cmp(&b.expiry_date)
        } else {
            urgency_order
        }
    });

    Ok(DashboardResponse { counts, lots })
}

/// Maps an urgency string to a sort rank (lower = more urgent).
fn urgency_rank(urgency: &str) -> u8 {
    match urgency {
        "expired" => 0,
        "today" => 1,
        "alert_window" => 2,
        "next_30_days" => 3,
        _ => 4, // future / unknown
    }
}

/// Counts how many rows fall into each urgency bucket.
fn count_by_urgency(rows: &[DashboardLotRow]) -> UrgencyCounts {
    let mut counts = UrgencyCounts::default();
    for row in rows {
        match row.urgency.as_str() {
            "expired" => counts.expired += 1,
            "today" => counts.today += 1,
            "alert_window" => counts.alert_window += 1,
            "next_30_days" => counts.next_30_days += 1,
            _ => {}
        }
    }
    counts
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_row(expiry_date: &str, alert_days: i32) -> DashboardLotRow {
        DashboardLotRow {
            lot_id: "lot-1".into(),
            product_id: "prod-1".into(),
            sku: "SKU001".into(),
            description: "Test Product".into(),
            store_id: "store-1".into(),
            store_name: "Store One".into(),
            location_id: None,
            location_name: None,
            quantity: 10.0,
            unit: "kg".into(),
            expiry_date: expiry_date.into(),
            alert_days_before: alert_days,
            batch_code: None,
            status: "active".into(),
            urgency: String::new(), // will be set by enrich_row
            days_remaining: 0,
        }
    }

    // ─── enrich_row ────────────────────────────────────────────────────────────

    #[test]
    fn enrich_row_sets_expired() {
        let row = make_row("2020-01-01", 30);
        let enriched = enrich_row(row);
        assert_eq!(enriched.urgency, "expired");
        assert!(enriched.days_remaining < 0);
    }

    #[test]
    fn enrich_row_sets_today() {
        let today_str = chrono::Utc::now()
            .date_naive()
            .format("%Y-%m-%d")
            .to_string();
        let row = make_row(&today_str, 30);
        let enriched = enrich_row(row);
        assert_eq!(enriched.urgency, "today");
        assert_eq!(enriched.days_remaining, 0);
    }

    #[test]
    fn enrich_row_sets_next_30_days() {
        let in_30 = (chrono::Utc::now().date_naive() + chrono::Duration::days(15))
            .format("%Y-%m-%d")
            .to_string();
        let row = make_row(&in_30, 30);
        let enriched = enrich_row(row);
        assert_eq!(enriched.urgency, "next_30_days");
        assert!(enriched.days_remaining > 0 && enriched.days_remaining <= 30);
    }

    #[test]
    fn enrich_row_sets_future() {
        let far_future = (chrono::Utc::now().date_naive() + chrono::Duration::days(60))
            .format("%Y-%m-%d")
            .to_string();
        let row = make_row(&far_future, 30);
        let enriched = enrich_row(row);
        assert_eq!(enriched.urgency, "future");
        assert!(enriched.days_remaining > 30);
    }

    // ─── matches_preset ────────────────────────────────────────────────────────

    #[test]
    fn matches_preset_none_returns_true() {
        assert!(matches_preset("expired", None));
        assert!(matches_preset("today", None));
        assert!(matches_preset("alert_window", None));
        assert!(matches_preset("next_30_days", None));
        assert!(matches_preset("future", None));
    }

    #[test]
    fn matches_preset_all_returns_true() {
        assert!(matches_preset("expired", Some(DashboardPreset::All)));
        assert!(matches_preset("today", Some(DashboardPreset::All)));
        assert!(matches_preset("future", Some(DashboardPreset::All)));
    }

    #[test]
    fn matches_preset_expired() {
        assert!(matches_preset("expired", Some(DashboardPreset::Expired)));
        assert!(!matches_preset("today", Some(DashboardPreset::Expired)));
        assert!(!matches_preset(
            "next_30_days",
            Some(DashboardPreset::Expired)
        ));
    }

    #[test]
    fn matches_preset_today() {
        assert!(!matches_preset("expired", Some(DashboardPreset::Today)));
        assert!(matches_preset("today", Some(DashboardPreset::Today)));
        assert!(!matches_preset(
            "alert_window",
            Some(DashboardPreset::Today)
        ));
    }

    #[test]
    fn matches_preset_next7days() {
        // next_7_days includes "today" and "next_30_days" rows.
        assert!(!matches_preset("expired", Some(DashboardPreset::Next7Days)));
        assert!(matches_preset("today", Some(DashboardPreset::Next7Days)));
        assert!(matches_preset(
            "next_30_days",
            Some(DashboardPreset::Next7Days)
        ));
        assert!(!matches_preset("future", Some(DashboardPreset::Next7Days)));
    }

    #[test]
    fn matches_preset_next30days() {
        assert!(!matches_preset("today", Some(DashboardPreset::Next30Days)));
        assert!(matches_preset(
            "next_30_days",
            Some(DashboardPreset::Next30Days)
        ));
        assert!(!matches_preset("future", Some(DashboardPreset::Next30Days)));
    }

    // ─── urgency_rank ───────────────────────────────────────────────────────────

    #[test]
    fn urgency_rank_order() {
        assert!(urgency_rank("expired") < urgency_rank("today"));
        assert!(urgency_rank("today") < urgency_rank("alert_window"));
        assert!(urgency_rank("alert_window") < urgency_rank("next_30_days"));
        assert!(urgency_rank("next_30_days") < urgency_rank("future"));
    }

    // ─── count_by_urgency ───────────────────────────────────────────────────────

    #[test]
    fn count_by_urgency_empty() {
        let counts = count_by_urgency(&[]);
        assert_eq!(counts.expired, 0);
        assert_eq!(counts.today, 0);
        assert_eq!(counts.alert_window, 0);
        assert_eq!(counts.next_30_days, 0);
    }

    #[test]
    fn count_by_urgency_sums_correctly() {
        let today_str = chrono::Utc::now()
            .date_naive()
            .format("%Y-%m-%d")
            .to_string();
        let in_15 = (chrono::Utc::now().date_naive() + chrono::Duration::days(15))
            .format("%Y-%m-%d")
            .to_string();

        // Pass through enrich_row so urgency field is populated.
        let raw_rows = vec![
            make_row("2020-01-01", 30), // expired
            make_row("2020-01-02", 30), // expired
            make_row(&today_str, 14),   // today
            make_row(&in_15, 30),       // next_30_days (alert_days=30 → not < 30)
            make_row(&in_15, 30),       // next_30_days
            make_row(&in_15, 30),       // next_30_days
        ];
        let rows: Vec<DashboardLotRow> = raw_rows.into_iter().map(enrich_row).collect();

        let counts = count_by_urgency(&rows);
        assert_eq!(counts.expired, 2);
        assert_eq!(counts.today, 1);
        assert_eq!(counts.alert_window, 0);
        assert_eq!(counts.next_30_days, 3);
    }
}
