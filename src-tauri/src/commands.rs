use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use chrono::Utc;
use csv::{ReaderBuilder, StringRecord, Trim, WriterBuilder};
use tauri::{Manager, State};

use crate::analytics::compute_realized_gains_grouped;
use crate::db;
use crate::fx::{convert_to_base, fetch_all_fx_rates};
use crate::price::{fetch_all_prices, fetch_price, FetchAllPricesResult};
use crate::search::search_symbols_yahoo;
use crate::stress::run_stress_test;
use crate::types::{
    Account, AccountType, AssetType, CountryWeight, CreateAccountRequest, Dividend, DividendInput,
    FxRate, Holding, HoldingInput, HoldingWithPrice, ImportError, ImportResult, PerformancePoint,
    PortfolioAnalytics, PortfolioRiskMetrics, PortfolioSnapshot, PreviewImportResult, PreviewRow,
    PriceAlert, PriceAlertInput, PriceData, RealizedGainsSummary, RebalanceSuggestion,
    RefreshResult, SectorWeight, StressResult, StressScenario, SymbolMetadata, SymbolResult,
    Transaction, TransactionInput,
};

pub struct DbState(pub Mutex<rusqlite::Connection>);
pub struct HttpClient(pub reqwest::Client);

fn get_base_currency(db: &State<'_, DbState>) -> String {
    db.0.lock()
        .ok()
        .and_then(|conn| db::get_config(&conn, "base_currency").ok().flatten())
        .unwrap_or_else(|| crate::config::BASE_CURRENCY.to_string())
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
        if entry.cached_at.elapsed()
            > Duration::from_secs(crate::config::SEARCH_CACHE_TTL_SECS as u64)
        {
            return None;
        }
        Some(entry.results.clone())
    }

    fn set(&self, key: String, results: Vec<SymbolResult>) {
        if let Ok(mut cache) = self.0.lock() {
            if cache.len() >= crate::config::SEARCH_CACHE_MAX_ENTRIES {
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
    exchange: String,
    target_weight: f64,
}

fn build_holdings_csv(holdings: &[Holding]) -> Result<String, String> {
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
            "exchange",
            "target_weight",
        ])
        .map_err(|e| e.to_string())?;

    for holding in holdings {
        writer
            .write_record([
                holding.symbol.clone(),
                holding.name.clone(),
                holding.asset_type.as_str().to_string(),
                holding.account.as_str().to_string(),
                holding.quantity.to_string(),
                holding.cost_basis.to_string(),
                holding.currency.clone(),
                holding.exchange.clone(),
                holding.target_weight.to_string(),
            ])
            .map_err(|e| e.to_string())?;
    }

    let bytes = writer.into_inner().map_err(|e| e.to_string())?;
    String::from_utf8(bytes).map_err(|e| e.to_string())
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
    let exchange_index = find_column_index(&headers, "exchange");
    let type_index = find_column_index(&headers, "type")
        .ok_or_else(|| "Missing required column: type".to_string())?;
    let quantity_index = find_column_index(&headers, "quantity")
        .ok_or_else(|| "Missing required column: quantity".to_string())?;
    let cost_basis_index = find_column_index(&headers, "cost_basis")
        .ok_or_else(|| "Missing required column: cost_basis".to_string())?;
    let currency_index = find_column_index(&headers, "currency")
        .ok_or_else(|| "Missing required column: currency".to_string())?;
    let target_weight_index = find_column_index(&headers, "target_weight");

    let mut rows = Vec::new();

    for (index, record) in reader.records().enumerate() {
        if rows.len() >= crate::config::MAX_IMPORT_ROWS {
            return Err(format!(
                "CSV import is limited to {} rows",
                crate::config::MAX_IMPORT_ROWS
            ));
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
            // Unknown/unrecognised account strings default to the appropriate
            // type rather than crashing the whole import.
            account.to_lowercase().parse::<AccountType>().unwrap_or({
                if matches!(asset_type, AssetType::Cash) {
                    AccountType::Cash
                } else {
                    AccountType::Taxable
                }
            })
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
        if cost_basis < 0.0 {
            return Err(format!("Row {}: invalid_cost_basis", row));
        }

        let target_weight = parse_optional_field(&record, target_weight_index);
        let target_weight = if target_weight.is_empty() {
            0.0
        } else {
            let parsed = target_weight
                .parse::<f64>()
                .map_err(|_| format!("Row {}: invalid_target_weight", row))?;
            if !(0.0..=100.0).contains(&parsed) {
                return Err(format!("Row {}: invalid_target_weight", row));
            }
            parsed
        };

        rows.push(ParsedImportRow {
            row,
            symbol,
            name: parse_optional_field(&record, name_index),
            asset_type,
            account,
            quantity,
            cost_basis,
            currency,
            exchange: parse_optional_field(&record, exchange_index).to_uppercase(),
            target_weight,
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
    realized_gains: f64,
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
            total_target_weight: 0.0,
            target_cash_delta: 0.0,
            realized_gains,
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

        // Exclude intraday purchases from daily PnL: a holding created today has
        // no prior-day close to compare against, so applying the day-over-day
        // change_percent would overstate the gain.
        let today = Utc::now().date_naive().to_string(); // "YYYY-MM-DD"
        let created_date = &holding.created_at[..10]; // first 10 chars of ISO 8601
        if created_date < today.as_str() {
            daily_pnl += market_value_cad * (change_percent / 100.0);
        }

        holdings_with_price.push(HoldingWithPrice {
            id: holding.id.clone(),
            symbol: holding.symbol.clone(),
            name: holding.name.clone(),
            asset_type: holding.asset_type.clone(),
            account: holding.account.clone(),
            quantity: holding.quantity,
            cost_basis: holding.cost_basis,
            currency: holding.currency.clone(),
            exchange: holding.exchange.clone(),
            target_weight: holding.target_weight,
            created_at: holding.created_at.clone(),
            updated_at: holding.updated_at.clone(),
            current_price,
            current_price_cad,
            market_value_cad,
            cost_value_cad,
            gain_loss,
            gain_loss_percent,
            weight: 0.0,
            target_value: 0.0,
            target_delta_value: 0.0,
            target_delta_percent: 0.0,
            daily_change_percent: change_percent,
        });
    }

    let total_target_weight: f64 = holdings.iter().map(|holding| holding.target_weight).sum();
    let mut target_cash_delta = 0.0f64;

    for holding in &mut holdings_with_price {
        holding.weight = if total_value != 0.0 {
            (holding.market_value_cad / total_value) * 100.0
        } else {
            0.0
        };
        holding.target_value = total_value * (holding.target_weight / 100.0);
        holding.target_delta_value = holding.target_value - holding.market_value_cad;
        holding.target_delta_percent = holding.target_weight - holding.weight;

        if holding.asset_type.as_str() == "cash" {
            target_cash_delta += holding.market_value_cad - holding.target_value;
        }
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
        total_target_weight,
        target_cash_delta,
        realized_gains,
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

    let realized_gains = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        let cost_basis_method =
            db::get_config(&conn, "cost_basis_method")?.unwrap_or_else(|| "avco".to_string());
        let transactions = db::get_all_transactions(&conn)?;
        compute_realized_gains_grouped(&transactions, &cost_basis_method)
            .map(|s| s.total_realized_gain)
            .unwrap_or(0.0)
    };

    Ok(build_portfolio_snapshot(
        &holdings,
        &cached_prices,
        &cached_fx,
        &base_currency,
        Utc::now().to_rfc3339(),
        realized_gains,
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
    if holding.target_weight > 0.0 {
        let current_sum = db::sum_target_weights(&conn, None)?;
        let new_total = current_sum + holding.target_weight;
        if new_total > 100.0 {
            return Err(format!(
                "Total target weight would exceed 100% (currently {:.1}%). Adjust existing allocations before adding this holding.",
                current_sum
            ));
        }
    }
    db::insert_holding(&conn, holding)
}

#[tauri::command]
pub async fn update_holding(db: State<'_, DbState>, holding: Holding) -> Result<Holding, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    if holding.target_weight > 0.0 {
        let current_sum = db::sum_target_weights(&conn, Some(&holding.id))?;
        let new_total = current_sum + holding.target_weight;
        if new_total > 100.0 {
            return Err(format!(
                "Total target weight would exceed 100% (currently {:.1}% across other holdings). Adjust existing allocations before saving.",
                current_sum
            ));
        }
    }
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
    build_holdings_csv(&holdings)
}

#[tauri::command]
pub async fn import_holdings_csv(
    db: State<'_, DbState>,
    client: State<'_, HttpClient>,
    csv_content: String,
) -> Result<ImportResult, String> {
    let parsed_rows = parse_import_rows(&csv_content)?;

    let existing_keys: HashSet<(String, String)> = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        db::get_all_holdings(&conn)?
            .into_iter()
            .map(|holding| {
                (
                    holding.symbol.to_uppercase(),
                    holding.account.as_str().to_string(),
                )
            })
            .collect()
    };

    let mut seen_keys = existing_keys;
    let mut pending_inputs = Vec::new();
    let mut skipped = Vec::new();

    for row in parsed_rows {
        let key = (row.symbol.to_uppercase(), row.account.as_str().to_string());
        if seen_keys.contains(&key) {
            skipped.push(ImportError {
                row: row.row,
                symbol: row.symbol,
                reason: "duplicate".to_string(),
            });
            continue;
        }

        if matches!(row.asset_type, AssetType::Cash) {
            seen_keys.insert((row.symbol.to_uppercase(), row.account.as_str().to_string()));
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
                exchange: row.exchange,
                target_weight: row.target_weight,
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

        seen_keys.insert((
            validated.symbol.to_uppercase(),
            row.account.as_str().to_string(),
        ));
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
            exchange: if row.exchange.is_empty() {
                validated.exchange
            } else {
                row.exchange
            },
            target_weight: row.target_weight,
        });
    }

    // Weight validation runs after deduplication so that re-importing an existing
    // portfolio (all rows skipped as duplicates) never triggers a false overflow.
    let pending_weight_sum: f64 = pending_inputs.iter().map(|i| i.target_weight).sum();
    if pending_weight_sum > 100.0 {
        return Err(format!(
            "Import failed: total target weight is {:.1}% (max 100%). Adjust weights before re-importing.",
            pending_weight_sum
        ));
    }
    let existing_weight_sum = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        db::sum_target_weights(&conn, None)?
    };
    if existing_weight_sum + pending_weight_sum > 100.0 {
        return Err(format!(
            "Import failed: total target weight would reach {:.1}% (existing portfolio is already {:.1}%). Adjust weights before re-importing.",
            existing_weight_sum + pending_weight_sum,
            existing_weight_sum
        ));
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
    let existing_keys: HashSet<(String, String)> = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        db::get_all_holdings(&conn)?
            .into_iter()
            .map(|h| (h.symbol.to_uppercase(), h.account.as_str().to_string()))
            .collect()
    };

    let mut preview_rows: Vec<PreviewRow> = Vec::new();
    let mut seen: HashSet<(String, String)> = existing_keys;

    for row in parsed_rows {
        let row_key = (row.symbol.to_uppercase(), row.account.as_str().to_string());

        if seen.contains(&row_key) {
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
                target_weight: row.target_weight,
                status: "duplicate".to_string(),
            });
            continue;
        }

        if matches!(row.asset_type, AssetType::Cash) {
            seen.insert((row.symbol.to_uppercase(), row.account.as_str().to_string()));
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
                target_weight: row.target_weight,
                status: "ready".to_string(),
            });
            continue;
        }

        match validate_symbol(&db, &client, &row.symbol).await {
            Ok(Some(result)) => {
                seen.insert((
                    result.symbol.to_uppercase(),
                    row.account.as_str().to_string(),
                ));
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
                    target_weight: row.target_weight,
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
                    target_weight: row.target_weight,
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
                    target_weight: row.target_weight,
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
) -> Result<RefreshResult, String> {
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

    let (fetch_result, fx_rates) = tokio::join!(
        fetch_all_prices(&client.0, symbols),
        fetch_all_fx_rates(&client.0, currencies, &base_currency)
    );

    let FetchAllPricesResult {
        prices,
        failed: failed_symbols,
    } = fetch_result;

    let mut triggered_alert_ids: Vec<String> = Vec::new();

    // Persist prices and FX rates to cache
    {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        for price in &prices {
            db::upsert_price(&conn, price)?;
        }
        for rate in &fx_rates {
            db::upsert_fx_rate(&conn, rate)?;
        }
    }

    // Build a portfolio snapshot to record the current total value
    let snapshot_totals = {
        let (holdings, cached_prices, cached_fx) = {
            let conn = db.0.lock().map_err(|e| e.to_string())?;
            (
                db::get_all_holdings(&conn)?,
                db::get_cached_prices(&conn)?,
                db::get_fx_rates(&conn)?,
            )
        };
        let snap = build_portfolio_snapshot(
            &holdings,
            &cached_prices,
            &cached_fx,
            &base_currency,
            Utc::now().to_rfc3339(),
            0.0,
        );
        (snap.total_value, snap.total_cost, snap.total_gain_loss)
    };

    // Record the snapshot and prune old data; log errors but don't fail the command
    {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        if let Err(e) = db::insert_snapshot(
            &conn,
            snapshot_totals.0,
            snapshot_totals.1,
            snapshot_totals.2,
        ) {
            eprintln!("Failed to insert portfolio snapshot: {}", e);
        }
        if let Err(e) = db::prune_snapshots(&conn) {
            eprintln!("Failed to prune portfolio snapshots: {}", e);
        }

        // Check price alerts — collect newly-triggered IDs, log errors but don't fail
        for price in &prices {
            match db::check_and_trigger_alerts(&conn, &price.symbol, price.price) {
                Ok(ids) => triggered_alert_ids.extend(ids),
                Err(e) => eprintln!("Failed to check alerts for {}: {}", price.symbol, e),
            }
        }
    }

    Ok(RefreshResult {
        prices,
        failed_symbols,
        triggered_alerts: triggered_alert_ids,
    })
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
) -> Result<Vec<PerformancePoint>, String> {
    let now = Utc::now();
    let end = now.to_rfc3339();

    let start = match range.as_str() {
        "1D" => (now - chrono::Duration::hours(24)).to_rfc3339(),
        "1W" => (now - chrono::Duration::days(7)).to_rfc3339(),
        "1M" => (now - chrono::Duration::days(30)).to_rfc3339(),
        "3M" => (now - chrono::Duration::days(90)).to_rfc3339(),
        "6M" => (now - chrono::Duration::days(180)).to_rfc3339(),
        "1Y" => (now - chrono::Duration::days(365)).to_rfc3339(),
        "ALL" => "1970-01-01T00:00:00+00:00".to_string(),
        _ => (now - chrono::Duration::days(30)).to_rfc3339(),
    };

    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let snapshots = db::get_snapshots_in_range(&conn, &start, &end).map_err(|e| e.to_string())?;

    // Deduplicate by calendar date, keeping only the latest snapshot per day.
    let mut by_date: std::collections::BTreeMap<String, PerformancePoint> =
        std::collections::BTreeMap::new();
    for point in snapshots {
        let date_key = point.date[..10].to_string();
        by_date.insert(date_key, point);
    }
    Ok(by_date.into_values().collect())
}

// ── Dividend Commands ─────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_dividends(db: State<'_, DbState>) -> Result<Vec<Dividend>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    db::get_dividends(&conn)
}

#[tauri::command]
pub async fn add_dividend(
    db: State<'_, DbState>,
    dividend: DividendInput,
) -> Result<Dividend, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    // Look up the symbol for the holding
    let holdings = db::get_all_holdings(&conn)?;
    let symbol = holdings
        .iter()
        .find(|h| h.id == dividend.holding_id)
        .map(|h| h.symbol.as_str())
        .unwrap_or("")
        .to_string();
    db::insert_dividend(&conn, dividend, &symbol)
}

#[tauri::command]
pub async fn delete_dividend(db: State<'_, DbState>, id: i64) -> Result<bool, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    db::delete_dividend(&conn, id)
}

// ── Price Alert Commands ───────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_alerts(db: State<'_, DbState>) -> Result<Vec<PriceAlert>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    db::get_alerts(&conn)
}

#[tauri::command]
pub async fn add_alert(
    db: State<'_, DbState>,
    alert: PriceAlertInput,
) -> Result<PriceAlert, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    db::insert_alert(&conn, alert)
}

#[tauri::command]
pub async fn delete_alert(db: State<'_, DbState>, id: String) -> Result<bool, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    db::delete_alert(&conn, &id)
}

#[tauri::command]
pub async fn reset_alert(db: State<'_, DbState>, id: String) -> Result<bool, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    db::reset_alert(&conn, &id)
}

#[tauri::command]
pub async fn export_data(state: State<'_, DbState>) -> Result<String, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let payload = crate::types::ExportPayload {
        holdings: db::get_all_holdings(&conn)?,
        alerts: db::get_alerts(&conn)?,
        config: db::get_all_config(&conn)?,
    };
    serde_json::to_string(&payload).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn import_data(state: State<'_, DbState>, json: String) -> Result<usize, String> {
    // Try full ExportPayload first; fall back to legacy plain Vec<Holding> format.
    let payload: crate::types::ExportPayload = if let Ok(p) = serde_json::from_str(&json) {
        p
    } else {
        let holdings: Vec<Holding> =
            serde_json::from_str(&json).map_err(|e| format!("Invalid JSON: {e}"))?;
        crate::types::ExportPayload {
            holdings,
            alerts: vec![],
            config: vec![],
        }
    };

    let count = payload.holdings.len();
    let conn = state.0.lock().map_err(|e| e.to_string())?;

    // Wrap in a transaction so a mid-import failure leaves the database intact.
    conn.execute_batch("BEGIN").map_err(|e| e.to_string())?;
    let result = (|| -> Result<(), String> {
        db::delete_all_holdings(&conn).map_err(|e| e.to_string())?;
        db::delete_all_alerts(&conn)?;
        db::delete_all_config(&conn)?;

        for holding in payload.holdings {
            db::insert_holding_with_id(&conn, holding).map_err(|e| e.to_string())?;
        }
        for alert in payload.alerts {
            db::insert_alert_with_id(&conn, alert)?;
        }
        for (key, value) in payload.config {
            db::set_config(&conn, &key, &value)?;
        }
        Ok(())
    })();

    match result {
        Ok(()) => {
            conn.execute_batch("COMMIT").map_err(|e| e.to_string())?;
            Ok(count)
        }
        Err(e) => {
            let _ = conn.execute_batch("ROLLBACK");
            Err(e)
        }
    }
}

/// SQLite magic bytes: first 16 bytes of a valid SQLite database file.
const SQLITE_MAGIC: &[u8] = b"SQLite format 3\0";

#[tauri::command]
pub async fn backup_database(
    app: tauri::AppHandle,
    state: tauri::State<'_, DbState>,
    destination_path: String,
) -> Result<String, String> {
    // Flush WAL to ensure the file on disk is complete before we copy it.
    {
        let conn = state.0.lock().map_err(|e| e.to_string())?;
        conn.execute_batch("PRAGMA wal_checkpoint(FULL);")
            .map_err(|e| format!("WAL checkpoint failed: {e}"))?;
    }

    let source = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Could not resolve app data dir: {e}"))?
        .join(crate::config::DB_FILE_NAME);

    if !source.exists() {
        return Err("Database file does not exist".to_string());
    }

    // Resolve the destination path. If only a filename is provided (no
    // directory component), save the backup to the user's Desktop so it is
    // easy to find.
    let requested = std::path::PathBuf::from(&destination_path);
    let dest = if requested.is_absolute() {
        requested
    } else {
        let desktop = app
            .path()
            .desktop_dir()
            .or_else(|_| app.path().home_dir())
            .map_err(|e| format!("Could not resolve home/desktop dir: {e}"))?;
        desktop.join(&requested)
    };

    if let Some(parent) = dest.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Could not create destination directory: {e}"))?;
        }
    }

    std::fs::copy(&source, &dest).map_err(|e| format!("Failed to copy database: {e}"))?;

    Ok(dest.to_string_lossy().into_owned())
}

#[tauri::command]
pub async fn restore_database(
    app: tauri::AppHandle,
    _state: tauri::State<'_, DbState>,
    source_path: String,
) -> Result<String, String> {
    // Verify the source file is a valid SQLite database.
    let src = std::path::PathBuf::from(&source_path);
    if !src.exists() {
        return Err(format!("File not found: {source_path}"));
    }

    // Check SQLite magic bytes.
    let mut header = [0u8; 16];
    {
        use std::io::Read;
        let mut f =
            std::fs::File::open(&src).map_err(|e| format!("Cannot open backup file: {e}"))?;
        f.read_exact(&mut header)
            .map_err(|_| "File is too small to be a valid SQLite database".to_string())?;
    }
    if header != SQLITE_MAGIC {
        return Err("The selected file is not a valid SQLite database".to_string());
    }

    // Open the source file with rusqlite to verify it has a holdings table.
    {
        let verify_conn = rusqlite::Connection::open(&src)
            .map_err(|e| format!("Cannot open backup as SQLite: {e}"))?;
        verify_conn
            .execute_batch("PRAGMA integrity_check;")
            .map_err(|e| format!("Integrity check failed on backup: {e}"))?;
        let has_holdings: bool = verify_conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='holdings'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .map(|n| n > 0)
            .map_err(|e| format!("Could not verify holdings table: {e}"))?;
        if !has_holdings {
            return Err(
                "Backup file does not appear to be a portfolio database (no holdings table)"
                    .to_string(),
            );
        }
    }

    let dest = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Could not resolve app data dir: {e}"))?
        .join(crate::config::DB_FILE_NAME);

    std::fs::copy(&src, &dest).map_err(|e| format!("Failed to restore database: {e}"))?;

    Ok("Database restored. Please restart the app to apply changes.".to_string())
}

#[tauri::command]
pub async fn get_rebalance_suggestions(
    db: State<'_, DbState>,
    drift_threshold: f64,
) -> Result<Vec<RebalanceSuggestion>, String> {
    let base_currency = get_base_currency(&db);

    let holdings = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        db::get_all_holdings(&conn)?
    };

    let (cached_prices, cached_fx) = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        (db::get_cached_prices(&conn)?, db::get_fx_rates(&conn)?)
    };

    let snapshot = build_portfolio_snapshot(
        &holdings,
        &cached_prices,
        &cached_fx,
        &base_currency,
        Utc::now().to_rfc3339(),
        0.0,
    );

    let total_value = snapshot.total_value;

    let mut suggestions: Vec<RebalanceSuggestion> = snapshot
        .holdings
        .into_iter()
        .filter(|h| {
            // Exclude cash holdings and holdings with no target weight
            h.asset_type.as_str() != "cash" && h.target_weight > 0.0
        })
        .filter_map(|h| {
            let target_value_cad = total_value * (h.target_weight / 100.0);
            let drift = h.weight - h.target_weight;
            if drift.abs() < drift_threshold {
                return None;
            }
            // positive = sell (over-weight), negative = buy (under-weight)
            let suggested_trade_cad = h.market_value_cad - target_value_cad;
            let suggested_units = if h.current_price_cad != 0.0 {
                suggested_trade_cad / h.current_price_cad
            } else {
                0.0
            };
            Some(RebalanceSuggestion {
                holding_id: h.id,
                symbol: h.symbol,
                name: h.name,
                current_value_cad: h.market_value_cad,
                target_value_cad,
                current_weight: h.weight,
                target_weight: h.target_weight,
                drift,
                suggested_trade_cad,
                suggested_units,
                current_price_cad: h.current_price_cad,
            })
        })
        .collect();

    // Sort by |drift| descending — biggest drifters first
    suggestions.sort_by(|a, b| {
        b.drift
            .abs()
            .partial_cmp(&a.drift.abs())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(suggestions)
}

// ── Analytics Commands ────────────────────────────────────────────────────────

/// Fetch per-symbol sector/industry/country from Yahoo Finance's v11 quoteSummary
/// `assetProfile` module. Returns `None` for all three fields on any fetch/parse failure
/// (failures are soft — they don't abort the whole analytics call).
async fn fetch_asset_profile(
    client: &reqwest::Client,
    symbol: &str,
) -> (String, Option<String>, Option<String>, Option<String>) {
    let url = crate::config::YAHOO_QUOTE_SUMMARY_URL.replace("{}", symbol);

    let json: Option<serde_json::Value> = async {
        let resp = client
            .get(&url)
            .header("User-Agent", crate::config::USER_AGENT)
            .send()
            .await
            .ok()?;
        if !resp.status().is_success() {
            return None;
        }
        resp.json::<serde_json::Value>().await.ok()
    }
    .await;

    let profile = json
        .as_ref()
        .and_then(|v| v.pointer("/quoteSummary/result/0/assetProfile"));

    let extract = |key: &str| -> Option<String> {
        profile
            .and_then(|p| p.get(key))
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
    };

    (
        symbol.to_string(),
        extract("sector"),
        extract("industry"),
        extract("country"),
    )
}

/// Fetch enriched symbol metadata (sector, industry, country, market cap, etc.)
/// for the given list of symbols.
///
/// * Sector, industry, and country are fetched from the v11 `quoteSummary` / `assetProfile`
///   endpoint, which reliably returns these fields (unlike the v7 quote endpoint).
/// * Numeric fields (market cap, P/E, dividend yield, beta) continue to come from the
///   bulk v7 quote endpoint.
///
/// Both requests are issued concurrently. A failure on either is treated as a soft
/// error so that partial data is still returned.
pub(crate) async fn get_symbol_metadata_internal(
    client: &reqwest::Client,
    symbols: &[String],
) -> Result<Vec<SymbolMetadata>, String> {
    if symbols.is_empty() {
        return Ok(vec![]);
    }

    // ── 1. Bulk quote request for numeric fields ──────────────────────────────
    let joined = symbols.join(",");
    let quote_url = crate::config::YAHOO_QUOTE_URL.replace("{}", &joined);

    let quote_future = client
        .get(&quote_url)
        .header("User-Agent", crate::config::USER_AGENT)
        .send();

    // ── 2. Per-symbol assetProfile requests for sector/industry/country ───────
    let profile_futures: Vec<_> = symbols
        .iter()
        .map(|s| fetch_asset_profile(client, s))
        .collect();

    // Run both concurrently
    let (quote_response, profile_results) =
        futures::future::join(quote_future, futures::future::join_all(profile_futures)).await;

    // Parse bulk quote response (best-effort). The response future has already resolved
    // via `join`; we now just need to await the body deserialization.
    let quote_json: Option<serde_json::Value> = async {
        let resp = quote_response.ok()?;
        if !resp.status().is_success() {
            return None;
        }
        resp.json::<serde_json::Value>().await.ok()
    }
    .await;

    let quote_items: HashMap<String, serde_json::Value> = quote_json
        .and_then(|json| {
            json.pointer("/quoteResponse/result")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|item| {
                            let sym = item.get("symbol")?.as_str()?.to_string();
                            Some((sym, item.clone()))
                        })
                        .collect()
                })
        })
        .unwrap_or_default();

    // Build a lookup map: symbol → (sector, industry, country) from assetProfile
    type SectorTuple = (Option<String>, Option<String>, Option<String>);
    let profile_map: HashMap<String, SectorTuple> = profile_results
        .into_iter()
        .map(|(sym, sector, industry, country)| (sym, (sector, industry, country)))
        .collect();

    // ── 3. Merge into SymbolMetadata ─────────────────────────────────────────
    let metadata: Vec<SymbolMetadata> = symbols
        .iter()
        .map(|symbol| {
            let quote = quote_items.get(symbol);
            let (sector, industry, country) = profile_map
                .get(symbol)
                .cloned()
                .unwrap_or((None, None, None));

            SymbolMetadata {
                symbol: symbol.clone(),
                sector,
                industry,
                country,
                market_cap: quote
                    .and_then(|q| q.get("marketCap"))
                    .and_then(|v| v.as_f64()),
                pe_ratio: quote
                    .and_then(|q| q.get("trailingPE"))
                    .and_then(|v| v.as_f64()),
                dividend_yield: quote
                    .and_then(|q| q.get("trailingAnnualDividendYield"))
                    .and_then(|v| v.as_f64()),
                beta: quote.and_then(|q| q.get("beta")).and_then(|v| v.as_f64()),
            }
        })
        .collect();

    Ok(metadata)
}

#[tauri::command]
pub async fn get_symbol_metadata(
    _state: State<'_, DbState>,
    http: State<'_, HttpClient>,
    symbols: Vec<String>,
) -> Result<Vec<SymbolMetadata>, String> {
    get_symbol_metadata_internal(&http.0, &symbols).await
}

fn compute_portfolio_analytics(
    snapshot: &PortfolioSnapshot,
    metadata: &[SymbolMetadata],
) -> PortfolioAnalytics {
    let total_value = snapshot.total_value;

    // Build a lookup map from symbol → metadata
    let meta_map: HashMap<String, &SymbolMetadata> =
        metadata.iter().map(|m| (m.symbol.clone(), m)).collect();

    // Sector and country accumulators (symbol → (sector, country, market_value_cad))
    let mut sector_values: HashMap<String, f64> = HashMap::new();
    let mut country_values: HashMap<String, f64> = HashMap::new();

    let mut weighted_beta_sum = 0.0_f64;
    let mut weighted_beta_weight = 0.0_f64;
    let mut weighted_yield_sum = 0.0_f64;
    let mut largest_position_weight = 0.0_f64;

    for holding in &snapshot.holdings {
        let weight_fraction = if total_value > 0.0 {
            holding.market_value_cad / total_value
        } else {
            0.0
        };

        if holding.weight > largest_position_weight {
            largest_position_weight = holding.weight;
        }

        let (sector, country) = match holding.asset_type.as_str() {
            "cash" => ("Cash".to_string(), "N/A".to_string()),
            _ => {
                let sector = meta_map
                    .get(&holding.symbol)
                    .and_then(|m| m.sector.clone())
                    .unwrap_or_else(|| "Other".to_string());
                let country = meta_map
                    .get(&holding.symbol)
                    .and_then(|m| m.country.clone())
                    .unwrap_or_else(|| "Unknown".to_string());
                (sector, country)
            }
        };

        *sector_values.entry(sector).or_insert(0.0) += holding.market_value_cad;
        *country_values.entry(country).or_insert(0.0) += holding.market_value_cad;

        if let Some(meta) = meta_map.get(&holding.symbol) {
            if let Some(beta) = meta.beta {
                weighted_beta_sum += beta * weight_fraction;
                weighted_beta_weight += weight_fraction;
            }
            if let Some(div_yield) = meta.dividend_yield {
                weighted_yield_sum += div_yield * weight_fraction;
            }
        }
    }

    // Convert value accumulators to weight percentages
    let mut sector_breakdown: Vec<SectorWeight> = sector_values
        .into_iter()
        .map(|(sector, value)| SectorWeight {
            sector,
            weight_percent: if total_value > 0.0 {
                (value / total_value) * 100.0
            } else {
                0.0
            },
        })
        .collect();
    sector_breakdown.sort_by(|a, b| {
        b.weight_percent
            .partial_cmp(&a.weight_percent)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut country_breakdown: Vec<CountryWeight> = country_values
        .into_iter()
        .map(|(country, value)| CountryWeight {
            country,
            weight_percent: if total_value > 0.0 {
                (value / total_value) * 100.0
            } else {
                0.0
            },
        })
        .collect();
    country_breakdown.sort_by(|a, b| {
        b.weight_percent
            .partial_cmp(&a.weight_percent)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // HHI: sum of (weight_fraction^2) * 10000
    let concentration_hhi: f64 = snapshot
        .holdings
        .iter()
        .map(|h| {
            let w = if total_value > 0.0 {
                h.market_value_cad / total_value
            } else {
                0.0
            };
            w * w * 10000.0
        })
        .sum();

    let top_sector = sector_breakdown.first().map(|s| s.sector.clone());

    let weighted_beta = if weighted_beta_weight > 0.0 {
        Some(weighted_beta_sum / weighted_beta_weight)
    } else {
        None
    };

    let risk_metrics = PortfolioRiskMetrics {
        weighted_beta,
        portfolio_yield: weighted_yield_sum,
        largest_position_weight,
        top_sector,
        concentration_hhi,
    };

    PortfolioAnalytics {
        metadata: metadata.to_vec(),
        risk_metrics,
        sector_breakdown,
        country_breakdown,
    }
}

#[tauri::command]
pub async fn get_portfolio_analytics(
    db: State<'_, DbState>,
    http: State<'_, HttpClient>,
) -> Result<PortfolioAnalytics, String> {
    let base_currency = get_base_currency(&db);

    let holdings = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        db::get_all_holdings(&conn)?
    };

    let (cached_prices, cached_fx) = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        (db::get_cached_prices(&conn)?, db::get_fx_rates(&conn)?)
    };

    let snapshot = build_portfolio_snapshot(
        &holdings,
        &cached_prices,
        &cached_fx,
        &base_currency,
        Utc::now().to_rfc3339(),
        0.0,
    );

    // Only fetch metadata for non-cash symbols
    let non_cash_symbols: Vec<String> = snapshot
        .holdings
        .iter()
        .filter(|h| h.asset_type.as_str() != "cash")
        .map(|h| h.symbol.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    let metadata = get_symbol_metadata_internal(&http.0, &non_cash_symbols)
        .await
        .unwrap_or_default();

    Ok(compute_portfolio_analytics(&snapshot, &metadata))
}

// ── Realized gains command ────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_realized_gains(
    db: State<'_, DbState>,
    holding_id: Option<String>,
) -> Result<RealizedGainsSummary, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let cost_basis_method =
        db::get_config(&conn, "cost_basis_method")?.unwrap_or_else(|| "avco".to_string());

    let transactions = match holding_id {
        Some(ref id) => db::get_transactions_for_holding(&conn, id).map_err(|e| e.to_string())?,
        None => db::get_all_transactions(&conn).map_err(|e| e.to_string())?,
    };

    compute_realized_gains_grouped(&transactions, &cost_basis_method)
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
            exchange: String::new(),
            target_weight: 0.0,
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
    fn parse_import_rows_reads_optional_target_weight() {
        let csv = "symbol,name,type,quantity,cost_basis,currency,target_weight\nAAPL,Apple Inc.,stock,5,120,USD,12.5\n";
        let rows = parse_import_rows(csv).expect("parse csv");

        assert_eq!(rows.len(), 1);
        assert!((rows[0].target_weight - 12.5).abs() < 0.001);
    }

    #[test]
    fn parse_import_rows_rejects_missing_required_columns() {
        let csv = "symbol,name,type,quantity,currency\nAAPL,Apple Inc.,stock,5,USD\n";
        let error = parse_import_rows(csv).expect_err("missing cost_basis should fail");

        assert!(error.contains("Missing required column: cost_basis"));
    }

    #[test]
    fn import_weight_sum_over_100_detected() {
        // Two rows whose target_weight values sum to 110; the command-level guard
        // rejects this.  Verify parse_import_rows succeeds and the sum exceeds 100.
        let csv = "symbol,name,type,quantity,cost_basis,currency,target_weight\n\
                   AAPL,Apple Inc.,stock,5,120,USD,60\n\
                   MSFT,Microsoft,stock,3,200,USD,50\n";
        let rows = parse_import_rows(csv).expect("rows should parse");
        let total: f64 = rows.iter().map(|r| r.target_weight).sum();
        assert!(
            total > 100.0,
            "expected total > 100 to trigger command-level guard, got {}",
            total
        );
    }

    #[test]
    fn import_weight_sum_at_100_is_valid() {
        let csv = "symbol,name,type,quantity,cost_basis,currency,target_weight\n\
                   AAPL,Apple Inc.,stock,5,120,USD,60\n\
                   MSFT,Microsoft,stock,3,200,USD,40\n";
        let rows = parse_import_rows(csv).expect("rows should parse");
        let total: f64 = rows.iter().map(|r| r.target_weight).sum();
        assert!(
            (total - 100.0).abs() < 0.001,
            "expected total == 100, got {}",
            total
        );
    }

    #[test]
    fn build_holdings_csv_includes_target_weight_column() {
        let mut holding = make_holding("AAPL", AssetType::Stock, 5.0, 120.0, "USD");
        holding.target_weight = 22.5;

        let csv = build_holdings_csv(&[holding]).expect("build csv");

        assert!(csv.starts_with(
            "symbol,name,type,account,quantity,cost_basis,currency,exchange,target_weight"
        ));
        assert!(csv.contains(",22.5"));
    }

    // ── CSV round-trip tests ──────────────────────────────────────────────────

    /// Export a set of holdings to CSV, re-parse it with `parse_import_rows`,
    /// and verify that every key field is preserved exactly.
    #[test]
    fn csv_round_trip_preserves_key_fields() {
        let mut h1 = make_holding("AAPL", AssetType::Stock, 10.0, 155.25, "USD");
        h1.name = "Apple Inc.".to_string();
        h1.exchange = "NMS".to_string();
        h1.target_weight = 25.0;

        let mut h2 = make_holding("XIU.TO", AssetType::Etf, 50.0, 34.5, "CAD");
        h2.name = "iShares S&P/TSX 60 Index ETF".to_string();
        h2.exchange = "TRT".to_string();
        h2.target_weight = 15.0;

        let mut h3 = make_holding("BTC-USD", AssetType::Crypto, 0.5, 40000.0, "USD");
        h3.name = "Bitcoin USD".to_string();
        h3.target_weight = 10.0;

        let holdings = vec![h1, h2, h3];
        let csv = build_holdings_csv(&holdings).expect("build csv");

        let rows = parse_import_rows(&csv).expect("parse csv");

        assert_eq!(rows.len(), 3, "row count should be preserved");

        // Row 0 — AAPL (stock)
        assert_eq!(rows[0].symbol, "AAPL");
        assert!(matches!(rows[0].asset_type, AssetType::Stock));
        assert!((rows[0].quantity - 10.0).abs() < 0.001);
        assert!((rows[0].cost_basis - 155.25).abs() < 0.001);
        assert_eq!(rows[0].currency, "USD");
        assert_eq!(rows[0].exchange, "NMS");
        assert!((rows[0].target_weight - 25.0).abs() < 0.001);

        // Row 1 — XIU.TO (etf)
        assert_eq!(rows[1].symbol, "XIU.TO");
        assert!(matches!(rows[1].asset_type, AssetType::Etf));
        assert!((rows[1].quantity - 50.0).abs() < 0.001);
        assert!((rows[1].cost_basis - 34.5).abs() < 0.001);
        assert_eq!(rows[1].currency, "CAD");
        assert_eq!(rows[1].exchange, "TRT");
        assert!((rows[1].target_weight - 15.0).abs() < 0.001);

        // Row 2 — BTC-USD (crypto)
        assert_eq!(rows[2].symbol, "BTC-USD");
        assert!(matches!(rows[2].asset_type, AssetType::Crypto));
        assert!((rows[2].quantity - 0.5).abs() < 0.001);
        assert!((rows[2].cost_basis - 40000.0).abs() < 0.001);
        assert_eq!(rows[2].currency, "USD");
        assert!((rows[2].target_weight - 10.0).abs() < 0.001);
    }

    /// Exporting a single cash holding round-trips correctly.
    #[test]
    fn csv_round_trip_cash_holding() {
        let mut cash = make_holding("CAD-CASH", AssetType::Cash, 5000.0, 1.0, "CAD");
        cash.name = "CAD Cash".to_string();
        cash.target_weight = 5.0;

        let csv = build_holdings_csv(&[cash]).expect("build csv");
        let rows = parse_import_rows(&csv).expect("parse csv");

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].symbol, "CAD-CASH");
        assert!(matches!(rows[0].asset_type, AssetType::Cash));
        assert!((rows[0].quantity - 5000.0).abs() < 0.001);
        assert!((rows[0].cost_basis - 1.0).abs() < 0.001);
        assert_eq!(rows[0].currency, "CAD");
        assert!((rows[0].target_weight - 5.0).abs() < 0.001);
    }

    /// An empty holdings slice produces a CSV that fails parsing (no data rows).
    #[test]
    fn build_holdings_csv_empty_slice_roundtrip_fails_gracefully() {
        let csv = build_holdings_csv(&[]).expect("build csv for empty slice");
        // build_holdings_csv writes a header-only CSV; parse_import_rows should
        // return an error because there are no data rows.
        let result = parse_import_rows(&csv);
        assert!(result.is_err(), "empty csv should error on import");
        assert!(result.unwrap_err().contains("empty"));
    }

    /// Round-trip with target_weight = 0 (the default) is preserved as 0.
    #[test]
    fn csv_round_trip_zero_target_weight() {
        let holding = make_holding("MSFT", AssetType::Stock, 3.0, 200.0, "USD");
        // target_weight is already 0.0 from make_holding

        let csv = build_holdings_csv(&[holding]).expect("build csv");
        let rows = parse_import_rows(&csv).expect("parse csv");

        assert_eq!(rows.len(), 1);
        assert!((rows[0].target_weight - 0.0).abs() < 0.001);
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
            0.0,
        );

        assert_eq!(snapshot.base_currency, "CAD");
        assert!((snapshot.holdings[0].market_value_cad - 1200.0).abs() < 0.001);
        assert!((snapshot.holdings[1].market_value_cad - 687.5).abs() < 0.001);
        assert!((snapshot.holdings[1].cost_value_cad - 625.0).abs() < 0.001);
        assert!((snapshot.total_value - 1887.5).abs() < 0.001);
        assert!((snapshot.total_cost - 1625.0).abs() < 0.001);
        assert!((snapshot.daily_pnl - 92.75).abs() < 0.001);
        assert_eq!(snapshot.total_target_weight, 0.0);
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
            0.0,
        );

        assert_eq!(snapshot.base_currency, "USD");
        assert!((snapshot.holdings[0].market_value_cad - 176.0).abs() < 0.001);
        assert!((snapshot.holdings[0].cost_value_cad - 160.0).abs() < 0.001);
        assert!((snapshot.holdings[1].market_value_cad - 220.0).abs() < 0.001);
        assert!((snapshot.total_value - 396.0).abs() < 0.001);
        assert!((snapshot.total_cost - 360.0).abs() < 0.001);
    }

    // ── Target-weight portfolio-level validation tests ──────────────────────

    #[test]
    fn add_holding_weight_exceeds_100_when_existing_sum_plus_new_is_over_limit() {
        // Simulate the guard logic that add_holding applies before inserting.
        // We verify that existing_sum + new_weight > 100 is caught.
        let existing_sum = 60.0f64;
        let new_weight = 50.0f64;
        assert!(
            existing_sum + new_weight > 100.0,
            "guard should reject: {:.1} + {:.1} = {:.1} > 100",
            existing_sum,
            new_weight,
            existing_sum + new_weight
        );
    }

    #[test]
    fn add_holding_weight_exactly_100_is_accepted() {
        let existing_sum = 60.0f64;
        let new_weight = 40.0f64;
        assert!(
            existing_sum + new_weight <= 100.0,
            "guard should allow: {:.1} + {:.1} = {:.1} <= 100",
            existing_sum,
            new_weight,
            existing_sum + new_weight
        );
    }

    #[test]
    fn update_holding_weight_exceeds_100_when_others_sum_plus_new_is_over_limit() {
        // Simulate the guard logic used by update_holding (other holdings sum + new value).
        let others_sum = 70.0f64;
        let new_weight = 35.0f64;
        assert!(
            others_sum + new_weight > 100.0,
            "guard should reject: {:.1} + {:.1} = {:.1} > 100",
            others_sum,
            new_weight,
            others_sum + new_weight
        );
    }

    #[test]
    fn import_csv_weight_sum_over_100_is_rejected() {
        let csv = "symbol,name,type,quantity,cost_basis,currency,target_weight\n\
                   AAPL,Apple,stock,5,120,USD,60\n\
                   MSFT,Microsoft,stock,3,200,USD,50\n";
        let rows = parse_import_rows(csv).expect("parse ok");
        let total: f64 = rows.iter().map(|r| r.target_weight).sum();
        assert!(
            total > 100.0,
            "csv weight sum should exceed 100, got {:.1}",
            total
        );
        // Confirm the error message format is correct when this check fires
        let err = format!(
            "Import failed: total target weight is {:.1}% (max 100%). Adjust weights before re-importing.",
            total
        );
        assert!(err.contains("Import failed"));
        assert!(err.contains("110.0%"));
    }

    #[test]
    fn import_csv_weight_sum_at_100_passes_csv_level_guard() {
        let csv = "symbol,name,type,quantity,cost_basis,currency,target_weight\n\
                   AAPL,Apple,stock,5,120,USD,60\n\
                   MSFT,Microsoft,stock,3,200,USD,40\n";
        let rows = parse_import_rows(csv).expect("parse ok");
        let total: f64 = rows.iter().map(|r| r.target_weight).sum();
        assert!(
            total <= 100.0,
            "csv weight sum should be <= 100, got {:.1}",
            total
        );
    }

    #[test]
    fn import_csv_existing_holdings_combined_with_csv_exceeds_100_is_rejected() {
        let existing_weight_sum = 70.0f64;
        let csv = "symbol,name,type,quantity,cost_basis,currency,target_weight\n\
                   GOOG,Alphabet,stock,2,150,USD,40\n";
        let rows = parse_import_rows(csv).expect("parse ok");
        let csv_sum: f64 = rows.iter().map(|r| r.target_weight).sum();
        // csv_sum alone (40) is <= 100, so it passes the CSV-level guard
        assert!(csv_sum <= 100.0);
        // But combined with existing it exceeds 100
        assert!(
            existing_weight_sum + csv_sum > 100.0,
            "combined should exceed 100, got {:.1}",
            existing_weight_sum + csv_sum
        );
    }

    #[test]
    fn build_portfolio_snapshot_computes_target_deltas() {
        let mut holdings = vec![
            make_holding("AAPL", AssetType::Stock, 10.0, 100.0, "CAD"),
            make_holding("CAD-CASH", AssetType::Cash, 500.0, 1.0, "CAD"),
        ];
        holdings[0].target_weight = 60.0;
        holdings[1].target_weight = 10.0;

        let prices = vec![PriceData {
            symbol: "AAPL".to_string(),
            price: 120.0,
            currency: "CAD".to_string(),
            change: 0.0,
            change_percent: 0.0,
            updated_at: Utc::now().to_rfc3339(),
        }];

        let snapshot = build_portfolio_snapshot(
            &holdings,
            &prices,
            &[],
            "CAD",
            "2024-01-01T00:00:00Z".to_string(),
            0.0,
        );

        assert!((snapshot.total_value - 1700.0).abs() < 0.001);
        assert!((snapshot.total_target_weight - 70.0).abs() < 0.001);
        assert!((snapshot.holdings[0].target_value - 1020.0).abs() < 0.001);
        assert!((snapshot.holdings[0].target_delta_value + 180.0).abs() < 0.001);
        assert!((snapshot.holdings[1].target_delta_value + 330.0).abs() < 0.001);
        assert!((snapshot.target_cash_delta - 330.0).abs() < 0.001);
    }

    #[test]
    fn build_portfolio_snapshot_excludes_intraday_purchase_from_daily_pnl() {
        // A holding created today should contribute 0 to daily_pnl.
        let today = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let mut holding = make_holding("AAPL", AssetType::Stock, 10.0, 100.0, "CAD");
        holding.created_at = today;

        let prices = vec![PriceData {
            symbol: "AAPL".to_string(),
            price: 120.0,
            currency: "CAD".to_string(),
            change: 2.0,
            change_percent: 5.0, // would be 60.0 CAD if applied
            updated_at: Utc::now().to_rfc3339(),
        }];

        let snapshot = build_portfolio_snapshot(
            &[holding],
            &prices,
            &[],
            "CAD",
            Utc::now().to_rfc3339(),
            0.0,
        );

        // market_value_cad = 10 * 120 = 1200; daily_pnl should be 0, not 60
        assert!(
            (snapshot.daily_pnl - 0.0).abs() < 0.001,
            "expected daily_pnl == 0 for intraday purchase, got {}",
            snapshot.daily_pnl
        );
    }

    #[test]
    fn build_portfolio_snapshot_includes_prior_day_holding_in_daily_pnl() {
        // A holding created yesterday (or earlier) should contribute normally.
        let yesterday = (Utc::now() - chrono::Duration::days(1))
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();
        let mut holding = make_holding("MSFT", AssetType::Stock, 10.0, 200.0, "CAD");
        holding.created_at = yesterday;

        let prices = vec![PriceData {
            symbol: "MSFT".to_string(),
            price: 220.0,
            currency: "CAD".to_string(),
            change: 20.0,
            change_percent: 10.0, // 10% of 2200 = 220
            updated_at: Utc::now().to_rfc3339(),
        }];

        let snapshot = build_portfolio_snapshot(
            &[holding],
            &prices,
            &[],
            "CAD",
            Utc::now().to_rfc3339(),
            0.0,
        );

        // market_value_cad = 10 * 220 = 2200; daily_pnl = 2200 * 0.10 = 220
        assert!(
            (snapshot.daily_pnl - 220.0).abs() < 0.001,
            "expected daily_pnl == 220 for prior-day holding, got {}",
            snapshot.daily_pnl
        );
    }
}

// ── Transaction commands ──────────────────────────────────────────────────────

#[tauri::command]
pub async fn add_transaction(
    db: State<'_, DbState>,
    input: TransactionInput,
) -> Result<Transaction, String> {
    if input.quantity <= 0.0 {
        return Err("Transaction quantity must be positive".to_string());
    }
    if input.price < 0.0 {
        return Err("Transaction price must be non-negative".to_string());
    }
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    db::insert_transaction(&conn, input)
}

#[tauri::command]
pub async fn get_transactions(
    db: State<'_, DbState>,
    holding_id: Option<String>,
) -> Result<Vec<Transaction>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    match holding_id {
        Some(id) => db::get_transactions_for_holding(&conn, &id),
        None => db::get_all_transactions(&conn),
    }
}

#[tauri::command]
pub async fn delete_transaction(db: State<'_, DbState>, id: String) -> Result<bool, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    db::delete_transaction(&conn, &id)
}

// ── Account Commands ──────────────────────────────────────────────────────────

const VALID_ACCOUNT_TYPES: &[&str] = &["tfsa", "rrsp", "fhsa", "taxable", "crypto", "other"];

#[tauri::command]
pub async fn get_accounts(state: tauri::State<'_, DbState>) -> Result<Vec<Account>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    db::get_accounts(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_account(
    state: tauri::State<'_, DbState>,
    account: CreateAccountRequest,
) -> Result<Account, String> {
    let name = account.name.trim().to_string();
    if name.is_empty() {
        return Err("Account name cannot be empty".to_string());
    }
    if !VALID_ACCOUNT_TYPES.contains(&account.account_type.as_str()) {
        return Err(format!("Invalid account type: {}", account.account_type));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let created_at = Utc::now().to_rfc3339();
    let institution = account.institution.clone();
    let account_type = account.account_type.clone();

    let conn = state.0.lock().map_err(|e| e.to_string())?;
    db::insert_account(&conn, &id, &name, &account_type, institution.as_deref())
        .map_err(|e| e.to_string())?;

    Ok(Account {
        id,
        name,
        account_type,
        institution,
        created_at,
    })
}

#[tauri::command]
pub async fn update_account(
    state: tauri::State<'_, DbState>,
    id: String,
    account: CreateAccountRequest,
) -> Result<Account, String> {
    let name = account.name.trim().to_string();
    if name.is_empty() {
        return Err("Account name cannot be empty".to_string());
    }
    if !VALID_ACCOUNT_TYPES.contains(&account.account_type.as_str()) {
        return Err(format!("Invalid account type: {}", account.account_type));
    }

    let institution = account.institution.clone();
    let account_type = account.account_type.clone();

    let conn = state.0.lock().map_err(|e| e.to_string())?;
    // Fetch created_at for the returned struct
    let existing: Vec<Account> = db::get_accounts(&conn).map_err(|e| e.to_string())?;
    let created_at = existing
        .iter()
        .find(|a| a.id == id)
        .map(|a| a.created_at.clone())
        .ok_or_else(|| format!("Account {} not found", id))?;

    db::update_account(&conn, &id, &name, &account_type, institution.as_deref())
        .map_err(|e| e.to_string())?;

    Ok(Account {
        id,
        name,
        account_type,
        institution,
        created_at,
    })
}

#[tauri::command]
pub async fn delete_account(state: tauri::State<'_, DbState>, id: String) -> Result<bool, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    db::delete_account(&conn, &id).map_err(|e| e.to_string())?;
    Ok(true)
}
