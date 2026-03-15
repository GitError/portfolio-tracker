mod commands;
mod config;
mod db;
mod fx;
mod price;
mod search;
mod stress;
mod types;

use commands::{DbState, HttpClient, SearchCacheState};
use rusqlite::Connection;
use std::sync::Mutex;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let result = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()?;

            std::fs::create_dir_all(&app_data_dir)?;

            let db_path = app_data_dir.join(config::DB_FILE_NAME);
            let conn = Connection::open(&db_path)
                .map_err(|e| format!("Failed to open SQLite database: {e}"))?;

            db::init_db(&conn).map_err(|e| format!("Failed to initialize database schema: {e}"))?;

            let http_client = reqwest::Client::builder()
                .user_agent(config::USER_AGENT)
                .build()
                .map_err(|e| format!("Failed to create HTTP client: {e}"))?;

            app.manage(DbState(Mutex::new(conn)));
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
            commands::get_config_cmd,
            commands::set_config_cmd,
            commands::export_data,
            commands::import_data,
            commands::get_alerts,
            commands::add_alert,
            commands::delete_alert,
            commands::reset_alert,
            commands::get_rebalance_suggestions,
        ])
        .run(tauri::generate_context!());

    if let Err(e) = result {
        eprintln!("error while running tauri application: {e}");
        std::process::exit(1);
    }
}
