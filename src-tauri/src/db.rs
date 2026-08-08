use chrono::Utc;
use sqlx::{SqliteConnection, SqlitePool};
use std::str::FromStr;
use uuid::Uuid;

use crate::types::{
    Account, AccountType, AlertDirection, AlertId, AssetType, Dividend, DividendId, DividendInput,
    FxRate, Holding, HoldingId, HoldingInput, PerformancePoint, PriceAlert, PriceAlertInput,
    PriceData, SymbolMetadata, SymbolResult, Transaction, TransactionId, TransactionInput,
    TransactionType, Watchlist, WatchlistId, WatchlistItemId, WatchlistItemWithSnapshot,
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

// ── Holdings ──────────────────────────────────────────────────────────────────

pub async fn insert_holding(pool: &SqlitePool, input: HoldingInput) -> Result<Holding, String> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    let effective_account_id: Option<String> = if let Some(account_id) = input.account_id.clone() {
        Some(account_id)
    } else {
        use sqlx::Row;
        sqlx::query("SELECT id FROM accounts WHERE type = $1 ORDER BY created_at ASC LIMIT 1")
            .bind(input.account.as_str())
            .fetch_optional(pool)
            .await
            .map_err(|e| format!("Account lookup failed: {e}"))?
            .map(|r| r.get::<String, _>(0))
    };

    if effective_account_id.is_none() {
        tracing::warn!(
            "No account of type '{}' found; holding inserted without account assignment",
            input.account.as_str()
        );
    }

    sqlx::query(
        "INSERT INTO holdings
         (id, symbol, name, asset_type, account, account_id, quantity, cost_basis, currency, exchange, target_weight, created_at, updated_at, indicated_annual_dividend, indicated_annual_dividend_currency, dividend_frequency, maturity_date)
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
    .await
    .map_err(|e| e.to_string())?;

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

/// Same as `insert_holding` but operates on an existing transaction connection,
/// enabling atomic bulk inserts (e.g. CSV import).
pub async fn insert_holding_in_tx(
    conn: &mut SqliteConnection,
    input: HoldingInput,
) -> Result<Holding, String> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    // Account ID lookup runs on the same connection so it participates in the tx.
    let effective_account_id: Option<String> = if let Some(account_id) = input.account_id.clone() {
        Some(account_id)
    } else {
        use sqlx::Row;
        sqlx::query("SELECT id FROM accounts WHERE type = $1 ORDER BY created_at ASC LIMIT 1")
            .bind(input.account.as_str())
            .fetch_optional(&mut *conn)
            .await
            .map_err(|e| format!("Account lookup failed: {e}"))?
            .map(|r| r.get::<String, _>(0))
    };

    if effective_account_id.is_none() {
        tracing::warn!(
            "No account of type '{}' found; holding inserted without account assignment",
            input.account.as_str()
        );
    }

    sqlx::query(
        "INSERT INTO holdings
         (id, symbol, name, asset_type, account, account_id, quantity, cost_basis, currency, exchange, target_weight, created_at, updated_at, indicated_annual_dividend, indicated_annual_dividend_currency, dividend_frequency, maturity_date)
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
    .execute(&mut *conn)
    .await
    .map_err(|e| e.to_string())?;

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

pub async fn update_holding(pool: &SqlitePool, holding: Holding) -> Result<Holding, String> {
    let now = Utc::now().to_rfc3339();

    let effective_account_id: Option<String> = if let Some(account_id) = holding.account_id.clone()
    {
        Some(account_id)
    } else {
        use sqlx::Row;
        sqlx::query("SELECT id FROM accounts WHERE type = $1 ORDER BY created_at ASC LIMIT 1")
            .bind(holding.account.as_str())
            .fetch_optional(pool)
            .await
            .map_err(|e| format!("Account lookup failed: {e}"))?
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

pub async fn delete_holding(pool: &SqlitePool, id: &HoldingId) -> Result<bool, String> {
    let result = sqlx::query(
        "UPDATE holdings SET deleted_at = CURRENT_TIMESTAMP WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id.0.as_str())
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(result.rows_affected() > 0)
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
    .await
    .map_err(|e| e.to_string())?;

    let holdings = rows
        .into_iter()
        .map(|r| {
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
        })
        .collect();

    Ok(holdings)
}

/// Look up a holding's symbol and currency by id (single-row lookup, avoids
/// scanning the full holdings collection just to read one field).
pub async fn get_holding_symbol_and_currency(
    pool: &SqlitePool,
    id: &str,
) -> Result<Option<(String, String)>, String> {
    use sqlx::Row;
    let row = sqlx::query("SELECT symbol, currency FROM holdings WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(row.map(|r| (r.get(0), r.get(1))))
}

// ── Price cache ───────────────────────────────────────────────────────────────

pub async fn upsert_price(pool: &SqlitePool, price: &PriceData) -> Result<(), String> {
    sqlx::query(
        "INSERT INTO price_cache (symbol, price, currency, change, change_percent, updated_at, open, previous_close, volume)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
         ON CONFLICT(symbol) DO UPDATE SET
           price=excluded.price,
           currency=excluded.currency,
           change=excluded.change,
           change_percent=excluded.change_percent,
           updated_at=excluded.updated_at,
           open=excluded.open,
           previous_close=excluded.previous_close,
           volume=excluded.volume",
    )
    .bind(&price.symbol)
    .bind(price.price)
    .bind(&price.currency)
    .bind(price.change)
    .bind(price.change_percent)
    .bind(&price.updated_at)
    .bind(price.open)
    .bind(price.previous_close)
    .bind(price.volume)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn get_cached_prices(pool: &SqlitePool) -> Result<Vec<PriceData>, String> {
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT symbol, price, currency, change, change_percent, updated_at, open, previous_close, volume FROM price_cache",
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
            open: r.get::<Option<f64>, _>(6),
            previous_close: r.get::<Option<f64>, _>(7),
            volume: r.get::<Option<i64>, _>(8),
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
    let symbol_upper = result.symbol.to_uppercase();
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
    .bind(&symbol_upper)
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

/// Escapes SQLite `LIKE` wildcards (`%`, `_`) and the escape character itself
/// (`\`) in user-supplied search input, so a query like `A_B` matches the
/// literal string `A_B` instead of `A` + any-character + `B`. Callers must
/// pair this with `ESCAPE '\'` on the `LIKE` clause.
fn escape_like_pattern(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

pub async fn search_symbol_cache(
    pool: &SqlitePool,
    query: &str,
) -> Result<Vec<SymbolResult>, String> {
    use sqlx::Row;
    let pattern = format!("%{}%", escape_like_pattern(&query.to_lowercase()));
    let sym_prefix = format!("{}%", escape_like_pattern(&query.to_uppercase()));

    let rows = sqlx::query(
        "SELECT symbol, name, asset_type, exchange, currency FROM symbol_cache
         WHERE symbol LIKE $1 ESCAPE '\\' OR LOWER(name) LIKE $2 ESCAPE '\\'
         ORDER BY CASE WHEN symbol LIKE $1 ESCAPE '\\' THEN 0 ELSE 1 END
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

/// Like [`search_symbol_cache`] but only returns rows whose `updated_at` is
/// within `max_age_secs` of now. Used to serve symbol search results straight
/// from SQLite without hitting the Yahoo Finance API (#580).
pub async fn search_symbol_cache_fresh(
    pool: &SqlitePool,
    query: &str,
    max_age_secs: i64,
) -> Result<Vec<SymbolResult>, String> {
    use sqlx::Row;
    let pattern = format!("%{}%", escape_like_pattern(&query.to_lowercase()));
    let sym_prefix = format!("{}%", escape_like_pattern(&query.to_uppercase()));
    let cutoff = (Utc::now() - chrono::Duration::seconds(max_age_secs)).to_rfc3339();

    let rows = sqlx::query(
        "SELECT symbol, name, asset_type, exchange, currency FROM symbol_cache
         WHERE (symbol LIKE $1 ESCAPE '\\' OR LOWER(name) LIKE $2 ESCAPE '\\') AND updated_at >= $3
         ORDER BY CASE WHEN symbol LIKE $1 ESCAPE '\\' THEN 0 ELSE 1 END
         LIMIT 8",
    )
    .bind(&sym_prefix)
    .bind(&pattern)
    .bind(&cutoff)
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
         WHERE symbol = UPPER($1)
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

// ── Symbol fundamentals cache ─────────────────────────────────────────────────

/// Persist fundamentals fields into symbol_cache for a given symbol.
///
/// Only updates the fundamentals columns on conflict; basic symbol info
/// (name, asset_type, exchange, currency) is left untouched for a row that
/// already exists (e.g. populated earlier via symbol search). When *inserting*
/// a brand-new row, `name`/`asset_type`/`exchange`/`currency` are taken from
/// the caller-supplied values (the same API response fundamentals came from)
/// and only fall back to a placeholder when that data is genuinely absent.
pub async fn upsert_symbol_fundamentals(
    pool: &SqlitePool,
    meta: &SymbolMetadata,
    name: Option<&str>,
    asset_type: Option<AssetType>,
    exchange: Option<&str>,
    currency: Option<&str>,
) -> Result<(), String> {
    let now = Utc::now().to_rfc3339();
    let symbol_upper = meta.symbol.to_uppercase();
    let name = name.filter(|s| !s.is_empty()).unwrap_or(&symbol_upper);
    let asset_type = asset_type.map(|a| a.as_str()).unwrap_or("stock");
    let exchange = exchange.unwrap_or("");
    let currency = currency.filter(|s| !s.is_empty()).unwrap_or("USD");
    sqlx::query(
        "INSERT INTO symbol_cache (symbol, name, asset_type, exchange, currency, sector, industry, country, beta, pe_ratio, dividend_yield, eps, market_cap, fundamentals_updated_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $14)
         ON CONFLICT(symbol) DO UPDATE SET
           sector=excluded.sector,
           industry=excluded.industry,
           country=excluded.country,
           beta=excluded.beta,
           pe_ratio=excluded.pe_ratio,
           dividend_yield=excluded.dividend_yield,
           eps=excluded.eps,
           market_cap=excluded.market_cap,
           fundamentals_updated_at=excluded.fundamentals_updated_at",
    )
    .bind(&symbol_upper)
    .bind(name)
    .bind(asset_type)
    .bind(exchange)
    .bind(currency)
    .bind(&meta.sector)
    .bind(&meta.industry)
    .bind(&meta.country)
    .bind(meta.beta)
    .bind(meta.pe_ratio)
    .bind(meta.dividend_yield)
    .bind(meta.eps)
    .bind(meta.market_cap)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Batch version of the old single-symbol fundamentals lookup: fetches all cached
/// fundamentals for the given symbols in one SQL query and returns only entries
/// that are still fresh (younger than `max_age_secs`).
pub async fn get_symbol_fundamentals_from_cache_batch(
    pool: &SqlitePool,
    symbols: &[&str],
    max_age_secs: i64,
) -> Result<Vec<SymbolMetadata>, String> {
    if symbols.is_empty() {
        return Ok(vec![]);
    }

    use sqlx::{QueryBuilder, Row};

    // Use QueryBuilder for dynamic IN clause — this is sqlx's idiomatic way to
    // build parameterised queries with a variable number of bind values.
    let mut builder = QueryBuilder::new(
        "SELECT symbol, sector, industry, country, beta, pe_ratio, dividend_yield, eps, \
         market_cap, fundamentals_updated_at \
         FROM symbol_cache WHERE symbol IN (",
    );
    let mut sep = builder.separated(", ");
    for s in symbols {
        sep.push_bind(s.to_uppercase());
    }
    builder.push(") AND fundamentals_updated_at IS NOT NULL");

    let rows = builder
        .build()
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;

    let cutoff = Utc::now() - chrono::Duration::seconds(max_age_secs);

    let results = rows
        .into_iter()
        .filter_map(|row| {
            let updated_at: String = row.get(9);
            let cached_time = chrono::DateTime::parse_from_rfc3339(&updated_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now() - chrono::Duration::seconds(max_age_secs + 1));
            if cached_time < cutoff {
                return None; // stale
            }
            Some(SymbolMetadata {
                symbol: row.get(0),
                sector: row.get::<Option<String>, _>(1),
                industry: row.get::<Option<String>, _>(2),
                country: row.get::<Option<String>, _>(3),
                beta: row.get::<Option<f64>, _>(4),
                pe_ratio: row.get::<Option<f64>, _>(5),
                dividend_yield: row.get::<Option<f64>, _>(6),
                eps: row.get::<Option<f64>, _>(7),
                market_cap: row.get::<Option<f64>, _>(8),
            })
        })
        .collect();

    Ok(results)
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
            let date = chrono::DateTime::parse_from_rfc3339(&recorded_at)
                .map(|dt| dt.format("%Y-%m-%d").to_string())
                .unwrap_or_else(|_| {
                    tracing::warn!("Could not parse recorded_at date: {}", recorded_at);
                    recorded_at.chars().take(10).collect()
                });
            PerformancePoint {
                date,
                value: total_value,
            }
        })
        .collect())
}

pub async fn prune_snapshots(pool: &SqlitePool) -> Result<(), String> {
    // Keep only the latest snapshot per calendar day, retaining the 730
    // most-recent distinct days (≈ 2 years). Combines deduplication and
    // age-pruning into a single pass so no intermediate state can be observed.
    sqlx::query(
        "DELETE FROM portfolio_snapshots
         WHERE id NOT IN (
           SELECT id FROM portfolio_snapshots
           WHERE id IN (
             SELECT MAX(id) FROM portfolio_snapshots
             GROUP BY DATE(recorded_at)
           )
           ORDER BY DATE(recorded_at) DESC
           LIMIT 730
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
            sqlx::query("SELECT COALESCE(SUM(target_weight), 0.0) FROM holdings WHERE id != $1 AND deleted_at IS NULL")
                .bind(id)
                .fetch_one(pool)
                .await
                .map_err(|e| e.to_string())
                .map(|r| r.get::<f64, _>(0))?
        }

        None => sqlx::query("SELECT COALESCE(SUM(target_weight), 0.0) FROM holdings WHERE deleted_at IS NULL")
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
    let symbol_upper = input.symbol.to_uppercase();
    sqlx::query(
        "INSERT INTO price_alerts (id, symbol, direction, threshold, currency, note, triggered, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, 0, $7)",
    )
    .bind(&id)
    .bind(&symbol_upper)
    .bind(input.direction.as_str())
    .bind(input.threshold)
    .bind(&input.currency)
    .bind(&input.note)
    .bind(&created_at)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(PriceAlert {
        id: AlertId(id),
        symbol: symbol_upper,
        direction: input.direction,
        threshold: input.threshold,
        currency: input.currency,
        note: input.note,
        triggered: false,
        created_at,
    })
}

pub async fn get_alerts(pool: &SqlitePool) -> Result<Vec<PriceAlert>, String> {
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT id, symbol, direction, threshold, currency, note, triggered, created_at
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

pub async fn delete_alert(pool: &SqlitePool, id: &AlertId) -> Result<bool, String> {
    let result = sqlx::query("DELETE FROM price_alerts WHERE id = $1")
        .bind(id.0.as_str())
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(result.rows_affected() > 0)
}

/// Fetch all non-triggered alerts in a single query.
///
/// Returns a `Vec<(id, symbol_uppercase, direction_str, threshold)>` suitable
/// for building an in-memory lookup map, avoiding one DB round-trip per symbol
/// in the price-refresh hot path.
pub async fn get_all_active_alerts(
    pool: &SqlitePool,
) -> Result<Vec<(String, String, String, f64)>, String> {
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT id, UPPER(symbol), direction, threshold FROM price_alerts WHERE triggered = 0",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(rows
        .into_iter()
        .map(|r| (r.get(0), r.get(1), r.get(2), r.get(3)))
        .collect())
}

/// Mark a single alert as triggered by its ID.
///
/// Called once per triggered alert after the in-memory check in the price-refresh
/// hot path; replaces the per-symbol `check_and_trigger_alerts` DB round-trip.
pub async fn mark_alert_triggered(pool: &SqlitePool, id: &str) -> Result<(), String> {
    sqlx::query("UPDATE price_alerts SET triggered = 1 WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn reset_alert(pool: &SqlitePool, id: &AlertId) -> Result<bool, String> {
    let result = sqlx::query("UPDATE price_alerts SET triggered = 0 WHERE id = $1")
        .bind(id.0.as_str())
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
    .bind(input.holding_id.0.as_str())
    .bind(input.transaction_type.as_str())
    .bind(input.quantity)
    .bind(input.price)
    .bind(&input.transacted_at)
    .bind(&created_at)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

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

pub async fn get_transactions_for_holding(
    pool: &SqlitePool,
    holding_id: &HoldingId,
) -> Result<Vec<Transaction>, String> {
    let rows = sqlx::query(
        "SELECT id, holding_id, transaction_type, quantity, price, transacted_at, created_at
         FROM transactions WHERE holding_id = $1 AND deleted_at IS NULL ORDER BY transacted_at ASC",
    )
    .bind(holding_id.0.as_str())
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    rows.into_iter().map(|r| row_to_transaction(&r)).collect()
}

pub async fn get_all_transactions(pool: &SqlitePool) -> Result<Vec<Transaction>, String> {
    let rows = sqlx::query(
        "SELECT id, holding_id, transaction_type, quantity, price, transacted_at, created_at
         FROM transactions WHERE deleted_at IS NULL ORDER BY transacted_at ASC",
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
        id: TransactionId(row.get(0)),
        holding_id: HoldingId(row.get(1)),
        transaction_type,
        quantity: row.get(3),
        price: row.get(4),
        transacted_at: row.get(5),
        created_at: row.get(6),
    })
}

pub async fn delete_transaction(pool: &SqlitePool, id: &TransactionId) -> Result<bool, String> {
    let result = sqlx::query(
        "UPDATE transactions SET deleted_at = CURRENT_TIMESTAMP WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id.0.as_str())
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
    .await
    .map_err(|e| e.to_string())?;

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

pub async fn get_dividends(pool: &SqlitePool) -> Result<Vec<Dividend>, String> {
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT d.id, d.holding_id, h.symbol, d.amount_per_unit, d.currency,
                d.ex_date, d.pay_date, d.created_at
         FROM dividends d
         JOIN holdings h ON h.id = d.holding_id
         WHERE d.deleted_at IS NULL
         ORDER BY d.ex_date DESC",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

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

pub async fn delete_dividend(pool: &SqlitePool, id: &DividendId) -> Result<bool, String> {
    let result = sqlx::query(
        "UPDATE dividends SET deleted_at = CURRENT_TIMESTAMP WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id.0.as_str())
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
         WHERE d.pay_date >= $1 AND d.deleted_at IS NULL",
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
                tracing::warn!("no FX rate found for {currency_upper}/{base_upper}, using 1:1 fallback for dividend income calculation");
                1.0
            }
        };

        total += raw_amount * fx_rate;
    }

    Ok(total)
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

/// Look up an account's created_at timestamp by id (single-row lookup, avoids
/// scanning the full accounts collection just to read one field).
pub async fn get_account_created_at(pool: &SqlitePool, id: &str) -> Result<Option<String>, String> {
    sqlx::query_scalar("SELECT created_at FROM accounts WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())
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

    // Guard: refuse deletion when non-deleted holdings reference this account by id.
    let count_row =
        sqlx::query("SELECT COUNT(*) FROM holdings WHERE account_id = $1 AND deleted_at IS NULL")
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

// ── Pagination helpers ────────────────────────────────────────────────────────

fn total_pages(total: i64, page_size: i64) -> i64 {
    if page_size <= 0 {
        return 0;
    }
    (total + page_size - 1) / page_size
}

pub async fn get_holdings_paginated(
    pool: &SqlitePool,
    page: i64,
    page_size: i64,
) -> Result<crate::types::PaginatedResult<Holding>, String> {
    use sqlx::Row;
    let offset = (page - 1).max(0) * page_size;

    let count_row = sqlx::query("SELECT COUNT(*) FROM holdings WHERE deleted_at IS NULL")
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;
    let total: i64 = count_row.get(0);

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
         LIMIT $1 OFFSET $2",
    )
    .bind(page_size)
    .bind(offset)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let items = rows
        .into_iter()
        .map(|r| {
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
        })
        .collect();

    Ok(crate::types::PaginatedResult {
        items,
        total,
        page,
        page_size,
        total_pages: total_pages(total, page_size),
    })
}

pub async fn get_transactions_paginated(
    pool: &SqlitePool,
    holding_id: Option<&str>,
    page: i64,
    page_size: i64,
) -> Result<crate::types::PaginatedResult<Transaction>, String> {
    let offset = (page - 1).max(0) * page_size;

    let (count_sql, items_sql) = if holding_id.is_some() {
        (
            "SELECT COUNT(*) FROM transactions WHERE holding_id = $1 AND deleted_at IS NULL",
            "SELECT id, holding_id, transaction_type, quantity, price, transacted_at, created_at
             FROM transactions WHERE holding_id = $1 AND deleted_at IS NULL ORDER BY transacted_at ASC
             LIMIT $2 OFFSET $3",
        )
    } else {
        (
            "SELECT COUNT(*) FROM transactions WHERE deleted_at IS NULL",
            "SELECT id, holding_id, transaction_type, quantity, price, transacted_at, created_at
             FROM transactions WHERE deleted_at IS NULL ORDER BY transacted_at ASC
             LIMIT $1 OFFSET $2",
        )
    };

    let total: i64 = if let Some(hid) = holding_id {
        use sqlx::Row;
        sqlx::query(count_sql)
            .bind(hid)
            .fetch_one(pool)
            .await
            .map_err(|e| e.to_string())?
            .get(0)
    } else {
        use sqlx::Row;
        sqlx::query(count_sql)
            .fetch_one(pool)
            .await
            .map_err(|e| e.to_string())?
            .get(0)
    };

    let rows = if let Some(hid) = holding_id {
        sqlx::query(items_sql)
            .bind(hid)
            .bind(page_size)
            .bind(offset)
            .fetch_all(pool)
            .await
            .map_err(|e| e.to_string())?
    } else {
        sqlx::query(items_sql)
            .bind(page_size)
            .bind(offset)
            .fetch_all(pool)
            .await
            .map_err(|e| e.to_string())?
    };

    let items: Result<Vec<Transaction>, String> =
        rows.into_iter().map(|r| row_to_transaction(&r)).collect();

    Ok(crate::types::PaginatedResult {
        items: items?,
        total,
        page,
        page_size,
        total_pages: total_pages(total, page_size),
    })
}

pub async fn get_alerts_paginated(
    pool: &SqlitePool,
    page: i64,
    page_size: i64,
) -> Result<crate::types::PaginatedResult<PriceAlert>, String> {
    use sqlx::Row;
    let offset = (page - 1).max(0) * page_size;

    let total: i64 = sqlx::query("SELECT COUNT(*) FROM price_alerts")
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?
        .get(0);

    let rows = sqlx::query(
        "SELECT id, symbol, direction, threshold, currency, note, triggered, created_at
         FROM price_alerts ORDER BY created_at DESC
         LIMIT $1 OFFSET $2",
    )
    .bind(page_size)
    .bind(offset)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let items = rows
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

    Ok(crate::types::PaginatedResult {
        items,
        total,
        page,
        page_size,
        total_pages: total_pages(total, page_size),
    })
}

pub async fn get_dividends_paginated(
    pool: &SqlitePool,
    page: i64,
    page_size: i64,
) -> Result<crate::types::PaginatedResult<Dividend>, String> {
    use sqlx::Row;
    let offset = (page - 1).max(0) * page_size;

    let total: i64 = sqlx::query("SELECT COUNT(*) FROM dividends WHERE deleted_at IS NULL")
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?
        .get(0);

    let rows = sqlx::query(
        "SELECT d.id, d.holding_id, h.symbol, d.amount_per_unit, d.currency,
                d.ex_date, d.pay_date, d.created_at
         FROM dividends d
         JOIN holdings h ON h.id = d.holding_id
         WHERE d.deleted_at IS NULL
         ORDER BY d.ex_date DESC
         LIMIT $1 OFFSET $2",
    )
    .bind(page_size)
    .bind(offset)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let items = rows
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
        .collect();

    Ok(crate::types::PaginatedResult {
        items,
        total,
        page,
        page_size,
        total_pages: total_pages(total, page_size),
    })
}

// ── Research watchlists (#769) ──────────────────────────────────────────────

/// A cached market-data snapshot is considered stale once it's older than this.
const WATCHLIST_SNAPSHOT_STALE_SECS: i64 = 15 * 60; // 15 minutes

/// Minimum time between `refresh_watchlist_item` calls for the same item,
/// to avoid hammering Yahoo Finance.
pub const WATCHLIST_REFRESH_COOLDOWN_SECS: i64 = 5 * 60; // 5 minutes

/// Pure staleness check, split out so it can be unit tested without a DB or
/// real wall-clock waits: a malformed timestamp is treated as stale (fail safe).
pub(crate) fn is_watchlist_snapshot_stale(retrieved_at: &str, now: chrono::DateTime<Utc>) -> bool {
    match chrono::DateTime::parse_from_rfc3339(retrieved_at) {
        Ok(dt) => (now - dt.with_timezone(&Utc)).num_seconds() > WATCHLIST_SNAPSHOT_STALE_SECS,
        Err(_) => true,
    }
}

/// Seconds remaining in the refresh cooldown, or `None` if the item has never
/// been refreshed or the cooldown has already elapsed. Pure function, unit
/// tested independently of any DB or real wall-clock wait.
pub(crate) fn watchlist_refresh_cooldown_remaining(
    retrieved_at: Option<&str>,
    now: chrono::DateTime<Utc>,
) -> Option<i64> {
    let dt = chrono::DateTime::parse_from_rfc3339(retrieved_at?)
        .ok()?
        .with_timezone(&Utc);
    let elapsed = (now - dt).num_seconds();
    if elapsed < WATCHLIST_REFRESH_COOLDOWN_SECS {
        Some(WATCHLIST_REFRESH_COOLDOWN_SECS - elapsed)
    } else {
        None
    }
}

pub async fn insert_watchlist(pool: &SqlitePool, name: &str) -> Result<Watchlist, String> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO watchlists (id, name, created_at, updated_at) VALUES ($1, $2, $3, $3)",
    )
    .bind(&id)
    .bind(name)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(Watchlist {
        id: WatchlistId(id),
        name: name.to_string(),
        created_at: now.clone(),
        updated_at: now,
    })
}

pub async fn get_watchlists(pool: &SqlitePool) -> Result<Vec<Watchlist>, String> {
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT id, name, created_at, updated_at FROM watchlists ORDER BY created_at ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(rows
        .into_iter()
        .map(|r| Watchlist {
            id: WatchlistId(r.get(0)),
            name: r.get(1),
            created_at: r.get(2),
            updated_at: r.get(3),
        })
        .collect())
}

/// Hard-deletes a watchlist and its items/snapshots. Children are deleted
/// explicitly inside a transaction rather than relying solely on the schema's
/// `ON DELETE CASCADE` — that FK only fires when SQLite's `foreign_keys`
/// pragma is enabled on the connection (true for the production pool, not
/// guaranteed for every test pool), so explicit deletes keep behavior
/// deterministic everywhere.
pub async fn delete_watchlist(pool: &SqlitePool, id: &WatchlistId) -> Result<bool, String> {
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    sqlx::query(
        "DELETE FROM watchlist_item_snapshots
         WHERE watchlist_item_id IN (SELECT id FROM watchlist_items WHERE watchlist_id = $1)",
    )
    .bind(&id.0)
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query("DELETE FROM watchlist_items WHERE watchlist_id = $1")
        .bind(&id.0)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

    let result = sqlx::query("DELETE FROM watchlists WHERE id = $1")
        .bind(&id.0)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(result.rows_affected() > 0)
}

#[allow(clippy::too_many_arguments)]
pub async fn insert_watchlist_item(
    pool: &SqlitePool,
    watchlist_id: &WatchlistId,
    symbol: &str,
    currency: &str,
    thesis: Option<&str>,
    catalysts: Option<&str>,
    risks: Option<&str>,
    entry_price_low: Option<f64>,
    entry_price_high: Option<f64>,
) -> Result<WatchlistItemId, String> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let symbol_upper = symbol.to_uppercase();

    sqlx::query(
        "INSERT INTO watchlist_items
            (id, watchlist_id, symbol, currency, thesis, catalysts, risks,
             entry_price_low, entry_price_high, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $10)",
    )
    .bind(&id)
    .bind(&watchlist_id.0)
    .bind(&symbol_upper)
    .bind(currency)
    .bind(thesis)
    .bind(catalysts)
    .bind(risks)
    .bind(entry_price_low)
    .bind(entry_price_high)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(WatchlistItemId(id))
}

#[allow(clippy::too_many_arguments)]
pub async fn update_watchlist_item(
    pool: &SqlitePool,
    id: &WatchlistItemId,
    thesis: Option<&str>,
    catalysts: Option<&str>,
    risks: Option<&str>,
    entry_price_low: Option<f64>,
    entry_price_high: Option<f64>,
) -> Result<bool, String> {
    let now = Utc::now().to_rfc3339();
    let result = sqlx::query(
        "UPDATE watchlist_items
         SET thesis = $1, catalysts = $2, risks = $3,
             entry_price_low = $4, entry_price_high = $5, updated_at = $6
         WHERE id = $7",
    )
    .bind(thesis)
    .bind(catalysts)
    .bind(risks)
    .bind(entry_price_low)
    .bind(entry_price_high)
    .bind(&now)
    .bind(&id.0)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(result.rows_affected() > 0)
}

/// Hard-deletes a single watchlist item and its cached snapshot (see
/// `delete_watchlist` for why the child row is deleted explicitly).
pub async fn delete_watchlist_item(
    pool: &SqlitePool,
    id: &WatchlistItemId,
) -> Result<bool, String> {
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    sqlx::query("DELETE FROM watchlist_item_snapshots WHERE watchlist_item_id = $1")
        .bind(&id.0)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

    let result = sqlx::query("DELETE FROM watchlist_items WHERE id = $1")
        .bind(&id.0)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(result.rows_affected() > 0)
}

fn row_to_watchlist_item_with_snapshot(row: &sqlx::sqlite::SqliteRow) -> WatchlistItemWithSnapshot {
    use sqlx::Row;
    let retrieved_at: Option<String> = row.get(20);
    // A `None` retrieved_at means "never fetched", which is distinct from
    // "stale" — the UI surfaces that state separately instead of warning.
    let is_stale = retrieved_at
        .as_deref()
        .is_some_and(|ts| is_watchlist_snapshot_stale(ts, Utc::now()));

    WatchlistItemWithSnapshot {
        id: WatchlistItemId(row.get(0)),
        watchlist_id: WatchlistId(row.get(1)),
        symbol: row.get(2),
        name: row.get::<Option<String>, _>(3),
        currency: row.get(4),
        thesis: row.get::<Option<String>, _>(5),
        catalysts: row.get::<Option<String>, _>(6),
        risks: row.get::<Option<String>, _>(7),
        entry_price_low: row.get::<Option<f64>, _>(8),
        entry_price_high: row.get::<Option<f64>, _>(9),
        created_at: row.get(10),
        updated_at: row.get(11),
        price: row.get::<Option<f64>, _>(12),
        market_cap: row.get::<Option<f64>, _>(13),
        fifty_two_week_low: row.get::<Option<f64>, _>(14),
        fifty_two_week_high: row.get::<Option<f64>, _>(15),
        ytd_return: row.get::<Option<f64>, _>(16),
        one_year_return: row.get::<Option<f64>, _>(17),
        dividend_yield: row.get::<Option<f64>, _>(18),
        pe_ratio: row.get::<Option<f64>, _>(19),
        retrieved_at,
        is_stale,
        snapshot_error: row.get::<Option<String>, _>(21),
    }
}

pub async fn get_watchlist_item_with_snapshot(
    pool: &SqlitePool,
    id: &WatchlistItemId,
) -> Result<Option<WatchlistItemWithSnapshot>, String> {
    let row = sqlx::query(
        "SELECT
            i.id, i.watchlist_id, i.symbol, s.name, i.currency, i.thesis, i.catalysts, i.risks,
            i.entry_price_low, i.entry_price_high, i.created_at, i.updated_at,
            s.price, s.market_cap, s.fifty_two_week_low, s.fifty_two_week_high,
            s.ytd_return, s.one_year_return, s.dividend_yield, s.pe_ratio,
            s.retrieved_at, s.error
         FROM watchlist_items i
         LEFT JOIN watchlist_item_snapshots s ON s.watchlist_item_id = i.id
         WHERE i.id = $1",
    )
    .bind(&id.0)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(row.as_ref().map(row_to_watchlist_item_with_snapshot))
}

pub async fn list_watchlist_items_with_snapshots(
    pool: &SqlitePool,
    watchlist_id: &WatchlistId,
) -> Result<Vec<WatchlistItemWithSnapshot>, String> {
    let rows = sqlx::query(
        "SELECT
            i.id, i.watchlist_id, i.symbol, s.name, i.currency, i.thesis, i.catalysts, i.risks,
            i.entry_price_low, i.entry_price_high, i.created_at, i.updated_at,
            s.price, s.market_cap, s.fifty_two_week_low, s.fifty_two_week_high,
            s.ytd_return, s.one_year_return, s.dividend_yield, s.pe_ratio,
            s.retrieved_at, s.error
         FROM watchlist_items i
         LEFT JOIN watchlist_item_snapshots s ON s.watchlist_item_id = i.id
         WHERE i.watchlist_id = $1
         ORDER BY i.created_at ASC",
    )
    .bind(&watchlist_id.0)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(rows
        .iter()
        .map(row_to_watchlist_item_with_snapshot)
        .collect())
}

/// Upserts a symbol's market-data snapshot. `snapshot` is `None` when the
/// fetch failed entirely — in that case `error` is stored and every numeric
/// field is cleared, but `retrieved_at` is still updated so the refresh
/// cooldown applies (a failing symbol shouldn't be retried on every render).
pub async fn upsert_watchlist_item_snapshot(
    pool: &SqlitePool,
    item_id: &WatchlistItemId,
    snapshot: Option<&crate::price::WatchlistSnapshotData>,
    error: Option<&str>,
) -> Result<(), String> {
    let retrieved_at = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO watchlist_item_snapshots
            (watchlist_item_id, name, price, currency, market_cap, fifty_two_week_low,
             fifty_two_week_high, ytd_return, one_year_return, dividend_yield,
             pe_ratio, retrieved_at, error)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
         ON CONFLICT(watchlist_item_id) DO UPDATE SET
            name = excluded.name,
            price = excluded.price,
            currency = excluded.currency,
            market_cap = excluded.market_cap,
            fifty_two_week_low = excluded.fifty_two_week_low,
            fifty_two_week_high = excluded.fifty_two_week_high,
            ytd_return = excluded.ytd_return,
            one_year_return = excluded.one_year_return,
            dividend_yield = excluded.dividend_yield,
            pe_ratio = excluded.pe_ratio,
            retrieved_at = excluded.retrieved_at,
            error = excluded.error",
    )
    .bind(&item_id.0)
    .bind(snapshot.and_then(|s| s.name.clone()))
    .bind(snapshot.and_then(|s| s.price))
    .bind(snapshot.and_then(|s| s.currency.clone()))
    .bind(snapshot.and_then(|s| s.market_cap))
    .bind(snapshot.and_then(|s| s.fifty_two_week_low))
    .bind(snapshot.and_then(|s| s.fifty_two_week_high))
    .bind(snapshot.and_then(|s| s.ytd_return))
    .bind(snapshot.and_then(|s| s.one_year_return))
    .bind(snapshot.and_then(|s| s.dividend_yield))
    .bind(snapshot.and_then(|s| s.pe_ratio))
    .bind(&retrieved_at)
    .bind(error)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[cfg(test)]
mod watchlist_tests {
    use super::*;
    use crate::price::WatchlistSnapshotData;

    fn sample_snapshot() -> WatchlistSnapshotData {
        WatchlistSnapshotData {
            name: Some("Apple Inc.".to_string()),
            price: Some(195.89),
            currency: Some("USD".to_string()),
            market_cap: Some(3_000_000_000_000.0),
            fifty_two_week_low: Some(150.0),
            fifty_two_week_high: Some(200.0),
            ytd_return: Some(12.5),
            one_year_return: Some(18.3),
            dividend_yield: Some(0.005),
            pe_ratio: Some(32.1),
        }
    }

    // ── Pure staleness / cooldown logic ────────────────────────────────────

    #[test]
    fn is_watchlist_snapshot_stale_false_when_recent() {
        let now = Utc::now();
        let retrieved_at = (now - chrono::Duration::minutes(5)).to_rfc3339();
        assert!(!is_watchlist_snapshot_stale(&retrieved_at, now));
    }

    #[test]
    fn is_watchlist_snapshot_stale_true_when_older_than_15_minutes() {
        let now = Utc::now();
        let retrieved_at = (now - chrono::Duration::minutes(16)).to_rfc3339();
        assert!(is_watchlist_snapshot_stale(&retrieved_at, now));
    }

    #[test]
    fn is_watchlist_snapshot_stale_boundary_at_exactly_15_minutes_is_not_stale() {
        let now = Utc::now();
        let retrieved_at = (now - chrono::Duration::minutes(15)).to_rfc3339();
        assert!(!is_watchlist_snapshot_stale(&retrieved_at, now));
    }

    #[test]
    fn is_watchlist_snapshot_stale_treats_malformed_timestamp_as_stale() {
        let now = Utc::now();
        assert!(is_watchlist_snapshot_stale("not-a-timestamp", now));
    }

    #[test]
    fn watchlist_refresh_cooldown_remaining_none_when_never_refreshed() {
        assert_eq!(watchlist_refresh_cooldown_remaining(None, Utc::now()), None);
    }

    #[test]
    fn watchlist_refresh_cooldown_remaining_active_just_after_refresh() {
        let now = Utc::now();
        let retrieved_at = (now - chrono::Duration::seconds(30)).to_rfc3339();
        let remaining = watchlist_refresh_cooldown_remaining(Some(&retrieved_at), now);
        assert!(remaining.is_some());
        // 300s cooldown - 30s elapsed = 270s remaining
        assert!((remaining.unwrap() - 270).abs() <= 1);
    }

    #[test]
    fn watchlist_refresh_cooldown_remaining_elapsed_after_5_minutes() {
        let now = Utc::now();
        let retrieved_at =
            (now - chrono::Duration::minutes(5) - chrono::Duration::seconds(1)).to_rfc3339();
        assert_eq!(
            watchlist_refresh_cooldown_remaining(Some(&retrieved_at), now),
            None
        );
    }

    #[test]
    fn watchlist_refresh_cooldown_remaining_boundary_at_exactly_5_minutes_is_elapsed() {
        let now = Utc::now();
        let retrieved_at = (now - chrono::Duration::minutes(5)).to_rfc3339();
        assert_eq!(
            watchlist_refresh_cooldown_remaining(Some(&retrieved_at), now),
            None
        );
    }

    // ── Watchlist CRUD ───────────────────────────────────────────────────────

    #[tokio::test]
    async fn insert_and_list_watchlists_roundtrip() {
        let pool = open_test_db().await;
        let created = insert_watchlist(&pool, "Growth Ideas")
            .await
            .expect("insert");
        assert_eq!(created.name, "Growth Ideas");

        let all = get_watchlists(&pool).await.expect("list");
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, created.id);
    }

    #[tokio::test]
    async fn delete_watchlist_removes_items_and_snapshots() {
        let pool = open_test_db().await;
        let watchlist = insert_watchlist(&pool, "Growth Ideas")
            .await
            .expect("insert watchlist");
        let item_id = insert_watchlist_item(
            &pool,
            &watchlist.id,
            "AAPL",
            "USD",
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .expect("insert item");
        upsert_watchlist_item_snapshot(&pool, &item_id, Some(&sample_snapshot()), None)
            .await
            .expect("upsert snapshot");

        let deleted = delete_watchlist(&pool, &watchlist.id)
            .await
            .expect("delete");
        assert!(deleted);

        let items = list_watchlist_items_with_snapshots(&pool, &watchlist.id)
            .await
            .expect("list items after delete");
        assert!(items.is_empty());

        let remaining_item = get_watchlist_item_with_snapshot(&pool, &item_id)
            .await
            .expect("query item after delete");
        assert!(remaining_item.is_none());
    }

    #[tokio::test]
    async fn delete_watchlist_nonexistent_id_returns_false() {
        let pool = open_test_db().await;
        let fake_id = WatchlistId(uuid::Uuid::new_v4().to_string());
        let deleted = delete_watchlist(&pool, &fake_id).await.expect("delete");
        assert!(!deleted);
    }

    // ── Watchlist item CRUD ──────────────────────────────────────────────────

    #[tokio::test]
    async fn insert_watchlist_item_uppercases_symbol() {
        let pool = open_test_db().await;
        let watchlist = insert_watchlist(&pool, "Ideas")
            .await
            .expect("insert watchlist");
        let item_id = insert_watchlist_item(
            &pool,
            &watchlist.id,
            "aapl",
            "USD",
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .expect("insert item");

        let item = get_watchlist_item_with_snapshot(&pool, &item_id)
            .await
            .expect("get item")
            .expect("item exists");
        assert_eq!(item.symbol, "AAPL");
        assert_eq!(item.retrieved_at, None);
        assert!(!item.is_stale);
    }

    #[tokio::test]
    async fn insert_watchlist_item_stores_research_fields() {
        let pool = open_test_db().await;
        let watchlist = insert_watchlist(&pool, "Ideas")
            .await
            .expect("insert watchlist");
        let item_id = insert_watchlist_item(
            &pool,
            &watchlist.id,
            "MSFT",
            "USD",
            Some("Cloud growth"),
            Some("Earnings call Q3"),
            Some("Regulatory risk"),
            Some(300.0),
            Some(350.0),
        )
        .await
        .expect("insert item");

        let item = get_watchlist_item_with_snapshot(&pool, &item_id)
            .await
            .expect("get item")
            .expect("item exists");
        assert_eq!(item.thesis.as_deref(), Some("Cloud growth"));
        assert_eq!(item.catalysts.as_deref(), Some("Earnings call Q3"));
        assert_eq!(item.risks.as_deref(), Some("Regulatory risk"));
        assert_eq!(item.entry_price_low, Some(300.0));
        assert_eq!(item.entry_price_high, Some(350.0));
    }

    #[tokio::test]
    async fn update_watchlist_item_overwrites_research_fields_only() {
        let pool = open_test_db().await;
        let watchlist = insert_watchlist(&pool, "Ideas")
            .await
            .expect("insert watchlist");
        let item_id = insert_watchlist_item(
            &pool,
            &watchlist.id,
            "MSFT",
            "USD",
            Some("old thesis"),
            None,
            None,
            None,
            None,
        )
        .await
        .expect("insert item");

        let updated = update_watchlist_item(
            &pool,
            &item_id,
            Some("new thesis"),
            Some("new catalyst"),
            None,
            Some(100.0),
            Some(120.0),
        )
        .await
        .expect("update");
        assert!(updated);

        let item = get_watchlist_item_with_snapshot(&pool, &item_id)
            .await
            .expect("get item")
            .expect("item exists");
        assert_eq!(item.symbol, "MSFT"); // symbol untouched
        assert_eq!(item.thesis.as_deref(), Some("new thesis"));
        assert_eq!(item.catalysts.as_deref(), Some("new catalyst"));
        assert_eq!(item.risks, None);
        assert_eq!(item.entry_price_low, Some(100.0));
        assert_eq!(item.entry_price_high, Some(120.0));
    }

    #[tokio::test]
    async fn update_watchlist_item_nonexistent_id_returns_false() {
        let pool = open_test_db().await;
        let fake_id = WatchlistItemId(uuid::Uuid::new_v4().to_string());
        let updated = update_watchlist_item(&pool, &fake_id, None, None, None, None, None)
            .await
            .expect("update");
        assert!(!updated);
    }

    #[tokio::test]
    async fn delete_watchlist_item_removes_item_and_snapshot() {
        let pool = open_test_db().await;
        let watchlist = insert_watchlist(&pool, "Ideas")
            .await
            .expect("insert watchlist");
        let item_id = insert_watchlist_item(
            &pool,
            &watchlist.id,
            "AAPL",
            "USD",
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .expect("insert item");
        upsert_watchlist_item_snapshot(&pool, &item_id, Some(&sample_snapshot()), None)
            .await
            .expect("upsert snapshot");

        let deleted = delete_watchlist_item(&pool, &item_id)
            .await
            .expect("delete");
        assert!(deleted);

        let item = get_watchlist_item_with_snapshot(&pool, &item_id)
            .await
            .expect("get item");
        assert!(item.is_none());
    }

    #[tokio::test]
    async fn insert_watchlist_item_unique_symbol_per_watchlist_rejects_duplicate() {
        let pool = open_test_db().await;
        let watchlist = insert_watchlist(&pool, "Ideas")
            .await
            .expect("insert watchlist");
        insert_watchlist_item(
            &pool,
            &watchlist.id,
            "AAPL",
            "USD",
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .expect("first insert");

        let result = insert_watchlist_item(
            &pool,
            &watchlist.id,
            "AAPL",
            "USD",
            None,
            None,
            None,
            None,
            None,
        )
        .await;
        assert!(result.is_err());
    }

    // ── Snapshot upsert + staleness through the DB layer ─────────────────────

    #[tokio::test]
    async fn upsert_watchlist_item_snapshot_populates_market_data() {
        let pool = open_test_db().await;
        let watchlist = insert_watchlist(&pool, "Ideas")
            .await
            .expect("insert watchlist");
        let item_id = insert_watchlist_item(
            &pool,
            &watchlist.id,
            "AAPL",
            "USD",
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .expect("insert item");

        upsert_watchlist_item_snapshot(&pool, &item_id, Some(&sample_snapshot()), None)
            .await
            .expect("upsert snapshot");

        let item = get_watchlist_item_with_snapshot(&pool, &item_id)
            .await
            .expect("get item")
            .expect("item exists");
        assert_eq!(item.name, Some("Apple Inc.".to_string()));
        assert_eq!(item.price, Some(195.89));
        assert_eq!(item.market_cap, Some(3_000_000_000_000.0));
        assert_eq!(item.pe_ratio, Some(32.1));
        assert!(item.retrieved_at.is_some());
        assert!(
            !item.is_stale,
            "freshly-inserted snapshot must not be stale"
        );
        assert_eq!(item.snapshot_error, None);
    }

    #[tokio::test]
    async fn upsert_watchlist_item_snapshot_second_call_overwrites_first() {
        let pool = open_test_db().await;
        let watchlist = insert_watchlist(&pool, "Ideas")
            .await
            .expect("insert watchlist");
        let item_id = insert_watchlist_item(
            &pool,
            &watchlist.id,
            "AAPL",
            "USD",
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .expect("insert item");

        upsert_watchlist_item_snapshot(&pool, &item_id, Some(&sample_snapshot()), None)
            .await
            .expect("first upsert");

        let mut updated_snapshot = sample_snapshot();
        updated_snapshot.price = Some(210.0);
        upsert_watchlist_item_snapshot(&pool, &item_id, Some(&updated_snapshot), None)
            .await
            .expect("second upsert");

        let item = get_watchlist_item_with_snapshot(&pool, &item_id)
            .await
            .expect("get item")
            .expect("item exists");
        assert_eq!(item.price, Some(210.0));
    }

    #[tokio::test]
    async fn upsert_watchlist_item_snapshot_records_error_and_clears_fields() {
        let pool = open_test_db().await;
        let watchlist = insert_watchlist(&pool, "Ideas")
            .await
            .expect("insert watchlist");
        let item_id = insert_watchlist_item(
            &pool,
            &watchlist.id,
            "BADSYM",
            "USD",
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .expect("insert item");

        upsert_watchlist_item_snapshot(&pool, &item_id, None, Some("HTTP 404 for symbol BADSYM"))
            .await
            .expect("upsert error snapshot");

        let item = get_watchlist_item_with_snapshot(&pool, &item_id)
            .await
            .expect("get item")
            .expect("item exists");
        assert_eq!(item.price, None);
        assert_eq!(
            item.snapshot_error.as_deref(),
            Some("HTTP 404 for symbol BADSYM")
        );
        // retrieved_at is still set on failure so the refresh cooldown applies.
        assert!(item.retrieved_at.is_some());
    }

    #[tokio::test]
    async fn list_watchlist_items_with_snapshots_marks_old_snapshot_stale() {
        let pool = open_test_db().await;
        let watchlist = insert_watchlist(&pool, "Ideas")
            .await
            .expect("insert watchlist");
        let item_id = insert_watchlist_item(
            &pool,
            &watchlist.id,
            "AAPL",
            "USD",
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .expect("insert item");

        // Manually insert an old snapshot (bypassing upsert's Utc::now()) to
        // simulate one fetched 20 minutes ago.
        let old_retrieved_at = (Utc::now() - chrono::Duration::minutes(20)).to_rfc3339();
        sqlx::query(
            "INSERT INTO watchlist_item_snapshots (watchlist_item_id, price, retrieved_at)
             VALUES ($1, $2, $3)",
        )
        .bind(&item_id.0)
        .bind(195.89_f64)
        .bind(&old_retrieved_at)
        .execute(&pool)
        .await
        .expect("insert stale snapshot");

        let items = list_watchlist_items_with_snapshots(&pool, &watchlist.id)
            .await
            .expect("list items");
        assert_eq!(items.len(), 1);
        assert!(items[0].is_stale);
    }

    #[tokio::test]
    async fn list_watchlist_items_with_snapshots_empty_watchlist_returns_empty() {
        let pool = open_test_db().await;
        let watchlist = insert_watchlist(&pool, "Empty")
            .await
            .expect("insert watchlist");
        let items = list_watchlist_items_with_snapshots(&pool, &watchlist.id)
            .await
            .expect("list items");
        assert!(items.is_empty());
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// Create an in-memory SQLite pool with all migrations applied.
/// Accessible from other test modules within the crate (including `maintenance`).
#[cfg(test)]
pub(crate) async fn open_test_db() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("in-memory db");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrations");
    pool
}

#[cfg(test)]
mod tests {
    use super::*;

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
            target_weight: None,
            indicated_annual_dividend: None,
            indicated_annual_dividend_currency: None,
            dividend_frequency: None,
            maturity_date: None,
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
    async fn get_holding_symbol_and_currency_returns_matching_row() {
        let pool = open_test_db().await;
        let inserted = insert_holding(&pool, make_input("AAPL"))
            .await
            .expect("insert");
        let result = get_holding_symbol_and_currency(&pool, inserted.id.0.as_str())
            .await
            .expect("lookup");
        assert_eq!(result, Some(("AAPL".to_string(), "CAD".to_string())));
    }

    #[tokio::test]
    async fn get_holding_symbol_and_currency_returns_none_for_missing_id() {
        let pool = open_test_db().await;
        let result = get_holding_symbol_and_currency(&pool, "does-not-exist")
            .await
            .expect("lookup");
        assert_eq!(result, None);
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
            target_weight: Some(12.5),
            ..inserted
        };
        let updated = update_holding(&pool, updated_holding)
            .await
            .expect("update");
        assert!((updated.quantity - 20.0).abs() < 0.001);
        assert!((updated.cost_basis - 150.0).abs() < 0.001);
        assert!((updated.target_weight.expect("target_weight") - 12.5).abs() < 0.001);
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
        use crate::types::HoldingId;
        let pool = open_test_db().await;
        let deleted = delete_holding(&pool, &HoldingId("nonexistent-id".to_string()))
            .await
            .expect("delete");
        assert!(!deleted);
    }

    #[tokio::test]
    async fn upsert_fx_rate_and_get() {
        let pool = open_test_db().await;
        let now = chrono::Utc::now().to_rfc3339();
        let rate = FxRate {
            pair: "USDCAD".to_string(),
            rate: 1.36,
            updated_at: now.clone(),
        };
        upsert_fx_rate(&pool, &rate).await.expect("upsert fx");
        let rate2 = FxRate {
            pair: "USDCAD".to_string(),
            rate: 1.37,
            updated_at: now,
        };
        upsert_fx_rate(&pool, &rate2).await.expect("upsert fx 2");
        let rates = get_fx_rates(&pool).await.expect("get fx rates");
        assert_eq!(rates.len(), 1);
        assert!((rates[0].rate - 1.37).abs() < 0.001);
    }

    #[tokio::test]
    async fn get_cached_prices_returns_rows_older_than_60_minutes() {
        // Regression guard for #316/#577: get_cached_prices must NOT filter out
        // rows older than 60 minutes. Stale prices should still surface to the
        // caller so build_portfolio_snapshot can use them instead of cost_basis.
        let pool = open_test_db().await;
        let three_hours_ago = (chrono::Utc::now() - chrono::Duration::hours(3)).to_rfc3339();
        let price = PriceData {
            symbol: "AAPL".to_string(),
            price: 150.0,
            currency: "USD".to_string(),
            change: 1.0,
            change_percent: 0.5,
            updated_at: three_hours_ago,
            open: None,
            previous_close: None,
            volume: None,
        };
        upsert_price(&pool, &price).await.expect("upsert price");

        let prices = get_cached_prices(&pool).await.expect("get cached prices");
        assert_eq!(prices.len(), 1);
        assert!((prices[0].price - 150.0).abs() < 0.001);
    }

    #[tokio::test]
    async fn get_fx_rates_returns_rows_older_than_60_minutes() {
        // Regression guard for #316/#577: get_fx_rates must NOT filter out
        // rows older than 60 minutes.
        let pool = open_test_db().await;
        let three_hours_ago = (chrono::Utc::now() - chrono::Duration::hours(3)).to_rfc3339();
        let rate = FxRate {
            pair: "USDCAD".to_string(),
            rate: 1.4,
            updated_at: three_hours_ago,
        };
        upsert_fx_rate(&pool, &rate).await.expect("upsert fx");

        let rates = get_fx_rates(&pool).await.expect("get fx rates");
        assert_eq!(rates.len(), 1);
        assert!((rates[0].rate - 1.4).abs() < 0.001);
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
    async fn search_symbol_cache_fresh_returns_recently_cached_rows() {
        let pool = open_test_db().await;
        let symbol = SymbolResult {
            symbol: "AAPL".to_string(),
            name: "Apple Inc.".to_string(),
            asset_type: AssetType::Stock,
            exchange: "NMS".to_string(),
            currency: "USD".to_string(),
        };
        upsert_symbol(&pool, &symbol).await.expect("upsert symbol");

        let results = search_symbol_cache_fresh(&pool, "aapl", 300)
            .await
            .expect("query fresh cache");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].symbol, "AAPL");
    }

    #[tokio::test]
    async fn search_symbol_cache_fresh_excludes_stale_rows() {
        // Regression guard for #580: a DB cache entry older than the TTL must
        // NOT be served as "fresh" — the caller falls through to a live Yahoo call.
        let pool = open_test_db().await;
        let symbol = SymbolResult {
            symbol: "AAPL".to_string(),
            name: "Apple Inc.".to_string(),
            asset_type: AssetType::Stock,
            exchange: "NMS".to_string(),
            currency: "USD".to_string(),
        };
        upsert_symbol(&pool, &symbol).await.expect("upsert symbol");

        let ten_minutes_ago = (Utc::now() - chrono::Duration::minutes(10)).to_rfc3339();
        sqlx::query("UPDATE symbol_cache SET updated_at = $1 WHERE symbol = 'AAPL'")
            .bind(&ten_minutes_ago)
            .execute(&pool)
            .await
            .expect("backdate symbol cache row");

        let results = search_symbol_cache_fresh(&pool, "aapl", 300)
            .await
            .expect("query fresh cache");
        assert!(
            results.is_empty(),
            "stale row should not be returned as fresh"
        );

        // The unfiltered lookup must still find it — it remains usable as a
        // fallback when Yahoo is unreachable or rate-limited.
        let stale_results = search_symbol_cache(&pool, "aapl")
            .await
            .expect("query full cache");
        assert_eq!(stale_results.len(), 1);
    }

    #[tokio::test]
    async fn search_symbol_cache_fresh_respects_custom_max_age() {
        let pool = open_test_db().await;
        let symbol = SymbolResult {
            symbol: "MSFT".to_string(),
            name: "Microsoft Corp.".to_string(),
            asset_type: AssetType::Stock,
            exchange: "NMS".to_string(),
            currency: "USD".to_string(),
        };
        upsert_symbol(&pool, &symbol).await.expect("upsert symbol");

        let two_minutes_ago = (Utc::now() - chrono::Duration::minutes(2)).to_rfc3339();
        sqlx::query("UPDATE symbol_cache SET updated_at = $1 WHERE symbol = 'MSFT'")
            .bind(&two_minutes_ago)
            .execute(&pool)
            .await
            .expect("backdate symbol cache row");

        let too_strict = search_symbol_cache_fresh(&pool, "msft", 60)
            .await
            .expect("query with 60s max age");
        assert!(
            too_strict.is_empty(),
            "2-minute-old row exceeds 60s max age"
        );

        let lenient = search_symbol_cache_fresh(&pool, "msft", 300)
            .await
            .expect("query with 300s max age");
        assert_eq!(lenient.len(), 1, "2-minute-old row is within 300s max age");
    }

    #[tokio::test]
    async fn search_symbol_cache_treats_underscore_as_literal_not_wildcard() {
        // Regression guard for #679: `_` is a SQLite LIKE single-char wildcard,
        // so searching "A_B" must NOT also match "AXB" once escaped.
        let pool = open_test_db().await;
        for (symbol, name) in [("A_B", "Underscore Co"), ("AXB", "Wildcard Match Co")] {
            upsert_symbol(
                &pool,
                &SymbolResult {
                    symbol: symbol.to_string(),
                    name: name.to_string(),
                    asset_type: AssetType::Stock,
                    exchange: "NMS".to_string(),
                    currency: "USD".to_string(),
                },
            )
            .await
            .expect("upsert symbol");
        }

        let results = search_symbol_cache(&pool, "A_B")
            .await
            .expect("query cache");

        assert_eq!(
            results
                .iter()
                .map(|r| r.symbol.as_str())
                .collect::<Vec<_>>(),
            vec!["A_B"],
            "underscore must match literally, not as a single-char wildcard"
        );
    }

    #[tokio::test]
    async fn search_symbol_cache_treats_percent_as_literal_not_wildcard() {
        // A literal `%` in the query must not act as a multi-char wildcard.
        let pool = open_test_db().await;
        for (symbol, name) in [("A%B", "Percent Co"), ("AZZZB", "Unrelated Co")] {
            upsert_symbol(
                &pool,
                &SymbolResult {
                    symbol: symbol.to_string(),
                    name: name.to_string(),
                    asset_type: AssetType::Stock,
                    exchange: "NMS".to_string(),
                    currency: "USD".to_string(),
                },
            )
            .await
            .expect("upsert symbol");
        }

        let results = search_symbol_cache(&pool, "A%B")
            .await
            .expect("query cache");

        assert_eq!(
            results
                .iter()
                .map(|r| r.symbol.as_str())
                .collect::<Vec<_>>(),
            vec!["A%B"],
            "percent must match literally, not as a multi-char wildcard"
        );
    }

    // ── upsert_symbol_fundamentals ────────────────────────────────────────────

    fn make_symbol_metadata(symbol: &str) -> SymbolMetadata {
        SymbolMetadata {
            symbol: symbol.to_string(),
            sector: Some("Technology".to_string()),
            industry: None,
            country: Some("US".to_string()),
            market_cap: Some(3_000_000_000_000.0),
            pe_ratio: None,
            dividend_yield: None,
            beta: None,
            eps: None,
        }
    }

    #[tokio::test]
    async fn upsert_symbol_fundamentals_new_row_uses_provided_metadata() {
        // Regression guard for #610: a brand-new symbol_cache row must be created
        // with the real name/asset_type/exchange/currency from the API response,
        // not the hardcoded 'stock'/blank placeholders — and the insert must
        // actually succeed (updated_at is NOT NULL with no default).
        let pool = open_test_db().await;
        let meta = make_symbol_metadata("SHOP");

        upsert_symbol_fundamentals(
            &pool,
            &meta,
            Some("Shopify Inc."),
            Some(AssetType::Etf),
            Some("TSX"),
            Some("CAD"),
        )
        .await
        .expect("upsert should succeed for a new row");

        use sqlx::Row;
        let row = sqlx::query(
            "SELECT name, asset_type, exchange, currency FROM symbol_cache WHERE symbol = 'SHOP'",
        )
        .fetch_one(&pool)
        .await
        .expect("row should exist");
        assert_eq!(row.get::<String, _>(0), "Shopify Inc.");
        assert_eq!(row.get::<String, _>(1), "etf");
        assert_eq!(row.get::<String, _>(2), "TSX");
        assert_eq!(row.get::<String, _>(3), "CAD");
    }

    #[tokio::test]
    async fn upsert_symbol_fundamentals_new_row_falls_back_when_data_absent() {
        // When the API genuinely didn't return name/asset_type/exchange/currency,
        // fall back to sensible defaults rather than failing the insert.
        let pool = open_test_db().await;
        let meta = make_symbol_metadata("XYZ");

        upsert_symbol_fundamentals(&pool, &meta, None, None, None, None)
            .await
            .expect("upsert should succeed even with no name/type/exchange/currency");

        use sqlx::Row;
        let row = sqlx::query(
            "SELECT name, asset_type, exchange, currency FROM symbol_cache WHERE symbol = 'XYZ'",
        )
        .fetch_one(&pool)
        .await
        .expect("row should exist");
        assert_eq!(row.get::<String, _>(0), "XYZ");
        assert_eq!(row.get::<String, _>(1), "stock");
        assert_eq!(row.get::<String, _>(2), "");
        assert_eq!(row.get::<String, _>(3), "USD");
    }

    #[tokio::test]
    async fn upsert_symbol_fundamentals_conflict_updates_fundamentals_only() {
        let pool = open_test_db().await;
        let symbol = SymbolResult {
            symbol: "AAPL".to_string(),
            name: "Apple Inc.".to_string(),
            asset_type: AssetType::Stock,
            exchange: "NMS".to_string(),
            currency: "USD".to_string(),
        };
        upsert_symbol(&pool, &symbol).await.expect("upsert symbol");

        let meta = make_symbol_metadata("AAPL");
        upsert_symbol_fundamentals(&pool, &meta, None, None, None, None)
            .await
            .expect("upsert fundamentals on existing row");

        use sqlx::Row;
        let row = sqlx::query(
            "SELECT name, asset_type, exchange, currency, sector FROM symbol_cache WHERE symbol = 'AAPL'",
        )
        .fetch_one(&pool)
        .await
        .expect("row should exist");
        // Existing name/asset_type/exchange/currency from the search cache are untouched.
        assert_eq!(row.get::<String, _>(0), "Apple Inc.");
        assert_eq!(row.get::<String, _>(1), "stock");
        assert_eq!(row.get::<String, _>(2), "NMS");
        assert_eq!(row.get::<String, _>(3), "USD");
        assert_eq!(
            row.get::<Option<String>, _>(4),
            Some("Technology".to_string())
        );
    }

    // ── get_symbol_fundamentals_from_cache_batch ──────────────────────────────

    #[tokio::test]
    async fn get_symbol_fundamentals_from_cache_batch_returns_only_fresh_hits() {
        let pool = open_test_db().await;

        // AAPL: fresh (just upserted).
        upsert_symbol_fundamentals(&pool, &make_symbol_metadata("AAPL"), None, None, None, None)
            .await
            .expect("upsert AAPL");

        // MSFT: stale (fundamentals_updated_at far in the past).
        upsert_symbol_fundamentals(&pool, &make_symbol_metadata("MSFT"), None, None, None, None)
            .await
            .expect("upsert MSFT");
        let stale = (Utc::now() - chrono::Duration::days(2)).to_rfc3339();
        sqlx::query("UPDATE symbol_cache SET fundamentals_updated_at = $1 WHERE symbol = 'MSFT'")
            .bind(&stale)
            .execute(&pool)
            .await
            .expect("age MSFT");

        // GOOG: never cached at all.
        let symbols = ["AAPL", "MSFT", "GOOG"];
        let results = get_symbol_fundamentals_from_cache_batch(&pool, &symbols, 86_400)
            .await
            .expect("batch lookup");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].symbol, "AAPL");
    }

    #[tokio::test]
    async fn get_symbol_fundamentals_from_cache_batch_empty_symbols_returns_empty() {
        let pool = open_test_db().await;
        let results = get_symbol_fundamentals_from_cache_batch(&pool, &[], 86_400)
            .await
            .expect("batch lookup");
        assert!(results.is_empty());
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
        input_a.target_weight = Some(40.0);
        let mut input_b = make_input("MSFT");
        input_b.target_weight = Some(35.0);
        insert_holding(&pool, input_a).await.expect("insert a");
        insert_holding(&pool, input_b).await.expect("insert b");
        let sum = sum_target_weights(&pool, None).await.expect("sum");
        assert!((sum - 75.0).abs() < 0.001);
    }

    #[tokio::test]
    async fn sum_target_weights_excludes_specified_id() {
        let pool = open_test_db().await;
        let mut input_a = make_input("AAPL");
        input_a.target_weight = Some(40.0);
        let mut input_b = make_input("MSFT");
        input_b.target_weight = Some(35.0);
        let holding_a = insert_holding(&pool, input_a).await.expect("insert a");
        insert_holding(&pool, input_b).await.expect("insert b");
        let sum = sum_target_weights(&pool, Some(holding_a.id.0.as_str()))
            .await
            .expect("sum excluding a");
        assert!((sum - 35.0).abs() < 0.001);
    }

    #[tokio::test]
    async fn sum_target_weights_ignores_unset_targets() {
        let pool = open_test_db().await;
        let mut input_a = make_input("AAPL");
        input_a.target_weight = Some(40.0);
        let input_b = make_input("MSFT"); // target_weight: None (unset)
        insert_holding(&pool, input_a).await.expect("insert a");
        insert_holding(&pool, input_b).await.expect("insert b");
        let sum = sum_target_weights(&pool, None).await.expect("sum");
        assert!((sum - 40.0).abs() < 0.001);
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
        assert!(!tx.id.0.is_empty());
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
    async fn soft_delete_holding_preserves_transactions() {
        // Soft-delete: holding disappears from queries but its transactions are
        // retained so cost-basis history can be reconstructed.
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
        // Holding no longer visible in the active list
        let all = get_all_holdings(&pool).await.expect("get all");
        assert!(all.iter().all(|h| h.id != holding.id));
        // Transactions are still retrievable (history preserved)
        let txs = get_transactions_for_holding(&pool, &holding.id)
            .await
            .expect("get txs");
        assert_eq!(txs.len(), 1);
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
    async fn get_account_created_at_returns_matching_timestamp() {
        let pool = open_test_db().await;
        insert_account(&pool, "acc-1", "My TFSA", "tfsa", Some("Questrade"))
            .await
            .expect("insert");
        let accounts = get_accounts(&pool).await.expect("get accounts");
        let expected = accounts
            .iter()
            .find(|a| a.id == "acc-1")
            .unwrap()
            .created_at
            .clone();
        let result = get_account_created_at(&pool, "acc-1")
            .await
            .expect("lookup");
        assert_eq!(result, Some(expected));
    }

    #[tokio::test]
    async fn get_account_created_at_returns_none_for_missing_id() {
        let pool = open_test_db().await;
        let result = get_account_created_at(&pool, "does-not-exist")
            .await
            .expect("lookup");
        assert_eq!(result, None);
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

    // ── Pagination boundary tests ─────────────────────────────────────────────

    #[tokio::test]
    async fn pagination_page_beyond_total_returns_empty() {
        // Insert 3 holdings, request page 3 with page_size 5 (offset would be 10).
        let pool = open_test_db().await;
        insert_holding(&pool, make_input("AAA"))
            .await
            .expect("insert");
        insert_holding(&pool, make_input("BBB"))
            .await
            .expect("insert");
        insert_holding(&pool, make_input("CCC"))
            .await
            .expect("insert");

        // page=3, page_size=5 → offset=(3-1)*5=10, beyond 3 rows
        let result = get_holdings_paginated(&pool, 3, 5)
            .await
            .expect("paginated query should not error");

        assert!(
            result.items.is_empty(),
            "items should be empty for page beyond total; got {} items",
            result.items.len()
        );
        assert_eq!(result.total, 3, "total count should still be 3");
    }

    #[tokio::test]
    async fn pagination_page_size_one() {
        // Insert 3 holdings, fetch one per page.
        let pool = open_test_db().await;
        insert_holding(&pool, make_input("P1"))
            .await
            .expect("insert");
        insert_holding(&pool, make_input("P2"))
            .await
            .expect("insert");
        insert_holding(&pool, make_input("P3"))
            .await
            .expect("insert");

        for page in 1..=3i64 {
            let result = get_holdings_paginated(&pool, page, 1)
                .await
                .expect("paginated query should not error");
            assert_eq!(
                result.items.len(),
                1,
                "page {} should return exactly 1 item",
                page
            );
        }
    }

    #[tokio::test]
    async fn pagination_exact_boundary() {
        // Insert exactly 5 holdings.
        let pool = open_test_db().await;
        for sym in ["E1", "E2", "E3", "E4", "E5"] {
            insert_holding(&pool, make_input(sym))
                .await
                .expect("insert");
        }

        // First page: expect all 5 items.
        let first = get_holdings_paginated(&pool, 1, 5)
            .await
            .expect("first page");
        assert_eq!(
            first.items.len(),
            5,
            "first page should return 5 items; got {}",
            first.items.len()
        );

        // Second page: expect 0 items.
        let second = get_holdings_paginated(&pool, 2, 5)
            .await
            .expect("second page");
        assert!(
            second.items.is_empty(),
            "second page should be empty; got {} items",
            second.items.len()
        );
    }

    // ── Account fallback tests ────────────────────────────────────────────────

    #[tokio::test]
    async fn account_fallback_no_matching_type_leaves_account_id_none() {
        // Insert a holding with account type 'tfsa' but no accounts table row for tfsa.
        // The holding should be inserted without error, account_id stays None.
        let pool = open_test_db().await;
        let input = HoldingInput {
            account: AccountType::Tfsa,
            account_id: None,
            ..make_input("NOBKT")
        };
        let holding = insert_holding(&pool, input)
            .await
            .expect("insert should succeed even without matching account");

        assert!(
            holding.account_id.is_none(),
            "account_id should remain None when no matching account type exists"
        );
    }

    #[tokio::test]
    async fn account_fallback_multiple_same_type_assigns_earliest() {
        // Insert 2 accounts of the same type at different timestamps.
        // A holding inserted without account_id should be assigned to the earlier one.
        let pool = open_test_db().await;

        // Insert first account (earlier created_at ensured by sequential inserts)
        insert_account(&pool, "acc-first", "First RRSP", "rrsp", None)
            .await
            .expect("insert acc-first");
        // Small delay via distinct timestamp is not guaranteed in-memory, so we
        // explicitly set created_at by inserting in known order and relying on
        // the ORDER BY created_at ASC in the fallback query.
        insert_account(&pool, "acc-second", "Second RRSP", "rrsp", None)
            .await
            .expect("insert acc-second");

        let input = HoldingInput {
            account: AccountType::Rrsp,
            account_id: None,
            ..make_input("RRSPHOLD")
        };
        let holding = insert_holding(&pool, input).await.expect("insert holding");

        assert_eq!(
            holding.account_id.as_deref(),
            Some("acc-first"),
            "holding should be assigned to the earliest-created account of matching type"
        );
    }

    #[tokio::test]
    async fn dividends_remain_visible_after_holding_soft_deleted() {
        // Regression guard for #673: transactions stay visible for a
        // soft-deleted holding (get_all_transactions doesn't join/filter on
        // holdings.deleted_at), but dividends previously vanished because
        // get_dividends joined holdings and filtered `h.deleted_at IS NULL`.
        // Align dividends with the existing transaction behavior.
        let pool = open_test_db().await;
        let holding = insert_holding(&pool, make_input("DIVSOFT"))
            .await
            .expect("insert holding");

        insert_dividend(
            &pool,
            DividendInput {
                holding_id: holding.id.clone(),
                amount_per_unit: 1.5,
                currency: "CAD".to_string(),
                ex_date: "2024-01-01".to_string(),
                pay_date: "2024-01-15".to_string(),
            },
            &holding.symbol,
        )
        .await
        .expect("insert dividend");

        delete_holding(&pool, &holding.id)
            .await
            .expect("soft-delete holding");

        let dividends = get_dividends(&pool).await.expect("get_dividends");
        assert_eq!(
            dividends.len(),
            1,
            "dividend should remain visible after its holding is soft-deleted, matching transaction behavior"
        );
    }
}
