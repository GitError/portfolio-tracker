mod analytics;
mod commands;
mod config;
mod csv;
mod db;
pub mod error;
mod fx;
mod portfolio;
mod price;
mod search;
mod stress;
mod types;

#[cfg(test)]
mod ts_binding_tests {
    use ts_rs::{Config, TS as _};

    use crate::types::{
        Account, AccountType, AlertDirection, AssetType, CountryWeight, CreateAccountRequest,
        Dividend, DividendInput, ExportPayload, FxRate, Holding, HoldingInput, HoldingWithPrice,
        ImportError, ImportResult, PerformancePoint, PortfolioAnalytics, PortfolioRiskMetrics,
        PortfolioSnapshot, PreviewImportResult, PreviewRow, PriceAlert, PriceAlertInput, PriceData,
        RealizedGainsSummary, RealizedLot, RebalanceSuggestion, RefreshResult, SectorWeight,
        StressHoldingResult, StressResult, StressScenario, SymbolMetadata, SymbolResult,
        Transaction, TransactionInput, TransactionType,
    };

    #[test]
    fn export_typescript_bindings() {
        let out_dir = "../frontend/types/bindings";
        std::fs::create_dir_all(out_dir).expect("Failed to create bindings directory");
        let cfg = Config::new().with_out_dir(out_dir);

        Account::export_all(&cfg).expect("Account");
        AccountType::export_all(&cfg).expect("AccountType");
        AlertDirection::export_all(&cfg).expect("AlertDirection");
        AssetType::export_all(&cfg).expect("AssetType");
        CountryWeight::export_all(&cfg).expect("CountryWeight");
        CreateAccountRequest::export_all(&cfg).expect("CreateAccountRequest");
        Dividend::export_all(&cfg).expect("Dividend");
        DividendInput::export_all(&cfg).expect("DividendInput");
        ExportPayload::export_all(&cfg).expect("ExportPayload");
        FxRate::export_all(&cfg).expect("FxRate");
        Holding::export_all(&cfg).expect("Holding");
        HoldingInput::export_all(&cfg).expect("HoldingInput");
        HoldingWithPrice::export_all(&cfg).expect("HoldingWithPrice");
        ImportError::export_all(&cfg).expect("ImportError");
        ImportResult::export_all(&cfg).expect("ImportResult");
        PerformancePoint::export_all(&cfg).expect("PerformancePoint");
        PortfolioAnalytics::export_all(&cfg).expect("PortfolioAnalytics");
        PortfolioRiskMetrics::export_all(&cfg).expect("PortfolioRiskMetrics");
        PortfolioSnapshot::export_all(&cfg).expect("PortfolioSnapshot");
        PreviewImportResult::export_all(&cfg).expect("PreviewImportResult");
        PreviewRow::export_all(&cfg).expect("PreviewRow");
        PriceAlert::export_all(&cfg).expect("PriceAlert");
        PriceAlertInput::export_all(&cfg).expect("PriceAlertInput");
        PriceData::export_all(&cfg).expect("PriceData");
        RealizedGainsSummary::export_all(&cfg).expect("RealizedGainsSummary");
        RealizedLot::export_all(&cfg).expect("RealizedLot");
        RebalanceSuggestion::export_all(&cfg).expect("RebalanceSuggestion");
        RefreshResult::export_all(&cfg).expect("RefreshResult");
        SectorWeight::export_all(&cfg).expect("SectorWeight");
        StressHoldingResult::export_all(&cfg).expect("StressHoldingResult");
        StressResult::export_all(&cfg).expect("StressResult");
        StressScenario::export_all(&cfg).expect("StressScenario");
        SymbolMetadata::export_all(&cfg).expect("SymbolMetadata");
        SymbolResult::export_all(&cfg).expect("SymbolResult");
        Transaction::export_all(&cfg).expect("Transaction");
        TransactionInput::export_all(&cfg).expect("TransactionInput");
        TransactionType::export_all(&cfg).expect("TransactionType");
    }
}

use commands::{DbState, HttpClient, RealizedGainsCacheState, SearchCacheState};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::str::FromStr;
use tauri::Manager;

/// Holds the sender half of the WAL checkpoint shutdown channel.
/// Managed as Tauri app state so the window-destroyed event can signal the background task.
struct WalShutdown(tokio::sync::watch::Sender<bool>);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .try_init();

    let result = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()?;

            std::fs::create_dir_all(&app_data_dir)?;

            let db_path = app_data_dir.join(config::DB_FILE_NAME);
            let db_url = format!("sqlite:{}", db_path.to_string_lossy());

            let options = SqliteConnectOptions::from_str(&db_url)
                .map_err(|e| e.to_string())?
                .create_if_missing(true)
                .foreign_keys(true)
                .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
                .busy_timeout(std::time::Duration::from_millis(5000));

            // `block_on` is intentional here: Tauri's setup callback is synchronous but
            // we need async DB operations (pool creation + migrations) to complete before
            // the app state is registered. Using block_on on the Tauri async runtime
            // avoids spawning a separate thread and is the recommended pattern for
            // one-time async init inside `setup`.
            let pool = tauri::async_runtime::block_on(async {
                SqlitePoolOptions::new()
                    .max_connections(5)
                    .connect_with(options)
                    .await
            })
            .map_err(|e| format!("Failed to open SQLite database: {e}"))?;

            tauri::async_runtime::block_on(async {
                sqlx::migrate!("./migrations").run(&pool).await
            })
            .map_err(|e| format!("Failed to run database migrations: {e}"))?;

            let http_client = reqwest::Client::builder()
                .user_agent(config::USER_AGENT)
                // Prevent indefinite hangs when Yahoo Finance is slow or unreachable.
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .map_err(|e| format!("Failed to create HTTP client: {e}"))?;

            // Spawn background WAL checkpoint task to prevent unbounded WAL growth.
            // A watch channel provides a graceful shutdown signal: the sender is stored in
            // app state so the on_window_event handler can signal shutdown on app exit.
            let (shutdown_tx, mut shutdown_rx) = tokio::sync::watch::channel(false);
            let wal_pool = pool.clone();
            tauri::async_runtime::spawn(async move {
                // Checkpoint WAL every 5 min to bound WAL file growth; safe for low-write desktop workloads.
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));
                interval.tick().await; // skip immediate first tick
                loop {
                    tokio::select! {
                        _ = interval.tick() => {
                            match sqlx::query("PRAGMA wal_checkpoint(RESTART)")
                                .execute(&wal_pool)
                                .await
                            {
                                Ok(_) => tracing::debug!("WAL checkpoint complete"),
                                Err(e) => tracing::warn!("WAL checkpoint failed: {}", e),
                            }
                        }
                        _ = shutdown_rx.changed() => {
                            if *shutdown_rx.borrow() {
                                tracing::info!("WAL checkpoint task shutting down");
                                break;
                            }
                        }
                    }
                }
            });

            app.manage(DbState(pool));
            app.manage(HttpClient(http_client));
            app.manage(SearchCacheState::new());
            app.manage(RealizedGainsCacheState::new());
            // Store the WAL shutdown sender so on_window_event can signal the task to exit.
            app.manage(WalShutdown(shutdown_tx));

            Ok(())
        })
        .on_window_event(|_window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                // Signal the WAL checkpoint background task to shut down cleanly.
                if let Some(state) = _window.try_state::<WalShutdown>() {
                    let _ = state.0.send(true);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_portfolio,
            commands::get_holdings,
            commands::add_holding,
            commands::update_holding,
            commands::delete_holding,
            commands::import_holdings_csv,
            commands::preview_import_csv,
            commands::export_holdings_csv,
            commands::refresh_prices,
            commands::run_stress_test_cmd,
            commands::get_performance,
            commands::search_symbols,
            commands::get_symbol_price,
            commands::get_cached_prices,
            commands::get_config_cmd,
            commands::set_config_cmd,
            commands::get_alerts,
            commands::add_alert,
            commands::delete_alert,
            commands::reset_alert,
            commands::get_rebalance_suggestions,
            commands::add_transaction,
            commands::get_transactions,
            commands::delete_transaction,
            commands::backup_database,
            commands::restore_database,
            commands::get_portfolio_analytics,
            commands::get_dividends,
            commands::add_dividend,
            commands::delete_dividend,
            commands::get_realized_gains,
            commands::get_accounts,
            commands::add_account,
            commands::update_account,
            commands::delete_account,
            commands::get_holdings_paginated,
            commands::get_transactions_paginated,
            commands::get_alerts_paginated,
            commands::get_dividends_paginated,
        ])
        .run(tauri::generate_context!());

    if let Err(e) = result {
        tracing::error!("error while running tauri application: {e}");
        std::process::exit(1);
    }
}
