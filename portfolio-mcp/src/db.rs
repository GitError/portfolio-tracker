use chrono::Utc;
use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};
use std::str::FromStr;
use uuid::Uuid;

use crate::types::{
    AccountType, AlertDirection, AlertId, AssetType, FxRate, Holding, HoldingId, HoldingInput,
    PriceAlert, PriceAlertInput, PriceData, Transaction, TransactionId, TransactionInput,
    TransactionType,
};

/// Open a connection pool to the portfolio SQLite database.
/// The database must already exist (created_if_missing = false).
pub async fn open_pool(db_path: &str) -> anyhow::Result<SqlitePool> {
    use sqlx::sqlite::{SqliteJournalMode, SqlitePoolOptions};

    let url = format!("sqlite:{db_path}");
    let opts = SqliteConnectOptions::from_str(&url)?
        .create_if_missing(false)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal);

    let pool = SqlitePoolOptions::new()
        .max_connections(3)
        .connect_with(opts)
        .await?;

    Ok(pool)
}

// ── Config ────────────────────────────────────────────────────────────────────

pub async fn get_config(pool: &SqlitePool, key: &str) -> anyhow::Result<Option<String>> {
    let row = sqlx::query("SELECT value FROM app_config WHERE key = $1")
        .bind(key)
        .fetch_optional(pool)
        .await?;

    Ok(row.map(|r| {
        use sqlx::Row;
        r.get::<String, _>(0)
    }))
}

pub async fn set_config(pool: &SqlitePool, key: &str, value: &str) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO app_config (key, value) VALUES ($1, $2)
         ON CONFLICT(key) DO UPDATE SET value=excluded.value",
    )
    .bind(key)
    .bind(value)
    .execute(pool)
    .await?;
    Ok(())
}

// ── Holdings ──────────────────────────────────────────────────────────────────

pub async fn get_all_holdings(pool: &SqlitePool) -> anyhow::Result<Vec<Holding>> {
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
            h.updated_at,
            h.indicated_annual_dividend,
            h.indicated_annual_dividend_currency,
            h.dividend_frequency,
            h.maturity_date
         FROM holdings h
         LEFT JOIN accounts a ON a.id = h.account_id
         WHERE h.deleted_at IS NULL
         ORDER BY h.created_at ASC",
    )
    .fetch_all(pool)
    .await?;

    let holdings = rows
        .into_iter()
        .map(|r| {
            let asset_type_str: String = r.get(3);
            let account_str: String = r.get(4);
            let asset_type = AssetType::from_str(&asset_type_str).unwrap_or(AssetType::Stock);
            let account = AccountType::from_str(&account_str).unwrap_or(AccountType::Taxable);
            Holding {
                id: HoldingId(r.get(0)),
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
                indicated_annual_dividend: r.get::<Option<f64>, _>(14),
                indicated_annual_dividend_currency: r.get::<Option<String>, _>(15),
                dividend_frequency: r.get::<Option<String>, _>(16),
                maturity_date: r.get::<Option<String>, _>(17),
            }
        })
        .collect();

    Ok(holdings)
}

pub async fn insert_holding(pool: &SqlitePool, input: HoldingInput) -> anyhow::Result<Holding> {
    use sqlx::Row;

    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    // Look up account_id by type if not provided.
    let effective_account_id: Option<String> = if let Some(account_id) = input.account_id.clone() {
        Some(account_id)
    } else {
        sqlx::query("SELECT id FROM accounts WHERE type = $1 ORDER BY created_at ASC LIMIT 1")
            .bind(input.account.as_str())
            .fetch_optional(pool)
            .await?
            .map(|r| r.get::<String, _>(0))
    };

    sqlx::query(
        "INSERT INTO holdings
         (id, symbol, name, asset_type, account, account_id, quantity, cost_basis, currency,
          exchange, target_weight, created_at, updated_at,
          indicated_annual_dividend, indicated_annual_dividend_currency,
          dividend_frequency, maturity_date)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)",
    )
    .bind(&id)
    .bind(&input.symbol)
    .bind(&input.name)
    .bind(input.asset_type.as_str())
    .bind(input.account.as_str())
    .bind(&effective_account_id)
    .bind(input.quantity)
    .bind(input.cost_basis)
    .bind(&input.currency)
    .bind(&input.exchange)
    .bind(input.target_weight)
    .bind(&now)
    .bind(&now)
    .bind(input.indicated_annual_dividend)
    .bind(&input.indicated_annual_dividend_currency)
    .bind(&input.dividend_frequency)
    .bind(&input.maturity_date)
    .execute(pool)
    .await?;

    Ok(Holding {
        id: HoldingId(id),
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
        indicated_annual_dividend: input.indicated_annual_dividend,
        indicated_annual_dividend_currency: input.indicated_annual_dividend_currency,
        dividend_frequency: input.dividend_frequency,
        maturity_date: input.maturity_date,
    })
}

pub async fn delete_holding(pool: &SqlitePool, id: &HoldingId) -> anyhow::Result<bool> {
    let result = sqlx::query(
        "UPDATE holdings SET deleted_at = CURRENT_TIMESTAMP WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id.0.as_str())
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

// ── Price cache ───────────────────────────────────────────────────────────────

pub async fn get_cached_prices(pool: &SqlitePool) -> anyhow::Result<Vec<PriceData>> {
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT symbol, price, currency, change, change_percent, updated_at,
                open, previous_close, volume
         FROM price_cache",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| PriceData {
            symbol: r.get(0),
            price: r.get(1),
            currency: r.get(2),
            change: r.get(3),
            change_percent: r.get(4),
            updated_at: r.get(5),
            open: r.get::<Option<f64>, _>(6),
            previous_close: r.get::<Option<f64>, _>(7),
            volume: r.get::<Option<i64>, _>(8),
        })
        .collect())
}

// ── FX rates ──────────────────────────────────────────────────────────────────

pub async fn get_fx_rates(pool: &SqlitePool) -> anyhow::Result<Vec<FxRate>> {
    use sqlx::Row;
    let rows = sqlx::query("SELECT pair, rate, updated_at FROM fx_rates")
        .fetch_all(pool)
        .await?;

    Ok(rows
        .into_iter()
        .map(|r| FxRate {
            pair: r.get(0),
            rate: r.get(1),
            updated_at: r.get(2),
        })
        .collect())
}

// ── Price Alerts ──────────────────────────────────────────────────────────────

pub async fn get_alerts(pool: &SqlitePool) -> anyhow::Result<Vec<PriceAlert>> {
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT id, symbol, direction, threshold, currency, note, triggered, created_at
         FROM price_alerts ORDER BY created_at DESC",
    )
    .fetch_all(pool)
    .await?;

    let alerts = rows
        .into_iter()
        .filter_map(|r| {
            let dir_str: String = r.get(2);
            let direction = dir_str.parse::<AlertDirection>().ok()?;
            let triggered: bool = r.get(6);
            Some(PriceAlert {
                id: AlertId(r.get(0)),
                symbol: r.get(1),
                direction,
                threshold: r.get(3),
                currency: r.get(4),
                note: r.get(5),
                triggered,
                created_at: r.get(7),
            })
        })
        .collect();

    Ok(alerts)
}

pub async fn insert_alert(pool: &SqlitePool, input: PriceAlertInput) -> anyhow::Result<PriceAlert> {
    let id = Uuid::new_v4().to_string();
    let created_at = Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO price_alerts (id, symbol, direction, threshold, currency, note, triggered, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, 0, $7)",
    )
    .bind(&id)
    .bind(&input.symbol)
    .bind(input.direction.as_str())
    .bind(input.threshold)
    .bind(&input.currency)
    .bind(&input.note)
    .bind(&created_at)
    .execute(pool)
    .await?;

    Ok(PriceAlert {
        id: AlertId(id),
        symbol: input.symbol,
        direction: input.direction,
        threshold: input.threshold,
        currency: input.currency,
        note: input.note,
        triggered: false,
        created_at,
    })
}

pub async fn delete_alert(pool: &SqlitePool, id: &AlertId) -> anyhow::Result<bool> {
    let result = sqlx::query("DELETE FROM price_alerts WHERE id = $1")
        .bind(id.0.as_str())
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn reset_alert(pool: &SqlitePool, id: &AlertId) -> anyhow::Result<bool> {
    let result = sqlx::query("UPDATE price_alerts SET triggered = 0 WHERE id = $1")
        .bind(id.0.as_str())
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

// ── Transactions ──────────────────────────────────────────────────────────────

pub async fn get_all_transactions(pool: &SqlitePool) -> anyhow::Result<Vec<Transaction>> {
    let rows = sqlx::query(
        "SELECT id, holding_id, transaction_type, quantity, price, transacted_at, created_at
         FROM transactions WHERE deleted_at IS NULL ORDER BY transacted_at ASC",
    )
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|r| row_to_transaction(&r))
        .collect::<Result<Vec<_>, _>>()
}

pub async fn insert_transaction(
    pool: &SqlitePool,
    input: TransactionInput,
) -> anyhow::Result<Transaction> {
    let id = Uuid::new_v4().to_string();
    let created_at = Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO transactions
         (id, holding_id, transaction_type, quantity, price, transacted_at, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(&id)
    .bind(input.holding_id.0.as_str())
    .bind(input.transaction_type.as_str())
    .bind(input.quantity)
    .bind(input.price)
    .bind(&input.transacted_at)
    .bind(&created_at)
    .execute(pool)
    .await?;

    Ok(Transaction {
        id: TransactionId(id),
        holding_id: input.holding_id,
        transaction_type: input.transaction_type,
        quantity: input.quantity,
        price: input.price,
        transacted_at: input.transacted_at,
        created_at,
    })
}

pub async fn delete_transaction(pool: &SqlitePool, id: &TransactionId) -> anyhow::Result<bool> {
    let result = sqlx::query(
        "UPDATE transactions SET deleted_at = CURRENT_TIMESTAMP WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id.0.as_str())
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

fn row_to_transaction(row: &sqlx::sqlite::SqliteRow) -> anyhow::Result<Transaction> {
    use sqlx::Row;
    let type_str: String = row.get(2);
    let transaction_type = type_str
        .parse::<TransactionType>()
        .map_err(|e| anyhow::anyhow!(e))?;
    Ok(Transaction {
        id: TransactionId(row.get(0)),
        holding_id: HoldingId(row.get(1)),
        transaction_type,
        quantity: row.get(3),
        price: row.get(4),
        transacted_at: row.get(5),
        created_at: row.get(6),
    })
}
