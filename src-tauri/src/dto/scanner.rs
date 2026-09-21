//! DTOs for the scanner/search workflow.
//!
//! `ScanSearchResult` is returned by the `find_product_by_scan` command and
//! carries the result of a barcode-first → SKU-second exact lookup. The
//! serialized `match_type` tag tells the frontend which branch was taken so
//! it can route the user to the appropriate flow (lot entry vs product detail).
//!
//! `ScannerResolveResult` is the discriminated union returned by the
//! `resolve_scanner_code` command introduced by `scanner-quick-operations`
//! (PR 1 backend foundation). The Scanner tab uses it to dispatch between
//! Sale, Registration, and Stock-out flows without re-querying. The
//! discriminator is `match_type` so the existing dashboard scan/search
//! surface (which uses the same tag with its own variants) can keep its
//! shape unchanged.

use serde::{Deserialize, Serialize};

use super::expiry_lots::ExpiryLotResponse;
use super::products::{ProductResponse, ProductSearchResult};

/// Union result returned by `find_product_by_scan`.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "match_type", rename_all = "snake_case")]
pub enum ScanSearchResult {
    /// The scanned value matched a known product.
    Found {
        /// The matched product.
        product: ProductSearchResult,
        /// Whether the product already has at least one active expiry lot
        /// in the current store context.
        has_lots: bool,
    },
    /// No barcode or SKU matched the scanned value.
    NotFound {
        /// The value that was scanned/typed — pre-fill hint for quick create.
        scanned_value: String,
    },
}

/// Input for `resolve_scanner_code`. The frontend submits the raw scanned
/// value as the user typed it; the command boundary trims leading/trailing
/// whitespace before the priority-ordered lookup.
#[derive(Debug, Deserialize)]
pub struct ScannerResolveInput {
    /// Raw scanned value, exactly as the user submitted it. Trimming happens
    /// inside the command boundary so the rejection for an empty value uses
    /// the same canonical `ScanValueEmpty` text the existing scan/search path
    /// already surfaces.
    pub scanned_value: String,
}

/// Discriminated union returned by the new `resolve_scanner_code` command.
///
/// The frontend dispatches by `match_type`:
///
/// - `lot_match`: the scanner resolved directly to one active lot in the
///   active store. The lot is already selected — FEFO does not apply, the
///   lot picker is NOT rendered, and the FEFO policy (even `require_fefo`)
///   MUST NOT replace the scanned lot with another lot. See spec
///   `Requirement: FEFO lot-selection policy` and the
///   `lot-code match bypasses the FEFO policy` scenario.
/// - `product_match`: the scanner resolved to an active product (via
///   barcode or SKU). The frontend applies the FEFO policy to the returned
///   `lots` vector (which is already ordered by
///   `expiry_date ASC, created_at ASC, id ASC`) per the active policy.
/// - `unknown`: the scanner did not match any lot, product barcode, or
///   product SKU. The frontend preserves `scanned_value` verbatim
///   (no upper-casing, lower-casing, or whitespace normalization) and
///   routes by mode (Sale: show inline error; Registration: start quick
///   product creation with the value as a candidate SKU/barcode;
///   Stock-out: show inline error).
///
/// The archived-product filter is enforced at the service layer: the
/// barcode/SKU branches only match products whose `is_active = true`.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "match_type", rename_all = "snake_case")]
pub enum ScannerResolveResult {
    /// The scanned value matched an active lot in the active store by exact
    /// `expiry_lots.batch_code`.
    LotMatch {
        /// The matched lot.
        lot: ExpiryLotResponse,
        /// The lot's parent product (resolved in the same command).
        product: ProductResponse,
    },
    /// The scanned value matched an active product by exact barcode or SKU.
    ProductMatch {
        /// The matched product.
        product: ProductResponse,
        /// The product's active lots for the active store, ordered by stable
        /// FEFO order `(expiry_date ASC, created_at ASC, id ASC)`.
        lots: Vec<ExpiryLotResponse>,
    },
    /// No lot, product barcode, or product SKU matched.
    Unknown {
        /// The trimmed scanned value, preserved verbatim so the frontend can
        /// pre-fill the Registration quick-create form.
        scanned_value: String,
    },
}
