use chrono::Utc;
use rusqlite::{params, Connection};
use std::str::FromStr;
use uuid::Uuid;

use crate::types::{AssetType, FxRate, Holding, HoldingInput, PriceData, SymbolResult};

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

        CREATE TABLE IF NOT EXISTS symbol_cache (
            symbol      TEXT PRIMARY KEY,
            name        TEXT NOT NULL,
            asset_type  TEXT NOT NULL,
            exchange    TEXT NOT NULL DEFAULT '',
            currency    TEXT NOT NULL DEFAULT 'USD',
            updated_at  TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS app_config (
            key     TEXT PRIMARY KEY,
            value   TEXT NOT NULL
        );
        ",
    )
    .map_err(|e| e.to_string())
}

pub fn get_config(conn: &Connection, key: &str) -> Result<Option<String>, String> {
    let mut stmt = conn
        .prepare("SELECT value FROM app_config WHERE key=?1")
        .map_err(|e| e.to_string())?;
    let mut rows = stmt.query(params![key]).map_err(|e| e.to_string())?;
    match rows.next().map_err(|e| e.to_string())? {
        Some(row) => Ok(Some(row.get(0).map_err(|e| e.to_string())?)),
        None => Ok(None),
    }
}

pub fn set_config(conn: &Connection, key: &str, value: &str) -> Result<(), String> {
    conn.execute(
        "INSERT INTO app_config (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        params![key, value],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
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
        .map(
            |(
                id,
                symbol,
                name,
                asset_type_str,
                quantity,
                cost_basis,
                currency,
                created_at,
                updated_at,
            )| {
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
            },
        )
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
        .prepare(
            "SELECT symbol, price, currency, change, change_percent, updated_at FROM price_cache",
        )
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

pub fn upsert_symbol(conn: &Connection, result: &SymbolResult) -> Result<(), String> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO symbol_cache (symbol, name, asset_type, exchange, currency, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(symbol) DO UPDATE SET
           name=excluded.name, asset_type=excluded.asset_type,
           exchange=excluded.exchange, currency=excluded.currency,
           updated_at=excluded.updated_at",
        params![
            result.symbol,
            result.name,
            result.asset_type.as_str(),
            result.exchange,
            result.currency,
            now
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn search_symbol_cache(conn: &Connection, query: &str) -> Result<Vec<SymbolResult>, String> {
    let pattern = format!("%{}%", query.to_lowercase());
    let sym_prefix = format!("{}%", query.to_uppercase());
    let mut stmt = conn
        .prepare(
            "SELECT symbol, name, asset_type, exchange, currency FROM symbol_cache
             WHERE symbol LIKE ?1 OR LOWER(name) LIKE ?2
             ORDER BY CASE WHEN symbol LIKE ?1 THEN 0 ELSE 1 END
             LIMIT 8",
        )
        .map_err(|e| e.to_string())?;

    let results = stmt
        .query_map(params![sym_prefix, pattern], |row| {
            let asset_type_str: String = row.get(2)?;
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                asset_type_str,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .map(|(symbol, name, asset_type_str, exchange, currency)| {
            let asset_type = AssetType::from_str(&asset_type_str).unwrap_or(AssetType::Stock);
            SymbolResult {
                symbol,
                name,
                asset_type,
                exchange,
                currency,
            }
        })
        .collect();

    Ok(results)
}

pub fn get_symbol_cache_exact(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<SymbolResult>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT symbol, name, asset_type, exchange, currency
             FROM symbol_cache
             WHERE UPPER(symbol) = UPPER(?1)
             LIMIT 1",
        )
        .map_err(|e| e.to_string())?;

    let mut rows = stmt.query(params![symbol]).map_err(|e| e.to_string())?;
    let Some(row) = rows.next().map_err(|e| e.to_string())? else {
        return Ok(None);
    };

    let asset_type_str: String = row.get(2).map_err(|e| e.to_string())?;
    let asset_type = AssetType::from_str(&asset_type_str).unwrap_or(AssetType::Stock);

    Ok(Some(SymbolResult {
        symbol: row.get(0).map_err(|e| e.to_string())?,
        name: row.get(1).map_err(|e| e.to_string())?,
        asset_type,
        exchange: row.get(3).map_err(|e| e.to_string())?,
        currency: row.get(4).map_err(|e| e.to_string())?,
    }))
}

pub fn holding_exists(conn: &Connection, symbol: &str) -> Result<bool, String> {
    let mut stmt = conn
        .prepare("SELECT 1 FROM holdings WHERE UPPER(symbol) = UPPER(?1) LIMIT 1")
        .map_err(|e| e.to_string())?;

    let mut rows = stmt.query(params![symbol]).map_err(|e| e.to_string())?;
    Ok(rows.next().map_err(|e| e.to_string())?.is_some())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_test_db() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory db");
        init_db(&conn).expect("init_db");
        conn
    }

    fn make_input(symbol: &str) -> HoldingInput {
        HoldingInput {
            symbol: symbol.to_string(),
            name: format!("{} Inc.", symbol),
            asset_type: AssetType::Stock,
            quantity: 10.0,
            cost_basis: 100.0,
            currency: "CAD".to_string(),
        }
    }

    #[test]
    fn init_db_creates_tables() {
        let conn = open_test_db();
        // Should be able to query all three tables without error
        conn.execute_batch(
            "SELECT 1 FROM holdings; SELECT 1 FROM price_cache; SELECT 1 FROM fx_rates;",
        )
        .expect("tables should exist");
    }

    #[test]
    fn insert_and_get_holdings() {
        let conn = open_test_db();
        insert_holding(&conn, make_input("AAPL")).expect("insert");
        insert_holding(&conn, make_input("MSFT")).expect("insert");
        let holdings = get_all_holdings(&conn).expect("get all");
        assert_eq!(holdings.len(), 2);
        let symbols: Vec<&str> = holdings.iter().map(|h| h.symbol.as_str()).collect();
        assert!(symbols.contains(&"AAPL"));
        assert!(symbols.contains(&"MSFT"));
    }

    #[test]
    fn update_holding_changes_fields() {
        let conn = open_test_db();
        let inserted = insert_holding(&conn, make_input("GOOG")).expect("insert");
        let updated_holding = Holding {
            quantity: 20.0,
            cost_basis: 150.0,
            ..inserted
        };
        let updated = update_holding(&conn, updated_holding).expect("update");
        assert!((updated.quantity - 20.0).abs() < 0.001);
        assert!((updated.cost_basis - 150.0).abs() < 0.001);
    }

    #[test]
    fn delete_holding_removes_row() {
        let conn = open_test_db();
        let holding = insert_holding(&conn, make_input("TSLA")).expect("insert");
        let deleted = delete_holding(&conn, &holding.id).expect("delete");
        assert!(deleted);
        let holdings = get_all_holdings(&conn).expect("get all");
        assert_eq!(holdings.len(), 0);
    }

    #[test]
    fn delete_nonexistent_holding_returns_false() {
        let conn = open_test_db();
        let deleted = delete_holding(&conn, "nonexistent-id").expect("delete");
        assert!(!deleted);
    }

    #[test]
    fn upsert_fx_rate_and_get() {
        let conn = open_test_db();
        let rate = FxRate {
            pair: "USDCAD".to_string(),
            rate: 1.36,
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };
        upsert_fx_rate(&conn, &rate).expect("upsert fx");
        // Upsert again with updated rate
        let rate2 = FxRate {
            pair: "USDCAD".to_string(),
            rate: 1.37,
            updated_at: "2024-01-02T00:00:00Z".to_string(),
        };
        upsert_fx_rate(&conn, &rate2).expect("upsert fx 2");
        let rates = get_fx_rates(&conn).expect("get fx rates");
        assert_eq!(rates.len(), 1);
        assert!((rates[0].rate - 1.37).abs() < 0.001);
    }

    #[test]
    fn get_symbol_cache_exact_finds_symbol_case_insensitively() {
        let conn = open_test_db();
        let symbol = SymbolResult {
            symbol: "AAPL".to_string(),
            name: "Apple Inc.".to_string(),
            asset_type: AssetType::Stock,
            exchange: "NMS".to_string(),
            currency: "USD".to_string(),
        };

        upsert_symbol(&conn, &symbol).expect("upsert symbol");

        let cached = get_symbol_cache_exact(&conn, "aapl").expect("query exact");
        assert!(cached.is_some());
        assert_eq!(cached.expect("cached").name, "Apple Inc.");
    }

    #[test]
    fn holding_exists_matches_case_insensitively() {
        let conn = open_test_db();
        insert_holding(&conn, make_input("MSFT")).expect("insert");

        assert!(holding_exists(&conn, "msft").expect("holding exists"));
        assert!(!holding_exists(&conn, "nvda").expect("holding exists"));
    }
}
