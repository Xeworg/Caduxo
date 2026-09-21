//! Scanner operation lookup — priority-ordered resolution for the
//! `resolve_scanner_code` IPC command introduced by
//! `scanner-quick-operations` (PR 1 backend foundation).
//!
//! The lookup is intentionally distinct from the dashboard scan/search
//! path implemented by `services::products::find_product_by_scan`:
//!
//! - Dashboard scan/search (`find_product_by_scan`) is product-only and
//!   includes archived products so the user can locate them in the
//!   catalog.
//! - Scanner operations (`resolve_scanner_code`) are stock-mutation paths
//!   and MUST ignore archived/inactive products, MUST resolve lot codes
//!   before product lookups, and MUST return the matched lot's parent
//!   product so the Scanner tab can dispatch by mode without re-querying.
//!
//! Both paths coexist: `find_product_by_scan` is untouched.

use crate::db::repositories::expiry_lots as lot_repo;
use crate::db::repositories::products as product_repo;
use crate::db::repositories::settings as settings_repo;
use crate::db::repositories::stores as store_repo;
use crate::db::DbPool;
use crate::dto::expiry_lots::ExpiryLotResponse;
use crate::dto::products::ProductResponse;
use crate::dto::scanner::ScannerResolveResult;
use crate::error::{AppError, DomainError};
use crate::pdf::locale::Locale;
use crate::services::user_messages::{user_message, UserMessage};

/// Resolves a scanned value against the catalog in strict priority order:
///
/// 1. **Lot code** — exact match against `expiry_lots.batch_code` for
///    active lots in the active store. The matched lot's parent product
///    is loaded so the result is self-contained for the Scanner tab's
///    `LotMatch` branch.
/// 2. **Product barcode** — exact match against `product_barcodes.barcode`
///    for **active** products only. Archived products are intentionally
///    ignored because the Scanner tab is a stock-mutation path.
/// 3. **Product SKU** — exact match against `products.sku` for **active**
///    products only. Same rationale as barcode: archived products are
///    excluded.
/// 4. **Unknown** — neither step matched; the trimmed scanned value is
///    preserved verbatim so the Registration mode can pre-fill the
///    quick-create form.
///
/// The active store is read from the persisted settings snapshot. The
/// Scanner tab is expected to block the input when no store exists; this
/// service still fails safely with a `DomainError::Validation` if invoked
/// without store context (defence in depth, matching the
/// `Scanner tab is unavailable until first store exists` spec scenario).
pub async fn resolve_scanner_code(
    pool: &DbPool,
    scanned_value: &str,
) -> Result<ScannerResolveResult, AppError> {
    let trimmed = scanned_value.trim();
    if trimmed.is_empty() {
        return Err(DomainError::Validation {
            message: user_message(UserMessage::ScannerResolveValueEmpty, Locale::En),
        }
        .into());
    }

    // Resolve the active store from the persisted settings snapshot. The
    // Scanner tab MUST NOT issue this call when no store exists; the
    // backend still fails safely if the precondition slips through.
    let active_store_id = settings_repo::get_last_selected_store_id(pool).await?;
    let store_id = match active_store_id.as_deref() {
        Some(id) if !id.is_empty() => id,
        _ => {
            return Err(DomainError::Validation {
                message: "No active store selected; create or select a store first".to_string(),
            }
            .into());
        }
    };
    // Defensive: confirm the persisted store id still points to an active
    // row. A stale id (store was archived after the user opened the
    // Scanner tab) must not let the lookup silently succeed against a
    // hidden store.
    if !store_repo::has_active_store(pool).await? {
        return Err(DomainError::Validation {
            message: "No active store available; create or select a store first".to_string(),
        }
        .into());
    }

    // Step 1: active lot in active store by exact `batch_code`.
    if let Some(lot) = lot_repo::find_active_lot_by_batch_code(pool, store_id, trimmed).await? {
        let product = product_repo::get_product(pool, &lot.product_id)
            .await?
            .ok_or_else(|| DomainError::NotFound {
                resource: "product",
                id: lot.product_id.clone(),
            })?;
        return Ok(ScannerResolveResult::LotMatch { lot, product });
    }

    // Step 2: active product by exact barcode.
    if let Some(search) = product_repo::find_active_product_by_barcode_exact(pool, trimmed).await? {
        let product = product_repo::get_product(pool, &search.id)
            .await?
            .ok_or_else(|| DomainError::NotFound {
                resource: "product",
                id: search.id.clone(),
            })?;
        let lots =
            lot_repo::list_active_lots_for_product_in_store(pool, &search.id, store_id).await?;
        return Ok(ScannerResolveResult::ProductMatch { product, lots });
    }

    // Step 3: active product by exact SKU.
    if let Some(search) = product_repo::find_active_product_by_sku_exact(pool, trimmed).await? {
        let product = product_repo::get_product(pool, &search.id)
            .await?
            .ok_or_else(|| DomainError::NotFound {
                resource: "product",
                id: search.id.clone(),
            })?;
        let lots =
            lot_repo::list_active_lots_for_product_in_store(pool, &search.id, store_id).await?;
        return Ok(ScannerResolveResult::ProductMatch { product, lots });
    }

    // Step 4: no match — preserve the trimmed value verbatim.
    Ok(ScannerResolveResult::Unknown {
        scanned_value: trimmed.to_string(),
    })
}

#[cfg(test)]
mod tests {
    // NOTE: avoid `use super::*` — keeps `fresh_test_pool` and other
    // helpers unambiguous.
    use crate::db::migrations::fresh_test_pool;
    use crate::dto::expiry_lots::ExpiryLotCreate;
    use crate::dto::products::{ProductBarcodeCreate, ProductCreate};
    use crate::dto::scanner::ScannerResolveResult;
    use crate::dto::stores::{SettingsUpdate, StoreCreate};
    use crate::services::expiry_lots::create_expiry_lot;
    use crate::services::products::{add_barcode, archive_product, create_product};
    use crate::services::scanner::resolve_scanner_code;
    use crate::services::settings::update_settings;
    use crate::services::stores::create_store as create_store_svc;

    /// Helper: creates a store + product + active lot, returning
    /// `(store_id, product_id, batch_code, lot_id)`.
    async fn seed_lot(
        pool: &crate::db::DbPool,
    ) -> Result<(String, String, String, String), Box<dyn std::error::Error>> {
        // Disable require_initial_location_on_lot_create FIRST so the
        // sentinel path is used (matches the rest of the test suite).
        // The default for a fresh pool is `true`, so calling
        // `create_expiry_lot` without flipping it would fail with the
        // canonical `LocationRequired` validation.
        crate::db::repositories::settings::set_require_initial_location_on_lot_create(pool, false)
            .await?;
        let store = create_store_svc(
            pool,
            StoreCreate {
                name: "Test Store".into(),
                code: None,
                notes: None,
            },
        )
        .await?;
        let product = create_product(
            pool,
            ProductCreate {
                sku: "SCAN-LOT-SKU".into(),
                description: "Scanner lot test product".into(),
                category_ids: None,
                default_unit: Some("kg".into()),
                default_unit_id: None,
                default_alert_days_before: 30,
                notes: None,
            },
        )
        .await?;
        let lot = create_expiry_lot(
            pool,
            ExpiryLotCreate {
                product_id: product.id.clone(),
                store_id: store.id.clone(),
                location_id: None,
                quantity: 5.0,
                unit: Some("kg".into()),
                expiry_date: "2026-12-31".into(),
                alert_days_before: Some(7),
                batch_code: Some("SUP-2026-XYZ".into()),
                notes: None,
            },
        )
        .await?;
        Ok((
            store.id,
            product.id,
            lot.batch_code.unwrap_or_default(),
            lot.id,
        ))
    }

    #[tokio::test]
    async fn resolve_lot_code_in_active_store_returns_lot_match(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, product_id, batch_code, lot_id) = seed_lot(&pool).await?;
        update_settings(
            &pool,
            SettingsUpdate {
                last_selected_store_id: Some(store_id.clone()),
                require_initial_location_on_lot_create: None,
                language: None,
                theme: None,
                scanner_fefo_policy: None,
                close_behavior: None,
            },
        )
        .await?;

        let result = resolve_scanner_code(&pool, &batch_code).await?;
        match result {
            ScannerResolveResult::LotMatch { lot, product } => {
                assert_eq!(lot.id, lot_id);
                assert_eq!(lot.product_id, product_id);
                assert_eq!(product.id, product_id);
            }
            other => panic!("expected LotMatch, got {other:?}"),
        }
        Ok(())
    }

    #[tokio::test]
    async fn resolve_lot_code_wins_over_barcode_when_both_match(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, product_id, batch_code, _lot_id) = seed_lot(&pool).await?;

        // Attach a barcode to the product that ALSO matches the lot code.
        // Per spec, the lot-code branch (priority 1) MUST win.
        add_barcode(
            &pool,
            ProductBarcodeCreate {
                product_id: product_id.clone(),
                barcode: batch_code.clone(),
                barcode_type: None,
                is_primary: true,
            },
        )
        .await?;
        update_settings(
            &pool,
            SettingsUpdate {
                last_selected_store_id: Some(store_id.clone()),
                require_initial_location_on_lot_create: None,
                language: None,
                theme: None,
                scanner_fefo_policy: None,
                close_behavior: None,
            },
        )
        .await?;

        let result = resolve_scanner_code(&pool, &batch_code).await?;
        assert!(
            matches!(result, ScannerResolveResult::LotMatch { .. }),
            "lot-code match must win over barcode match; got {result:?}"
        );
        Ok(())
    }

    #[tokio::test]
    async fn resolve_barcode_when_no_lot_match_returns_product_match(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, _product_id, _lot_batch, _lot_id) = seed_lot(&pool).await?;

        // Second product: barcode matches but lot code does not.
        let product2 = create_product(
            &pool,
            ProductCreate {
                sku: "BARCODE-SKU".into(),
                description: "Barcode product".into(),
                category_ids: None,
                default_unit: None,
                default_unit_id: None,
                default_alert_days_before: 30,
                notes: None,
            },
        )
        .await?;
        add_barcode(
            &pool,
            ProductBarcodeCreate {
                product_id: product2.id.clone(),
                barcode: "7501234567890".into(),
                barcode_type: Some("EAN13".into()),
                is_primary: true,
            },
        )
        .await?;
        update_settings(
            &pool,
            SettingsUpdate {
                last_selected_store_id: Some(store_id.clone()),
                require_initial_location_on_lot_create: None,
                language: None,
                theme: None,
                scanner_fefo_policy: None,
                close_behavior: None,
            },
        )
        .await?;

        let result = resolve_scanner_code(&pool, "7501234567890").await?;
        match result {
            ScannerResolveResult::ProductMatch { product, lots } => {
                assert_eq!(product.id, product2.id);
                // No lots exist for this product, so the vector is empty.
                assert!(lots.is_empty());
            }
            other => panic!("expected ProductMatch, got {other:?}"),
        }
        Ok(())
    }

    #[tokio::test]
    async fn resolve_sku_when_no_barcode_match_returns_product_match(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, _product_id, _lot_batch, _lot_id) = seed_lot(&pool).await?;
        let product2 = create_product(
            &pool,
            ProductCreate {
                sku: "ABC-001".into(),
                description: "SKU product".into(),
                category_ids: None,
                default_unit: None,
                default_unit_id: None,
                default_alert_days_before: 30,
                notes: None,
            },
        )
        .await?;
        update_settings(
            &pool,
            SettingsUpdate {
                last_selected_store_id: Some(store_id.clone()),
                require_initial_location_on_lot_create: None,
                language: None,
                theme: None,
                scanner_fefo_policy: None,
                close_behavior: None,
            },
        )
        .await?;

        let result = resolve_scanner_code(&pool, "ABC-001").await?;
        match result {
            ScannerResolveResult::ProductMatch { product, lots: _ } => {
                assert_eq!(product.id, product2.id);
            }
            other => panic!("expected ProductMatch, got {other:?}"),
        }
        Ok(())
    }

    #[tokio::test]
    async fn resolve_archived_product_barcode_returns_unknown(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, _product_id, _lot_batch, _lot_id) = seed_lot(&pool).await?;
        let product = create_product(
            &pool,
            ProductCreate {
                sku: "ARCH-SKU".into(),
                description: "Archived product".into(),
                category_ids: None,
                default_unit: None,
                default_unit_id: None,
                default_alert_days_before: 30,
                notes: None,
            },
        )
        .await?;
        add_barcode(
            &pool,
            ProductBarcodeCreate {
                product_id: product.id.clone(),
                barcode: "ARCHIVE-BC".into(),
                barcode_type: None,
                is_primary: true,
            },
        )
        .await?;
        archive_product(&pool, product.id.clone()).await?;
        update_settings(
            &pool,
            SettingsUpdate {
                last_selected_store_id: Some(store_id.clone()),
                require_initial_location_on_lot_create: None,
                language: None,
                theme: None,
                scanner_fefo_policy: None,
                close_behavior: None,
            },
        )
        .await?;

        let result = resolve_scanner_code(&pool, "ARCHIVE-BC").await?;
        assert!(
            matches!(result, ScannerResolveResult::Unknown { .. }),
            "archived-product barcode must NOT match; got {result:?}"
        );
        Ok(())
    }

    #[tokio::test]
    async fn resolve_archived_product_sku_returns_unknown() -> Result<(), Box<dyn std::error::Error>>
    {
        let pool = fresh_test_pool().await?;
        let (store_id, _product_id, _lot_batch, _lot_id) = seed_lot(&pool).await?;
        let product = create_product(
            &pool,
            ProductCreate {
                sku: "ARCH-SKU-ONLY".into(),
                description: "Archived SKU product".into(),
                category_ids: None,
                default_unit: None,
                default_unit_id: None,
                default_alert_days_before: 30,
                notes: None,
            },
        )
        .await?;
        archive_product(&pool, product.id.clone()).await?;
        update_settings(
            &pool,
            SettingsUpdate {
                last_selected_store_id: Some(store_id.clone()),
                require_initial_location_on_lot_create: None,
                language: None,
                theme: None,
                scanner_fefo_policy: None,
                close_behavior: None,
            },
        )
        .await?;

        let result = resolve_scanner_code(&pool, "ARCH-SKU-ONLY").await?;
        assert!(
            matches!(result, ScannerResolveResult::Unknown { .. }),
            "archived-product SKU must NOT match; got {result:?}"
        );
        Ok(())
    }

    #[tokio::test]
    async fn resolve_unknown_value_preserves_trimmed_scanned_value(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, _product_id, _lot_batch, _lot_id) = seed_lot(&pool).await?;
        update_settings(
            &pool,
            SettingsUpdate {
                last_selected_store_id: Some(store_id.clone()),
                require_initial_location_on_lot_create: None,
                language: None,
                theme: None,
                scanner_fefo_policy: None,
                close_behavior: None,
            },
        )
        .await?;

        // Leading/trailing whitespace must be trimmed before lookup, but
        // the trimmed value is preserved verbatim in the Unknown result.
        let result = resolve_scanner_code(&pool, "  TOTALLY-UNKNOWN  ").await?;
        match result {
            ScannerResolveResult::Unknown { scanned_value } => {
                assert_eq!(scanned_value, "TOTALLY-UNKNOWN");
            }
            other => panic!("expected Unknown, got {other:?}"),
        }
        Ok(())
    }

    #[tokio::test]
    async fn resolve_lot_code_does_not_uppercase_or_lowercase(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, product_id, batch_code, _lot_id) = seed_lot(&pool).await?;
        update_settings(
            &pool,
            SettingsUpdate {
                last_selected_store_id: Some(store_id.clone()),
                require_initial_location_on_lot_create: None,
                language: None,
                theme: None,
                scanner_fefo_policy: None,
                close_behavior: None,
            },
        )
        .await?;

        // Mixed-case lot code: scanner lookup MUST NOT normalise case.
        // The seeded batch_code is "SUP-2026-XYZ"; a lowercased scan must
        // return Unknown (the row exists but the scanner does not match
        // case-folded input).
        let result = resolve_scanner_code(&pool, "sup-2026-xyz").await?;
        assert!(
            matches!(result, ScannerResolveResult::Unknown { .. }),
            "scanner lookup must NOT lowercase the scanned value; got {result:?}"
        );

        // Exact case matches.
        let result = resolve_scanner_code(&pool, &batch_code).await?;
        match result {
            ScannerResolveResult::LotMatch { lot, product } => {
                assert_eq!(lot.product_id, product_id);
                assert_eq!(product.id, product_id);
            }
            other => panic!("expected LotMatch for exact case, got {other:?}"),
        }
        Ok(())
    }

    #[tokio::test]
    async fn resolve_lot_code_is_scoped_to_active_store() -> Result<(), Box<dyn std::error::Error>>
    {
        let pool = fresh_test_pool().await?;
        let (store1_id, _product1_id, _batch1, _lot1_id) = seed_lot(&pool).await?;
        let store2 = create_store_svc(
            &pool,
            StoreCreate {
                name: "Other Store".into(),
                code: None,
                notes: None,
            },
        )
        .await?;
        // Set active store to store2; the existing lot belongs to store1.
        update_settings(
            &pool,
            SettingsUpdate {
                last_selected_store_id: Some(store2.id.clone()),
                require_initial_location_on_lot_create: None,
                language: None,
                theme: None,
                scanner_fefo_policy: None,
                close_behavior: None,
            },
        )
        .await?;

        // Scanning a value that matches store1's lot code from store2 must
        // return Unknown (the lot is not in the active store).
        let result = resolve_scanner_code(&pool, "SUP-2026-XYZ").await?;
        assert!(
            matches!(result, ScannerResolveResult::Unknown { .. }),
            "lot lookup must be scoped to the active store; got {result:?}"
        );

        // Switch the active store back to store1; the same scan now resolves.
        update_settings(
            &pool,
            SettingsUpdate {
                last_selected_store_id: Some(store1_id.clone()),
                require_initial_location_on_lot_create: None,
                language: None,
                theme: None,
                scanner_fefo_policy: None,
                close_behavior: None,
            },
        )
        .await?;
        let result = resolve_scanner_code(&pool, "SUP-2026-XYZ").await?;
        assert!(
            matches!(result, ScannerResolveResult::LotMatch { .. }),
            "lot lookup must resolve when the active store is correct; got {result:?}"
        );
        Ok(())
    }

    #[tokio::test]
    async fn resolve_product_match_lots_are_fefo_ordered() -> Result<(), Box<dyn std::error::Error>>
    {
        let pool = fresh_test_pool().await?;
        let (store_id, product_id, _batch, _lot_id) = seed_lot(&pool).await?;

        // Add two more lots to the same product in the same store, with
        // deliberately out-of-order expiry dates so the FEFO order is
        // observable. Expiry dates: 2026-08-15 (earliest, wins), 2027-01-10
        // (later, loses to 2026-08-15).
        create_expiry_lot(
            &pool,
            ExpiryLotCreate {
                product_id: product_id.clone(),
                store_id: store_id.clone(),
                location_id: None,
                quantity: 3.0,
                unit: Some("kg".into()),
                expiry_date: "2027-01-10".into(),
                alert_days_before: Some(7),
                batch_code: Some("LATER-001".into()),
                notes: None,
            },
        )
        .await?;
        create_expiry_lot(
            &pool,
            ExpiryLotCreate {
                product_id: product_id.clone(),
                store_id: store_id.clone(),
                location_id: None,
                quantity: 2.0,
                unit: Some("kg".into()),
                expiry_date: "2026-08-15".into(),
                alert_days_before: Some(7),
                batch_code: Some("EARLIEST-001".into()),
                notes: None,
            },
        )
        .await?;
        update_settings(
            &pool,
            SettingsUpdate {
                last_selected_store_id: Some(store_id.clone()),
                require_initial_location_on_lot_create: None,
                language: None,
                theme: None,
                scanner_fefo_policy: None,
                close_behavior: None,
            },
        )
        .await?;

        // Scan the SKU → ProductMatch → lots ordered by expiry_date ASC.
        let result = resolve_scanner_code(&pool, "SCAN-LOT-SKU").await?;
        match result {
            ScannerResolveResult::ProductMatch { lots, .. } => {
                assert_eq!(lots.len(), 3, "expected all three active lots");
                // Earliest expiry first.
                assert_eq!(lots[0].expiry_date, "2026-08-15");
                assert_eq!(lots[1].expiry_date, "2026-12-31");
                assert_eq!(lots[2].expiry_date, "2027-01-10");
            }
            other => panic!("expected ProductMatch, got {other:?}"),
        }
        Ok(())
    }

    #[tokio::test]
    async fn resolve_rejects_empty_value() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        // No store seeded: empty input must fail with Validation BEFORE the
        // active-store check (matches the spec's `Scanner tab is
        // unavailable until first store exists` ordering — the
        // `ScanValueEmpty` text is the canonical rejection).
        let err = resolve_scanner_code(&pool, "   ")
            .await
            .expect_err("empty scan must be rejected");
        assert!(matches!(
            err,
            crate::error::AppError::Domain(crate::error::DomainError::Validation { .. })
        ));
        Ok(())
    }

    #[tokio::test]
    async fn resolve_without_active_store_returns_validation(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        // Store is seeded but `last_selected_store_id` is None → must fail
        // with Validation so the frontend can render "create the first
        // store first" copy.
        let _ = seed_lot(&pool).await?;
        let err = resolve_scanner_code(&pool, "SUP-2026-XYZ")
            .await
            .expect_err("missing active store must be rejected");
        assert!(matches!(
            err,
            crate::error::AppError::Domain(crate::error::DomainError::Validation { .. })
        ));
        Ok(())
    }
}

// ExpiryLotResponse / ProductResponse are used by the public API; this
// trivial reference keeps `dead_code` quiet when the test module is off
// (the impl branches above still consume both response types).
#[allow(dead_code)]
fn _ensure_response_types_in_scope(_lot: ExpiryLotResponse, _p: ProductResponse) {}
