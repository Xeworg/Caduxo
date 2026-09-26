//! Tauri commands for expiry lot management: create, update, archive,
//! list, get, and partial resolution.

use tauri::State;

use crate::dto::expiry_lots::{
    ArchiveLotInput, ExpiryLotCreate, ExpiryLotDistributedCreate, ExpiryLotDistributedCreateResult,
    ExpiryLotResolve, ExpiryLotResolveResult, ExpiryLotResponse, ExpiryLotUpdate,
    LotResolutionEventResponse,
};
use crate::error::{AppError, CommandError};
use crate::pdf::locale::Locale;
use crate::services::expiry_lots as service;
use crate::services::user_messages::{
    localize_business_rule, localize_duplicate_field, localize_internal, localize_not_found,
    localize_validation,
};
use crate::state::AppState;

/// Resolves an optional BCP-47 locale tag into a [`Locale`], falling back to
/// English when the frontend does not supply one. Used at every expiry-lot
/// command boundary that accepts an optional locale so dynamic BusinessRule
/// messages (status / quantity / unit) and the adjacent resolve-quantity
/// Validation reach the UI in the active locale while English stays the
/// safe default for legacy callers that omit the argument.
fn resolve_locale(locale: Option<String>) -> Locale {
    locale.as_deref().map(Locale::parse).unwrap_or(Locale::En)
}

/// Returns all active expiry lots across all stores, ordered by expiry date.
#[tauri::command]
pub async fn list_expiry_lots(
    state: State<'_, AppState>,
) -> Result<Vec<ExpiryLotResponse>, CommandError> {
    let pool = state.pool().await;
    service::list_all_expiry_lots(&pool)
        .await
        .map_err(AppError::into)
}

/// Returns all active expiry lots for a given store.
#[tauri::command]
pub async fn list_expiry_lots_by_store(
    state: State<'_, AppState>,
    store_id: String,
) -> Result<Vec<ExpiryLotResponse>, CommandError> {
    let pool = state.pool().await;
    service::list_expiry_lots_by_store(&pool, store_id)
        .await
        .map_err(AppError::into)
}

/// Returns all active expiry lots for a given product, ordered by expiry date.
#[tauri::command]
pub async fn list_expiry_lots_by_product(
    state: State<'_, AppState>,
    product_id: String,
) -> Result<Vec<ExpiryLotResponse>, CommandError> {
    let pool = state.pool().await;
    service::list_expiry_lots_by_product(&pool, product_id)
        .await
        .map_err(AppError::into)
}

/// Returns a single expiry lot by id.
#[tauri::command]
pub async fn get_expiry_lot(
    state: State<'_, AppState>,
    id: String,
) -> Result<ExpiryLotResponse, CommandError> {
    let pool = state.pool().await;
    service::get_expiry_lot(&pool, id)
        .await
        .map_err(AppError::into)
}

/// Creates a new expiry lot. Pre-fills unit and alert-days from the product
/// defaults when those fields are omitted (None). Fails if no active store
/// exists.
///
/// `locale` (BCP-47 tag) is forwarded to `localize_validation` and
/// `localize_business_rule` so the `LocationRequired` validation surfaced
/// by the service and the constant store-required BusinessRule reach the UI
/// in the active locale. Unknown tags fall back to English via
/// `Locale::parse`.
///
/// The argument is required (not `Option<String>`) to preserve the existing
/// frontend contract: `createExpiryLot` already passes `locale` from the
/// wrapper. Keeping it required avoids an IPC signature change for active
/// callers while still chaining both helpers at the boundary.
#[tauri::command]
pub async fn create_expiry_lot(
    state: State<'_, AppState>,
    input: ExpiryLotCreate,
    locale: String,
) -> Result<ExpiryLotResponse, CommandError> {
    let pool = state.pool().await;
    let loc = resolve_locale(Some(locale));
    service::create_expiry_lot(&pool, input)
        .await
        .map_err(|e| localize_validation(e, loc))
        .map_err(|e| localize_business_rule(e, loc))
        .map_err(|e| localize_not_found(e, loc))
        .map_err(|e| localize_duplicate_field(e, loc))
        .map_err(|e| localize_internal(e, loc))
        .map_err(AppError::into)
}

/// Updates an existing active expiry lot.
///
/// `locale` (BCP-47 tag, optional) is forwarded to `localize_validation`
/// and `localize_business_rule` so the dynamic BusinessRule messages
/// (`Cannot update lot: status is ...` and the metadata-only quantity
/// guard) reach the UI in the active locale. Omitting the argument keeps
/// English, preserving backwards compatibility with callers that don't yet
/// pass a locale.
#[tauri::command]
pub async fn update_expiry_lot(
    state: State<'_, AppState>,
    input: ExpiryLotUpdate,
    locale: Option<String>,
) -> Result<ExpiryLotResponse, CommandError> {
    let pool = state.pool().await;
    let loc = resolve_locale(locale);
    service::update_expiry_lot(&pool, input)
        .await
        .map_err(|e| localize_validation(e, loc))
        .map_err(|e| localize_business_rule(e, loc))
        .map_err(|e| localize_not_found(e, loc))
        .map_err(|e| localize_duplicate_field(e, loc))
        .map_err(|e| localize_internal(e, loc))
        .map_err(AppError::into)
}

/// Creates one expiry lot whose initial quantity is distributed across
/// multiple active locations in the same store, atomically.
///
/// The backend emits one `entry:initial` movement for the full lot
/// total to the first allocation's location, then one `transfer` per
/// remaining allocation, all in a single DB transaction. Either every
/// write commits or every write rolls back.
///
/// `locale` (BCP-47 tag, optional) is forwarded through the same
/// `localize_*` chain as `create_expiry_lot` so every user-visible
/// Validation / BusinessRule message (empty distribution, duplicate
/// location, total mismatch, non-store or inactive location,
/// fractional-quantity rejection for integer-unit products) reaches
/// the UI in the active locale. Omitting the argument keeps English.
#[tauri::command]
pub async fn create_expiry_lot_distributed(
    state: State<'_, AppState>,
    input: ExpiryLotDistributedCreate,
    locale: Option<String>,
) -> Result<ExpiryLotDistributedCreateResult, CommandError> {
    let pool = state.pool().await;
    let loc = resolve_locale(locale);
    service::create_expiry_lot_distributed(&pool, input)
        .await
        .map_err(|e| localize_validation(e, loc))
        .map_err(|e| localize_business_rule(e, loc))
        .map_err(|e| localize_not_found(e, loc))
        .map_err(|e| localize_duplicate_field(e, loc))
        .map_err(|e| localize_internal(e, loc))
        .map_err(AppError::into)
}

/// Soft-archives an expiry lot (status = 'archived') with a required
/// justification. Persists the archive reason + notes in `lot_movements`
/// as an `exit:other` marker in the same transaction. Archived lots are
/// excluded from active lists and dashboard queries.
///
/// `locale` (BCP-47 tag, optional) is forwarded to `localize_validation`
/// and `localize_business_rule` so the dynamic BusinessRule messages
/// (`Cannot archive lot: status is already ...` and the race-recovered
/// `Lot is no longer active ...`) reach the UI in the active locale.
/// Omitting the argument keeps English.
#[tauri::command]
pub async fn archive_expiry_lot(
    state: State<'_, AppState>,
    input: ArchiveLotInput,
    locale: Option<String>,
) -> Result<(), CommandError> {
    let pool = state.pool().await;
    let loc = resolve_locale(locale);
    service::archive_expiry_lot(&pool, input)
        .await
        .map_err(|e| localize_validation(e, loc))
        .map_err(|e| localize_business_rule(e, loc))
        .map_err(|e| localize_not_found(e, loc))
        .map_err(|e| localize_duplicate_field(e, loc))
        .map_err(|e| localize_internal(e, loc))
        .map_err(AppError::into)
}

/// Resolves (consumes, discards, or transfers) a quantity from an expiry lot.
/// Records a resolution event and marks the lot as fully resolved when
/// remaining quantity reaches zero.
///
/// `locale` (BCP-47 tag, optional) is forwarded to `localize_validation`
/// and `localize_business_rule` so both surfaces — the
/// `Cannot resolve lot: lot is already ...` BusinessRule and the
/// `Cannot resolve {qty} {unit}: only {n} {unit} remain` Validation —
/// reach the UI in the active locale. Omitting the argument keeps
/// English.
#[tauri::command]
pub async fn resolve_expiry_lot(
    state: State<'_, AppState>,
    input: ExpiryLotResolve,
    locale: Option<String>,
) -> Result<ExpiryLotResolveResult, CommandError> {
    let pool = state.pool().await;
    let loc = resolve_locale(locale);
    service::resolve_expiry_lot(&pool, input)
        .await
        .map_err(|e| localize_validation(e, loc))
        .map_err(|e| localize_business_rule(e, loc))
        .map_err(|e| localize_not_found(e, loc))
        .map_err(|e| localize_internal(e, loc))
        .map_err(AppError::into)
}

/// Returns all resolution events for a given expiry lot, newest first.
#[tauri::command]
pub async fn list_lot_resolution_events(
    state: State<'_, AppState>,
    lot_id: String,
) -> Result<Vec<LotResolutionEventResponse>, CommandError> {
    let pool = state.pool().await;
    service::list_resolution_events(&pool, lot_id)
        .await
        .map_err(AppError::into)
}
