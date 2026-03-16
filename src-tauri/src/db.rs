use chrono::Utc;
use rusqlite::{params, Connection};
use std::str::FromStr;
use uuid::Uuid;

use crate::types::{
    Account, AccountType, AlertDirection, AssetType, Dividend, DividendInput, FxRate, Holding,
    HoldingInput, PerformancePoint, PriceAlert, PriceAlertInput, PriceData, SymbolResult,
    Transaction, TransactionInput, TransactionType,
};

fn table_has_column(conn: &Connection, table: &str, column: &str) -> Result<bool, String> {
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({})", table))
        .map_err(|e| e.to_string())?;
    let columns = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|e| e.to_string())?;

    for name in columns {
        if name.map_err(|e| e.to_string())? == column {
            return Ok(true);
        }
    }

    Ok(false)
}

pub fn init_db(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS holdings (
            id          TEXT PRIMARY KEY,
            symbol      TEXT NOT NULL,
            name        TEXT NOT NULL,
            asset_type  TEXT NOT NULL,
            account     TEXT NOT NULL DEFAULT 'taxable',
            quantity    REAL NOT NULL,
            cost_basis  REAL NOT NULL,
            currency    TEXT NOT NULL,
            exchange    TEXT NOT NULL DEFAULT '',
            target_weight REAL NOT NULL DEFAULT 0,
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

        CREATE TABLE IF NOT EXISTS portfolio_snapshots (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            total_value REAL    NOT NULL,
            total_cost  REAL    NOT NULL,
            gain_loss   REAL    NOT NULL,
            recorded_at TEXT    NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_snapshots_recorded_at
            ON portfolio_snapshots(recorded_at);

        CREATE TABLE IF NOT EXISTS price_alerts (
            id          TEXT PRIMARY KEY,
            symbol      TEXT NOT NULL,
            direction   TEXT NOT NULL,
            threshold   REAL NOT NULL,
            note        TEXT NOT NULL DEFAULT '',
            triggered   INTEGER NOT NULL DEFAULT 0,
            created_at  TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS transactions (
            id               TEXT PRIMARY KEY,
            holding_id       TEXT NOT NULL,
            transaction_type TEXT NOT NULL,
            quantity         REAL NOT NULL,
            price            REAL NOT NULL,
            transacted_at    TEXT NOT NULL,
            created_at       TEXT NOT NULL,
            FOREIGN KEY (holding_id) REFERENCES holdings(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_transactions_holding_id
            ON transactions(holding_id);

        CREATE INDEX IF NOT EXISTS idx_transactions_transacted_at
            ON transactions(transacted_at);

        CREATE TABLE IF NOT EXISTS dividends (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            holding_id      TEXT    NOT NULL REFERENCES holdings(id) ON DELETE CASCADE,
            amount_per_unit REAL    NOT NULL,
            currency        TEXT    NOT NULL,
            ex_date         TEXT    NOT NULL,
            pay_date        TEXT    NOT NULL,
            created_at      TEXT    NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_dividends_holding_id
            ON dividends(holding_id);

        CREATE TABLE IF NOT EXISTS accounts (
            id          TEXT PRIMARY KEY,
            name        TEXT NOT NULL,
            type        TEXT NOT NULL DEFAULT 'other'
                        CHECK(type IN ('tfsa','rrsp','fhsa','taxable','crypto','other')),
            institution TEXT,
            created_at  TEXT NOT NULL
        );
        ",
    )
    .map_err(|e| e.to_string())?;

    if !table_has_column(conn, "holdings", "account")? {
        conn.execute(
            "ALTER TABLE holdings ADD COLUMN account TEXT NOT NULL DEFAULT 'taxable'",
            [],
        )
        .map_err(|e| e.to_string())?;
    }

    if !table_has_column(conn, "holdings", "target_weight")? {
        conn.execute(
            "ALTER TABLE holdings ADD COLUMN target_weight REAL NOT NULL DEFAULT 0",
            [],
        )
        .map_err(|e| e.to_string())?;
    }

    if !table_has_column(conn, "holdings", "exchange")? {
        conn.execute(
            "ALTER TABLE holdings ADD COLUMN exchange TEXT NOT NULL DEFAULT ''",
            [],
        )
        .map_err(|e| e.to_string())?;
    }

    conn.execute(
        "UPDATE holdings SET account='cash' WHERE asset_type='cash' AND account='taxable'",
        [],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
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

#[allow(dead_code)]
pub fn set_config(conn: &Connection, key: &str, value: &str) -> Result<(), String> {
    conn.execute(
        "INSERT INTO app_config (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        params![key, value],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn get_all_config(conn: &Connection) -> Result<Vec<(String, String)>, String> {
    let mut stmt = conn
        .prepare("SELECT key, value FROM app_config ORDER BY key")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(rows)
}

pub fn delete_all_alerts(conn: &Connection) -> Result<(), String> {
    conn.execute("DELETE FROM price_alerts", [])
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn delete_all_config(conn: &Connection) -> Result<(), String> {
    conn.execute("DELETE FROM app_config", [])
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Insert a price alert preserving its original ID and triggered state (for restore).
pub fn insert_alert_with_id(conn: &Connection, alert: PriceAlert) -> Result<(), String> {
    conn.execute(
        "INSERT OR REPLACE INTO price_alerts
         (id, symbol, direction, threshold, note, triggered, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            alert.id,
            alert.symbol,
            alert.direction.as_str(),
            alert.threshold,
            alert.note,
            alert.triggered,
            alert.created_at,
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn insert_holding(conn: &Connection, input: HoldingInput) -> Result<Holding, String> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let asset_type_str = input.asset_type.as_str();

    conn.execute(
        "INSERT INTO holdings (id, symbol, name, asset_type, account, quantity, cost_basis, currency, exchange, target_weight, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            id,
            input.symbol,
            input.name,
            asset_type_str,
            input.account.as_str(),
            input.quantity,
            input.cost_basis,
            input.currency,
            input.exchange,
            input.target_weight,
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
        account: input.account,
        quantity: input.quantity,
        cost_basis: input.cost_basis,
        currency: input.currency,
        exchange: input.exchange,
        target_weight: input.target_weight,
        created_at: now.clone(),
        updated_at: now,
    })
}

pub fn update_holding(conn: &Connection, holding: Holding) -> Result<Holding, String> {
    let now = Utc::now().to_rfc3339();
    let asset_type_str = holding.asset_type.as_str();

    let rows = conn
        .execute(
            "UPDATE holdings SET symbol=?1, name=?2, asset_type=?3, account=?4, quantity=?5, cost_basis=?6, currency=?7, exchange=?8, target_weight=?9, updated_at=?10
             WHERE id=?11",
            params![
                holding.symbol,
                holding.name,
                asset_type_str,
                holding.account.as_str(),
                holding.quantity,
                holding.cost_basis,
                holding.currency,
                holding.exchange,
                holding.target_weight,
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

pub fn delete_all_holdings(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute("DELETE FROM holdings", [])?;
    Ok(())
}

pub fn insert_holding_with_id(conn: &Connection, holding: Holding) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT OR REPLACE INTO holdings (id, symbol, name, asset_type, account, quantity, cost_basis, currency, exchange, target_weight, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            holding.id,
            holding.symbol,
            holding.name,
            holding.asset_type.as_str(),
            holding.account.as_str(),
            holding.quantity,
            holding.cost_basis,
            holding.currency,
            holding.exchange,
            holding.target_weight,
            holding.created_at,
            holding.updated_at
        ],
    )?;
    Ok(())
}

pub fn get_all_holdings(conn: &Connection) -> Result<Vec<Holding>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, symbol, name, asset_type, account, quantity, cost_basis, currency, exchange, target_weight, created_at, updated_at
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
                row.get::<_, String>(4)?,
                row.get::<_, f64>(5)?,
                row.get::<_, f64>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, String>(8)?,
                row.get::<_, f64>(9)?,
                row.get::<_, String>(10)?,
                row.get::<_, String>(11)?,
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
                account_str,
                quantity,
                cost_basis,
                currency,
                exchange,
                target_weight,
                created_at,
                updated_at,
            )| {
                let asset_type = AssetType::from_str(&asset_type_str).unwrap_or(AssetType::Stock);
                let account = AccountType::from_str(&account_str).unwrap_or(AccountType::Taxable);
                Holding {
                    id,
                    symbol,
                    name,
                    asset_type,
                    account,
                    quantity,
                    cost_basis,
                    currency,
                    exchange,
                    target_weight,
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

pub fn insert_snapshot(
    conn: &Connection,
    total_value: f64,
    total_cost: f64,
    gain_loss: f64,
) -> Result<(), rusqlite::Error> {
    let recorded_at = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO portfolio_snapshots (total_value, total_cost, gain_loss, recorded_at)
         VALUES (?1, ?2, ?3, ?4)",
        params![total_value, total_cost, gain_loss, recorded_at],
    )?;
    Ok(())
}

pub fn get_snapshots_in_range(
    conn: &Connection,
    start: &str,
    end: &str,
) -> Result<Vec<PerformancePoint>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT recorded_at, total_value
         FROM portfolio_snapshots
         WHERE recorded_at >= ?1 AND recorded_at <= ?2
         ORDER BY recorded_at ASC",
    )?;

    let points = stmt
        .query_map(params![start, end], |row| {
            let recorded_at: String = row.get(0)?;
            let total_value: f64 = row.get(1)?;
            // Truncate ISO timestamp to date portion for the chart
            let date = recorded_at.get(..10).unwrap_or(&recorded_at).to_string();
            Ok(PerformancePoint {
                date,
                value: total_value,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(points)
}

pub fn prune_snapshots(conn: &Connection) -> Result<(), rusqlite::Error> {
    // Keep all snapshots from the last 30 days; beyond that, keep only the latest per day.
    let cutoff = (Utc::now() - chrono::Duration::days(30))
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string();

    // Delete older rows that are not the latest snapshot for their day.
    conn.execute(
        "DELETE FROM portfolio_snapshots
         WHERE recorded_at < ?1
           AND id NOT IN (
               SELECT MAX(id)
               FROM portfolio_snapshots
               WHERE recorded_at < ?1
               GROUP BY DATE(recorded_at)
           )",
        params![cutoff],
    )?;

    Ok(())
}

/// Returns the sum of all `target_weight` values in the holdings table,
/// optionally excluding a specific holding by id (used during updates).
pub fn sum_target_weights(conn: &Connection, exclude_id: Option<&str>) -> Result<f64, String> {
    let sum: f64 = match exclude_id {
        Some(id) => conn
            .query_row(
                "SELECT COALESCE(SUM(target_weight), 0.0) FROM holdings WHERE id != ?1",
                params![id],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?,
        None => conn
            .query_row(
                "SELECT COALESCE(SUM(target_weight), 0.0) FROM holdings",
                [],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?,
    };
    Ok(sum)
}

// ── Price Alerts ──────────────────────────────────────────────────────────────

pub fn insert_alert(conn: &Connection, input: PriceAlertInput) -> Result<PriceAlert, String> {
    let id = Uuid::new_v4().to_string();
    let created_at = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO price_alerts (id, symbol, direction, threshold, note, triggered, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6)",
        params![
            id,
            input.symbol,
            input.direction.as_str(),
            input.threshold,
            input.note,
            created_at
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(PriceAlert {
        id,
        symbol: input.symbol,
        direction: input.direction,
        threshold: input.threshold,
        note: input.note,
        triggered: false,
        created_at,
    })
}

pub fn get_alerts(conn: &Connection) -> Result<Vec<PriceAlert>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, symbol, direction, threshold, note, triggered, created_at
             FROM price_alerts ORDER BY created_at DESC",
        )
        .map_err(|e| e.to_string())?;

    let alerts = stmt
        .query_map([], |row| {
            let direction_str: String = row.get(2)?;
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                direction_str,
                row.get::<_, f64>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, bool>(5)?,
                row.get::<_, String>(6)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .filter_map(
            |(id, symbol, dir_str, threshold, note, triggered, created_at)| {
                let direction = dir_str.parse::<AlertDirection>().ok()?;
                Some(PriceAlert {
                    id,
                    symbol,
                    direction,
                    threshold,
                    note,
                    triggered,
                    created_at,
                })
            },
        )
        .collect();

    Ok(alerts)
}

pub fn delete_alert(conn: &Connection, id: &str) -> Result<bool, String> {
    let n = conn
        .execute("DELETE FROM price_alerts WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(n > 0)
}

/// Mark alerts as triggered for a symbol when threshold is crossed.
/// Returns the IDs of newly-triggered alerts.
pub fn check_and_trigger_alerts(
    conn: &Connection,
    symbol: &str,
    price: f64,
) -> Result<Vec<String>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, direction, threshold FROM price_alerts
             WHERE symbol = ?1 AND triggered = 0",
        )
        .map_err(|e| e.to_string())?;

    let candidates: Vec<(String, String, f64)> = stmt
        .query_map(params![symbol], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let mut triggered = Vec::new();
    for (id, dir_str, threshold) in candidates {
        let crossed = match dir_str.as_str() {
            "above" => price >= threshold,
            "below" => price <= threshold,
            _ => false,
        };
        if crossed {
            conn.execute(
                "UPDATE price_alerts SET triggered = 1 WHERE id = ?1",
                params![id],
            )
            .map_err(|e| e.to_string())?;
            triggered.push(id);
        }
    }

    Ok(triggered)
}

pub fn reset_alert(conn: &Connection, id: &str) -> Result<bool, String> {
    let n = conn
        .execute(
            "UPDATE price_alerts SET triggered = 0 WHERE id = ?1",
            params![id],
        )
        .map_err(|e| e.to_string())?;
    Ok(n > 0)
}

// ── Transactions ──────────────────────────────────────────────────────────────

pub fn insert_transaction(
    conn: &Connection,
    input: TransactionInput,
) -> Result<Transaction, String> {
    let id = Uuid::new_v4().to_string();
    let created_at = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO transactions (id, holding_id, transaction_type, quantity, price, transacted_at, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            id,
            input.holding_id,
            input.transaction_type.as_str(),
            input.quantity,
            input.price,
            input.transacted_at,
            created_at,
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(Transaction {
        id,
        holding_id: input.holding_id,
        transaction_type: input.transaction_type,
        quantity: input.quantity,
        price: input.price,
        transacted_at: input.transacted_at,
        created_at,
    })
}

pub fn get_transactions_for_holding(
    conn: &Connection,
    holding_id: &str,
) -> Result<Vec<Transaction>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, holding_id, transaction_type, quantity, price, transacted_at, created_at
             FROM transactions WHERE holding_id = ?1 ORDER BY transacted_at ASC",
        )
        .map_err(|e| e.to_string())?;

    let mut rows = stmt.query(params![holding_id]).map_err(|e| e.to_string())?;
    let mut result = Vec::new();
    while let Some(row) = rows.next().map_err(|e| e.to_string())? {
        result.push(row_to_transaction(row)?);
    }
    Ok(result)
}

pub fn get_all_transactions(conn: &Connection) -> Result<Vec<Transaction>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, holding_id, transaction_type, quantity, price, transacted_at, created_at
             FROM transactions ORDER BY transacted_at ASC",
        )
        .map_err(|e| e.to_string())?;

    let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
    let mut result = Vec::new();
    while let Some(row) = rows.next().map_err(|e| e.to_string())? {
        result.push(row_to_transaction(row)?);
    }
    Ok(result)
}

fn row_to_transaction(row: &rusqlite::Row<'_>) -> Result<Transaction, String> {
    let type_str: String = row.get(2).map_err(|e| e.to_string())?;
    let transaction_type = type_str.parse::<TransactionType>()?;
    Ok(Transaction {
        id: row.get(0).map_err(|e| e.to_string())?,
        holding_id: row.get(1).map_err(|e| e.to_string())?,
        transaction_type,
        quantity: row.get(3).map_err(|e| e.to_string())?,
        price: row.get(4).map_err(|e| e.to_string())?,
        transacted_at: row.get(5).map_err(|e| e.to_string())?,
        created_at: row.get(6).map_err(|e| e.to_string())?,
    })
}

pub fn delete_transaction(conn: &Connection, id: &str) -> Result<bool, String> {
    let n = conn
        .execute("DELETE FROM transactions WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(n > 0)
}

// ── Dividends ─────────────────────────────────────────────────────────────────

pub fn insert_dividend(
    conn: &Connection,
    input: DividendInput,
    symbol: &str,
) -> Result<Dividend, String> {
    let created_at = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO dividends (holding_id, amount_per_unit, currency, ex_date, pay_date, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            input.holding_id,
            input.amount_per_unit,
            input.currency,
            input.ex_date,
            input.pay_date,
            created_at
        ],
    )
    .map_err(|e| e.to_string())?;

    let id = conn.last_insert_rowid();
    Ok(Dividend {
        id,
        holding_id: input.holding_id,
        symbol: symbol.to_string(),
        amount_per_unit: input.amount_per_unit,
        currency: input.currency,
        ex_date: input.ex_date,
        pay_date: input.pay_date,
        created_at,
    })
}

pub fn get_dividends(conn: &Connection) -> Result<Vec<Dividend>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT d.id, d.holding_id, h.symbol, d.amount_per_unit, d.currency,
                    d.ex_date, d.pay_date, d.created_at
             FROM dividends d
             JOIN holdings h ON h.id = d.holding_id
             ORDER BY d.ex_date DESC",
        )
        .map_err(|e| e.to_string())?;

    let dividends = stmt
        .query_map([], |row| {
            Ok(Dividend {
                id: row.get(0)?,
                holding_id: row.get(1)?,
                symbol: row.get(2)?,
                amount_per_unit: row.get(3)?,
                currency: row.get(4)?,
                ex_date: row.get(5)?,
                pay_date: row.get(6)?,
                created_at: row.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(dividends)
}

pub fn delete_dividend(conn: &Connection, id: i64) -> Result<bool, String> {
    let n = conn
        .execute("DELETE FROM dividends WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(n > 0)
}

#[allow(dead_code)]
pub fn holding_exists(conn: &Connection, symbol: &str) -> Result<bool, String> {
    let mut stmt = conn
        .prepare("SELECT 1 FROM holdings WHERE UPPER(symbol) = UPPER(?1) LIMIT 1")
        .map_err(|e| e.to_string())?;

    let mut rows = stmt.query(params![symbol]).map_err(|e| e.to_string())?;
    Ok(rows.next().map_err(|e| e.to_string())?.is_some())
}

// ── Accounts ──────────────────────────────────────────────────────────────────

pub fn insert_account(
    conn: &Connection,
    id: &str,
    name: &str,
    account_type: &str,
    institution: Option<&str>,
) -> Result<(), rusqlite::Error> {
    let created_at = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO accounts (id, name, type, institution, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![id, name, account_type, institution, created_at],
    )?;
    Ok(())
}

pub fn get_accounts(conn: &Connection) -> Result<Vec<Account>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, name, type, institution, created_at FROM accounts ORDER BY created_at ASC",
    )?;

    let accounts = stmt
        .query_map([], |row| {
            Ok(Account {
                id: row.get(0)?,
                name: row.get(1)?,
                account_type: row.get(2)?,
                institution: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(accounts)
}

pub fn update_account(
    conn: &Connection,
    id: &str,
    name: &str,
    account_type: &str,
    institution: Option<&str>,
) -> Result<(), rusqlite::Error> {
    let rows = conn.execute(
        "UPDATE accounts SET name=?1, type=?2, institution=?3 WHERE id=?4",
        params![name, account_type, institution, id],
    )?;
    if rows == 0 {
        return Err(rusqlite::Error::QueryReturnedNoRows);
    }
    Ok(())
}

/// Delete an account by id. Returns an error if any holding references this account's name.
pub fn delete_account(conn: &Connection, id: &str) -> Result<(), rusqlite::Error> {
    // Look up the account name first
    let name: String = conn.query_row(
        "SELECT name FROM accounts WHERE id=?1",
        params![id],
        |row| row.get(0),
    )?;

    // Guard: refuse deletion when holdings reference this account name
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM holdings WHERE account=?1",
        params![name],
        |row| row.get(0),
    )?;

    if count > 0 {
        return Err(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error {
                code: rusqlite::ffi::ErrorCode::ConstraintViolation,
                extended_code: 0,
            },
            Some(format!(
                "Cannot delete account '{}': {} holding(s) still reference it",
                name, count
            )),
        ));
    }

    conn.execute("DELETE FROM accounts WHERE id=?1", params![id])?;
    Ok(())
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
            account: AccountType::Taxable,
            quantity: 10.0,
            cost_basis: 100.0,
            currency: "CAD".to_string(),
            exchange: String::new(),
            target_weight: 0.0,
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
            target_weight: 12.5,
            ..inserted
        };
        let updated = update_holding(&conn, updated_holding).expect("update");
        assert!((updated.quantity - 20.0).abs() < 0.001);
        assert!((updated.cost_basis - 150.0).abs() < 0.001);
        assert!((updated.target_weight - 12.5).abs() < 0.001);
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

    #[test]
    fn insert_snapshot_and_retrieve_in_range() {
        let conn = open_test_db();

        // Insert two snapshots
        insert_snapshot(&conn, 100_000.0, 90_000.0, 10_000.0).expect("insert snapshot 1");
        insert_snapshot(&conn, 110_000.0, 90_000.0, 20_000.0).expect("insert snapshot 2");

        let start = "1970-01-01T00:00:00+00:00";
        let end = "2099-12-31T23:59:59+00:00";
        let points = get_snapshots_in_range(&conn, start, end).expect("get snapshots");

        assert_eq!(points.len(), 2);
        assert!((points[0].value - 100_000.0).abs() < 0.001);
        assert!((points[1].value - 110_000.0).abs() < 0.001);
    }

    #[test]
    fn get_snapshots_in_range_respects_date_bounds() {
        let conn = open_test_db();

        // Insert a snapshot with a known timestamp in the past
        conn.execute(
            "INSERT INTO portfolio_snapshots (total_value, total_cost, gain_loss, recorded_at)
             VALUES (50000.0, 45000.0, 5000.0, '2020-01-15T12:00:00+00:00')",
            [],
        )
        .expect("manual insert");

        // Query a range that excludes that date
        let points = get_snapshots_in_range(
            &conn,
            "2021-01-01T00:00:00+00:00",
            "2099-12-31T23:59:59+00:00",
        )
        .expect("get snapshots");

        assert_eq!(points.len(), 0);

        // Query a range that includes it
        let points = get_snapshots_in_range(
            &conn,
            "2020-01-01T00:00:00+00:00",
            "2020-12-31T23:59:59+00:00",
        )
        .expect("get snapshots");

        assert_eq!(points.len(), 1);
        assert!((points[0].value - 50_000.0).abs() < 0.001);
        assert_eq!(points[0].date, "2020-01-15");
    }

    #[test]
    fn prune_snapshots_keeps_recent_and_daily_max_for_old() {
        let conn = open_test_db();

        // Insert 3 snapshots on the same old date (> 30 days ago) — only the highest id should survive
        conn.execute(
            "INSERT INTO portfolio_snapshots (total_value, total_cost, gain_loss, recorded_at)
             VALUES (1000.0, 900.0, 100.0, '2020-06-01T08:00:00+00:00')",
            [],
        )
        .expect("insert old 1");
        conn.execute(
            "INSERT INTO portfolio_snapshots (total_value, total_cost, gain_loss, recorded_at)
             VALUES (1050.0, 900.0, 150.0, '2020-06-01T12:00:00+00:00')",
            [],
        )
        .expect("insert old 2");
        conn.execute(
            "INSERT INTO portfolio_snapshots (total_value, total_cost, gain_loss, recorded_at)
             VALUES (1100.0, 900.0, 200.0, '2020-06-01T18:00:00+00:00')",
            [],
        )
        .expect("insert old 3");

        // Insert a recent snapshot (today) — must NOT be pruned
        insert_snapshot(&conn, 200_000.0, 180_000.0, 20_000.0).expect("insert recent");

        prune_snapshots(&conn).expect("prune");

        let all = get_snapshots_in_range(
            &conn,
            "1970-01-01T00:00:00+00:00",
            "2099-12-31T23:59:59+00:00",
        )
        .expect("get all");

        // 1 old (latest of that day) + 1 recent = 2
        assert_eq!(all.len(), 2);

        // The surviving old snapshot should be the last inserted (1100.0)
        let old_point = all.iter().find(|p| p.date == "2020-06-01");
        assert!(old_point.is_some());
        assert!((old_point.unwrap().value - 1100.0).abs() < 0.001);
    }

    #[test]
    fn sum_target_weights_returns_zero_for_empty_table() {
        let conn = open_test_db();
        let sum = sum_target_weights(&conn, None).expect("sum");
        assert!((sum - 0.0).abs() < 0.001);
    }

    #[test]
    fn sum_target_weights_sums_all_holdings() {
        let conn = open_test_db();
        let mut input_a = make_input("AAPL");
        input_a.target_weight = 40.0;
        let mut input_b = make_input("MSFT");
        input_b.target_weight = 35.0;
        insert_holding(&conn, input_a).expect("insert a");
        insert_holding(&conn, input_b).expect("insert b");
        let sum = sum_target_weights(&conn, None).expect("sum");
        assert!((sum - 75.0).abs() < 0.001);
    }

    #[test]
    fn sum_target_weights_excludes_specified_id() {
        let conn = open_test_db();
        let mut input_a = make_input("AAPL");
        input_a.target_weight = 40.0;
        let mut input_b = make_input("MSFT");
        input_b.target_weight = 35.0;
        let holding_a = insert_holding(&conn, input_a).expect("insert a");
        insert_holding(&conn, input_b).expect("insert b");
        let sum = sum_target_weights(&conn, Some(&holding_a.id)).expect("sum excluding a");
        assert!((sum - 35.0).abs() < 0.001);
    }

    #[test]
    fn exchange_field_round_trips_through_insert_and_get() {
        let conn = open_test_db();
        let input = HoldingInput {
            exchange: "NYSE".to_string(),
            ..make_input("AAPL")
        };
        insert_holding(&conn, input).expect("insert");
        let holdings = get_all_holdings(&conn).expect("get all");
        assert_eq!(holdings.len(), 1);
        assert_eq!(holdings[0].exchange, "NYSE");
    }

    // ── Config persistence ────────────────────────────────────────────────────

    #[test]
    fn set_and_get_config_round_trips_value() {
        let conn = open_test_db();
        set_config(&conn, "base_currency", "USD").expect("set config");
        let val = get_config(&conn, "base_currency").expect("get config");
        assert_eq!(val, Some("USD".to_string()));
    }

    #[test]
    fn get_config_returns_none_for_missing_key() {
        let conn = open_test_db();
        let val = get_config(&conn, "nonexistent_key").expect("get config");
        assert_eq!(val, None);
    }

    #[test]
    fn set_config_upserts_existing_key() {
        let conn = open_test_db();
        set_config(&conn, "theme", "dark").expect("initial set");
        set_config(&conn, "theme", "light").expect("update set");
        let val = get_config(&conn, "theme").expect("get config");
        assert_eq!(val, Some("light".to_string()));
    }

    #[test]
    fn set_config_stores_multiple_independent_keys() {
        let conn = open_test_db();
        set_config(&conn, "base_currency", "CAD").expect("set base_currency");
        set_config(&conn, "theme", "dark").expect("set theme");
        assert_eq!(
            get_config(&conn, "base_currency").expect("get"),
            Some("CAD".to_string())
        );
        assert_eq!(
            get_config(&conn, "theme").expect("get"),
            Some("dark".to_string())
        );
    }

    #[test]
    fn set_config_persists_empty_string_value() {
        let conn = open_test_db();
        set_config(&conn, "greeting", "").expect("set empty");
        let val = get_config(&conn, "greeting").expect("get config");
        assert_eq!(val, Some(String::new()));
    }

    // ── Transaction tests ────────────────────────────────────────────────────

    #[test]
    fn insert_and_get_transactions_for_holding() {
        let conn = open_test_db();
        let holding = insert_holding(&conn, make_input("AAPL")).expect("insert holding");

        let tx = insert_transaction(
            &conn,
            TransactionInput {
                holding_id: holding.id.clone(),
                transaction_type: TransactionType::Buy,
                quantity: 10.0,
                price: 150.0,
                transacted_at: "2024-01-10T10:00:00Z".to_string(),
            },
        )
        .expect("insert tx");
        assert!(!tx.id.is_empty());

        let txs = get_transactions_for_holding(&conn, &holding.id).expect("get txs");
        assert_eq!(txs.len(), 1);
        assert_eq!(txs[0].transaction_type, TransactionType::Buy);
        assert!((txs[0].quantity - 10.0).abs() < 0.001);
        assert!((txs[0].price - 150.0).abs() < 0.001);
    }

    #[test]
    fn get_transactions_ordered_by_transacted_at_desc() {
        let conn = open_test_db();
        let holding = insert_holding(&conn, make_input("MSFT")).expect("insert holding");

        insert_transaction(
            &conn,
            TransactionInput {
                holding_id: holding.id.clone(),
                transaction_type: TransactionType::Buy,
                quantity: 5.0,
                price: 100.0,
                transacted_at: "2024-01-01T09:00:00Z".to_string(),
            },
        )
        .expect("insert tx1");
        insert_transaction(
            &conn,
            TransactionInput {
                holding_id: holding.id.clone(),
                transaction_type: TransactionType::Sell,
                quantity: 2.0,
                price: 120.0,
                transacted_at: "2024-03-01T09:00:00Z".to_string(),
            },
        )
        .expect("insert tx2");

        let txs = get_transactions_for_holding(&conn, &holding.id).expect("get txs");
        assert_eq!(txs.len(), 2);
        // Oldest first (ASC order, needed for FIFO/AVCO calculations)
        assert_eq!(txs[0].transaction_type, TransactionType::Buy);
        assert_eq!(txs[1].transaction_type, TransactionType::Sell);
    }

    #[test]
    fn get_all_transactions_returns_all() {
        let conn = open_test_db();
        let h1 = insert_holding(&conn, make_input("AAPL")).expect("insert h1");
        let h2 = insert_holding(&conn, make_input("GOOG")).expect("insert h2");

        insert_transaction(
            &conn,
            TransactionInput {
                holding_id: h1.id.clone(),
                transaction_type: TransactionType::Buy,
                quantity: 10.0,
                price: 100.0,
                transacted_at: "2024-01-01T00:00:00Z".to_string(),
            },
        )
        .expect("tx1");
        insert_transaction(
            &conn,
            TransactionInput {
                holding_id: h2.id.clone(),
                transaction_type: TransactionType::Buy,
                quantity: 500.0,
                price: 1.0,
                transacted_at: "2024-02-01T00:00:00Z".to_string(),
            },
        )
        .expect("tx2");

        let all = get_all_transactions(&conn).expect("get all txs");
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn delete_transaction_removes_row() {
        let conn = open_test_db();
        let holding = insert_holding(&conn, make_input("TSLA")).expect("insert");

        let tx = insert_transaction(
            &conn,
            TransactionInput {
                holding_id: holding.id.clone(),
                transaction_type: TransactionType::Buy,
                quantity: 1.0,
                price: 200.0,
                transacted_at: "2024-01-01T00:00:00Z".to_string(),
            },
        )
        .expect("insert tx");

        delete_transaction(&conn, &tx.id).expect("delete tx");
        let txs = get_transactions_for_holding(&conn, &holding.id).expect("get txs");
        assert_eq!(txs.len(), 0);
    }

    #[test]
    fn transactions_cascade_on_holding_delete() {
        let conn = open_test_db();
        // Enable cascading FK constraints (required in SQLite)
        conn.execute_batch("PRAGMA foreign_keys = ON")
            .expect("pragma");
        let holding = insert_holding(&conn, make_input("NVDA")).expect("insert");

        insert_transaction(
            &conn,
            TransactionInput {
                holding_id: holding.id.clone(),
                transaction_type: TransactionType::Buy,
                quantity: 5.0,
                price: 300.0,
                transacted_at: "2024-01-01T00:00:00Z".to_string(),
            },
        )
        .expect("insert tx");

        delete_holding(&conn, &holding.id).expect("delete holding");
        let txs = get_transactions_for_holding(&conn, &holding.id).expect("get txs");
        assert_eq!(txs.len(), 0);
    }

    // ── Account CRUD ─────────────────────────────────────────────────────────

    #[test]
    fn insert_and_get_accounts() {
        let conn = open_test_db();
        insert_account(&conn, "acc-1", "My TFSA", "tfsa", Some("Questrade")).expect("insert");
        insert_account(&conn, "acc-2", "RRSP", "rrsp", None).expect("insert");
        let accounts = get_accounts(&conn).expect("get accounts");
        assert_eq!(accounts.len(), 2);
        let names: Vec<&str> = accounts.iter().map(|a| a.name.as_str()).collect();
        assert!(names.contains(&"My TFSA"));
        assert!(names.contains(&"RRSP"));
        let tfsa = accounts.iter().find(|a| a.id == "acc-1").unwrap();
        assert_eq!(tfsa.institution, Some("Questrade".to_string()));
        assert_eq!(tfsa.account_type, "tfsa");
    }

    #[test]
    fn update_account_changes_fields() {
        let conn = open_test_db();
        insert_account(&conn, "acc-1", "Old Name", "taxable", None).expect("insert");
        update_account(&conn, "acc-1", "New Name", "rrsp", Some("TD")).expect("update");
        let accounts = get_accounts(&conn).expect("get accounts");
        let acct = accounts.iter().find(|a| a.id == "acc-1").unwrap();
        assert_eq!(acct.name, "New Name");
        assert_eq!(acct.account_type, "rrsp");
        assert_eq!(acct.institution, Some("TD".to_string()));
    }

    #[test]
    fn delete_account_succeeds_when_no_holdings() {
        let conn = open_test_db();
        insert_account(&conn, "acc-1", "Empty Account", "tfsa", None).expect("insert");
        delete_account(&conn, "acc-1").expect("delete should succeed");
        let accounts = get_accounts(&conn).expect("get accounts");
        assert_eq!(accounts.len(), 0);
    }

    #[test]
    fn delete_account_fails_when_holdings_reference_it() {
        let conn = open_test_db();
        // Insert an account named "taxable" — this matches make_input's AccountType::Taxable
        insert_account(&conn, "acc-1", "taxable", "taxable", None).expect("insert account");
        // Insert a holding that references account "taxable" (the default in make_input)
        let input = make_input("AAPL");
        insert_holding(&conn, input).expect("insert holding");
        // Attempt deletion should fail
        let result = delete_account(&conn, "acc-1");
        assert!(
            result.is_err(),
            "delete should fail with referenced holdings"
        );
    }

    #[test]
    fn update_account_returns_error_for_nonexistent_id() {
        let conn = open_test_db();
        let result = update_account(&conn, "nonexistent", "Name", "tfsa", None);
        assert!(result.is_err());
    }
}
