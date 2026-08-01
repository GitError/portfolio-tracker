use chrono::Utc;
use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};
use std::str::FromStr;
use uuid::Uuid;

use crate::types::{
    Account, AccountType, AlertDirection, AlertId, AssetType, Dividend, DividendId, DividendInput,
    FxRate, Holding, HoldingId, HoldingInput, PriceAlert, PriceAlertInput, PriceData, Transaction,
    TransactionId, TransactionInput, TransactionType,
};

/// Cap applied to MCP list queries that don't take explicit pagination
/// parameters (`list_holdings`, `list_transactions`, `list_alerts`, ...).
/// Mirrors the max `page_size` accepted by the Tauri `*_paginated` commands
/// (`validate_pagination` in `src-tauri/src/commands/mod.rs`), so an
/// unbounded portfolio can't make these tools return an unbounded result set.
pub const PAGINATION_FETCH_ALL_SIZE: i64 = 500;

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
         ORDER BY h.created_at ASC
         LIMIT $1",
    )
    .bind(PAGINATION_FETCH_ALL_SIZE)
    .fetch_all(pool)
    .await?;

    let holdings = rows.into_iter().map(row_to_holding).collect();

    Ok(holdings)
}

/// Look up a single holding by id (used by `update_holding` to preserve
/// `created_at`/`account_name` across the update).
pub async fn get_holding_by_id(pool: &SqlitePool, id: &str) -> anyhow::Result<Option<Holding>> {
    let row = sqlx::query(
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
         WHERE h.id = $1 AND h.deleted_at IS NULL",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(row_to_holding))
}

fn row_to_holding(r: sqlx::sqlite::SqliteRow) -> Holding {
    use sqlx::Row;
    let asset_type_str: String = r.get(3);
    let account_str: String = r.get(4);
    let asset_type = AssetType::from_str(&asset_type_str).unwrap_or_else(|_| {
        tracing::warn!(raw = %asset_type_str, "unrecognised asset_type; defaulting to Stock");
        AssetType::Stock
    });
    let account = AccountType::from_str(&account_str).unwrap_or_else(|_| {
        tracing::warn!(raw = %account_str, "unrecognised account_type; defaulting to Taxable");
        AccountType::Taxable
    });
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
        target_weight: r.get::<Option<f64>, _>(11),
        created_at: r.get(12),
        updated_at: r.get(13),
        indicated_annual_dividend: r.get::<Option<f64>, _>(14),
        indicated_annual_dividend_currency: r.get::<Option<String>, _>(15),
        dividend_frequency: r.get::<Option<String>, _>(16),
        maturity_date: r.get::<Option<String>, _>(17),
    }
}

/// Mirrors `update_holding` in `src-tauri/src/db.rs`.
pub async fn update_holding(pool: &SqlitePool, holding: Holding) -> anyhow::Result<Holding> {
    let now = Utc::now().to_rfc3339();

    let effective_account_id: Option<String> = if let Some(account_id) = holding.account_id.clone()
    {
        Some(account_id)
    } else {
        use sqlx::Row;
        sqlx::query("SELECT id FROM accounts WHERE type = $1 ORDER BY created_at ASC LIMIT 1")
            .bind(holding.account.as_str())
            .fetch_optional(pool)
            .await?
            .map(|r| r.get::<String, _>(0))
    };

    if effective_account_id.is_none() {
        tracing::warn!(
            "No account of type '{}' found; holding updated without account assignment",
            holding.account.as_str()
        );
    }

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
             updated_at=$11,
             indicated_annual_dividend=$12,
             indicated_annual_dividend_currency=$13,
             dividend_frequency=$14,
             maturity_date=$15
         WHERE id=$16",
    )
    .bind(&holding.symbol)
    .bind(&holding.name)
    .bind(holding.asset_type.as_str())
    .bind(holding.account.as_str())
    .bind(&effective_account_id)
    .bind(holding.quantity)
    .bind(holding.cost_basis)
    .bind(&holding.currency)
    .bind(&holding.exchange)
    .bind(holding.target_weight)
    .bind(&now)
    .bind(holding.indicated_annual_dividend)
    .bind(&holding.indicated_annual_dividend_currency)
    .bind(&holding.dividend_frequency)
    .bind(&holding.maturity_date)
    .bind(holding.id.0.as_str())
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        anyhow::bail!("Holding {} not found", holding.id);
    }

    Ok(Holding {
        updated_at: now,
        account_id: effective_account_id,
        ..holding
    })
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

/// Sum of `target_weight` across all non-deleted holdings, optionally excluding
/// one holding (used when validating an update to that holding).
pub async fn sum_target_weights(
    pool: &SqlitePool,
    exclude_id: Option<&str>,
) -> anyhow::Result<f64> {
    use sqlx::Row;
    let sum: f64 = match exclude_id {
        Some(id) => {
            sqlx::query(
                "SELECT COALESCE(SUM(target_weight), 0.0) FROM holdings WHERE id != $1 AND deleted_at IS NULL",
            )
            .bind(id)
            .fetch_one(pool)
            .await?
            .get::<f64, _>(0)
        }
        None => {
            sqlx::query(
                "SELECT COALESCE(SUM(target_weight), 0.0) FROM holdings WHERE deleted_at IS NULL",
            )
            .fetch_one(pool)
            .await?
            .get::<f64, _>(0)
        }
    };
    Ok(sum)
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
         FROM price_alerts ORDER BY created_at DESC
         LIMIT $1",
    )
    .bind(PAGINATION_FETCH_ALL_SIZE)
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
         FROM transactions WHERE deleted_at IS NULL ORDER BY transacted_at ASC
         LIMIT $1",
    )
    .bind(PAGINATION_FETCH_ALL_SIZE)
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

// ── Accounts ──────────────────────────────────────────────────────────────────

pub async fn get_accounts(pool: &SqlitePool) -> anyhow::Result<Vec<Account>> {
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT id, name, type, institution, created_at FROM accounts
         ORDER BY created_at ASC
         LIMIT $1",
    )
    .bind(PAGINATION_FETCH_ALL_SIZE)
    .fetch_all(pool)
    .await?;

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

pub async fn insert_account(
    pool: &SqlitePool,
    name: &str,
    account_type: &str,
    institution: Option<&str>,
) -> anyhow::Result<Account> {
    let id = Uuid::new_v4().to_string();
    let created_at = Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO accounts (id, name, type, institution, created_at)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(&id)
    .bind(name)
    .bind(account_type)
    .bind(institution)
    .bind(&created_at)
    .execute(pool)
    .await?;

    Ok(Account {
        id,
        name: name.to_string(),
        account_type: account_type.to_string(),
        institution: institution.map(|s| s.to_string()),
        created_at,
    })
}

// ── Dividends ─────────────────────────────────────────────────────────────────

/// Look up a holding's symbol and currency by id. Mirrors
/// `get_holding_symbol_and_currency` in `src-tauri/src/db.rs`, used to
/// validate/attribute an incoming dividend against its parent holding.
pub async fn get_holding_symbol_and_currency(
    pool: &SqlitePool,
    id: &str,
) -> anyhow::Result<Option<(String, String)>> {
    use sqlx::Row;
    let row = sqlx::query("SELECT symbol, currency FROM holdings WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|r| (r.get(0), r.get(1))))
}

pub async fn get_dividends(
    pool: &SqlitePool,
    holding_id: Option<&str>,
) -> anyhow::Result<Vec<Dividend>> {
    use sqlx::Row;
    let rows = if let Some(hid) = holding_id {
        sqlx::query(
            "SELECT d.id, d.holding_id, h.symbol, d.amount_per_unit, d.currency,
                    d.ex_date, d.pay_date, d.created_at
             FROM dividends d
             JOIN holdings h ON h.id = d.holding_id
             WHERE d.deleted_at IS NULL AND d.holding_id = $1
             ORDER BY d.ex_date DESC
             LIMIT $2",
        )
        .bind(hid)
        .bind(PAGINATION_FETCH_ALL_SIZE)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            "SELECT d.id, d.holding_id, h.symbol, d.amount_per_unit, d.currency,
                    d.ex_date, d.pay_date, d.created_at
             FROM dividends d
             JOIN holdings h ON h.id = d.holding_id
             WHERE d.deleted_at IS NULL
             ORDER BY d.ex_date DESC
             LIMIT $1",
        )
        .bind(PAGINATION_FETCH_ALL_SIZE)
        .fetch_all(pool)
        .await?
    };

    Ok(rows
        .into_iter()
        .map(|r| Dividend {
            id: DividendId(r.get(0)),
            holding_id: HoldingId(r.get(1)),
            symbol: r.get(2),
            amount_per_unit: r.get(3),
            currency: r.get(4),
            ex_date: r.get(5),
            pay_date: r.get(6),
            created_at: r.get(7),
        })
        .collect())
}

pub async fn insert_dividend(
    pool: &SqlitePool,
    input: DividendInput,
    symbol: &str,
) -> anyhow::Result<Dividend> {
    let id = Uuid::new_v4().to_string();
    let created_at = Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO dividends (id, holding_id, amount_per_unit, currency, ex_date, pay_date, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(&id)
    .bind(input.holding_id.0.as_str())
    .bind(input.amount_per_unit)
    .bind(&input.currency)
    .bind(&input.ex_date)
    .bind(&input.pay_date)
    .bind(&created_at)
    .execute(pool)
    .await?;

    Ok(Dividend {
        id: DividendId(id),
        holding_id: input.holding_id,
        symbol: symbol.to_string(),
        amount_per_unit: input.amount_per_unit,
        currency: input.currency,
        ex_date: input.ex_date,
        pay_date: input.pay_date,
        created_at,
    })
}

pub async fn delete_dividend(pool: &SqlitePool, id: &DividendId) -> anyhow::Result<bool> {
    let result = sqlx::query(
        "UPDATE dividends SET deleted_at = CURRENT_TIMESTAMP WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id.0.as_str())
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}
