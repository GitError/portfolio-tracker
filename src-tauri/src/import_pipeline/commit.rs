//! Commits the rows a user selected from an `ImportPlan` to the database,
//! and computes the insight deltas shown in the post-import summary panel.

use std::collections::{HashMap, HashSet};
use std::str::FromStr;

use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

use crate::db;
use crate::error::AppError;
use crate::types::{
    AccountType, AssetType, Holding, HoldingInput, ImportCommitRequest, ImportCommitResult,
    RowAction,
};

/// Mirrors `portfolio_core::snapshot`'s private staleness check (24 h) —
/// duplicated locally rather than made `pub` across the crate boundary for
/// what is a small, self-contained calculation.
const PRICE_STALE_SECS: i64 = 24 * 3600;

fn is_price_stale(updated_at: &str) -> bool {
    DateTime::parse_from_rfc3339(updated_at)
        .ok()
        .map(|t| {
            Utc::now()
                .signed_duration_since(t.with_timezone(&Utc))
                .num_seconds()
                > PRICE_STALE_SECS
        })
        .unwrap_or(true)
}

fn empty_result() -> ImportCommitResult {
    ImportCommitResult {
        created: 0,
        updated: 0,
        skipped: 0,
        errors: Vec::new(),
        new_symbols: Vec::new(),
        changed_symbols: Vec::new(),
        missing_from_import: Vec::new(),
        stale_symbols: Vec::new(),
    }
}

/// Commits the rows selected by the user (`request.plan_rows`) to the DB.
///
/// Rows whose `action` is `Skip` or `NeedsFix` are counted as skipped and
/// never written. Rows whose resolved `asset_type` doesn't map to one of the
/// app's four persisted asset types (`stock`/`etf`/`crypto`/`cash` — there is
/// no "Other" asset type in the schema today, see
/// `import_pipeline::aliases::map_asset_class`) are reported as a per-row
/// error and skipped, rather than silently miscategorized or panicking.
/// Cash rows are only committed when `include_cash` is set.
///
/// Each row is committed independently (not inside one all-or-nothing
/// transaction, unlike the legacy CSV importer): a failure on one row is
/// recorded in `errors` and does not prevent the remaining rows from being
/// committed.
pub async fn commit_import_rows(
    pool: &SqlitePool,
    request: &ImportCommitRequest,
) -> Result<ImportCommitResult, AppError> {
    let mut result = empty_result();

    let existing_holdings = db::get_all_holdings(pool).await?;
    let existing_by_symbol: HashMap<String, &Holding> = existing_holdings
        .iter()
        .filter(|h| h.account_id.as_deref() == Some(request.account_id.as_str()))
        .map(|h| (h.symbol.to_uppercase(), h))
        .collect();

    let mut committed_symbols: HashSet<String> = HashSet::new();
    // Guards against two rows in the same request resolving to the same
    // symbol (e.g. a duplicate the plan flagged as `Skip` gets flipped back
    // on by the client): `existing_by_symbol` is a snapshot taken once
    // before this loop and is never updated as rows are written, so without
    // this a second matching row would independently miss it and insert a
    // duplicate holding rather than being treated as an update.
    let mut attempted_symbols: HashSet<String> = HashSet::new();
    let symbols_in_request: HashSet<String> = request
        .plan_rows
        .iter()
        .filter_map(|r| r.resolved_symbol.clone().or_else(|| r.symbol.clone()))
        .map(|s| s.to_uppercase())
        .collect();

    for row in &request.plan_rows {
        if !matches!(
            row.action,
            RowAction::Create | RowAction::Update | RowAction::Warning
        ) {
            result.skipped += 1;
            continue;
        }

        let Some(asset_type_str) = row.asset_type.as_deref() else {
            result.errors.push(format!(
                "Row {}: missing asset type; skipped",
                row.row_number
            ));
            result.skipped += 1;
            continue;
        };

        if asset_type_str.eq_ignore_ascii_case("cash") && !request.include_cash {
            result.skipped += 1;
            continue;
        }

        let Ok(asset_type) = AssetType::from_str(&asset_type_str.to_lowercase()) else {
            result.errors.push(format!(
                "Row {}: asset type '{}' is not yet supported for holdings (no live pricing model); skipped",
                row.row_number, asset_type_str
            ));
            result.skipped += 1;
            continue;
        };

        let (Some(symbol), Some(quantity), Some(cost_basis), Some(currency)) = (
            row.resolved_symbol.clone().or_else(|| row.symbol.clone()),
            row.quantity,
            row.cost_basis,
            row.currency.clone(),
        ) else {
            result.errors.push(format!(
                "Row {}: missing required field(s) at commit time; skipped",
                row.row_number
            ));
            result.skipped += 1;
            continue;
        };

        if quantity < 0.0 {
            result.errors.push(format!(
                "Row {}: negative quantity ({}) — short positions are not supported; skipped",
                row.row_number, quantity
            ));
            result.skipped += 1;
            continue;
        }

        let symbol_key = symbol.to_uppercase();
        if !attempted_symbols.insert(symbol_key.clone()) {
            result.errors.push(format!(
                "Row {}: duplicate symbol '{}' already processed earlier in this commit request; skipped",
                row.row_number, symbol
            ));
            result.skipped += 1;
            continue;
        }

        let name = row
            .name
            .clone()
            .filter(|n| !n.trim().is_empty())
            .unwrap_or_else(|| symbol.clone());
        let account =
            AccountType::from_str(&row.account_type.to_lowercase()).unwrap_or(AccountType::Other);

        if let Some(existing) = existing_by_symbol.get(&symbol_key) {
            let quantity_changed = (existing.quantity - quantity).abs() > f64::EPSILON;
            let cost_basis_changed = (existing.cost_basis - cost_basis).abs() > f64::EPSILON;
            let updated = Holding {
                id: existing.id.clone(),
                symbol: symbol.clone(),
                name,
                asset_type,
                account,
                account_id: Some(request.account_id.clone()),
                account_name: existing.account_name.clone(),
                quantity,
                cost_basis,
                currency,
                exchange: row
                    .exchange
                    .clone()
                    .unwrap_or_else(|| existing.exchange.clone()),
                target_weight: row.target_weight.or(existing.target_weight),
                created_at: existing.created_at.clone(),
                updated_at: existing.updated_at.clone(),
                indicated_annual_dividend: existing.indicated_annual_dividend,
                indicated_annual_dividend_currency: existing
                    .indicated_annual_dividend_currency
                    .clone(),
                dividend_frequency: existing.dividend_frequency.clone(),
                maturity_date: existing.maturity_date.clone(),
            };
            match db::update_holding(pool, updated).await {
                Ok(_) => {
                    result.updated += 1;
                    committed_symbols.insert(symbol_key);
                    if quantity_changed || cost_basis_changed {
                        result.changed_symbols.push(symbol);
                    }
                }
                Err(e) => {
                    result.errors.push(format!(
                        "Row {}: failed to update {}: {}",
                        row.row_number, symbol, e
                    ));
                    result.skipped += 1;
                }
            }
        } else {
            let input = HoldingInput {
                symbol: symbol.clone(),
                name,
                asset_type,
                account,
                account_id: Some(request.account_id.clone()),
                quantity,
                cost_basis,
                currency,
                exchange: row.exchange.clone().unwrap_or_default(),
                target_weight: row.target_weight,
                indicated_annual_dividend: None,
                indicated_annual_dividend_currency: None,
                dividend_frequency: None,
                maturity_date: None,
            };
            match db::insert_holding(pool, input).await {
                Ok(_) => {
                    result.created += 1;
                    committed_symbols.insert(symbol_key);
                    result.new_symbols.push(symbol);
                }
                Err(e) => {
                    result.errors.push(format!(
                        "Row {}: failed to create {}: {}",
                        row.row_number, symbol, e
                    ));
                    result.skipped += 1;
                }
            }
        }
    }

    // Existing holdings in the target account whose symbol never appeared
    // anywhere in the submitted plan rows — review candidates only, never
    // auto-deleted.
    for (symbol_key, holding) in &existing_by_symbol {
        if !symbols_in_request.contains(symbol_key) {
            result.missing_from_import.push(holding.symbol.clone());
        }
    }

    // Stale/unpriced symbols among what was actually committed this run.
    if !committed_symbols.is_empty() {
        let cached_prices = db::get_cached_prices(pool).await?;
        let price_by_symbol: HashMap<String, &crate::types::PriceData> = cached_prices
            .iter()
            .map(|p| (p.symbol.to_uppercase(), p))
            .collect();
        for symbol_key in &committed_symbols {
            if symbol_key.ends_with("-CASH") {
                continue; // cash never has a live price
            }
            match price_by_symbol.get(symbol_key) {
                None => result.stale_symbols.push(symbol_key.clone()),
                Some(price) if is_price_stale(&price.updated_at) => {
                    result.stale_symbols.push(symbol_key.clone())
                }
                Some(_) => {}
            }
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{NormalizedImportRow, PriceData};

    fn base_row(row_number: usize) -> NormalizedImportRow {
        NormalizedImportRow {
            row_number,
            action: RowAction::Create,
            symbol: Some("AAPL".to_string()),
            resolved_symbol: Some("AAPL".to_string()),
            name: Some("Apple Inc.".to_string()),
            asset_type: Some("Stock".to_string()),
            quantity: Some(10.0),
            cost_basis: Some(150.0),
            cost_basis_source: Some("average_cost".to_string()),
            currency: Some("USD".to_string()),
            book_value: None,
            market_value: None,
            exchange: None,
            target_weight: None,
            account_type: "taxable".to_string(),
            account_name: None,
            warnings: Vec::new(),
            errors: Vec::new(),
            raw: HashMap::new(),
            dividend_yield: None,
            annualized_income: None,
            ex_dividend_date: None,
        }
    }

    #[tokio::test]
    async fn commit_creates_a_new_holding() {
        let pool = db::open_test_db().await;
        db::insert_account(&pool, "acct-1", "Taxable", "taxable", None)
            .await
            .unwrap();

        let request = ImportCommitRequest {
            plan_rows: vec![base_row(8)],
            account_id: "acct-1".to_string(),
            include_cash: false,
        };
        let result = commit_import_rows(&pool, &request).await.expect("commit");

        assert_eq!(result.created, 1);
        assert_eq!(result.updated, 0);
        assert_eq!(result.new_symbols, vec!["AAPL".to_string()]);

        let holdings = db::get_all_holdings(&pool).await.unwrap();
        assert_eq!(holdings.len(), 1);
        assert_eq!(holdings[0].symbol, "AAPL");
    }

    #[tokio::test]
    async fn commit_updates_existing_holding_and_reports_changed_symbol() {
        let pool = db::open_test_db().await;
        db::insert_account(&pool, "acct-1", "Taxable", "taxable", None)
            .await
            .unwrap();
        db::insert_holding(
            &pool,
            HoldingInput {
                symbol: "AAPL".to_string(),
                name: "Apple Inc.".to_string(),
                asset_type: AssetType::Stock,
                account: AccountType::Taxable,
                account_id: Some("acct-1".to_string()),
                quantity: 5.0,
                cost_basis: 100.0,
                currency: "USD".to_string(),
                exchange: String::new(),
                target_weight: None,
                indicated_annual_dividend: None,
                indicated_annual_dividend_currency: None,
                dividend_frequency: None,
                maturity_date: None,
            },
        )
        .await
        .unwrap();

        let mut row = base_row(8);
        row.action = RowAction::Update;
        row.quantity = Some(10.0);
        let request = ImportCommitRequest {
            plan_rows: vec![row],
            account_id: "acct-1".to_string(),
            include_cash: false,
        };
        let result = commit_import_rows(&pool, &request).await.expect("commit");

        assert_eq!(result.updated, 1);
        assert_eq!(result.created, 0);
        assert_eq!(result.changed_symbols, vec!["AAPL".to_string()]);
    }

    #[tokio::test]
    async fn commit_creates_a_new_holding_with_exchange_and_target_weight_from_the_row() {
        let pool = db::open_test_db().await;
        db::insert_account(&pool, "acct-1", "Taxable", "taxable", None)
            .await
            .unwrap();

        let mut row = base_row(8);
        row.exchange = Some("NASDAQ".to_string());
        row.target_weight = Some(12.5);
        let request = ImportCommitRequest {
            plan_rows: vec![row],
            account_id: "acct-1".to_string(),
            include_cash: false,
        };
        let result = commit_import_rows(&pool, &request).await.expect("commit");
        assert_eq!(result.created, 1);

        let holdings = db::get_all_holdings(&pool).await.unwrap();
        assert_eq!(holdings[0].exchange, "NASDAQ");
        assert_eq!(holdings[0].target_weight, Some(12.5));
    }

    #[tokio::test]
    async fn commit_update_writes_exchange_and_target_weight_when_row_provides_them() {
        let pool = db::open_test_db().await;
        db::insert_account(&pool, "acct-1", "Taxable", "taxable", None)
            .await
            .unwrap();
        db::insert_holding(
            &pool,
            HoldingInput {
                symbol: "AAPL".to_string(),
                name: "Apple Inc.".to_string(),
                asset_type: AssetType::Stock,
                account: AccountType::Taxable,
                account_id: Some("acct-1".to_string()),
                quantity: 5.0,
                cost_basis: 100.0,
                currency: "USD".to_string(),
                exchange: String::new(),
                target_weight: None,
                indicated_annual_dividend: None,
                indicated_annual_dividend_currency: None,
                dividend_frequency: None,
                maturity_date: None,
            },
        )
        .await
        .unwrap();

        let mut row = base_row(8);
        row.action = RowAction::Update;
        row.exchange = Some("NYSE".to_string());
        row.target_weight = Some(8.0);
        let request = ImportCommitRequest {
            plan_rows: vec![row],
            account_id: "acct-1".to_string(),
            include_cash: false,
        };
        let result = commit_import_rows(&pool, &request).await.expect("commit");
        assert_eq!(result.updated, 1);

        let holdings = db::get_all_holdings(&pool).await.unwrap();
        assert_eq!(holdings[0].exchange, "NYSE");
        assert_eq!(holdings[0].target_weight, Some(8.0));
    }

    #[tokio::test]
    async fn commit_update_preserves_existing_exchange_and_target_weight_when_row_omits_them() {
        let pool = db::open_test_db().await;
        db::insert_account(&pool, "acct-1", "Taxable", "taxable", None)
            .await
            .unwrap();
        db::insert_holding(
            &pool,
            HoldingInput {
                symbol: "AAPL".to_string(),
                name: "Apple Inc.".to_string(),
                asset_type: AssetType::Stock,
                account: AccountType::Taxable,
                account_id: Some("acct-1".to_string()),
                quantity: 5.0,
                cost_basis: 100.0,
                currency: "USD".to_string(),
                exchange: "NASDAQ".to_string(),
                target_weight: Some(20.0),
                indicated_annual_dividend: None,
                indicated_annual_dividend_currency: None,
                dividend_frequency: None,
                maturity_date: None,
            },
        )
        .await
        .unwrap();

        let mut row = base_row(8);
        row.action = RowAction::Update;
        row.quantity = Some(10.0);
        let request = ImportCommitRequest {
            plan_rows: vec![row],
            account_id: "acct-1".to_string(),
            include_cash: false,
        };
        let result = commit_import_rows(&pool, &request).await.expect("commit");
        assert_eq!(result.updated, 1);

        let holdings = db::get_all_holdings(&pool).await.unwrap();
        assert_eq!(holdings[0].exchange, "NASDAQ");
        assert_eq!(holdings[0].target_weight, Some(20.0));
    }

    #[tokio::test]
    async fn commit_skips_needs_fix_and_skip_rows() {
        let pool = db::open_test_db().await;
        db::insert_account(&pool, "acct-1", "Taxable", "taxable", None)
            .await
            .unwrap();

        let mut needs_fix = base_row(8);
        needs_fix.action = RowAction::NeedsFix;
        let mut skip = base_row(9);
        skip.action = RowAction::Skip;

        let request = ImportCommitRequest {
            plan_rows: vec![needs_fix, skip],
            account_id: "acct-1".to_string(),
            include_cash: false,
        };
        let result = commit_import_rows(&pool, &request).await.expect("commit");

        assert_eq!(result.created, 0);
        assert_eq!(result.updated, 0);
        assert_eq!(result.skipped, 2);
    }

    #[tokio::test]
    async fn commit_rejects_unsupported_asset_type_other_with_error() {
        let pool = db::open_test_db().await;
        db::insert_account(&pool, "acct-1", "Taxable", "taxable", None)
            .await
            .unwrap();

        let mut row = base_row(8);
        row.action = RowAction::Warning;
        row.asset_type = Some("Other".to_string());
        let request = ImportCommitRequest {
            plan_rows: vec![row],
            account_id: "acct-1".to_string(),
            include_cash: false,
        };
        let result = commit_import_rows(&pool, &request).await.expect("commit");

        assert_eq!(result.created, 0);
        assert_eq!(result.skipped, 1);
        assert!(result
            .errors
            .iter()
            .any(|e| e.contains("not yet supported")));
    }

    #[tokio::test]
    async fn commit_excludes_cash_rows_when_include_cash_is_false() {
        let pool = db::open_test_db().await;
        db::insert_account(&pool, "acct-1", "Cash", "cash", None)
            .await
            .unwrap();

        let mut row = base_row(5);
        row.symbol = Some("CAD-CASH".to_string());
        row.resolved_symbol = Some("CAD-CASH".to_string());
        row.asset_type = Some("Cash".to_string());
        row.cost_basis = Some(1.0);
        row.currency = Some("CAD".to_string());

        let request = ImportCommitRequest {
            plan_rows: vec![row],
            account_id: "acct-1".to_string(),
            include_cash: false,
        };
        let result = commit_import_rows(&pool, &request).await.expect("commit");

        assert_eq!(result.created, 0);
        assert_eq!(result.skipped, 1);
    }

    #[tokio::test]
    async fn commit_rejects_negative_quantity_with_a_friendly_error_not_a_raw_db_error() {
        let pool = db::open_test_db().await;
        db::insert_account(&pool, "acct-1", "Taxable", "taxable", None)
            .await
            .unwrap();

        let mut row = base_row(8);
        row.action = RowAction::Warning;
        row.quantity = Some(-5.0);
        let request = ImportCommitRequest {
            plan_rows: vec![row],
            account_id: "acct-1".to_string(),
            include_cash: false,
        };
        let result = commit_import_rows(&pool, &request).await.expect("commit");

        assert_eq!(result.created, 0);
        assert_eq!(result.skipped, 1);
        assert!(result
            .errors
            .iter()
            .any(|e| e.contains("negative quantity") && !e.contains("CHECK")));
    }

    #[tokio::test]
    async fn commit_rejects_second_row_with_a_duplicate_symbol_in_the_same_request() {
        let pool = db::open_test_db().await;
        db::insert_account(&pool, "acct-1", "Taxable", "taxable", None)
            .await
            .unwrap();

        let mut second = base_row(9);
        second.quantity = Some(20.0);
        let request = ImportCommitRequest {
            plan_rows: vec![base_row(8), second],
            account_id: "acct-1".to_string(),
            include_cash: false,
        };
        let result = commit_import_rows(&pool, &request).await.expect("commit");

        assert_eq!(
            result.created, 1,
            "only the first occurrence should be created"
        );
        assert_eq!(result.skipped, 1);
        assert!(result.errors.iter().any(|e| e.contains("duplicate symbol")));

        let holdings = db::get_all_holdings(&pool).await.unwrap();
        assert_eq!(
            holdings.len(),
            1,
            "the duplicate must not create a second row"
        );
    }

    #[tokio::test]
    async fn missing_from_import_lists_existing_holdings_not_in_the_request() {
        let pool = db::open_test_db().await;
        db::insert_account(&pool, "acct-1", "Taxable", "taxable", None)
            .await
            .unwrap();
        db::insert_holding(
            &pool,
            HoldingInput {
                symbol: "MSFT".to_string(),
                name: "Microsoft".to_string(),
                asset_type: AssetType::Stock,
                account: AccountType::Taxable,
                account_id: Some("acct-1".to_string()),
                quantity: 5.0,
                cost_basis: 200.0,
                currency: "USD".to_string(),
                exchange: String::new(),
                target_weight: None,
                indicated_annual_dividend: None,
                indicated_annual_dividend_currency: None,
                dividend_frequency: None,
                maturity_date: None,
            },
        )
        .await
        .unwrap();

        let request = ImportCommitRequest {
            plan_rows: vec![base_row(8)], // only AAPL in this import
            account_id: "acct-1".to_string(),
            include_cash: false,
        };
        let result = commit_import_rows(&pool, &request).await.expect("commit");

        assert_eq!(result.missing_from_import, vec!["MSFT".to_string()]);
    }

    #[tokio::test]
    async fn stale_symbols_reported_when_no_cached_price_exists() {
        let pool = db::open_test_db().await;
        db::insert_account(&pool, "acct-1", "Taxable", "taxable", None)
            .await
            .unwrap();

        let request = ImportCommitRequest {
            plan_rows: vec![base_row(8)],
            account_id: "acct-1".to_string(),
            include_cash: false,
        };
        let result = commit_import_rows(&pool, &request).await.expect("commit");

        assert_eq!(result.stale_symbols, vec!["AAPL".to_string()]);
    }

    #[tokio::test]
    async fn fresh_cached_price_is_not_reported_as_stale() {
        let pool = db::open_test_db().await;
        db::insert_account(&pool, "acct-1", "Taxable", "taxable", None)
            .await
            .unwrap();
        db::upsert_price(
            &pool,
            &PriceData {
                symbol: "AAPL".to_string(),
                price: 150.0,
                currency: "USD".to_string(),
                change: 0.0,
                change_percent: 0.0,
                updated_at: Utc::now().to_rfc3339(),
                open: None,
                previous_close: None,
                volume: None,
            },
        )
        .await
        .unwrap();

        let request = ImportCommitRequest {
            plan_rows: vec![base_row(8)],
            account_id: "acct-1".to_string(),
            include_cash: false,
        };
        let result = commit_import_rows(&pool, &request).await.expect("commit");

        assert!(result.stale_symbols.is_empty());
    }
}
