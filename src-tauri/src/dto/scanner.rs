//! DTOs for the scanner/search workflow.
//!
//! `ScanSearchResult` is returned by the `find_product_by_scan` command and
//! carries the result of a barcode-first → SKU-second exact lookup. The
//! serialized `match_type` tag tells the frontend which branch was taken so
//! it can route the user to the appropriate flow (lot entry vs product detail).

use serde::Serialize;

use super::products::ProductSearchResult;

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
