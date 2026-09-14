//! Reports service — preview, filter translation, metadata assembly (Slice 11a).
//!
//! Pure business logic on top of `services::dashboard`: validates the report
//! request, translates it into a `DashboardFilters` shape that the dashboard
//! service already understands, applies category / date-range post-filters
//! when needed, and assembles the report metadata.
//!
//! No SQL lives here — the dashboard repository already produces the lot rows.
//! No PDF / CSV rendering lives here either; the preview payload is a
//! structured DTO that the future PDF / UI / CSV layers can reuse.
//!
//! ## Architecture guardrails
//!
//! - Reuse the dashboard service so urgency classification, sort order, and
//!   active-lot filtering are defined in one place.
//! - No raw SKU / barcode / description / notes content is logged. Tracing
//!   uses aggregate counts and type tags only.
//! - Filter validation rejects malformed dates and inverted ranges before any
//!   DB work happens.

use chrono::Utc;

use crate::db::DbPool;
use crate::dto::dashboard::{DashboardFilters, DashboardLotRow, DashboardPreset};
use crate::dto::reports::{ReportData, ReportFilters, ReportMetadata, ReportRequest, ReportType};
use crate::error::{AppError, DomainError};

// ============================================================
// Filter parsing / validation
// ============================================================

/// Parses an optional ISO-8601 date (YYYY-MM-DD). Treats blank strings as
/// "no filter" (returns `Ok(None)`).
fn optional_date(
    label: &str,
    value: Option<&str>,
) -> Result<Option<chrono::NaiveDate>, DomainError> {
    let Some(raw) = value else {
        return Ok(None);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    chrono::NaiveDate::parse_from_str(trimmed, "%Y-%m-%d")
        .map(Some)
        .map_err(|_| DomainError::Validation {
            message: format!("Invalid {label} `{raw}` (expected YYYY-MM-DD)"),
        })
}

/// Validates the shape of the filters before any DB work happens. Returns
/// `Validation` for malformed dates or an inverted date range.
fn validate_filters(filters: &ReportFilters) -> Result<(), DomainError> {
    let from = optional_date("date_from", filters.date_from.as_deref())?;
    let to = optional_date("date_to", filters.date_to.as_deref())?;
    if let (Some(f), Some(t)) = (from, to) {
        if f > t {
            return Err(DomainError::Validation {
                message: format!(
                    "date_from `{}` must be on or before date_to `{}`",
                    filters.date_from.as_deref().unwrap_or(""),
                    filters.date_to.as_deref().unwrap_or("")
                ),
            });
        }
    }
    Ok(())
}

// ============================================================
// Filter translation
// ============================================================

/// Maps a built-in report type to its matching dashboard preset. `Custom`
/// reports use the user-supplied `urgency` instead.
fn preset_for_type(report_type: ReportType) -> Option<DashboardPreset> {
    match report_type {
        ReportType::InAlertWindow => Some(DashboardPreset::AlertWindow),
        ReportType::Expired => Some(DashboardPreset::Expired),
        ReportType::Next30Days => Some(DashboardPreset::Next30Days),
        ReportType::Custom => None,
    }
}

/// Translates a `ReportRequest` into the dashboard filter shape. Built-in
/// reports pin the preset; `Custom` reports pass through `urgency` so the
/// dashboard service can map it back to a preset (or use `All`).
fn to_dashboard_filters(request: &ReportRequest) -> DashboardFilters {
    let filters = request.filters.clone().unwrap_or_default();
    let preset = preset_for_type(request.kind);
    let urgency = if preset.is_none() {
        filters
            .urgency
            .as_deref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    } else {
        None
    };
    DashboardFilters {
        store_id: filters.store_id,
        location_id: filters.location_id,
        preset,
        urgency,
    }
}

// ============================================================
// Metadata
// ============================================================

/// Builds the metadata snapshot for a report. Carries the report type, a
/// human-readable description, the effective filters, and the row count.
fn build_metadata(
    request: &ReportRequest,
    effective_filters: ReportFilters,
    row_count: usize,
) -> ReportMetadata {
    ReportMetadata {
        report_type: request.kind.as_str().to_string(),
        description: request.kind.description().to_string(),
        filters_used: effective_filters,
        generated_at: Utc::now().to_rfc3339(),
        row_count,
    }
}

// ============================================================
// Post-filters
// ============================================================

/// Filters dashboard rows by an optional category id. Per-row `get_product`
/// is used to look up the product's `category_id` — an N+1 pattern that's
/// acceptable for MVP report sizes but should be replaced with a batch query
/// when the dashboard row count grows.
///
/// When `category_id` is `None` or the row's product cannot be found, the
/// row is preserved (no filtering applied). When `category_id` is `Some(_)`
/// but the row's product has no category assigned, the row is dropped.
async fn filter_by_category(
    pool: &DbPool,
    rows: Vec<DashboardLotRow>,
    category_id: Option<&str>,
) -> Result<Vec<DashboardLotRow>, AppError> {
    let Some(target) = category_id else {
        return Ok(rows);
    };
    let target = target.trim();
    if target.is_empty() {
        return Ok(rows);
    }

    let mut kept: Vec<DashboardLotRow> = Vec::with_capacity(rows.len());
    for row in rows {
        let product = crate::db::repositories::products::get_product(pool, &row.product_id).await?;
        match product {
            Some(p) if p.category_id.as_deref() == Some(target) => kept.push(row),
            Some(_) => { /* category mismatch — drop */ }
            None => { /* product missing (e.g. archived then deleted) — drop */ }
        }
    }
    Ok(kept)
}

/// Filters dashboard rows by an inclusive `expiry_date` range. Uses string
/// comparison because `expiry_date` is stored as ISO-8601 `YYYY-MM-DD`, which
/// is naturally lexicographically ordered.
fn filter_by_date_range(
    rows: Vec<DashboardLotRow>,
    date_from: Option<&str>,
    date_to: Option<&str>,
) -> Vec<DashboardLotRow> {
    let from = date_from.map(|s| s.trim()).filter(|s| !s.is_empty());
    let to = date_to.map(|s| s.trim()).filter(|s| !s.is_empty());
    if from.is_none() && to.is_none() {
        return rows;
    }
    rows.into_iter()
        .filter(|r| {
            if let Some(f) = from {
                if r.expiry_date.as_str() < f {
                    return false;
                }
            }
            if let Some(t) = to {
                if r.expiry_date.as_str() > t {
                    return false;
                }
            }
            true
        })
        .collect()
}

// ============================================================
// Service entry point
// ============================================================

/// Builds a `ReportData` payload for the given request.
///
/// Flow:
///   1. Validate filters (dates and inverted ranges).
///   2. Translate to `DashboardFilters` and fetch rows via the dashboard
///      service so urgency, sorting, and counts stay in one place.
///   3. Apply `category_id` and `date_from`/`date_to` post-filters when set.
///   4. Build metadata (type, description, effective filters, generation
///      timestamp, row count).
pub async fn preview_report(pool: &DbPool, request: ReportRequest) -> Result<ReportData, AppError> {
    let effective = request.filters.clone().unwrap_or_default();
    validate_filters(&effective).map_err(AppError::Domain)?;

    let dashboard_filters = to_dashboard_filters(&request);
    let mut response = crate::services::dashboard::get_dashboard(pool, dashboard_filters).await?;

    // Category and date-range post-filters apply to every report type — they
    // layer on top of the preset-based urgency filter rather than replacing
    // it. This lets built-in reports honour store / location / category /
    // date-range filters without forking the SQL.
    response.lots =
        filter_by_category(pool, response.lots, effective.category_id.as_deref()).await?;
    response.lots = filter_by_date_range(
        response.lots,
        effective.date_from.as_deref(),
        effective.date_to.as_deref(),
    );

    let row_count = response.lots.len();
    let metadata = build_metadata(&request, effective, row_count);
    Ok(ReportData {
        metadata,
        lots: response.lots,
    })
}

/// Renders the same report payload to a PDF file on disk. Returns the
/// absolute path, the number of pages written, the row count, and the byte
/// count written.
///
/// The PDF generator reuses the `ReportData` shape, so PDF rows always match
/// the rows returned by [`preview_report`] for the same request. The PDF
/// generator owns its own A4 landscape layout, pagination, and metadata
/// rendering; this service only orchestrates the data fetch + render call.
pub async fn export_report_pdf(
    pool: &DbPool,
    request: ReportRequest,
    path: std::path::PathBuf,
) -> Result<crate::pdf::report_pdf::RenderedReport, AppError> {
    let data = preview_report(pool, request).await?;
    let rendered = crate::pdf::report_pdf::render_report(&data, &path)?;
    Ok(rendered)
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    use chrono::{Duration, NaiveDate};
    use uuid::Uuid;

    use crate::db::migrations::fresh_test_pool;
    use crate::dto::products::{CategoryCreate, ProductCreate};
    use crate::dto::stores::StoreCreate;
    use crate::services::products as products_service;
    use crate::services::stores as stores_service;

    /// Fixture layout used by the report tests:
    ///
    /// ```text
    /// store A  ── expired lot        (-30 days, product PA, alert=14)
    /// store A  ── today lot          (today,      product PB, alert=14)
    /// store A  ── alert-window lot   (+5 days,    product PB, alert=14)
    /// store A  ── next-30-days lot   (+15 days,   product PB, alert=30)
    /// store A  ── future lot         (+90 days,   product PB, alert=30)
    /// store B  ── expired lot        (-15 days,   product PC, alert=14, different store)
    /// ```
    ///
    /// Categories:
    ///   - "Dairy" → PA, PB
    ///   - "Bakery" → PC
    async fn seed_report_fixture(pool: &DbPool) -> ReportFixture {
        let store_a = stores_service::create_store(
            pool,
            StoreCreate {
                name: "Store A".into(),
                code: None,
                notes: None,
            },
        )
        .await
        .expect("store A");

        let store_b = stores_service::create_store(
            pool,
            StoreCreate {
                name: "Store B".into(),
                code: None,
                notes: None,
            },
        )
        .await
        .expect("store B");

        let dairy = products_service::create_category(
            pool,
            CategoryCreate {
                name: "Dairy".into(),
            },
        )
        .await
        .expect("dairy category");

        let bakery = products_service::create_category(
            pool,
            CategoryCreate {
                name: "Bakery".into(),
            },
        )
        .await
        .expect("bakery category");

        let product_a = products_service::create_product(
            pool,
            ProductCreate {
                sku: "PA-001".into(),
                description: "Product A".into(),
                category_id: Some(dairy.id.clone()),
                default_unit: Some("L".into()),
                default_unit_id: None,
                default_alert_days_before: 14,
                notes: None,
            },
        )
        .await
        .expect("product A");

        let product_b = products_service::create_product(
            pool,
            ProductCreate {
                sku: "PB-001".into(),
                description: "Product B".into(),
                category_id: Some(dairy.id.clone()),
                default_unit: Some("kg".into()),
                default_unit_id: None,
                default_alert_days_before: 30,
                notes: None,
            },
        )
        .await
        .expect("product B");

        let product_c = products_service::create_product(
            pool,
            ProductCreate {
                sku: "PC-001".into(),
                description: "Product C".into(),
                category_id: Some(bakery.id.clone()),
                default_unit: Some("pcs".into()),
                default_unit_id: None,
                default_alert_days_before: 14,
                notes: None,
            },
        )
        .await
        .expect("product C");

        // Insert lots directly so we can pin expiry dates relative to today.
        let today = Utc::now().date_naive();
        let now = Utc::now().to_rfc3339();

        // Helper to insert a lot for a given (product, store, expiry).
        async fn insert_lot(
            pool: &DbPool,
            product_id: &str,
            store_id: &str,
            expiry: NaiveDate,
            alert_days: i32,
            qty: f64,
            unit: &str,
        ) {
            let lot_id = Uuid::new_v4().to_string();
            let now = Utc::now().to_rfc3339();
            sqlx::query(
                "INSERT INTO expiry_lots (id, product_id, store_id, quantity, unit,
                                          expiry_date, alert_days_before, status,
                                          created_at, updated_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, 'active', $8, $9)",
            )
            .bind(&lot_id)
            .bind(product_id)
            .bind(store_id)
            .bind(qty)
            .bind(unit)
            .bind(expiry.format("%Y-%m-%d").to_string())
            .bind(alert_days)
            .bind(&now)
            .bind(&now)
            .execute(pool)
            .await
            .expect("insert lot");
        }

        // Store A lots:
        insert_lot(
            pool,
            &product_a.id,
            &store_a.id,
            today - Duration::days(30),
            14,
            2.0,
            "L",
        )
        .await; // expired
        insert_lot(pool, &product_b.id, &store_a.id, today, 14, 5.0, "kg").await; // today
        insert_lot(
            pool,
            &product_b.id,
            &store_a.id,
            today + Duration::days(5),
            14,
            3.0,
            "kg",
        )
        .await; // alert window
        insert_lot(
            pool,
            &product_b.id,
            &store_a.id,
            today + Duration::days(15),
            30,
            4.0,
            "kg",
        )
        .await; // next-30-days (alert_days=30 → not AlertWindow)
        insert_lot(
            pool,
            &product_b.id,
            &store_a.id,
            today + Duration::days(90),
            30,
            6.0,
            "kg",
        )
        .await; // future

        // Store B lots:
        insert_lot(
            pool,
            &product_c.id,
            &store_b.id,
            today - Duration::days(15),
            14,
            1.0,
            "pcs",
        )
        .await; // expired

        let _ = now; // silence unused warning if the helper is inlined

        ReportFixture {
            store_a_id: store_a.id,
            _store_b_id: store_b.id,
            dairy_id: dairy.id,
            bakery_id: bakery.id,
            _product_a_id: product_a.id,
            _product_b_id: product_b.id,
            product_c_id: product_c.id,
        }
    }

    struct ReportFixture {
        store_a_id: String,
        _store_b_id: String,
        dairy_id: String,
        bakery_id: String,
        _product_a_id: String,
        _product_b_id: String,
        product_c_id: String,
    }

    fn empty_filters() -> Option<ReportFilters> {
        Some(ReportFilters::default())
    }

    fn request_of(kind: ReportType, filters: Option<ReportFilters>) -> ReportRequest {
        ReportRequest { kind, filters }
    }

    // ─── ReportType helpers ────────────────────────────────────────────────────

    #[test]
    fn report_type_as_str_matches_serde_rename() {
        assert_eq!(ReportType::InAlertWindow.as_str(), "in_alert_window");
        assert_eq!(ReportType::Expired.as_str(), "expired");
        assert_eq!(ReportType::Next30Days.as_str(), "next_30_days");
        assert_eq!(ReportType::Custom.as_str(), "custom");
    }

    #[test]
    fn report_type_descriptions_are_non_empty() {
        for kind in [
            ReportType::InAlertWindow,
            ReportType::Expired,
            ReportType::Next30Days,
            ReportType::Custom,
        ] {
            assert!(
                !kind.description().is_empty(),
                "description for {:?} must be non-empty",
                kind
            );
        }
    }

    // ─── Filter translation ────────────────────────────────────────────────────

    #[test]
    fn to_dashboard_filters_expired_pins_preset() {
        let req = request_of(ReportType::Expired, empty_filters());
        let f = to_dashboard_filters(&req);
        assert_eq!(f.preset, Some(DashboardPreset::Expired));
        assert!(f.urgency.is_none());
    }

    #[test]
    fn to_dashboard_filters_in_alert_window_pins_preset() {
        let req = request_of(ReportType::InAlertWindow, empty_filters());
        let f = to_dashboard_filters(&req);
        assert_eq!(f.preset, Some(DashboardPreset::AlertWindow));
        assert!(f.urgency.is_none());
    }

    #[test]
    fn to_dashboard_filters_next_30_days_pins_preset() {
        let req = request_of(ReportType::Next30Days, empty_filters());
        let f = to_dashboard_filters(&req);
        assert_eq!(f.preset, Some(DashboardPreset::Next30Days));
        assert!(f.urgency.is_none());
    }

    #[test]
    fn to_dashboard_filters_custom_passes_through_urgency() {
        let filters = ReportFilters {
            urgency: Some("today".to_string()),
            ..Default::default()
        };
        let req = request_of(ReportType::Custom, Some(filters));
        let f = to_dashboard_filters(&req);
        assert!(f.preset.is_none());
        assert_eq!(f.urgency.as_deref(), Some("today"));
    }

    #[test]
    fn to_dashboard_filters_custom_blank_urgency_is_none() {
        let filters = ReportFilters {
            urgency: Some("   ".to_string()),
            ..Default::default()
        };
        let req = request_of(ReportType::Custom, Some(filters));
        let f = to_dashboard_filters(&req);
        assert!(f.preset.is_none());
        assert!(
            f.urgency.is_none(),
            "blank urgency should be treated as None"
        );
    }

    #[test]
    fn to_dashboard_filters_forwards_store_and_location() {
        let filters = ReportFilters {
            store_id: Some("store-x".into()),
            location_id: Some("loc-y".into()),
            ..Default::default()
        };
        let req = request_of(ReportType::Expired, Some(filters));
        let f = to_dashboard_filters(&req);
        assert_eq!(f.store_id.as_deref(), Some("store-x"));
        assert_eq!(f.location_id.as_deref(), Some("loc-y"));
    }

    // ─── Filter validation ─────────────────────────────────────────────────────

    #[test]
    fn validate_filters_accepts_empty() {
        assert!(validate_filters(&ReportFilters::default()).is_ok());
    }

    #[test]
    fn validate_filters_accepts_well_formed_range() {
        let f = ReportFilters {
            date_from: Some("2025-01-01".into()),
            date_to: Some("2025-12-31".into()),
            ..Default::default()
        };
        assert!(validate_filters(&f).is_ok());
    }

    #[test]
    fn validate_filters_rejects_malformed_date() {
        let f = ReportFilters {
            date_from: Some("not-a-date".into()),
            ..Default::default()
        };
        let err = validate_filters(&f).expect_err("malformed date must be rejected");
        assert!(matches!(err, DomainError::Validation { .. }));
    }

    #[test]
    fn validate_filters_rejects_inverted_range() {
        let f = ReportFilters {
            date_from: Some("2025-12-31".into()),
            date_to: Some("2025-01-01".into()),
            ..Default::default()
        };
        let err = validate_filters(&f).expect_err("inverted range must be rejected");
        match err {
            DomainError::Validation { message } => {
                assert!(
                    message.contains("date_from") && message.contains("date_to"),
                    "message must mention both endpoints: {message}"
                );
            }
            other => panic!("expected Validation, got {other:?}"),
        }
    }

    #[test]
    fn validate_filters_treats_blank_strings_as_absent() {
        let f = ReportFilters {
            date_from: Some("   ".into()),
            date_to: Some("".into()),
            ..Default::default()
        };
        assert!(validate_filters(&f).is_ok());
    }

    // ─── Date range post-filter ────────────────────────────────────────────────

    #[test]
    fn date_range_post_filter_is_noop_without_bounds() {
        let rows = vec![make_row("2025-06-01"), make_row("2099-01-01")];
        let out = filter_by_date_range(rows.clone(), None, None);
        assert_eq!(out.len(), rows.len());
    }

    #[test]
    fn date_range_post_filter_inclusive_bounds() {
        let rows = vec![
            make_row("2025-01-01"),
            make_row("2025-06-15"),
            make_row("2025-12-31"),
            make_row("2026-01-01"),
        ];
        let out = filter_by_date_range(rows, Some("2025-01-01"), Some("2025-12-31"));
        assert_eq!(out.len(), 3);
        assert_eq!(out[0].expiry_date, "2025-01-01");
        assert_eq!(out[1].expiry_date, "2025-06-15");
        assert_eq!(out[2].expiry_date, "2025-12-31");
    }

    fn make_row(expiry_date: &str) -> DashboardLotRow {
        DashboardLotRow {
            lot_id: "lot".into(),
            product_id: "prod".into(),
            sku: "SKU".into(),
            description: "desc".into(),
            store_id: "store".into(),
            store_name: "Store".into(),
            location_id: None,
            location_name: None,
            quantity: 1.0,
            unit: "kg".into(),
            expiry_date: expiry_date.into(),
            alert_days_before: 14,
            batch_code: None,
            status: "active".into(),
            urgency: String::new(),
            days_remaining: 0,
            default_unit_id: None,
            unit_type: None,
        }
    }

    // ─── preview_report — built-in types ───────────────────────────────────────

    #[tokio::test]
    async fn preview_report_expired_returns_only_expired_lots() {
        let pool = fresh_test_pool().await.expect("pool");
        let fx = seed_report_fixture(&pool).await;

        let data = preview_report(
            &pool,
            request_of(
                ReportType::Expired,
                Some(ReportFilters {
                    store_id: Some(fx.store_a_id.clone()),
                    ..Default::default()
                }),
            ),
        )
        .await
        .expect("preview");

        assert_eq!(data.metadata.report_type, "expired");
        assert_eq!(data.metadata.row_count, 1);
        assert_eq!(data.lots.len(), 1);
        assert!(data.lots[0].expiry_date < Utc::now().date_naive().format("%Y-%m-%d").to_string());
    }

    #[tokio::test]
    async fn preview_report_in_alert_window_returns_alert_lots() {
        let pool = fresh_test_pool().await.expect("pool");
        let fx = seed_report_fixture(&pool).await;

        let data = preview_report(
            &pool,
            request_of(
                ReportType::InAlertWindow,
                Some(ReportFilters {
                    store_id: Some(fx.store_a_id.clone()),
                    ..Default::default()
                }),
            ),
        )
        .await
        .expect("preview");

        assert_eq!(data.metadata.report_type, "in_alert_window");
        // Fixture has exactly one alert-window lot in store A (+5 days, alert=14).
        assert_eq!(data.lots.len(), 1, "only the +5d lot should be in window");
        assert_eq!(data.metadata.row_count, 1);
    }

    #[tokio::test]
    async fn preview_report_next_30_days_returns_30d_lots() {
        let pool = fresh_test_pool().await.expect("pool");
        let fx = seed_report_fixture(&pool).await;

        let data = preview_report(
            &pool,
            request_of(
                ReportType::Next30Days,
                Some(ReportFilters {
                    store_id: Some(fx.store_a_id.clone()),
                    ..Default::default()
                }),
            ),
        )
        .await
        .expect("preview");

        assert_eq!(data.metadata.report_type, "next_30_days");
        // Fixture: +15d lot is Next30Days (alert=30 → not AlertWindow). The +5d
        // lot is in its AlertWindow and excluded from Next30Days; +90d is Future.
        assert_eq!(data.lots.len(), 1, "only the +15d lot should match");
    }

    // ─── preview_report — custom filters ───────────────────────────────────────

    #[tokio::test]
    async fn preview_report_custom_with_category_filter() {
        let pool = fresh_test_pool().await.expect("pool");
        let fx = seed_report_fixture(&pool).await;

        let data = preview_report(
            &pool,
            request_of(
                ReportType::Custom,
                Some(ReportFilters {
                    category_id: Some(fx.bakery_id.clone()),
                    ..Default::default()
                }),
            ),
        )
        .await
        .expect("preview");

        // Bakery only contains product C (expired in store B).
        assert_eq!(data.lots.len(), 1);
        assert_eq!(data.lots[0].product_id, fx.product_c_id);
    }

    #[tokio::test]
    async fn preview_report_custom_with_date_range() {
        let pool = fresh_test_pool().await.expect("pool");
        let _fx = seed_report_fixture(&pool).await;

        let today = Utc::now().date_naive();
        let date_from = (today + Duration::days(1)).format("%Y-%m-%d").to_string();
        let date_to = (today + Duration::days(20)).format("%Y-%m-%d").to_string();

        let data = preview_report(
            &pool,
            request_of(
                ReportType::Custom,
                Some(ReportFilters {
                    date_from: Some(date_from.clone()),
                    date_to: Some(date_to.clone()),
                    ..Default::default()
                }),
            ),
        )
        .await
        .expect("preview");

        // Fixture lots inside [today+1, today+20]: the +5d alert-window lot and
        // the +15d next-30-days lot. Expired (-30d, today, -15d) and future
        // (+90d) lots are excluded.
        assert_eq!(
            data.lots.len(),
            2,
            "expected +5d and +15d lots inside the date range"
        );
        for lot in &data.lots {
            assert!(
                lot.expiry_date.as_str() >= date_from.as_str(),
                "lot {} below lower bound",
                lot.expiry_date
            );
            assert!(
                lot.expiry_date.as_str() <= date_to.as_str(),
                "lot {} above upper bound",
                lot.expiry_date
            );
        }
    }

    #[tokio::test]
    async fn preview_report_custom_with_urgency_filter() {
        let pool = fresh_test_pool().await.expect("pool");
        let _fx = seed_report_fixture(&pool).await;

        let data = preview_report(
            &pool,
            request_of(
                ReportType::Custom,
                Some(ReportFilters {
                    urgency: Some("expired".to_string()),
                    ..Default::default()
                }),
            ),
        )
        .await
        .expect("preview");

        // All expired lots, across all stores.
        assert_eq!(data.lots.len(), 2, "store A + store B expired lots");
        for lot in &data.lots {
            assert_eq!(lot.urgency, "expired");
        }
    }

    #[tokio::test]
    async fn preview_report_custom_no_filters_returns_all_lots() {
        let pool = fresh_test_pool().await.expect("pool");
        let _fx = seed_report_fixture(&pool).await;

        let data = preview_report(&pool, request_of(ReportType::Custom, None))
            .await
            .expect("preview");

        // Fixture: 5 lots in store A + 1 in store B = 6 active lots.
        assert_eq!(data.lots.len(), 6);
    }

    // ─── preview_report — metadata ─────────────────────────────────────────────

    #[tokio::test]
    async fn preview_report_metadata_includes_type_description_and_count() {
        let pool = fresh_test_pool().await.expect("pool");
        let _fx = seed_report_fixture(&pool).await;

        let data = preview_report(&pool, request_of(ReportType::Expired, empty_filters()))
            .await
            .expect("preview");

        assert_eq!(data.metadata.report_type, "expired");
        assert!(!data.metadata.description.is_empty());
        assert!(
            !data.metadata.generated_at.is_empty(),
            "generated_at must be set"
        );
        // RFC3339 contains "T" and a timezone offset or "Z".
        assert!(data.metadata.generated_at.contains('T'));
        assert_eq!(
            data.metadata.row_count,
            data.lots.len(),
            "row_count must match the lot list"
        );
    }

    #[tokio::test]
    async fn preview_report_metadata_captures_effective_filters() {
        let pool = fresh_test_pool().await.expect("pool");
        let fx = seed_report_fixture(&pool).await;

        let data = preview_report(
            &pool,
            request_of(
                ReportType::Custom,
                Some(ReportFilters {
                    store_id: Some(fx.store_a_id.clone()),
                    category_id: Some(fx.dairy_id.clone()),
                    urgency: Some("expired".to_string()),
                    ..Default::default()
                }),
            ),
        )
        .await
        .expect("preview");

        assert_eq!(
            data.metadata.filters_used.store_id.as_deref(),
            Some(fx.store_a_id.as_str())
        );
        assert_eq!(
            data.metadata.filters_used.category_id.as_deref(),
            Some(fx.dairy_id.as_str())
        );
        assert_eq!(
            data.metadata.filters_used.urgency.as_deref(),
            Some("expired")
        );
    }

    // ─── preview_report — validation ───────────────────────────────────────────

    #[tokio::test]
    async fn preview_report_rejects_malformed_date() {
        let pool = fresh_test_pool().await.expect("pool");

        let err = preview_report(
            &pool,
            request_of(
                ReportType::Custom,
                Some(ReportFilters {
                    date_from: Some("bogus".into()),
                    ..Default::default()
                }),
            ),
        )
        .await
        .expect_err("malformed date must be rejected");
        assert!(matches!(
            err,
            AppError::Domain(DomainError::Validation { .. })
        ));
    }

    #[tokio::test]
    async fn preview_report_rejects_inverted_range() {
        let pool = fresh_test_pool().await.expect("pool");

        let err = preview_report(
            &pool,
            request_of(
                ReportType::Custom,
                Some(ReportFilters {
                    date_from: Some("2025-12-31".into()),
                    date_to: Some("2025-01-01".into()),
                    ..Default::default()
                }),
            ),
        )
        .await
        .expect_err("inverted range must be rejected");
        assert!(matches!(
            err,
            AppError::Domain(DomainError::Validation { .. })
        ));
    }
}
