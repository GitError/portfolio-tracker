use std::sync::Mutex;

use chrono::Utc;
use tauri::State;

use crate::db;
use crate::fx::fetch_all_fx_rates;
use crate::price::fetch_all_prices;
use crate::stress::run_stress_test;
use crate::types::{
    FxRate, Holding, HoldingInput, HoldingWithPrice, PortfolioSnapshot, PriceData, StressResult,
    StressScenario,
};

pub struct DbState(pub Mutex<rusqlite::Connection>);
pub struct HttpClient(pub reqwest::Client);

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

    let fx_map: std::collections::HashMap<String, &FxRate> = cached_fx
        .iter()
        .map(|r| (r.pair.clone(), r))
        .collect();

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
pub async fn add_holding(
    db: State<'_, DbState>,
    holding: HoldingInput,
) -> Result<Holding, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    db::insert_holding(&conn, holding)
}

#[tauri::command]
pub async fn update_holding(
    db: State<'_, DbState>,
    holding: Holding,
) -> Result<Holding, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    db::update_holding(&conn, holding)
}

#[tauri::command]
pub async fn delete_holding(db: State<'_, DbState>, id: String) -> Result<bool, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    db::delete_holding(&conn, &id)
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
