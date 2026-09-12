//! DTOs for the scanner/search workflow.
//!
//! `ScanSearchResult` is returned by the `find_product_by_scan` command and
//! carries the result of a barcode-first → SKU-second exact lookup.
//! `ScanMatchType` tells the frontend why the result matched so it can
//! route the user to the appropriate flow (lot entry vs product detail).

use serde::{Deserialize, Serialize};

use super::products::ProductSearchResult;

/// Tells the frontend which path produced a match.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanMatchType {
    /// The scanned value matched a barcode in `product_barcodes`.
    Barcode,
    /// The scanned value matched a product SKU exactly.
    Sku,
}

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
