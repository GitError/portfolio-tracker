use rusqlite::{Connection, params};
use chrono::Utc;
use uuid::Uuid;
use std::str::FromStr;

use crate::types::{AssetType, FxRate, Holding, HoldingInput, PriceData};

pub fn init_db(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS holdings (
            id          TEXT PRIMARY KEY,
            symbol      TEXT NOT NULL,
            name        TEXT NOT NULL,
            asset_type  TEXT NOT NULL,
            quantity    REAL NOT NULL,
            cost_basis  REAL NOT NULL,
            currency    TEXT NOT NULL,
            created_at  TEXT NOT NULL,
            updated_at  TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS price_cache (
            symbol          TEXT PRIMARY KEY,
            price           REAL NOT NULL,
            currency        TEXT NOT NULL,
            change          REAL NOT NULL DEFAULT 0,
            change_percent  REAL NOT NULL DEFAULT 0,
            updated_at      TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS fx_rates (
            pair        TEXT PRIMARY KEY,
            rate        REAL NOT NULL,
            updated_at  TEXT NOT NULL
        );
        ",
    )
    .map_err(|e| e.to_string())
}

pub fn insert_holding(conn: &Connection, input: HoldingInput) -> Result<Holding, String> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let asset_type_str = input.asset_type.as_str();

    conn.execute(
        "INSERT INTO holdings (id, symbol, name, asset_type, quantity, cost_basis, currency, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            id,
            input.symbol,
            input.name,
            asset_type_str,
            input.quantity,
            input.cost_basis,
            input.currency,
            now,
            now
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(Holding {
        id,
        symbol: input.symbol,
        name: input.name,
        asset_type: input.asset_type,
        quantity: input.quantity,
        cost_basis: input.cost_basis,
        currency: input.currency,
        created_at: now.clone(),
        updated_at: now,
    })
}

pub fn update_holding(conn: &Connection, holding: Holding) -> Result<Holding, String> {
    let now = Utc::now().to_rfc3339();
    let asset_type_str = holding.asset_type.as_str();

    let rows = conn
        .execute(
            "UPDATE holdings SET symbol=?1, name=?2, asset_type=?3, quantity=?4, cost_basis=?5, currency=?6, updated_at=?7
             WHERE id=?8",
            params![
                holding.symbol,
                holding.name,
                asset_type_str,
                holding.quantity,
                holding.cost_basis,
                holding.currency,
                now,
                holding.id
            ],
        )
        .map_err(|e| e.to_string())?;

    if rows == 0 {
        return Err(format!("Holding {} not found", holding.id));
    }

    Ok(Holding {
        updated_at: now,
        ..holding
    })
}

pub fn delete_holding(conn: &Connection, id: &str) -> Result<bool, String> {
    let rows = conn
        .execute("DELETE FROM holdings WHERE id=?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(rows > 0)
}

pub fn get_all_holdings(conn: &Connection) -> Result<Vec<Holding>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, symbol, name, asset_type, quantity, cost_basis, currency, created_at, updated_at
             FROM holdings ORDER BY created_at ASC",
        )
        .map_err(|e| e.to_string())?;

    let holdings = stmt
        .query_map([], |row| {
            let asset_type_str: String = row.get(3)?;
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                asset_type_str,
                row.get::<_, f64>(4)?,
                row.get::<_, f64>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, String>(8)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .map(|(id, symbol, name, asset_type_str, quantity, cost_basis, currency, created_at, updated_at)| {
            let asset_type = AssetType::from_str(&asset_type_str).unwrap_or(AssetType::Stock);
            Holding {
                id,
                symbol,
                name,
                asset_type,
                quantity,
                cost_basis,
                currency,
                created_at,
                updated_at,
            }
        })
        .collect();

    Ok(holdings)
}

pub fn upsert_price(conn: &Connection, price: &PriceData) -> Result<(), String> {
    conn.execute(
        "INSERT INTO price_cache (symbol, price, currency, change, change_percent, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(symbol) DO UPDATE SET price=excluded.price, currency=excluded.currency,
         change=excluded.change, change_percent=excluded.change_percent, updated_at=excluded.updated_at",
        params![
            price.symbol,
            price.price,
            price.currency,
            price.change,
            price.change_percent,
            price.updated_at
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn get_cached_prices(conn: &Connection) -> Result<Vec<PriceData>, String> {
    let mut stmt = conn
        .prepare("SELECT symbol, price, currency, change, change_percent, updated_at FROM price_cache")
        .map_err(|e| e.to_string())?;

    let prices = stmt
        .query_map([], |row| {
            Ok(PriceData {
                symbol: row.get(0)?,
                price: row.get(1)?,
                currency: row.get(2)?,
                change: row.get(3)?,
                change_percent: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(prices)
}

pub fn upsert_fx_rate(conn: &Connection, rate: &FxRate) -> Result<(), String> {
    conn.execute(
        "INSERT INTO fx_rates (pair, rate, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(pair) DO UPDATE SET rate=excluded.rate, updated_at=excluded.updated_at",
        params![rate.pair, rate.rate, rate.updated_at],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn get_fx_rates(conn: &Connection) -> Result<Vec<FxRate>, String> {
    let mut stmt = conn
        .prepare("SELECT pair, rate, updated_at FROM fx_rates")
        .map_err(|e| e.to_string())?;

    let rates = stmt
        .query_map([], |row| {
            Ok(FxRate {
                pair: row.get(0)?,
                rate: row.get(1)?,
                updated_at: row.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(rates)
}
