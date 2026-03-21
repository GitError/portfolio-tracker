use chrono::Utc;
use sqlx::SqlitePool;
use std::str::FromStr;
use uuid::Uuid;

use crate::types::{
    Account, AccountType, AlertDirection, AssetType, Dividend, DividendInput, FxRate, Holding,
    HoldingInput, PerformancePoint, PriceAlert, PriceAlertInput, PriceData, SymbolResult,
    Transaction, TransactionInput, TransactionType,
};

// ── Config ────────────────────────────────────────────────────────────────────

pub async fn get_config(pool: &SqlitePool, key: &str) -> Result<Option<String>, String> {
    let row = sqlx::query("SELECT value FROM app_config WHERE key = $1")
        .bind(key)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(row.map(|r| {
        use sqlx::Row;
        r.get::<String, _>(0)
    }))
}

#[allow(dead_code)]
pub async fn set_config(pool: &SqlitePool, key: &str, value: &str) -> Result<(), String> {
    sqlx::query(
        "INSERT INTO app_config (key, value) VALUES ($1, $2)
         ON CONFLICT(key) DO UPDATE SET value=excluded.value",
    )
    .bind(key)
    .bind(value)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn get_all_config(pool: &SqlitePool) -> Result<Vec<(String, String)>, String> {
    use sqlx::Row;
    let rows = sqlx::query("SELECT key, value FROM app_config ORDER BY key")
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(rows
        .into_iter()
        .map(|r| (r.get::<String, _>(0), r.get::<String, _>(1)))
        .collect())
}

pub async fn delete_all_alerts(pool: &SqlitePool) -> Result<(), String> {
    sqlx::query("DELETE FROM price_alerts")
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn delete_all_config(pool: &SqlitePool) -> Result<(), String> {
    sqlx::query("DELETE FROM app_config")
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ── Alerts (restore) ──────────────────────────────────────────────────────────

/// Insert a price alert preserving its original ID and triggered state (for restore).
pub async fn insert_alert_with_id(pool: &SqlitePool, alert: PriceAlert) -> Result<(), String> {
    sqlx::query(
        "INSERT OR REPLACE INTO price_alerts
         (id, symbol, direction, threshold, note, triggered, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(&alert.id)
    .bind(&alert.symbol)
    .bind(alert.direction.as_str())
    .bind(alert.threshold)
    .bind(&alert.note)
    .bind(alert.triggered)
    .bind(&alert.created_at)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

// ── Holdings ──────────────────────────────────────────────────────────────────

pub async fn insert_holding(pool: &SqlitePool, input: HoldingInput) -> Result<Holding, String> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let asset_type_str = input.asset_type.as_str().to_string();

    let effective_account_id: Option<String> = if let Some(account_id) = input.account_id.clone() {
        Some(account_id)
    } else {
        use sqlx::Row;
        sqlx::query("SELECT id FROM accounts WHERE type = $1 ORDER BY created_at ASC LIMIT 1")
            .bind(input.account.as_str())
            .fetch_optional(pool)
            .await
            .ok()
            .flatten()
            .map(|r| r.get::<String, _>(0))
    };

    sqlx::query(
        "INSERT INTO holdings
         (id, symbol, name, asset_type, account, account_id, quantity, cost_basis, currency, exchange, target_weight, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
    )
    .bind(&id)
    .bind(&input.symbol)
    .bind(&input.name)
    .bind(&asset_type_str)
    .bind(input.account.as_str())
    .bind(&effective_account_id)
    .bind(input.quantity)
    .bind(input.cost_basis)
    .bind(&input.currency)
    .bind(&input.exchange)
    .bind(input.target_weight)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(Holding {
        id,
        symbol: input.symbol,
        name: input.name,
        asset_type: input.asset_type,
        account: input.account,
        account_id: effective_account_id,
        account_name: None,
        quantity: input.quantity,
        cost_basis: input.cost_basis,
        currency: input.currency,
        exchange: input.exchange,
        target_weight: input.target_weight,
        created_at: now.clone(),
        updated_at: now,
    })
}

pub async fn update_holding(pool: &SqlitePool, holding: Holding) -> Result<Holding, String> {
    let now = Utc::now().to_rfc3339();
    let asset_type_str = holding.asset_type.as_str().to_string();

    let effective_account_id: Option<String> = if let Some(account_id) = holding.account_id.clone()
    {
        Some(account_id)
    } else {
        use sqlx::Row;
        sqlx::query("SELECT id FROM accounts WHERE type = $1 ORDER BY created_at ASC LIMIT 1")
            .bind(holding.account.as_str())
            .fetch_optional(pool)
            .await
            .ok()
            .flatten()
            .map(|r| r.get::<String, _>(0))
    };

    let result = sqlx::query(
        "UPDATE holdings SET
             symbol=$1,
             name=$2,
             asset_type=$3,
             account=$4,
             account_id=$5,
             quantity=$6,
             cost_basis=$7,
             currency=$8,
             exchange=$9,
             target_weight=$10,
             updated_at=$11
         WHERE id=$12",
    )
    .bind(&holding.symbol)
    .bind(&holding.name)
    .bind(&asset_type_str)
    .bind(holding.account.as_str())
    .bind(&effective_account_id)
    .bind(holding.quantity)
    .bind(holding.cost_basis)
    .bind(&holding.currency)
    .bind(&holding.exchange)
    .bind(holding.target_weight)
    .bind(&now)
    .bind(&holding.id)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    if result.rows_affected() == 0 {
        return Err(format!("Holding {} not found", holding.id));
    }

    Ok(Holding {
        updated_at: now,
        account_id: effective_account_id,
        ..holding
    })
}

pub async fn delete_holding(pool: &SqlitePool, id: &str) -> Result<bool, String> {
    let result = sqlx::query("DELETE FROM holdings WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(result.rows_affected() > 0)
}

pub async fn delete_all_holdings(pool: &SqlitePool) -> Result<(), String> {
    sqlx::query("DELETE FROM holdings")
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn insert_holding_with_id(pool: &SqlitePool, holding: Holding) -> Result<(), String> {
    sqlx::query(
        "INSERT OR REPLACE INTO holdings
         (id, symbol, name, asset_type, account, account_id, quantity, cost_basis, currency, exchange, target_weight, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
    )
    .bind(&holding.id)
    .bind(&holding.symbol)
    .bind(&holding.name)
    .bind(holding.asset_type.as_str())
    .bind(holding.account.as_str())
    .bind(&holding.account_id)
    .bind(holding.quantity)
    .bind(holding.cost_basis)
    .bind(&holding.currency)
    .bind(&holding.exchange)
    .bind(holding.target_weight)
    .bind(&holding.created_at)
    .bind(&holding.updated_at)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn get_all_holdings(pool: &SqlitePool) -> Result<Vec<Holding>, String> {
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT
            h.id,
            h.symbol,
            h.name,
            h.asset_type,
            h.account,
            h.account_id,
            a.name AS account_name,
            h.quantity,
            h.cost_basis,
            h.currency,
            h.exchange,
            h.target_weight,
            h.created_at,
            h.updated_at
         FROM holdings h
         LEFT JOIN accounts a ON a.id = h.account_id
         ORDER BY h.created_at ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let holdings = rows
        .into_iter()
        .map(|r| {
            let asset_type_str: String = r.get(3);
            let account_str: String = r.get(4);
            let asset_type = AssetType::from_str(&asset_type_str).unwrap_or(AssetType::Stock);
            let account = AccountType::from_str(&account_str).unwrap_or(AccountType::Taxable);
            Holding {
                id: r.get(0),
                symbol: r.get(1),
                name: r.get(2),
                asset_type,
                account,
                account_id: r.get(5),
                account_name: r.get(6),
                quantity: r.get(7),
                cost_basis: r.get(8),
                currency: r.get(9),
                exchange: r.get(10),
                target_weight: r.get(11),
                created_at: r.get(12),
                updated_at: r.get(13),
            }
        })
        .collect();

    Ok(holdings)
}

// ── Price cache ───────────────────────────────────────────────────────────────

pub async fn upsert_price(pool: &SqlitePool, price: &PriceData) -> Result<(), String> {
    sqlx::query(
        "INSERT INTO price_cache (symbol, price, currency, change, change_percent, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6)
         ON CONFLICT(symbol) DO UPDATE SET
           price=excluded.price,
           currency=excluded.currency,
           change=excluded.change,
           change_percent=excluded.change_percent,
           updated_at=excluded.updated_at",
    )
    .bind(&price.symbol)
    .bind(price.price)
    .bind(&price.currency)
    .bind(price.change)
    .bind(price.change_percent)
    .bind(&price.updated_at)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn get_cached_prices(pool: &SqlitePool) -> Result<Vec<PriceData>, String> {
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT symbol, price, currency, change, change_percent, updated_at FROM price_cache",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(rows
        .into_iter()
        .map(|r| PriceData {
            symbol: r.get(0),
            price: r.get(1),
            currency: r.get(2),
            change: r.get(3),
            change_percent: r.get(4),
            updated_at: r.get(5),
        })
        .collect())
}

// ── FX rates ──────────────────────────────────────────────────────────────────

pub async fn upsert_fx_rate(pool: &SqlitePool, rate: &FxRate) -> Result<(), String> {
    sqlx::query(
        "INSERT INTO fx_rates (pair, rate, updated_at) VALUES ($1, $2, $3)
         ON CONFLICT(pair) DO UPDATE SET rate=excluded.rate, updated_at=excluded.updated_at",
    )
    .bind(&rate.pair)
    .bind(rate.rate)
    .bind(&rate.updated_at)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn get_fx_rates(pool: &SqlitePool) -> Result<Vec<FxRate>, String> {
    use sqlx::Row;
    let rows = sqlx::query("SELECT pair, rate, updated_at FROM fx_rates")
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(rows
        .into_iter()
        .map(|r| FxRate {
            pair: r.get(0),
            rate: r.get(1),
            updated_at: r.get(2),
        })
        .collect())
}

// ── Symbol cache ──────────────────────────────────────────────────────────────

pub async fn upsert_symbol(pool: &SqlitePool, result: &SymbolResult) -> Result<(), String> {
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO symbol_cache (symbol, name, asset_type, exchange, currency, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6)
         ON CONFLICT(symbol) DO UPDATE SET
           name=excluded.name,
           asset_type=excluded.asset_type,
           exchange=excluded.exchange,
           currency=excluded.currency,
           updated_at=excluded.updated_at",
    )
    .bind(&result.symbol)
    .bind(&result.name)
    .bind(result.asset_type.as_str())
    .bind(&result.exchange)
    .bind(&result.currency)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn search_symbol_cache(
    pool: &SqlitePool,
    query: &str,
) -> Result<Vec<SymbolResult>, String> {
    use sqlx::Row;
    let pattern = format!("%{}%", query.to_lowercase());
    let sym_prefix = format!("{}%", query.to_uppercase());

    let rows = sqlx::query(
        "SELECT symbol, name, asset_type, exchange, currency FROM symbol_cache
         WHERE symbol LIKE $1 OR LOWER(name) LIKE $2
         ORDER BY CASE WHEN symbol LIKE $1 THEN 0 ELSE 1 END
         LIMIT 8",
    )
    .bind(&sym_prefix)
    .bind(&pattern)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(rows
        .into_iter()
        .map(|r| {
            let asset_type_str: String = r.get(2);
            let asset_type = AssetType::from_str(&asset_type_str).unwrap_or(AssetType::Stock);
            SymbolResult {
                symbol: r.get(0),
                name: r.get(1),
                asset_type,
                exchange: r.get(3),
                currency: r.get(4),
            }
        })
        .collect())
}

pub async fn get_symbol_cache_exact(
    pool: &SqlitePool,
    symbol: &str,
) -> Result<Option<SymbolResult>, String> {
    use sqlx::Row;
    let row = sqlx::query(
        "SELECT symbol, name, asset_type, exchange, currency
         FROM symbol_cache
         WHERE UPPER(symbol) = UPPER($1)
         LIMIT 1",
    )
    .bind(symbol)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(row.map(|r| {
        let asset_type_str: String = r.get(2);
        let asset_type = AssetType::from_str(&asset_type_str).unwrap_or(AssetType::Stock);
        SymbolResult {
            symbol: r.get(0),
            name: r.get(1),
            asset_type,
            exchange: r.get(3),
            currency: r.get(4),
        }
    }))
}

// ── Portfolio snapshots ───────────────────────────────────────────────────────

pub async fn insert_snapshot(
    pool: &SqlitePool,
    total_value: f64,
    total_cost: f64,
    gain_loss: f64,
) -> Result<(), String> {
    let recorded_at = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO portfolio_snapshots (total_value, total_cost, gain_loss, recorded_at)
         VALUES ($1, $2, $3, $4)",
    )
    .bind(total_value)
    .bind(total_cost)
    .bind(gain_loss)
    .bind(&recorded_at)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn get_snapshots_in_range(
    pool: &SqlitePool,
    start: &str,
    end: &str,
) -> Result<Vec<PerformancePoint>, String> {
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT recorded_at, total_value
         FROM portfolio_snapshots
         WHERE recorded_at >= $1 AND recorded_at <= $2
         ORDER BY recorded_at ASC",
    )
    .bind(start)
    .bind(end)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(rows
        .into_iter()
        .map(|r| {
            let recorded_at: String = r.get(0);
            let total_value: f64 = r.get(1);
            let date = recorded_at.get(..10).unwrap_or(&recorded_at).to_string();
            PerformancePoint {
                date,
                value: total_value,
            }
        })
        .collect())
}

pub async fn prune_snapshots(pool: &SqlitePool) -> Result<(), String> {
    // Step 1: deduplicate — keep only the latest snapshot per calendar day.
    sqlx::query(
        "DELETE FROM portfolio_snapshots
         WHERE id NOT IN (
             SELECT MAX(id)
             FROM portfolio_snapshots
             GROUP BY DATE(recorded_at)
         )",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    // Step 2: retain the 730 most-recent distinct days (≈ 2 years).
    // Any day older than the 730th-most-recent is removed entirely.
    sqlx::query(
        "DELETE FROM portfolio_snapshots
         WHERE DATE(recorded_at) < (
             SELECT DATE(recorded_at)
             FROM (
                 SELECT DISTINCT DATE(recorded_at) AS recorded_at
                 FROM portfolio_snapshots
                 ORDER BY recorded_at DESC
                 LIMIT 730
             )
             ORDER BY recorded_at ASC
             LIMIT 1
         )",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Returns the sum of all `target_weight` values in the holdings table,
/// optionally excluding a specific holding by id (used during updates).
pub async fn sum_target_weights(
    pool: &SqlitePool,
    exclude_id: Option<&str>,
) -> Result<f64, String> {
    use sqlx::Row;
    let sum: f64 = match exclude_id {
        Some(id) => {
            sqlx::query("SELECT COALESCE(SUM(target_weight), 0.0) FROM holdings WHERE id != $1")
                .bind(id)
                .fetch_one(pool)
                .await
                .map_err(|e| e.to_string())
                .map(|r| r.get::<f64, _>(0))?
        }

        None => sqlx::query("SELECT COALESCE(SUM(target_weight), 0.0) FROM holdings")
            .fetch_one(pool)
            .await
            .map_err(|e| e.to_string())
            .map(|r| r.get::<f64, _>(0))?,
    };
    Ok(sum)
}

// ── Price Alerts ──────────────────────────────────────────────────────────────

pub async fn insert_alert(pool: &SqlitePool, input: PriceAlertInput) -> Result<PriceAlert, String> {
    let id = Uuid::new_v4().to_string();
    let created_at = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO price_alerts (id, symbol, direction, threshold, note, triggered, created_at)
         VALUES ($1, $2, $3, $4, $5, 0, $6)",
    )
    .bind(&id)
    .bind(&input.symbol)
    .bind(input.direction.as_str())
    .bind(input.threshold)
    .bind(&input.note)
    .bind(&created_at)
    .execute(pool)
    .await
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

pub async fn get_alerts(pool: &SqlitePool) -> Result<Vec<PriceAlert>, String> {
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT id, symbol, direction, threshold, note, triggered, created_at
         FROM price_alerts ORDER BY created_at DESC",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let alerts = rows
        .into_iter()
        .filter_map(|r| {
            let dir_str: String = r.get(2);
            let direction = dir_str.parse::<AlertDirection>().ok()?;
            let triggered: bool = r.get(5);
            Some(PriceAlert {
                id: r.get(0),
                symbol: r.get(1),
                direction,
                threshold: r.get(3),
                note: r.get(4),
                triggered,
                created_at: r.get(6),
            })
        })
        .collect();

    Ok(alerts)
}

pub async fn delete_alert(pool: &SqlitePool, id: &str) -> Result<bool, String> {
    let result = sqlx::query("DELETE FROM price_alerts WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(result.rows_affected() > 0)
}

/// Mark alerts as triggered for a symbol when threshold is crossed.
/// Returns the IDs of newly-triggered alerts.
pub async fn check_and_trigger_alerts(
    pool: &SqlitePool,
    symbol: &str,
    price: f64,
) -> Result<Vec<String>, String> {
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT id, direction, threshold FROM price_alerts
         WHERE symbol = $1 AND triggered = 0",
    )
    .bind(symbol)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let candidates: Vec<(String, String, f64)> = rows
        .into_iter()
        .map(|r| (r.get(0), r.get(1), r.get(2)))
        .collect();

    let mut triggered = Vec::new();
    for (id, dir_str, threshold) in candidates {
        let crossed = match dir_str.as_str() {
            "above" => price >= threshold,
            "below" => price <= threshold,
            _ => false,
        };
        if crossed {
            sqlx::query("UPDATE price_alerts SET triggered = 1 WHERE id = $1")
                .bind(&id)
                .execute(pool)
                .await
                .map_err(|e| e.to_string())?;
            triggered.push(id);
        }
    }

    Ok(triggered)
}

pub async fn reset_alert(pool: &SqlitePool, id: &str) -> Result<bool, String> {
    let result = sqlx::query("UPDATE price_alerts SET triggered = 0 WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(result.rows_affected() > 0)
}

// ── Transactions ──────────────────────────────────────────────────────────────

pub async fn insert_transaction(
    pool: &SqlitePool,
    input: TransactionInput,
) -> Result<Transaction, String> {
    let id = Uuid::new_v4().to_string();
    let created_at = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO transactions
         (id, holding_id, transaction_type, quantity, price, transacted_at, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(&id)
    .bind(&input.holding_id)
    .bind(input.transaction_type.as_str())
    .bind(input.quantity)
    .bind(input.price)
    .bind(&input.transacted_at)
    .bind(&created_at)
    .execute(pool)
    .await
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

pub async fn get_transactions_for_holding(
    pool: &SqlitePool,
    holding_id: &str,
) -> Result<Vec<Transaction>, String> {
    let rows = sqlx::query(
        "SELECT id, holding_id, transaction_type, quantity, price, transacted_at, created_at
         FROM transactions WHERE holding_id = $1 ORDER BY transacted_at ASC",
    )
    .bind(holding_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    rows.into_iter().map(|r| row_to_transaction(&r)).collect()
}

pub async fn get_all_transactions(pool: &SqlitePool) -> Result<Vec<Transaction>, String> {
    let rows = sqlx::query(
        "SELECT id, holding_id, transaction_type, quantity, price, transacted_at, created_at
         FROM transactions ORDER BY transacted_at ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    rows.into_iter().map(|r| row_to_transaction(&r)).collect()
}

fn row_to_transaction(row: &sqlx::sqlite::SqliteRow) -> Result<Transaction, String> {
    use sqlx::Row;
    let type_str: String = row.get(2);
    let transaction_type = type_str.parse::<TransactionType>()?;
    Ok(Transaction {
        id: row.get(0),
        holding_id: row.get(1),
        transaction_type,
        quantity: row.get(3),
        price: row.get(4),
        transacted_at: row.get(5),
        created_at: row.get(6),
    })
}

pub async fn delete_transaction(pool: &SqlitePool, id: &str) -> Result<bool, String> {
    let result = sqlx::query("DELETE FROM transactions WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(result.rows_affected() > 0)
}

// ── Dividends ─────────────────────────────────────────────────────────────────

pub async fn insert_dividend(
    pool: &SqlitePool,
    input: DividendInput,
    symbol: &str,
) -> Result<Dividend, String> {
    let created_at = Utc::now().to_rfc3339();
    let result = sqlx::query(
        "INSERT INTO dividends (holding_id, amount_per_unit, currency, ex_date, pay_date, created_at)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(&input.holding_id)
    .bind(input.amount_per_unit)
    .bind(&input.currency)
    .bind(&input.ex_date)
    .bind(&input.pay_date)
    .bind(&created_at)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    let id = result.last_insert_rowid();
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

pub async fn get_dividends(pool: &SqlitePool) -> Result<Vec<Dividend>, String> {
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT d.id, d.holding_id, h.symbol, d.amount_per_unit, d.currency,
                d.ex_date, d.pay_date, d.created_at
         FROM dividends d
         JOIN holdings h ON h.id = d.holding_id
         ORDER BY d.ex_date DESC",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(rows
        .into_iter()
        .map(|r| Dividend {
            id: r.get(0),
            holding_id: r.get(1),
            symbol: r.get(2),
            amount_per_unit: r.get(3),
            currency: r.get(4),
            ex_date: r.get(5),
            pay_date: r.get(6),
            created_at: r.get(7),
        })
        .collect())
}

pub async fn delete_dividend(pool: &SqlitePool, id: i64) -> Result<bool, String> {
    let result = sqlx::query("DELETE FROM dividends WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(result.rows_affected() > 0)
}

/// Returns the sum of `amount_per_unit * quantity` (converted to `base_currency`)
/// for all dividends whose `pay_date` falls within the last 365 days.
pub async fn get_annual_dividend_income(
    pool: &SqlitePool,
    base_currency: &str,
    fx_rates: &[FxRate],
) -> Result<f64, String> {
    use sqlx::Row;
    let cutoff = (Utc::now() - chrono::Duration::days(365))
        .format("%Y-%m-%d")
        .to_string();

    let rows = sqlx::query(
        "SELECT d.amount_per_unit * h.quantity, d.currency
         FROM dividends d
         JOIN holdings h ON h.id = d.holding_id
         WHERE d.pay_date >= $1",
    )
    .bind(&cutoff)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let base_upper = base_currency.to_uppercase();
    let mut total = 0.0_f64;

    for row in rows {
        let raw_amount: f64 = row.get(0);
        let currency: String = row.get(1);
        let currency_upper = currency.to_uppercase();

        let fx_rate = if currency_upper == base_upper {
            1.0
        } else {
            let direct = format!("{}{}", currency_upper, base_upper);
            let inverted = format!("{}{}", base_upper, currency_upper);
            if let Some(r) = fx_rates.iter().find(|r| r.pair == direct) {
                r.rate
            } else if let Some(r) = fx_rates.iter().find(|r| r.pair == inverted) {
                if r.rate != 0.0 {
                    1.0 / r.rate
                } else {
                    1.0
                }
            } else {
                eprintln!("Warning: no FX rate found for {currency_upper}/{base_upper}, using 1:1 fallback for dividend income calculation");
                1.0
            }
        };

        total += raw_amount * fx_rate;
    }

    Ok(total)
}

#[allow(dead_code)]
pub async fn holding_exists(pool: &SqlitePool, symbol: &str) -> Result<bool, String> {
    let row = sqlx::query("SELECT 1 FROM holdings WHERE UPPER(symbol) = UPPER($1) LIMIT 1")
        .bind(symbol)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(row.is_some())
}

// ── Accounts ──────────────────────────────────────────────────────────────────

pub async fn insert_account(
    pool: &SqlitePool,
    id: &str,
    name: &str,
    account_type: &str,
    institution: Option<&str>,
) -> Result<(), String> {
    let created_at = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO accounts (id, name, type, institution, created_at)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(id)
    .bind(name)
    .bind(account_type)
    .bind(institution)
    .bind(&created_at)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn get_accounts(pool: &SqlitePool) -> Result<Vec<Account>, String> {
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT id, name, type, institution, created_at FROM accounts ORDER BY created_at ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(rows
        .into_iter()
        .map(|r| Account {
            id: r.get(0),
            name: r.get(1),
            account_type: r.get(2),
            institution: r.get(3),
            created_at: r.get(4),
        })
        .collect())
}

pub async fn update_account(
    pool: &SqlitePool,
    id: &str,
    name: &str,
    account_type: &str,
    institution: Option<&str>,
) -> Result<(), String> {
    let result = sqlx::query("UPDATE accounts SET name=$1, type=$2, institution=$3 WHERE id=$4")
        .bind(name)
        .bind(account_type)
        .bind(institution)
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    if result.rows_affected() == 0 {
        return Err(format!("Account {} not found", id));
    }
    Ok(())
}

/// Delete an account by id. Returns an error if any holding references this account's type.
pub async fn delete_account(pool: &SqlitePool, id: &str) -> Result<(), String> {
    use sqlx::Row;

    // Look up the account name and type
    let row = sqlx::query("SELECT name, type FROM accounts WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Account {} not found", id))?;

    let name: String = row.get(0);
    let _account_type: String = row.get(1);

    // Guard: refuse deletion when holdings reference this account by id.
    let count_row = sqlx::query("SELECT COUNT(*) FROM holdings WHERE account_id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;

    let count: i64 = count_row.get(0);
    if count > 0 {
        return Err(format!(
            "Cannot delete account '{}': {} holding(s) still reference it",
            name, count
        ));
    }

    sqlx::query("DELETE FROM accounts WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    async fn open_test_db() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("in-memory db");
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("migrations");
        pool
    }

    fn make_input(symbol: &str) -> HoldingInput {
        HoldingInput {
            symbol: symbol.to_string(),
            name: format!("{} Inc.", symbol),
            asset_type: AssetType::Stock,
            account: AccountType::Taxable,
            account_id: None,
            quantity: 10.0,
            cost_basis: 100.0,
            currency: "CAD".to_string(),
            exchange: String::new(),
            target_weight: 0.0,
        }
    }

    #[tokio::test]
    async fn insert_and_get_holdings() {
        let pool = open_test_db().await;
        insert_holding(&pool, make_input("AAPL"))
            .await
            .expect("insert");
        insert_holding(&pool, make_input("MSFT"))
            .await
            .expect("insert");
        let holdings = get_all_holdings(&pool).await.expect("get all");
        assert_eq!(holdings.len(), 2);
        let symbols: Vec<&str> = holdings.iter().map(|h| h.symbol.as_str()).collect();
        assert!(symbols.contains(&"AAPL"));
        assert!(symbols.contains(&"MSFT"));
    }

    #[tokio::test]
    async fn update_holding_changes_fields() {
        let pool = open_test_db().await;
        let inserted = insert_holding(&pool, make_input("GOOG"))
            .await
            .expect("insert");
        let updated_holding = Holding {
            quantity: 20.0,
            cost_basis: 150.0,
            target_weight: 12.5,
            ..inserted
        };
        let updated = update_holding(&pool, updated_holding)
            .await
            .expect("update");
        assert!((updated.quantity - 20.0).abs() < 0.001);
        assert!((updated.cost_basis - 150.0).abs() < 0.001);
        assert!((updated.target_weight - 12.5).abs() < 0.001);
    }

    #[tokio::test]
    async fn delete_holding_removes_row() {
        let pool = open_test_db().await;
        let holding = insert_holding(&pool, make_input("TSLA"))
            .await
            .expect("insert");
        let deleted = delete_holding(&pool, &holding.id).await.expect("delete");
        assert!(deleted);
        let holdings = get_all_holdings(&pool).await.expect("get all");
        assert_eq!(holdings.len(), 0);
    }

    #[tokio::test]
    async fn delete_nonexistent_holding_returns_false() {
        let pool = open_test_db().await;
        let deleted = delete_holding(&pool, "nonexistent-id")
            .await
            .expect("delete");
        assert!(!deleted);
    }

    #[tokio::test]
    async fn upsert_fx_rate_and_get() {
        let pool = open_test_db().await;
        let rate = FxRate {
            pair: "USDCAD".to_string(),
            rate: 1.36,
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };
        upsert_fx_rate(&pool, &rate).await.expect("upsert fx");
        let rate2 = FxRate {
            pair: "USDCAD".to_string(),
            rate: 1.37,
            updated_at: "2024-01-02T00:00:00Z".to_string(),
        };
        upsert_fx_rate(&pool, &rate2).await.expect("upsert fx 2");
        let rates = get_fx_rates(&pool).await.expect("get fx rates");
        assert_eq!(rates.len(), 1);
        assert!((rates[0].rate - 1.37).abs() < 0.001);
    }

    #[tokio::test]
    async fn get_symbol_cache_exact_finds_symbol_case_insensitively() {
        let pool = open_test_db().await;
        let symbol = SymbolResult {
            symbol: "AAPL".to_string(),
            name: "Apple Inc.".to_string(),
            asset_type: AssetType::Stock,
            exchange: "NMS".to_string(),
            currency: "USD".to_string(),
        };
        upsert_symbol(&pool, &symbol).await.expect("upsert symbol");
        let cached = get_symbol_cache_exact(&pool, "aapl")
            .await
            .expect("query exact");
        assert!(cached.is_some());
        assert_eq!(cached.expect("cached").name, "Apple Inc.");
    }

    #[tokio::test]
    async fn holding_exists_matches_case_insensitively() {
        let pool = open_test_db().await;
        insert_holding(&pool, make_input("MSFT"))
            .await
            .expect("insert");
        assert!(holding_exists(&pool, "msft").await.expect("holding exists"));
        assert!(!holding_exists(&pool, "nvda").await.expect("holding exists"));
    }

    #[tokio::test]
    async fn insert_snapshot_and_retrieve_in_range() {
        let pool = open_test_db().await;
        insert_snapshot(&pool, 100_000.0, 90_000.0, 10_000.0)
            .await
            .expect("insert snapshot 1");
        insert_snapshot(&pool, 110_000.0, 90_000.0, 20_000.0)
            .await
            .expect("insert snapshot 2");
        let start = "1970-01-01T00:00:00+00:00";
        let end = "2099-12-31T23:59:59+00:00";
        let points = get_snapshots_in_range(&pool, start, end)
            .await
            .expect("get snapshots");
        assert_eq!(points.len(), 2);
        assert!((points[0].value - 100_000.0).abs() < 0.001);
        assert!((points[1].value - 110_000.0).abs() < 0.001);
    }

    #[tokio::test]
    async fn get_snapshots_in_range_respects_date_bounds() {
        let pool = open_test_db().await;
        sqlx::query(
            "INSERT INTO portfolio_snapshots (total_value, total_cost, gain_loss, recorded_at)
             VALUES (50000.0, 45000.0, 5000.0, '2020-01-15T12:00:00+00:00')",
        )
        .execute(&pool)
        .await
        .expect("manual insert");

        let points = get_snapshots_in_range(
            &pool,
            "2021-01-01T00:00:00+00:00",
            "2099-12-31T23:59:59+00:00",
        )
        .await
        .expect("get snapshots");
        assert_eq!(points.len(), 0);

        let points = get_snapshots_in_range(
            &pool,
            "2020-01-01T00:00:00+00:00",
            "2020-12-31T23:59:59+00:00",
        )
        .await
        .expect("get snapshots");
        assert_eq!(points.len(), 1);
        assert!((points[0].value - 50_000.0).abs() < 0.001);
        assert_eq!(points[0].date, "2020-01-15");
    }

    #[tokio::test]
    async fn prune_snapshots_keeps_recent_and_daily_max_for_old() {
        let pool = open_test_db().await;
        for (value, ts) in &[
            (1000.0_f64, "2020-06-01T08:00:00+00:00"),
            (1050.0_f64, "2020-06-01T12:00:00+00:00"),
            (1100.0_f64, "2020-06-01T18:00:00+00:00"),
        ] {
            sqlx::query(
                "INSERT INTO portfolio_snapshots (total_value, total_cost, gain_loss, recorded_at)
                 VALUES ($1, 900.0, $2, $3)",
            )
            .bind(value)
            .bind(value - 900.0)
            .bind(ts)
            .execute(&pool)
            .await
            .expect("insert old");
        }
        insert_snapshot(&pool, 200_000.0, 180_000.0, 20_000.0)
            .await
            .expect("insert recent");
        prune_snapshots(&pool).await.expect("prune");
        let all = get_snapshots_in_range(
            &pool,
            "1970-01-01T00:00:00+00:00",
            "2099-12-31T23:59:59+00:00",
        )
        .await
        .expect("get all");
        assert_eq!(all.len(), 2);
        let old_point = all.iter().find(|p| p.date == "2020-06-01");
        assert!(old_point.is_some());
        assert!((old_point.unwrap().value - 1100.0).abs() < 0.001);
    }

    #[tokio::test]
    async fn sum_target_weights_returns_zero_for_empty_table() {
        let pool = open_test_db().await;
        let sum = sum_target_weights(&pool, None).await.expect("sum");
        assert!((sum - 0.0).abs() < 0.001);
    }

    #[tokio::test]
    async fn sum_target_weights_sums_all_holdings() {
        let pool = open_test_db().await;
        let mut input_a = make_input("AAPL");
        input_a.target_weight = 40.0;
        let mut input_b = make_input("MSFT");
        input_b.target_weight = 35.0;
        insert_holding(&pool, input_a).await.expect("insert a");
        insert_holding(&pool, input_b).await.expect("insert b");
        let sum = sum_target_weights(&pool, None).await.expect("sum");
        assert!((sum - 75.0).abs() < 0.001);
    }

    #[tokio::test]
    async fn sum_target_weights_excludes_specified_id() {
        let pool = open_test_db().await;
        let mut input_a = make_input("AAPL");
        input_a.target_weight = 40.0;
        let mut input_b = make_input("MSFT");
        input_b.target_weight = 35.0;
        let holding_a = insert_holding(&pool, input_a).await.expect("insert a");
        insert_holding(&pool, input_b).await.expect("insert b");
        let sum = sum_target_weights(&pool, Some(&holding_a.id))
            .await
            .expect("sum excluding a");
        assert!((sum - 35.0).abs() < 0.001);
    }

    #[tokio::test]
    async fn exchange_field_round_trips_through_insert_and_get() {
        let pool = open_test_db().await;
        let input = HoldingInput {
            exchange: "NYSE".to_string(),
            ..make_input("AAPL")
        };
        insert_holding(&pool, input).await.expect("insert");
        let holdings = get_all_holdings(&pool).await.expect("get all");
        assert_eq!(holdings.len(), 1);
        assert_eq!(holdings[0].exchange, "NYSE");
    }

    // ── Config persistence ────────────────────────────────────────────────────

    #[tokio::test]
    async fn set_and_get_config_round_trips_value() {
        let pool = open_test_db().await;
        set_config(&pool, "base_currency", "USD")
            .await
            .expect("set config");
        let val = get_config(&pool, "base_currency")
            .await
            .expect("get config");
        assert_eq!(val, Some("USD".to_string()));
    }

    #[tokio::test]
    async fn get_config_returns_none_for_missing_key() {
        let pool = open_test_db().await;
        let val = get_config(&pool, "nonexistent_key")
            .await
            .expect("get config");
        assert_eq!(val, None);
    }

    #[tokio::test]
    async fn set_config_upserts_existing_key() {
        let pool = open_test_db().await;
        set_config(&pool, "theme", "dark")
            .await
            .expect("initial set");
        set_config(&pool, "theme", "light")
            .await
            .expect("update set");
        let val = get_config(&pool, "theme").await.expect("get config");
        assert_eq!(val, Some("light".to_string()));
    }

    #[tokio::test]
    async fn set_config_stores_multiple_independent_keys() {
        let pool = open_test_db().await;
        set_config(&pool, "base_currency", "CAD")
            .await
            .expect("set base_currency");
        set_config(&pool, "theme", "dark").await.expect("set theme");
        assert_eq!(
            get_config(&pool, "base_currency").await.expect("get"),
            Some("CAD".to_string())
        );
        assert_eq!(
            get_config(&pool, "theme").await.expect("get"),
            Some("dark".to_string())
        );
    }

    #[tokio::test]
    async fn set_config_persists_empty_string_value() {
        let pool = open_test_db().await;
        set_config(&pool, "greeting", "").await.expect("set empty");
        let val = get_config(&pool, "greeting").await.expect("get config");
        assert_eq!(val, Some(String::new()));
    }

    // ── Transaction tests ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn insert_and_get_transactions_for_holding() {
        let pool = open_test_db().await;
        let holding = insert_holding(&pool, make_input("AAPL"))
            .await
            .expect("insert holding");
        let tx = insert_transaction(
            &pool,
            TransactionInput {
                holding_id: holding.id.clone(),
                transaction_type: TransactionType::Buy,
                quantity: 10.0,
                price: 150.0,
                transacted_at: "2024-01-10T10:00:00Z".to_string(),
            },
        )
        .await
        .expect("insert tx");
        assert!(!tx.id.is_empty());
        let txs = get_transactions_for_holding(&pool, &holding.id)
            .await
            .expect("get txs");
        assert_eq!(txs.len(), 1);
        assert_eq!(txs[0].transaction_type, TransactionType::Buy);
        assert!((txs[0].quantity - 10.0).abs() < 0.001);
        assert!((txs[0].price - 150.0).abs() < 0.001);
    }

    #[tokio::test]
    async fn get_transactions_ordered_by_transacted_at_asc() {
        let pool = open_test_db().await;
        let holding = insert_holding(&pool, make_input("MSFT"))
            .await
            .expect("insert holding");
        insert_transaction(
            &pool,
            TransactionInput {
                holding_id: holding.id.clone(),
                transaction_type: TransactionType::Buy,
                quantity: 5.0,
                price: 100.0,
                transacted_at: "2024-01-01T09:00:00Z".to_string(),
            },
        )
        .await
        .expect("insert tx1");
        insert_transaction(
            &pool,
            TransactionInput {
                holding_id: holding.id.clone(),
                transaction_type: TransactionType::Sell,
                quantity: 2.0,
                price: 120.0,
                transacted_at: "2024-03-01T09:00:00Z".to_string(),
            },
        )
        .await
        .expect("insert tx2");
        let txs = get_transactions_for_holding(&pool, &holding.id)
            .await
            .expect("get txs");
        assert_eq!(txs.len(), 2);
        assert_eq!(txs[0].transaction_type, TransactionType::Buy);
        assert_eq!(txs[1].transaction_type, TransactionType::Sell);
    }

    #[tokio::test]
    async fn get_all_transactions_returns_all() {
        let pool = open_test_db().await;
        let h1 = insert_holding(&pool, make_input("AAPL"))
            .await
            .expect("insert h1");
        let h2 = insert_holding(&pool, make_input("GOOG"))
            .await
            .expect("insert h2");
        insert_transaction(
            &pool,
            TransactionInput {
                holding_id: h1.id.clone(),
                transaction_type: TransactionType::Buy,
                quantity: 10.0,
                price: 100.0,
                transacted_at: "2024-01-01T00:00:00Z".to_string(),
            },
        )
        .await
        .expect("tx1");
        insert_transaction(
            &pool,
            TransactionInput {
                holding_id: h2.id.clone(),
                transaction_type: TransactionType::Buy,
                quantity: 500.0,
                price: 1.0,
                transacted_at: "2024-02-01T00:00:00Z".to_string(),
            },
        )
        .await
        .expect("tx2");
        let all = get_all_transactions(&pool).await.expect("get all txs");
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn delete_transaction_removes_row() {
        let pool = open_test_db().await;
        let holding = insert_holding(&pool, make_input("TSLA"))
            .await
            .expect("insert");
        let tx = insert_transaction(
            &pool,
            TransactionInput {
                holding_id: holding.id.clone(),
                transaction_type: TransactionType::Buy,
                quantity: 1.0,
                price: 200.0,
                transacted_at: "2024-01-01T00:00:00Z".to_string(),
            },
        )
        .await
        .expect("insert tx");
        delete_transaction(&pool, &tx.id).await.expect("delete tx");
        let txs = get_transactions_for_holding(&pool, &holding.id)
            .await
            .expect("get txs");
        assert_eq!(txs.len(), 0);
    }

    #[tokio::test]
    async fn transactions_cascade_on_holding_delete() {
        let pool = open_test_db().await;
        let holding = insert_holding(&pool, make_input("NVDA"))
            .await
            .expect("insert");
        insert_transaction(
            &pool,
            TransactionInput {
                holding_id: holding.id.clone(),
                transaction_type: TransactionType::Buy,
                quantity: 5.0,
                price: 300.0,
                transacted_at: "2024-01-01T00:00:00Z".to_string(),
            },
        )
        .await
        .expect("insert tx");
        delete_holding(&pool, &holding.id)
            .await
            .expect("delete holding");
        let txs = get_transactions_for_holding(&pool, &holding.id)
            .await
            .expect("get txs");
        assert_eq!(txs.len(), 0);
    }

    // ── Account CRUD ──────────────────────────────────────────────────────────

    #[tokio::test]
    async fn insert_and_get_accounts() {
        let pool = open_test_db().await;
        insert_account(&pool, "acc-1", "My TFSA", "tfsa", Some("Questrade"))
            .await
            .expect("insert");
        insert_account(&pool, "acc-2", "RRSP", "rrsp", None)
            .await
            .expect("insert");
        let accounts = get_accounts(&pool).await.expect("get accounts");
        assert_eq!(accounts.len(), 2);
        let names: Vec<&str> = accounts.iter().map(|a| a.name.as_str()).collect();
        assert!(names.contains(&"My TFSA"));
        assert!(names.contains(&"RRSP"));
        let tfsa = accounts.iter().find(|a| a.id == "acc-1").unwrap();
        assert_eq!(tfsa.institution, Some("Questrade".to_string()));
        assert_eq!(tfsa.account_type, "tfsa");
    }

    #[tokio::test]
    async fn update_account_changes_fields() {
        let pool = open_test_db().await;
        insert_account(&pool, "acc-1", "Old Name", "taxable", None)
            .await
            .expect("insert");
        update_account(&pool, "acc-1", "New Name", "rrsp", Some("TD"))
            .await
            .expect("update");
        let accounts = get_accounts(&pool).await.expect("get accounts");
        let acct = accounts.iter().find(|a| a.id == "acc-1").unwrap();
        assert_eq!(acct.name, "New Name");
        assert_eq!(acct.account_type, "rrsp");
        assert_eq!(acct.institution, Some("TD".to_string()));
    }

    #[tokio::test]
    async fn delete_account_succeeds_when_no_holdings() {
        let pool = open_test_db().await;
        insert_account(&pool, "acc-1", "Empty Account", "tfsa", None)
            .await
            .expect("insert");
        delete_account(&pool, "acc-1")
            .await
            .expect("delete should succeed");
        let accounts = get_accounts(&pool).await.expect("get accounts");
        assert_eq!(accounts.len(), 0);
    }

    #[tokio::test]
    async fn delete_account_fails_when_holdings_reference_it() {
        let pool = open_test_db().await;
        insert_account(&pool, "acc-1", "taxable", "taxable", None)
            .await
            .expect("insert account");
        let input = make_input("AAPL");
        insert_holding(&pool, input).await.expect("insert holding");
        let result = delete_account(&pool, "acc-1").await;
        assert!(
            result.is_err(),
            "delete should fail with referenced holdings"
        );
    }

    #[tokio::test]
    async fn update_account_returns_error_for_nonexistent_id() {
        let pool = open_test_db().await;
        let result = update_account(&pool, "nonexistent", "Name", "tfsa", None).await;
        assert!(result.is_err());
    }
}
