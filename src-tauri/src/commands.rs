use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use chrono::Utc;
use csv::{ReaderBuilder, StringRecord, Trim, WriterBuilder};
use tauri::State;

use crate::db;
use crate::fx::{convert_to_base, fetch_all_fx_rates};
use crate::price::{fetch_all_prices, fetch_price};
use crate::search::search_symbols_yahoo;
use crate::stress::run_stress_test;
use crate::types::{
    AccountType, AssetType, FxRate, Holding, HoldingInput, HoldingWithPrice, ImportError,
    ImportResult, PortfolioSnapshot, PreviewImportResult, PreviewRow, PriceData, StressResult,
    StressScenario, SymbolResult,
};

const MAX_IMPORT_ROWS: usize = 500;

pub struct DbState(pub Mutex<rusqlite::Connection>);
pub struct HttpClient(pub reqwest::Client);

fn get_base_currency(db: &State<'_, DbState>) -> String {
    db.0.lock()
        .ok()
        .and_then(|conn| db::get_config(&conn, "base_currency").ok().flatten())
        .unwrap_or_else(|| "CAD".to_string())
}

#[allow(dead_code)]
#[tauri::command]
pub async fn get_config_cmd(db: State<'_, DbState>, key: String) -> Result<Option<String>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    db::get_config(&conn, &key)
}

#[allow(dead_code)]
#[tauri::command]
pub async fn set_config_cmd(
    db: State<'_, DbState>,
    key: String,
    value: String,
) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    db::set_config(&conn, &key, &value)
}

pub(crate) struct SearchCacheEntry {
    results: Vec<SymbolResult>,
    cached_at: Instant,
}

pub struct SearchCacheState(pub Mutex<HashMap<String, SearchCacheEntry>>);

impl SearchCacheState {
    pub fn new() -> Self {
        SearchCacheState(Mutex::new(HashMap::new()))
    }

    fn get(&self, key: &str) -> Option<Vec<SymbolResult>> {
        let cache = self.0.lock().ok()?;
        let entry = cache.get(key)?;
        if entry.cached_at.elapsed() > Duration::from_secs(300) {
            return None;
        }
        Some(entry.results.clone())
    }

    fn set(&self, key: String, results: Vec<SymbolResult>) {
        if let Ok(mut cache) = self.0.lock() {
            if cache.len() >= 200 {
                cache.clear();
            }
            cache.insert(
                key,
                SearchCacheEntry {
                    results,
                    cached_at: Instant::now(),
                },
            );
        }
    }
}
#[derive(Debug)]
struct ParsedImportRow {
    row: usize,
    symbol: String,
    name: String,
    asset_type: AssetType,
    account: AccountType,
    quantity: f64,
    cost_basis: f64,
    currency: String,
}

fn detect_csv_delimiter(content: &str) -> u8 {
    let first_line = content.lines().next().unwrap_or_default();
    if first_line.contains(';') && !first_line.contains(',') {
        b';'
    } else {
        b','
    }
}

fn find_column_index(headers: &StringRecord, field: &str) -> Option<usize> {
    headers.iter().position(|header| {
        header
            .trim_start_matches('\u{feff}')
            .trim()
            .eq_ignore_ascii_case(field)
    })
}

fn parse_required_field(
    record: &StringRecord,
    index: usize,
    row: usize,
    field: &str,
) -> Result<String, String> {
    let value = record.get(index).unwrap_or_default().trim();
    if value.is_empty() {
        return Err(format!("Row {}: missing_{}", row, field));
    }
    Ok(value.to_string())
}

fn parse_optional_field(record: &StringRecord, index: Option<usize>) -> String {
    index
        .and_then(|i| record.get(i))
        .unwrap_or_default()
        .trim()
        .to_string()
}

/// Convert `SYMBOL:COUNTRY` notation to a Yahoo Finance symbol.
/// Plain symbols are returned unchanged (uppercased).
/// Examples: `BMO:CA` → `BMO.TO`, `AAPL:US` → `AAPL`, `BARC:GB` → `BARC.L`
fn normalize_symbol_for_import(raw: &str) -> String {
    let trimmed = raw.trim();
    if let Some((sym, country)) = trimmed.split_once(':') {
        let sym = sym.trim().to_uppercase();
        match country.trim().to_uppercase().as_str() {
            "CA" => format!("{}.TO", sym),
            "GB" => format!("{}.L", sym),
            "AU" => format!("{}.AX", sym),
            "DE" => format!("{}.DE", sym),
            "FR" => format!("{}.PA", sym),
            "JP" => format!("{}.T", sym),
            "HK" => format!("{}.HK", sym),
            _ => sym, // US or unrecognised: no exchange suffix
        }
    } else {
        trimmed.to_uppercase()
    }
}

fn parse_import_rows(csv_content: &str) -> Result<Vec<ParsedImportRow>, String> {
    let content = csv_content.trim_start_matches('\u{feff}');
    let mut reader = ReaderBuilder::new()
        .trim(Trim::All)
        .delimiter(detect_csv_delimiter(content))
        .from_reader(content.as_bytes());

    let headers = reader
        .headers()
        .map_err(|e| format!("Invalid CSV header: {}", e))?
        .clone();
    let symbol_index = find_column_index(&headers, "symbol")
        .ok_or_else(|| "Missing required column: symbol".to_string())?;
    let name_index = find_column_index(&headers, "name");
    let account_index = find_column_index(&headers, "account");
    let type_index = find_column_index(&headers, "type")
        .ok_or_else(|| "Missing required column: type".to_string())?;
    let quantity_index = find_column_index(&headers, "quantity")
        .ok_or_else(|| "Missing required column: quantity".to_string())?;
    let cost_basis_index = find_column_index(&headers, "cost_basis")
        .ok_or_else(|| "Missing required column: cost_basis".to_string())?;
    let currency_index = find_column_index(&headers, "currency")
        .ok_or_else(|| "Missing required column: currency".to_string())?;

    let mut rows = Vec::new();

    for (index, record) in reader.records().enumerate() {
        if rows.len() >= MAX_IMPORT_ROWS {
            return Err(format!("CSV import is limited to {} rows", MAX_IMPORT_ROWS));
        }

        let row = index + 2;
        let record = record.map_err(|e| format!("Invalid CSV row {}: {}", row, e))?;
        if record.iter().all(|field| field.trim().is_empty()) {
            continue;
        }

        let asset_type = parse_required_field(&record, type_index, row, "type")?
            .to_lowercase()
            .parse::<AssetType>()
            .map_err(|_| format!("Row {}: invalid_type", row))?;
        let account = parse_optional_field(&record, account_index);
        let account = if account.is_empty() {
            if matches!(asset_type, AssetType::Cash) {
                AccountType::Cash
            } else {
                AccountType::Taxable
            }
        } else {
            account
                .to_lowercase()
                .parse::<AccountType>()
                .map_err(|_| format!("Row {}: invalid_account", row))?
        };
        let currency =
            parse_required_field(&record, currency_index, row, "currency")?.to_uppercase();
        let raw_symbol = parse_optional_field(&record, Some(symbol_index));
        let symbol = if matches!(asset_type, AssetType::Cash) {
            if raw_symbol.is_empty() || raw_symbol.eq_ignore_ascii_case("CASH") {
                format!("{}-CASH", currency)
            } else {
                normalize_symbol_for_import(&raw_symbol)
            }
        } else if raw_symbol.is_empty() {
            return Err(format!("Row {}: missing_symbol", row));
        } else {
            normalize_symbol_for_import(&raw_symbol)
        };

        let quantity = parse_required_field(&record, quantity_index, row, "quantity")?
            .parse::<f64>()
            .map_err(|_| format!("Row {}: invalid_quantity", row))?;
        if quantity <= 0.0 {
            return Err(format!("Row {}: invalid_quantity", row));
        }

        let cost_basis = parse_required_field(&record, cost_basis_index, row, "cost_basis")?
            .parse::<f64>()
            .map_err(|_| format!("Row {}: invalid_cost_basis", row))?;
        if cost_basis <= 0.0 {
            return Err(format!("Row {}: invalid_cost_basis", row));
        }

        rows.push(ParsedImportRow {
            row,
            symbol,
            name: parse_optional_field(&record, name_index),
            asset_type,
            account,
            quantity,
            cost_basis,
            currency,
        });
    }

    if rows.is_empty() {
        return Err("CSV file is empty".to_string());
    }

    Ok(rows)
}

async fn validate_symbol(
    db: &State<'_, DbState>,
    client: &State<'_, HttpClient>,
    symbol: &str,
) -> Result<Option<SymbolResult>, String> {
    if let Some(cached) = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        db::get_symbol_cache_exact(&conn, symbol)?
    } {
        return Ok(Some(cached));
    }

    let result = search_symbols_yahoo(&client.0, symbol)
        .await?
        .into_iter()
        .find(|candidate| candidate.symbol.eq_ignore_ascii_case(symbol));

    if let Some(ref symbol_result) = result {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        let _ = db::upsert_symbol(&conn, symbol_result);
    }

    Ok(result)
}

fn build_portfolio_snapshot(
    holdings: &[Holding],
    cached_prices: &[PriceData],
    cached_fx: &[FxRate],
    base_currency: &str,
    last_updated: String,
) -> PortfolioSnapshot {
    if holdings.is_empty() {
        return PortfolioSnapshot {
            holdings: vec![],
            total_value: 0.0,
            total_cost: 0.0,
            total_gain_loss: 0.0,
            total_gain_loss_percent: 0.0,
            daily_pnl: 0.0,
            last_updated,
            base_currency: base_currency.to_string(),
        };
    }

    let price_map: std::collections::HashMap<String, &PriceData> = cached_prices
        .iter()
        .map(|p| (p.symbol.clone(), p))
        .collect();

    let fx_map: std::collections::HashMap<String, &FxRate> =
        cached_fx.iter().map(|r| (r.pair.clone(), r)).collect();

    let mut holdings_with_price: Vec<HoldingWithPrice> = Vec::new();
    let mut total_value = 0.0f64;
    let mut total_cost = 0.0f64;
    let mut daily_pnl = 0.0f64;

    for holding in holdings {
        let (current_price, change_percent) = if holding.asset_type.as_str() == "cash" {
            (1.0f64, 0.0f64)
        } else {
            price_map
                .get(&holding.symbol)
                .map(|p| (p.price, p.change_percent))
                .unwrap_or((holding.cost_basis, 0.0))
        };

        let fx_pair = format!(
            "{}{}",
            holding.currency.to_uppercase(),
            base_currency.to_uppercase()
        );
        let fx_rate = if holding.currency.eq_ignore_ascii_case(base_currency) {
            1.0
        } else {
            fx_map.get(&fx_pair).map(|r| r.rate).unwrap_or_else(|| {
                convert_to_base(1.0, &holding.currency, base_currency, cached_fx)
            })
        };

        let current_price_cad = current_price * fx_rate;
        let market_value_cad = holding.quantity * current_price_cad;
        let cost_value_cad = holding.quantity * holding.cost_basis * fx_rate;
        let gain_loss = market_value_cad - cost_value_cad;
        let gain_loss_percent = if cost_value_cad != 0.0 {
            (gain_loss / cost_value_cad) * 100.0
        } else {
            0.0
        };

        total_value += market_value_cad;
        total_cost += cost_value_cad;
        daily_pnl += market_value_cad * (change_percent / 100.0);

        holdings_with_price.push(HoldingWithPrice {
            id: holding.id.clone(),
            symbol: holding.symbol.clone(),
            name: holding.name.clone(),
            asset_type: holding.asset_type.clone(),
            account: holding.account.clone(),
            quantity: holding.quantity,
            cost_basis: holding.cost_basis,
            currency: holding.currency.clone(),
            created_at: holding.created_at.clone(),
            updated_at: holding.updated_at.clone(),
            current_price,
            current_price_cad,
            market_value_cad,
            cost_value_cad,
            gain_loss,
            gain_loss_percent,
            weight: 0.0,
            daily_change_percent: change_percent,
        });
    }

    for holding in &mut holdings_with_price {
        holding.weight = if total_value != 0.0 {
            (holding.market_value_cad / total_value) * 100.0
        } else {
            0.0
        };
    }

    let total_gain_loss = total_value - total_cost;
    let total_gain_loss_percent = if total_cost != 0.0 {
        (total_gain_loss / total_cost) * 100.0
    } else {
        0.0
    };

    PortfolioSnapshot {
        holdings: holdings_with_price,
        total_value,
        total_cost,
        total_gain_loss,
        total_gain_loss_percent,
        daily_pnl,
        last_updated,
        base_currency: base_currency.to_string(),
    }
}
#[tauri::command]
pub async fn get_portfolio(
    db: State<'_, DbState>,
    _client: State<'_, HttpClient>,
) -> Result<PortfolioSnapshot, String> {
    let base_currency = get_base_currency(&db);

    let holdings = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        db::get_all_holdings(&conn)?
    };

    let (cached_prices, cached_fx) = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        (db::get_cached_prices(&conn)?, db::get_fx_rates(&conn)?)
    };

    Ok(build_portfolio_snapshot(
        &holdings,
        &cached_prices,
        &cached_fx,
        &base_currency,
        Utc::now().to_rfc3339(),
    ))
}

#[tauri::command]
pub async fn get_holdings(db: State<'_, DbState>) -> Result<Vec<Holding>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    db::get_all_holdings(&conn)
}

#[tauri::command]
pub async fn add_holding(db: State<'_, DbState>, holding: HoldingInput) -> Result<Holding, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    db::insert_holding(&conn, holding)
}

#[tauri::command]
pub async fn update_holding(db: State<'_, DbState>, holding: Holding) -> Result<Holding, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    db::update_holding(&conn, holding)
}

#[tauri::command]
pub async fn delete_holding(db: State<'_, DbState>, id: String) -> Result<bool, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    db::delete_holding(&conn, &id)
}

#[tauri::command]
pub async fn export_holdings_csv(db: State<'_, DbState>) -> Result<String, String> {
    let holdings = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        db::get_all_holdings(&conn)?
    };

    let mut writer = WriterBuilder::new().from_writer(vec![]);
    writer
        .write_record([
            "symbol",
            "name",
            "type",
            "account",
            "quantity",
            "cost_basis",
            "currency",
        ])
        .map_err(|e| e.to_string())?;

    for holding in holdings {
        writer
            .write_record([
                holding.symbol,
                holding.name,
                holding.asset_type.as_str().to_string(),
                holding.account.as_str().to_string(),
                holding.quantity.to_string(),
                holding.cost_basis.to_string(),
                holding.currency,
            ])
            .map_err(|e| e.to_string())?;
    }

    let bytes = writer.into_inner().map_err(|e| e.to_string())?;
    String::from_utf8(bytes).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn import_holdings_csv(
    db: State<'_, DbState>,
    client: State<'_, HttpClient>,
    csv_content: String,
) -> Result<ImportResult, String> {
    let parsed_rows = parse_import_rows(&csv_content)?;
    let existing_symbols = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        db::get_all_holdings(&conn)?
            .into_iter()
            .map(|holding| holding.symbol.to_uppercase())
            .collect::<HashSet<_>>()
    };

    let mut seen_symbols = existing_symbols;
    let mut pending_inputs = Vec::new();
    let mut skipped = Vec::new();

    for row in parsed_rows {
        if seen_symbols.contains(&row.symbol) {
            skipped.push(ImportError {
                row: row.row,
                symbol: row.symbol,
                reason: "duplicate".to_string(),
            });
            continue;
        }

        if matches!(row.asset_type, AssetType::Cash) {
            seen_symbols.insert(row.symbol.clone());
            pending_inputs.push(HoldingInput {
                symbol: row.symbol,
                name: if row.name.is_empty() {
                    format!("{} Cash", row.currency)
                } else {
                    row.name
                },
                asset_type: row.asset_type,
                account: row.account,
                quantity: row.quantity,
                cost_basis: row.cost_basis,
                currency: row.currency,
            });
            continue;
        }

        let validated = match validate_symbol(&db, &client, &row.symbol).await {
            Ok(Some(result)) => result,
            Ok(None) => {
                skipped.push(ImportError {
                    row: row.row,
                    symbol: row.symbol,
                    reason: "invalid_symbol".to_string(),
                });
                continue;
            }
            Err(_) => {
                skipped.push(ImportError {
                    row: row.row,
                    symbol: row.symbol,
                    reason: "validation_failed".to_string(),
                });
                continue;
            }
        };

        if !validated.currency.eq_ignore_ascii_case(&row.currency) {
            skipped.push(ImportError {
                row: row.row,
                symbol: row.symbol,
                reason: format!(
                    "currency_mismatch:{}_expected_{}",
                    row.currency,
                    validated.currency.to_uppercase()
                ),
            });
            continue;
        }

        seen_symbols.insert(validated.symbol.to_uppercase());
        pending_inputs.push(HoldingInput {
            symbol: validated.symbol,
            name: if row.name.is_empty() {
                validated.name
            } else {
                row.name
            },
            asset_type: row.asset_type,
            account: row.account,
            quantity: row.quantity,
            cost_basis: row.cost_basis,
            currency: row.currency,
        });
    }

    let mut imported = Vec::new();
    {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        for input in pending_inputs {
            imported.push(db::insert_holding(&conn, input)?);
        }
    }

    Ok(ImportResult {
        total_rows: imported.len() + skipped.len(),
        imported,
        skipped,
    })
}

#[tauri::command]
pub async fn preview_import_csv(
    db: State<'_, DbState>,
    client: State<'_, HttpClient>,
    csv_content: String,
) -> Result<PreviewImportResult, String> {
    let parsed_rows = parse_import_rows(&csv_content)?;
    let existing_symbols: HashSet<String> = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        db::get_all_holdings(&conn)?
            .into_iter()
            .map(|h| h.symbol.to_uppercase())
            .collect()
    };

    let mut preview_rows: Vec<PreviewRow> = Vec::new();
    let mut seen: HashSet<String> = existing_symbols;

    for row in parsed_rows {
        let sym_upper = row.symbol.to_uppercase();

        if seen.contains(&sym_upper) {
            preview_rows.push(PreviewRow {
                row: row.row,
                original_symbol: row.symbol.clone(),
                resolved_symbol: row.symbol.clone(),
                name: row.name,
                asset_type: row.asset_type.as_str().to_string(),
                currency: row.currency,
                exchange: String::new(),
                quantity: row.quantity,
                cost_basis: row.cost_basis,
                status: "duplicate".to_string(),
            });
            continue;
        }

        if matches!(row.asset_type, AssetType::Cash) {
            seen.insert(sym_upper);
            preview_rows.push(PreviewRow {
                row: row.row,
                original_symbol: row.symbol.clone(),
                resolved_symbol: row.symbol.clone(),
                name: if row.name.is_empty() {
                    format!("{} Cash", row.currency)
                } else {
                    row.name
                },
                asset_type: "cash".to_string(),
                currency: row.currency,
                exchange: String::new(),
                quantity: row.quantity,
                cost_basis: row.cost_basis,
                status: "ready".to_string(),
            });
            continue;
        }

        match validate_symbol(&db, &client, &row.symbol).await {
            Ok(Some(result)) => {
                seen.insert(result.symbol.to_uppercase());
                preview_rows.push(PreviewRow {
                    row: row.row,
                    original_symbol: row.symbol,
                    resolved_symbol: result.symbol,
                    name: if row.name.is_empty() {
                        result.name
                    } else {
                        row.name
                    },
                    asset_type: result.asset_type.as_str().to_string(),
                    currency: result.currency,
                    exchange: result.exchange,
                    quantity: row.quantity,
                    cost_basis: row.cost_basis,
                    status: "ready".to_string(),
                });
            }
            Ok(None) => {
                preview_rows.push(PreviewRow {
                    row: row.row,
                    original_symbol: row.symbol.clone(),
                    resolved_symbol: String::new(),
                    name: row.name,
                    asset_type: row.asset_type.as_str().to_string(),
                    currency: row.currency,
                    exchange: String::new(),
                    quantity: row.quantity,
                    cost_basis: row.cost_basis,
                    status: "invalid_symbol".to_string(),
                });
            }
            Err(_) => {
                preview_rows.push(PreviewRow {
                    row: row.row,
                    original_symbol: row.symbol.clone(),
                    resolved_symbol: String::new(),
                    name: row.name,
                    asset_type: row.asset_type.as_str().to_string(),
                    currency: row.currency,
                    exchange: String::new(),
                    quantity: row.quantity,
                    cost_basis: row.cost_basis,
                    status: "validation_failed".to_string(),
                });
            }
        }
    }

    let ready_count = preview_rows.iter().filter(|r| r.status == "ready").count();
    let skip_count = preview_rows.len() - ready_count;

    Ok(PreviewImportResult {
        rows: preview_rows,
        ready_count,
        skip_count,
    })
}

#[tauri::command]
pub async fn refresh_prices(
    db: State<'_, DbState>,
    client: State<'_, HttpClient>,
) -> Result<Vec<PriceData>, String> {
    let base_currency = get_base_currency(&db);

    let holdings = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        db::get_all_holdings(&conn)?
    };

    // Collect unique symbols (skip cash)
    let symbols: Vec<String> = holdings
        .iter()
        .filter(|h| h.asset_type.as_str() != "cash")
        .map(|h| h.symbol.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    // Collect all unique currencies; fetch_all_fx_rates will filter out the base
    let currencies: Vec<String> = holdings
        .iter()
        .map(|h| h.currency.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    let (prices, fx_rates) = tokio::join!(
        fetch_all_prices(&client.0, symbols),
        fetch_all_fx_rates(&client.0, currencies, &base_currency)
    );

    // Persist to cache
    {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        for price in &prices {
            db::upsert_price(&conn, price)?;
        }
        for rate in &fx_rates {
            db::upsert_fx_rate(&conn, rate)?;
        }
    }

    Ok(prices)
}

#[tauri::command]
pub async fn run_stress_test_cmd(
    db: State<'_, DbState>,
    client: State<'_, HttpClient>,
    scenario: StressScenario,
) -> Result<StressResult, String> {
    let snapshot = get_portfolio(db, client).await?;
    Ok(run_stress_test(&snapshot, &scenario))
}

#[tauri::command]
pub async fn search_symbols(
    query: String,
    client: State<'_, HttpClient>,
    cache: State<'_, SearchCacheState>,
    db: State<'_, DbState>,
) -> Result<Vec<SymbolResult>, String> {
    if query.trim().len() < 2 {
        return Ok(vec![]);
    }

    let key = query.trim().to_lowercase();

    // 1. In-memory cache (5-minute TTL)
    if let Some(cached) = cache.get(&key) {
        return Ok(cached);
    }

    // 2. SQLite persistent cache
    let db_results = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        db::search_symbol_cache(&conn, &key).unwrap_or_default()
    };

    // 3. Yahoo Finance API
    let results = match search_symbols_yahoo(&client.0, &query).await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Symbol search API failed: {}", e);
            return Ok(db_results);
        }
    };

    // Persist new results to SQLite and in-memory cache
    {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        for r in &results {
            let _ = db::upsert_symbol(&conn, r);
        }
    }
    cache.set(key, results.clone());

    Ok(results)
}

#[tauri::command]
pub async fn get_symbol_price(
    symbol: String,
    client: State<'_, HttpClient>,
) -> Result<PriceData, String> {
    fetch_price(&client.0, &symbol).await
}

#[tauri::command]
pub async fn get_performance(
    db: State<'_, DbState>,
    range: String,
) -> Result<Vec<serde_json::Value>, String> {
    // TODO: Implement real historical performance tracking using a snapshots table.
    // For v1, return mock data based on the requested range.
    let _ = db;
    let days = match range.as_str() {
        "1W" => 7,
        "1M" => 30,
        "3M" => 90,
        "6M" => 180,
        "1Y" => 365,
        _ => 30,
    };

    let now = Utc::now();
    let base_value = 50000.0f64;
    let mut data = Vec::new();

    for i in (0..=days).rev() {
        let date = now - chrono::Duration::days(i);
        let noise = (i as f64 * 0.7).sin() * 2000.0 + (i as f64 * 0.3).cos() * 1500.0;
        let trend = (days - i) as f64 * 50.0;
        let value = base_value + trend + noise;

        data.push(serde_json::json!({
            "date": date.format("%Y-%m-%d").to_string(),
            "value": (value * 100.0).round() / 100.0
        }));
    }

    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_holding(
        symbol: &str,
        asset_type: AssetType,
        quantity: f64,
        cost_basis: f64,
        currency: &str,
    ) -> Holding {
        Holding {
            id: symbol.to_string(),
            symbol: symbol.to_string(),
            name: symbol.to_string(),
            asset_type,
            account: AccountType::Taxable,
            quantity,
            cost_basis,
            currency: currency.to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn normalize_symbol_strips_country_suffix() {
        assert_eq!(normalize_symbol_for_import("BMO:CA"), "BMO.TO");
        assert_eq!(normalize_symbol_for_import("AAPL:US"), "AAPL");
        assert_eq!(normalize_symbol_for_import("BARC:GB"), "BARC.L");
        assert_eq!(normalize_symbol_for_import("CBA:AU"), "CBA.AX");
        assert_eq!(normalize_symbol_for_import("SAP:DE"), "SAP.DE");
        assert_eq!(normalize_symbol_for_import("AIR:FR"), "AIR.PA");
        assert_eq!(normalize_symbol_for_import("7203:JP"), "7203.T");
        assert_eq!(normalize_symbol_for_import("0700:HK"), "0700.HK");
    }

    #[test]
    fn normalize_symbol_passes_through_plain_symbols() {
        assert_eq!(normalize_symbol_for_import("AAPL"), "AAPL");
        assert_eq!(normalize_symbol_for_import("BMO.TO"), "BMO.TO");
        assert_eq!(normalize_symbol_for_import("bmo"), "BMO");
        assert_eq!(normalize_symbol_for_import(" MSFT "), "MSFT");
    }

    #[test]
    fn parse_import_rows_normalizes_country_suffix() {
        let csv =
            "symbol,name,type,quantity,cost_basis,currency\nBMO:CA,Bank of Montreal,stock,10,80,CAD\n";
        let rows = parse_import_rows(csv).expect("parse csv");

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].symbol, "BMO.TO");
    }

    #[test]
    fn parse_import_rows_supports_cash_defaults() {
        let csv = "symbol,name,type,quantity,cost_basis,currency\n, ,cash,1000,1,CAD\n";
        let rows = parse_import_rows(csv).expect("parse csv");

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].symbol, "CAD-CASH");
        assert_eq!(rows[0].currency, "CAD");
        assert!(matches!(rows[0].asset_type, AssetType::Cash));
    }

    #[test]
    fn parse_import_rows_supports_semicolon_delimiter() {
        let csv =
            "symbol;name;type;quantity;cost_basis;currency\nAAPL;Apple Inc.;stock;5;120;usd\n";
        let rows = parse_import_rows(csv).expect("parse csv");

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].symbol, "AAPL");
        assert_eq!(rows[0].currency, "USD");
    }

    #[test]
    fn parse_import_rows_rejects_missing_required_columns() {
        let csv = "symbol,name,type,quantity,currency\nAAPL,Apple Inc.,stock,5,USD\n";
        let error = parse_import_rows(csv).expect_err("missing cost_basis should fail");

        assert!(error.contains("Missing required column: cost_basis"));
    }

    #[test]
    fn build_portfolio_snapshot_converts_mixed_currency_holdings_into_base_currency() {
        let holdings = vec![
            make_holding("SHOP.TO", AssetType::Stock, 10.0, 100.0, "CAD"),
            make_holding("AAPL", AssetType::Stock, 5.0, 100.0, "USD"),
        ];
        let prices = vec![
            PriceData {
                symbol: "SHOP.TO".to_string(),
                price: 120.0,
                currency: "CAD".to_string(),
                change: 1.0,
                change_percent: 2.0,
                updated_at: Utc::now().to_rfc3339(),
            },
            PriceData {
                symbol: "AAPL".to_string(),
                price: 110.0,
                currency: "USD".to_string(),
                change: 1.0,
                change_percent: 10.0,
                updated_at: Utc::now().to_rfc3339(),
            },
        ];
        let fx = vec![FxRate {
            pair: "USDCAD".to_string(),
            rate: 1.25,
            updated_at: Utc::now().to_rfc3339(),
        }];

        let snapshot = build_portfolio_snapshot(
            &holdings,
            &prices,
            &fx,
            "CAD",
            "2024-01-01T00:00:00Z".to_string(),
        );

        assert_eq!(snapshot.base_currency, "CAD");
        assert!((snapshot.holdings[0].market_value_cad - 1200.0).abs() < 0.001);
        assert!((snapshot.holdings[1].market_value_cad - 687.5).abs() < 0.001);
        assert!((snapshot.holdings[1].cost_value_cad - 625.0).abs() < 0.001);
        assert!((snapshot.total_value - 1887.5).abs() < 0.001);
        assert!((snapshot.total_cost - 1625.0).abs() < 0.001);
        assert!((snapshot.daily_pnl - 92.75).abs() < 0.001);
    }

    #[test]
    fn build_portfolio_snapshot_supports_non_cad_base_currency() {
        let holdings = vec![
            make_holding("RY.TO", AssetType::Stock, 2.0, 100.0, "CAD"),
            make_holding("MSFT", AssetType::Stock, 1.0, 200.0, "USD"),
        ];
        let prices = vec![
            PriceData {
                symbol: "RY.TO".to_string(),
                price: 110.0,
                currency: "CAD".to_string(),
                change: 0.0,
                change_percent: 0.0,
                updated_at: Utc::now().to_rfc3339(),
            },
            PriceData {
                symbol: "MSFT".to_string(),
                price: 220.0,
                currency: "USD".to_string(),
                change: 0.0,
                change_percent: 0.0,
                updated_at: Utc::now().to_rfc3339(),
            },
        ];
        let fx = vec![FxRate {
            pair: "CADUSD".to_string(),
            rate: 0.8,
            updated_at: Utc::now().to_rfc3339(),
        }];

        let snapshot = build_portfolio_snapshot(
            &holdings,
            &prices,
            &fx,
            "USD",
            "2024-01-01T00:00:00Z".to_string(),
        );

        assert_eq!(snapshot.base_currency, "USD");
        assert!((snapshot.holdings[0].market_value_cad - 176.0).abs() < 0.001);
        assert!((snapshot.holdings[0].cost_value_cad - 160.0).abs() < 0.001);
        assert!((snapshot.holdings[1].market_value_cad - 220.0).abs() < 0.001);
        assert!((snapshot.total_value - 396.0).abs() < 0.001);
        assert!((snapshot.total_cost - 360.0).abs() < 0.001);
    }
}
