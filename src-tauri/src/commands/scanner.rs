//! Tauri command adapters for the scanner operation surface.
//!
//! PR 1 of `scanner-quick-operations` ships `resolve_scanner_code`, the
//! new priority-ordered scanner lookup command. The command is a thin
//! adapter around `services::scanner::resolve_scanner_code`; the IPC
//! boundary trims the scanned value before invoking the service so the
//! `ScanValueEmpty` rejection uses the same canonical text as the existing
//! `find_product_by_scan` path.

use tauri::State;

use crate::dto::scanner::{ScannerResolveInput, ScannerResolveResult};
use crate::error::{AppError, CommandError};
use crate::pdf::locale::Locale;
use crate::services::scanner as service;
use crate::services::user_messages::{localize_internal, localize_validation};
use crate::state::AppState;

/// Resolves a scanned value against the catalog in strict priority order:
/// lot code, product barcode, product SKU, Unknown.
///
/// 1. **Lot code** — exact match against `expiry_lots.batch_code` for
///    active lots in the active store. Returns `LotMatch { lot, product }`.
/// 2. **Product barcode** — exact match against `product_barcodes.barcode`
///    for active products only.
/// 3. **Product SKU** — exact match against `products.sku` for active
///    products only.
/// 4. **Unknown** — neither step matched; the trimmed scanned value is
///    preserved verbatim.
///
/// The Scanner tab MUST NOT issue this call when no active store exists;
/// the backend still fails safely with a `Validation` if the precondition
/// slips through. The command lives alongside `find_product_by_scan`
/// without colliding — the dashboard scan/search surface remains a
/// product-only path that continues to use the legacy command.
///
/// `locale` (BCP-47 tag) is forwarded to `localize_validation` so the
/// `ScanValueEmpty` rejection reaches the UI in the active locale. Unknown
/// tags fall back to English via `Locale::parse`.
#[tauri::command]
pub async fn resolve_scanner_code(
    state: State<'_, AppState>,
    input: ScannerResolveInput,
    locale: Option<String>,
) -> Result<ScannerResolveResult, CommandError> {
    let pool = state.pool().await;
    let loc = locale.as_deref().map(Locale::parse).unwrap_or(Locale::En);
    service::resolve_scanner_code(&pool, &input.scanned_value)
        .await
        .map_err(|e| localize_validation(e, loc))
        .map_err(|e| localize_internal(e, loc))
        .map_err(AppError::into)
}
