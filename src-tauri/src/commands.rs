use std::collections::HashSet;
use std::sync::Mutex;

use chrono::Utc;
use csv::{ReaderBuilder, StringRecord, Trim};
use tauri::State;

use crate::db;
use crate::fx::fetch_all_fx_rates;
use crate::price::{fetch_all_prices, fetch_price};
use crate::stress::run_stress_test;
use crate::types::{
    AssetType, FxRate, Holding, HoldingInput, HoldingWithPrice, ImportError, ImportResult,
    PortfolioSnapshot, PriceData, StressResult, StressScenario,
};

const MAX_IMPORT_ROWS: usize = 500;

pub struct DbState(pub Mutex<rusqlite::Connection>);
pub struct HttpClient(pub reqwest::Client);

#[derive(Debug)]
struct ParsedImportRow {
    row: usize,
    symbol: String,
    name: String,
    asset_type: AssetType,
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
        let currency =
            parse_required_field(&record, currency_index, row, "currency")?.to_uppercase();
        let raw_symbol = parse_optional_field(&record, Some(symbol_index));
        let symbol = if matches!(asset_type, AssetType::Cash) {
            if raw_symbol.is_empty() || raw_symbol.eq_ignore_ascii_case("CASH") {
                format!("{}-CASH", currency)
            } else {
                raw_symbol.to_uppercase()
            }
        } else if raw_symbol.is_empty() {
            return Err(format!("Row {}: missing_symbol", row));
        } else {
            raw_symbol.to_uppercase()
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

#[tauri::command]
pub async fn get_portfolio(
    db: State<'_, DbState>,
    _client: State<'_, HttpClient>,
) -> Result<PortfolioSnapshot, String> {
    let holdings = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        db::get_all_holdings(&conn)?
    };

    if holdings.is_empty() {
        return Ok(PortfolioSnapshot {
            holdings: vec![],
            total_value: 0.0,
            total_cost: 0.0,
            total_gain_loss: 0.0,
            total_gain_loss_percent: 0.0,
            daily_pnl: 0.0,
            last_updated: Utc::now().to_rfc3339(),
        });
    }

    // Get cached prices and FX rates
    let (cached_prices, cached_fx) = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        (db::get_cached_prices(&conn)?, db::get_fx_rates(&conn)?)
    };

    // Build lookup maps
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

    for holding in &holdings {
        let (current_price, change_percent) = if holding.asset_type.as_str() == "cash" {
            (1.0f64, 0.0f64)
        } else {
            price_map
                .get(&holding.symbol)
                .map(|p| (p.price, p.change_percent))
                .unwrap_or((holding.cost_basis, 0.0))
        };

        let fx_pair = format!("{}CAD", holding.currency.to_uppercase());
        let fx_rate = if holding.currency.to_uppercase() == "CAD" {
            1.0
        } else {
            fx_map.get(&fx_pair).map(|r| r.rate).unwrap_or(1.0)
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
            weight: 0.0, // filled below
            daily_change_percent: change_percent,
        });
    }

    // Back-fill weights now that we have total_value
    for h in &mut holdings_with_price {
        h.weight = if total_value != 0.0 {
            (h.market_value_cad / total_value) * 100.0
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

    Ok(PortfolioSnapshot {
        holdings: holdings_with_price,
        total_value,
        total_cost,
        total_gain_loss,
        total_gain_loss_percent,
        daily_pnl,
        last_updated: Utc::now().to_rfc3339(),
    })
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
pub async fn import_holdings_csv(
    db: State<'_, DbState>,
    client: State<'_, HttpClient>,
    csv_content: String,
) -> Result<ImportResult, String> {
    let parsed_rows = parse_import_rows(&csv_content)?;
    let mut seen_symbols = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        db::get_all_holdings(&conn)?
            .into_iter()
            .map(|holding| holding.symbol.to_uppercase())
            .collect::<HashSet<_>>()
    };

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

        if !matches!(row.asset_type, AssetType::Cash)
            && fetch_price(&client.0, &row.symbol).await.is_err()
        {
            skipped.push(ImportError {
                row: row.row,
                symbol: row.symbol,
                reason: "invalid_symbol".to_string(),
            });
            continue;
        }

        let symbol = row.symbol.to_uppercase();
        let name = if row.name.is_empty() {
            if matches!(row.asset_type, AssetType::Cash) {
                format!("{} Cash", row.currency)
            } else {
                symbol.clone()
            }
        } else {
            row.name
        };

        seen_symbols.insert(symbol.clone());
        pending_inputs.push(HoldingInput {
            symbol,
            name,
            asset_type: row.asset_type,
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

    let total_rows = imported.len() + skipped.len();

    Ok(ImportResult {
        imported,
        skipped,
        total_rows,
    })
}

#[tauri::command]
pub async fn refresh_prices(
    db: State<'_, DbState>,
    client: State<'_, HttpClient>,
) -> Result<Vec<PriceData>, String> {
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

    // Collect unique non-CAD currencies
    let currencies: Vec<String> = holdings
        .iter()
        .map(|h| h.currency.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .filter(|c| c.to_uppercase() != "CAD")
        .collect();

    let (prices, fx_rates) = tokio::join!(
        fetch_all_prices(&client.0, symbols),
        fetch_all_fx_rates(&client.0, currencies)
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
}
