use chrono::Utc;
use sqlx::SqlitePool;
use tauri::State;

use crate::db;
use crate::error::AppError;
use crate::types::{Watchlist, WatchlistId, WatchlistItemId, WatchlistItemWithSnapshot};

use super::{validate_id, DbState, HttpClient};

fn validate_watchlist_item_input(
    currency: &str,
    thesis: Option<&str>,
    catalysts: Option<&str>,
    risks: Option<&str>,
    entry_price_low: Option<f64>,
    entry_price_high: Option<f64>,
) -> Result<(), AppError> {
    portfolio_core::validation::validate_watchlist_item_currency(currency)
        .map_err(AppError::Validation)?;
    if let Some(thesis) = thesis {
        portfolio_core::validation::validate_watchlist_note("thesis", thesis)
            .map_err(AppError::Validation)?;
    }
    if let Some(catalysts) = catalysts {
        portfolio_core::validation::validate_watchlist_note("catalysts", catalysts)
            .map_err(AppError::Validation)?;
    }
    if let Some(risks) = risks {
        portfolio_core::validation::validate_watchlist_note("risks", risks)
            .map_err(AppError::Validation)?;
    }
    portfolio_core::validation::validate_entry_price_range(entry_price_low, entry_price_high)
        .map_err(AppError::Validation)?;
    Ok(())
}

/// Fetches a fresh market-data snapshot via the shared Yahoo Finance adapter
/// (`price::fetch_watchlist_snapshot`) and stores it — success or failure —
/// against the item. A failed fetch is stored as an error snapshot rather
/// than propagated, so adding/refreshing an item never fails outright just
/// because Yahoo Finance is unreachable for that symbol.
async fn fetch_and_store_snapshot(
    pool: &SqlitePool,
    client: &reqwest::Client,
    item_id: &WatchlistItemId,
    symbol: &str,
) -> Result<(), AppError> {
    match crate::price::fetch_watchlist_snapshot(client, symbol).await {
        Ok(snapshot) => {
            db::upsert_watchlist_item_snapshot(pool, item_id, Some(&snapshot), None).await?
        }
        Err(e) => {
            tracing::warn!("Failed to fetch watchlist snapshot for {}: {}", symbol, e);
            db::upsert_watchlist_item_snapshot(pool, item_id, None, Some(&e)).await?
        }
    }
    Ok(())
}

/// Refreshes one item's snapshot, respecting `WATCHLIST_REFRESH_COOLDOWN_SECS`.
/// Within the cooldown window this is a no-op that returns the item
/// unchanged (not an error), so `refresh_watchlist` calling this per-item
/// doesn't turn a partially-cooled-down watchlist into a wall of errors.
async fn refresh_item_snapshot(
    pool: &SqlitePool,
    client: &reqwest::Client,
    item: WatchlistItemWithSnapshot,
) -> Result<WatchlistItemWithSnapshot, AppError> {
    if let Some(remaining) =
        db::watchlist_refresh_cooldown_remaining(item.retrieved_at.as_deref(), Utc::now())
    {
        tracing::info!(
            "refresh_watchlist_item rate-limited for {}: {}s remaining in cooldown",
            item.symbol,
            remaining
        );
        return Ok(item);
    }

    fetch_and_store_snapshot(pool, client, &item.id, &item.symbol).await?;

    db::get_watchlist_item_with_snapshot(pool, &item.id)
        .await?
        .ok_or_else(|| AppError::NotFound("Watchlist item not found after refresh".to_string()))
}

#[tauri::command]
pub async fn list_watchlists(db: State<'_, DbState>) -> Result<Vec<Watchlist>, AppError> {
    let pool = &db.0;
    db::get_watchlists(pool).await.map_err(AppError::from)
}

#[tauri::command]
pub async fn create_watchlist(db: State<'_, DbState>, name: String) -> Result<Watchlist, AppError> {
    portfolio_core::validation::validate_watchlist_name(&name).map_err(AppError::Validation)?;
    let pool = &db.0;
    db::insert_watchlist(pool, name.trim())
        .await
        .map_err(AppError::from)
}

#[tauri::command]
pub async fn delete_watchlist(db: State<'_, DbState>, id: WatchlistId) -> Result<bool, AppError> {
    validate_id("watchlist ID", &id.0)?;
    let pool = &db.0;
    db::delete_watchlist(pool, &id)
        .await
        .map_err(AppError::from)
}

#[tauri::command]
pub async fn list_watchlist_items(
    db: State<'_, DbState>,
    watchlist_id: WatchlistId,
) -> Result<Vec<WatchlistItemWithSnapshot>, AppError> {
    validate_id("watchlist ID", &watchlist_id.0)?;
    let pool = &db.0;
    db::list_watchlist_items_with_snapshots(pool, &watchlist_id)
        .await
        .map_err(AppError::from)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn add_watchlist_item(
    db: State<'_, DbState>,
    client: State<'_, HttpClient>,
    watchlist_id: WatchlistId,
    symbol: String,
    currency: String,
    thesis: Option<String>,
    catalysts: Option<String>,
    risks: Option<String>,
    entry_price_low: Option<f64>,
    entry_price_high: Option<f64>,
) -> Result<WatchlistItemWithSnapshot, AppError> {
    validate_id("watchlist ID", &watchlist_id.0)?;
    let symbol_upper = symbol.trim().to_uppercase();
    crate::price::validate_symbol(&symbol_upper).map_err(AppError::Validation)?;
    validate_watchlist_item_input(
        &currency,
        thesis.as_deref(),
        catalysts.as_deref(),
        risks.as_deref(),
        entry_price_low,
        entry_price_high,
    )?;

    let pool = &db.0;
    let item_id = db::insert_watchlist_item(
        pool,
        &watchlist_id,
        &symbol_upper,
        &currency,
        thesis.as_deref(),
        catalysts.as_deref(),
        risks.as_deref(),
        entry_price_low,
        entry_price_high,
    )
    .await
    .map_err(AppError::from)?;

    fetch_and_store_snapshot(pool, &client.0, &item_id, &symbol_upper).await?;

    db::get_watchlist_item_with_snapshot(pool, &item_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Watchlist item not found after insert".to_string()))
}

#[tauri::command]
pub async fn update_watchlist_item(
    db: State<'_, DbState>,
    id: WatchlistItemId,
    thesis: Option<String>,
    catalysts: Option<String>,
    risks: Option<String>,
    entry_price_low: Option<f64>,
    entry_price_high: Option<f64>,
) -> Result<WatchlistItemWithSnapshot, AppError> {
    validate_id("watchlist item ID", &id.0)?;
    if let Some(thesis) = &thesis {
        portfolio_core::validation::validate_watchlist_note("thesis", thesis)
            .map_err(AppError::Validation)?;
    }
    if let Some(catalysts) = &catalysts {
        portfolio_core::validation::validate_watchlist_note("catalysts", catalysts)
            .map_err(AppError::Validation)?;
    }
    if let Some(risks) = &risks {
        portfolio_core::validation::validate_watchlist_note("risks", risks)
            .map_err(AppError::Validation)?;
    }
    portfolio_core::validation::validate_entry_price_range(entry_price_low, entry_price_high)
        .map_err(AppError::Validation)?;

    let pool = &db.0;
    let updated = db::update_watchlist_item(
        pool,
        &id,
        thesis.as_deref(),
        catalysts.as_deref(),
        risks.as_deref(),
        entry_price_low,
        entry_price_high,
    )
    .await
    .map_err(AppError::from)?;

    if !updated {
        return Err(AppError::NotFound("Watchlist item not found".to_string()));
    }

    db::get_watchlist_item_with_snapshot(pool, &id)
        .await?
        .ok_or_else(|| AppError::NotFound("Watchlist item not found".to_string()))
}

#[tauri::command]
pub async fn remove_watchlist_item(
    db: State<'_, DbState>,
    id: WatchlistItemId,
) -> Result<bool, AppError> {
    validate_id("watchlist item ID", &id.0)?;
    let pool = &db.0;
    db::delete_watchlist_item(pool, &id)
        .await
        .map_err(AppError::from)
}

#[tauri::command]
pub async fn refresh_watchlist_item(
    db: State<'_, DbState>,
    client: State<'_, HttpClient>,
    id: WatchlistItemId,
) -> Result<WatchlistItemWithSnapshot, AppError> {
    validate_id("watchlist item ID", &id.0)?;
    let pool = &db.0;
    let item = db::get_watchlist_item_with_snapshot(pool, &id)
        .await?
        .ok_or_else(|| AppError::NotFound("Watchlist item not found".to_string()))?;
    refresh_item_snapshot(pool, &client.0, item).await
}

#[tauri::command]
pub async fn refresh_watchlist(
    db: State<'_, DbState>,
    client: State<'_, HttpClient>,
    watchlist_id: WatchlistId,
) -> Result<Vec<WatchlistItemWithSnapshot>, AppError> {
    validate_id("watchlist ID", &watchlist_id.0)?;
    let pool = &db.0;
    let items = db::list_watchlist_items_with_snapshots(pool, &watchlist_id).await?;

    // Cap concurrent Yahoo Finance requests at 5, mirroring
    // `price::fetch_all_prices` and `analytics::get_symbol_metadata_with_cache`.
    use futures::stream::{self, StreamExt};
    let results: Vec<Result<WatchlistItemWithSnapshot, AppError>> = stream::iter(items)
        .map(|item| refresh_item_snapshot(pool, &client.0, item))
        .buffer_unordered(5)
        .collect()
        .await;

    results.into_iter().collect()
}
