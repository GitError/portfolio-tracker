use std::collections::HashSet;

use tauri::State;

use crate::csv::{build_holdings_csv, parse_import_rows};
use crate::db;
use crate::error::AppError;
use crate::types::{AssetType, HoldingInput, ImportError, ImportResult, PreviewImportResult, PreviewRow};

use super::{validate_symbol, DbState, HttpClient, WEIGHT_EPSILON};

#[tauri::command]
pub async fn export_holdings_csv(db: State<'_, DbState>) -> Result<String, AppError> {
    let pool = &db.0;
    let holdings = db::get_all_holdings(pool).await?;
    build_holdings_csv(&holdings).map_err(AppError::from)
}

#[tauri::command]
pub async fn import_holdings_csv(
    db: State<'_, DbState>,
    client: State<'_, HttpClient>,
    csv_content: String,
) -> Result<ImportResult, AppError> {
    let parsed_rows = parse_import_rows(&csv_content)?;

    let existing_keys: HashSet<(String, String)> = {
        let pool = &db.0;
        db::get_all_holdings(pool)
            .await?
            .into_iter()
            .map(|holding| {
                (
                    holding.symbol.to_uppercase(),
                    holding.account.as_str().to_string(),
                )
            })
            .collect()
    };

    let mut seen_keys = existing_keys;
    let mut pending_inputs = Vec::new();
    let mut skipped = Vec::new();

    for row in parsed_rows {
        let key = (row.symbol.to_uppercase(), row.account.as_str().to_string());
        if seen_keys.contains(&key) {
            skipped.push(ImportError {
                row: row.row,
                symbol: row.symbol,
                reason: "duplicate".to_string(),
            });
            continue;
        }

        if matches!(row.asset_type, AssetType::Cash) {
            seen_keys.insert((row.symbol.to_uppercase(), row.account.as_str().to_string()));
            pending_inputs.push(HoldingInput {
                symbol: row.symbol,
                name: if row.name.is_empty() {
                    format!("{} Cash", row.currency)
                } else {
                    row.name
                },
                asset_type: row.asset_type,
                account: row.account,
                account_id: None,
                quantity: row.quantity,
                cost_basis: row.cost_basis,
                currency: row.currency,
                exchange: row.exchange,
                target_weight: row.target_weight,
                indicated_annual_dividend: row.indicated_annual_dividend,
                indicated_annual_dividend_currency: row.indicated_annual_dividend_currency,
                dividend_frequency: row.dividend_frequency,
                maturity_date: row.maturity_date,
            });
            continue;
        }

        let validated = match validate_symbol(&db, &client, &row.symbol).await {
            Ok(Some(result)) => result,
            Ok(None) => {
                skipped.push(ImportError {
                    row: row.row,
                    symbol: row.symbol,
                    reason: "invalid_symbol".to_string(),
                });
                continue;
            }
            Err(_) => {
                skipped.push(ImportError {
                    row: row.row,
                    symbol: row.symbol,
                    reason: "validation_failed".to_string(),
                });
                continue;
            }
        };

        if !validated.currency.eq_ignore_ascii_case(&row.currency) {
            skipped.push(ImportError {
                row: row.row,
                symbol: row.symbol,
                reason: format!(
                    "currency_mismatch:{}_expected_{}",
                    row.currency,
                    validated.currency.to_uppercase()
                ),
            });
            continue;
        }

        seen_keys.insert((
            validated.symbol.to_uppercase(),
            row.account.as_str().to_string(),
        ));
        pending_inputs.push(HoldingInput {
            symbol: validated.symbol,
            name: if row.name.is_empty() {
                validated.name
            } else {
                row.name
            },
            asset_type: row.asset_type,
            account: row.account,
            account_id: None,
            quantity: row.quantity,
            cost_basis: row.cost_basis,
            currency: row.currency,
            exchange: if row.exchange.is_empty() {
                validated.exchange
            } else {
                row.exchange
            },
            target_weight: row.target_weight,
            indicated_annual_dividend: row.indicated_annual_dividend,
            indicated_annual_dividend_currency: row.indicated_annual_dividend_currency,
            dividend_frequency: row.dividend_frequency,
            maturity_date: row.maturity_date,
        });
    }

    // Weight validation runs after deduplication so that re-importing an existing
    // portfolio (all rows skipped as duplicates) never triggers a false overflow.
    // All pending inputs (cash and non-cash alike) are included in this sum.
    let import_weight_sum: f64 = pending_inputs.iter().map(|h| h.target_weight).sum();
    if import_weight_sum > 100.0 + WEIGHT_EPSILON {
        return Err(AppError::Validation(format!(
            "Combined target weights ({:.2}%) exceed 100%",
            import_weight_sum
        )));
    }
    let existing_weight_sum = {
        let pool = &db.0;
        db::sum_target_weights(pool, None).await?
    };
    if existing_weight_sum + import_weight_sum > 100.0 + WEIGHT_EPSILON {
        return Err(AppError::Validation(format!(
            "Import failed: total target weight would reach {:.1}% (existing portfolio is already {:.1}%). Adjust weights before re-importing.",
            existing_weight_sum + import_weight_sum,
            existing_weight_sum
        )));
    }

    let mut imported = Vec::new();
    {
        let pool = &db.0;
        let mut tx = pool.begin().await.map_err(AppError::from)?;
        for input in pending_inputs {
            match db::insert_holding_in_tx(&mut tx, input).await {
                Ok(holding) => imported.push(holding),
                Err(e) => {
                    tx.rollback().await.map_err(AppError::from)?;
                    return Err(AppError::from(e));
                }
            }
        }
        tx.commit().await.map_err(AppError::from)?;
    }

    Ok(ImportResult {
        total_rows: imported.len() + skipped.len(),
        imported,
        skipped,
    })
}

#[tauri::command]
pub async fn preview_import_csv(
    db: State<'_, DbState>,
    client: State<'_, HttpClient>,
    csv_content: String,
) -> Result<PreviewImportResult, AppError> {
    let parsed_rows = parse_import_rows(&csv_content)?;
    let existing_keys: HashSet<(String, String)> = {
        let pool = &db.0;
        db::get_all_holdings(pool)
            .await?
            .into_iter()
            .map(|h| (h.symbol.to_uppercase(), h.account.as_str().to_string()))
            .collect()
    };

    let mut preview_rows: Vec<PreviewRow> = Vec::new();
    let mut seen: HashSet<(String, String)> = existing_keys;

    for row in parsed_rows {
        let row_key = (row.symbol.to_uppercase(), row.account.as_str().to_string());

        if seen.contains(&row_key) {
            preview_rows.push(PreviewRow {
                row: row.row,
                original_symbol: row.symbol.clone(),
                resolved_symbol: row.symbol,
                name: row.name,
                asset_type: row.asset_type.as_str().to_string(),
                currency: row.currency,
                exchange: String::new(),
                quantity: row.quantity,
                cost_basis: row.cost_basis,
                target_weight: row.target_weight,
                status: "duplicate".to_string(),
            });
            continue;
        }

        if matches!(row.asset_type, AssetType::Cash) {
            seen.insert((row.symbol.to_uppercase(), row.account.as_str().to_string()));
            preview_rows.push(PreviewRow {
                row: row.row,
                original_symbol: row.symbol.clone(),
                resolved_symbol: row.symbol,
                name: if row.name.is_empty() {
                    format!("{} Cash", row.currency)
                } else {
                    row.name
                },
                asset_type: "cash".to_string(),
                currency: row.currency,
                exchange: String::new(),
                quantity: row.quantity,
                cost_basis: row.cost_basis,
                target_weight: row.target_weight,
                status: "ready".to_string(),
            });
            continue;
        }

        match validate_symbol(&db, &client, &row.symbol).await {
            Ok(Some(result)) => {
                seen.insert((
                    result.symbol.to_uppercase(),
                    row.account.as_str().to_string(),
                ));
                preview_rows.push(PreviewRow {
                    row: row.row,
                    original_symbol: row.symbol,
                    resolved_symbol: result.symbol,
                    name: if row.name.is_empty() {
                        result.name
                    } else {
                        row.name
                    },
                    asset_type: result.asset_type.as_str().to_string(),
                    currency: result.currency,
                    exchange: result.exchange,
                    quantity: row.quantity,
                    cost_basis: row.cost_basis,
                    target_weight: row.target_weight,
                    status: "ready".to_string(),
                });
            }
            Ok(None) => {
                preview_rows.push(PreviewRow {
                    row: row.row,
                    original_symbol: row.symbol,
                    resolved_symbol: String::new(),
                    name: row.name,
                    asset_type: row.asset_type.as_str().to_string(),
                    currency: row.currency,
                    exchange: String::new(),
                    quantity: row.quantity,
                    cost_basis: row.cost_basis,
                    target_weight: row.target_weight,
                    status: "invalid_symbol".to_string(),
                });
            }
            Err(_) => {
                preview_rows.push(PreviewRow {
                    row: row.row,
                    original_symbol: row.symbol,
                    resolved_symbol: String::new(),
                    name: row.name,
                    asset_type: row.asset_type.as_str().to_string(),
                    currency: row.currency,
                    exchange: String::new(),
                    quantity: row.quantity,
                    cost_basis: row.cost_basis,
                    target_weight: row.target_weight,
                    status: "validation_failed".to_string(),
                });
            }
        }
    }

    let ready_count = preview_rows.iter().filter(|r| r.status == "ready").count();
    let skip_count = preview_rows.len() - ready_count;

    Ok(PreviewImportResult {
        rows: preview_rows,
        ready_count,
        skip_count,
    })
}
