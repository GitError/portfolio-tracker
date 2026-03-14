mod commands;
mod db;
mod fx;
mod price;
mod stress;
mod types;

use commands::{DbState, HttpClient};
use rusqlite::Connection;
use std::sync::Mutex;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("Failed to get app data directory");

            std::fs::create_dir_all(&app_data_dir).expect("Failed to create app data directory");

            let db_path = app_data_dir.join("portfolio.db");
            let conn = Connection::open(&db_path).expect("Failed to open SQLite database");

            db::init_db(&conn).expect("Failed to initialize database schema");

            let http_client = reqwest::Client::builder()
                .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)")
                .build()
                .expect("Failed to create HTTP client");

            app.manage(DbState(Mutex::new(conn)));
            app.manage(HttpClient(http_client));

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_portfolio,
            commands::get_holdings,
            commands::add_holding,
            commands::update_holding,
            commands::delete_holding,
            commands::refresh_prices,
            commands::run_stress_test_cmd,
            commands::get_performance,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
