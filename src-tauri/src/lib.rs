mod analytics;
mod commands;
mod config;
mod csv;
mod db;
mod fx;
mod portfolio;
mod price;
mod search;
mod stress;
mod types;

#[cfg(test)]
mod ts_binding_tests {
    use ts_rs::TS as _;

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

        Account::export_all_to(out_dir).expect("Account");
        AccountType::export_all_to(out_dir).expect("AccountType");
        AlertDirection::export_all_to(out_dir).expect("AlertDirection");
        AssetType::export_all_to(out_dir).expect("AssetType");
        CountryWeight::export_all_to(out_dir).expect("CountryWeight");
        CreateAccountRequest::export_all_to(out_dir).expect("CreateAccountRequest");
        Dividend::export_all_to(out_dir).expect("Dividend");
        DividendInput::export_all_to(out_dir).expect("DividendInput");
        ExportPayload::export_all_to(out_dir).expect("ExportPayload");
        FxRate::export_all_to(out_dir).expect("FxRate");
        Holding::export_all_to(out_dir).expect("Holding");
        HoldingInput::export_all_to(out_dir).expect("HoldingInput");
        HoldingWithPrice::export_all_to(out_dir).expect("HoldingWithPrice");
        ImportError::export_all_to(out_dir).expect("ImportError");
        ImportResult::export_all_to(out_dir).expect("ImportResult");
        PerformancePoint::export_all_to(out_dir).expect("PerformancePoint");
        PortfolioAnalytics::export_all_to(out_dir).expect("PortfolioAnalytics");
        PortfolioRiskMetrics::export_all_to(out_dir).expect("PortfolioRiskMetrics");
        PortfolioSnapshot::export_all_to(out_dir).expect("PortfolioSnapshot");
        PreviewImportResult::export_all_to(out_dir).expect("PreviewImportResult");
        PreviewRow::export_all_to(out_dir).expect("PreviewRow");
        PriceAlert::export_all_to(out_dir).expect("PriceAlert");
        PriceAlertInput::export_all_to(out_dir).expect("PriceAlertInput");
        PriceData::export_all_to(out_dir).expect("PriceData");
        RealizedGainsSummary::export_all_to(out_dir).expect("RealizedGainsSummary");
        RealizedLot::export_all_to(out_dir).expect("RealizedLot");
        RebalanceSuggestion::export_all_to(out_dir).expect("RebalanceSuggestion");
        RefreshResult::export_all_to(out_dir).expect("RefreshResult");
        SectorWeight::export_all_to(out_dir).expect("SectorWeight");
        StressHoldingResult::export_all_to(out_dir).expect("StressHoldingResult");
        StressResult::export_all_to(out_dir).expect("StressResult");
        StressScenario::export_all_to(out_dir).expect("StressScenario");
        SymbolMetadata::export_all_to(out_dir).expect("SymbolMetadata");
        SymbolResult::export_all_to(out_dir).expect("SymbolResult");
        Transaction::export_all_to(out_dir).expect("Transaction");
        TransactionInput::export_all_to(out_dir).expect("TransactionInput");
        TransactionType::export_all_to(out_dir).expect("TransactionType");
    }
}

use commands::{DbState, HttpClient, SearchCacheState};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::str::FromStr;
use tauri::Manager;

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
                .build()
                .map_err(|e| format!("Failed to create HTTP client: {e}"))?;

            // Spawn background WAL checkpoint task to prevent unbounded WAL growth.
            let wal_pool = pool.clone();
            tauri::async_runtime::spawn(async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));
                interval.tick().await; // skip immediate first tick
                loop {
                    interval.tick().await;
                    match sqlx::query("PRAGMA wal_checkpoint(RESTART)")
                        .execute(&wal_pool)
                        .await
                    {
                        Ok(_) => tracing::debug!("WAL checkpoint complete"),
                        Err(e) => tracing::warn!("WAL checkpoint failed: {}", e),
                    }
                }
            });

            app.manage(DbState(pool));
            app.manage(HttpClient(http_client));
            app.manage(SearchCacheState::new());

            Ok(())
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
        ])
        .run(tauri::generate_context!());

    if let Err(e) = result {
        tracing::error!("error while running tauri application: {e}");
        std::process::exit(1);
    }
}
