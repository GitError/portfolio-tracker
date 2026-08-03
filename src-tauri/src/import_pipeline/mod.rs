//! Import Plus Insights backend pipeline: read file -> detect format ->
//! extract sections -> infer columns -> normalize rows -> build import plan
//! -> commit selected clean rows -> return summary and insights.
//!
//! See docs/superpowers/specs/2026-05-24-import-plus-insights-design.md.
//! This is a new, separate pipeline from the legacy `parse_import_rows`
//! path in `crate::csv` (still used by `import_holdings_csv`/
//! `preview_import_csv`), which remains as a compatibility path.

pub mod aliases;
pub mod commit;
pub mod normalize;
pub mod parser;

use std::collections::HashSet;

use sqlx::SqlitePool;

use crate::db;
use crate::error::AppError;
use crate::types::{ColumnMapping, ImportContext, ImportPlan, NormalizedImportRow, RowAction};

pub use commit::commit_import_rows;

fn read_import_file(file_path: &str) -> Result<String, AppError> {
    let path = std::fs::canonicalize(file_path)
        .map_err(|e| AppError::Validation(format!("Cannot resolve file path: {e}")))?;
    let metadata = std::fs::metadata(&path)
        .map_err(|e| AppError::Validation(format!("Cannot read file: {e}")))?;
    if metadata.len() > crate::config::MAX_IMPORT_FILE_BYTES {
        return Err(AppError::Validation(format!(
            "Import file exceeds the maximum size of {} bytes",
            crate::config::MAX_IMPORT_FILE_BYTES
        )));
    }
    std::fs::read_to_string(&path)
        .map_err(|e| AppError::Validation(format!("Cannot read file: {e}")))
}

/// Builds column mappings for a set of headers, tagging validated
/// multi-section headers with "profile" confidence and everything else with
/// "alias" confidence (or "unmapped" when no canonical field is known).
/// User-selected `column_overrides` always win, with "user" confidence.
fn build_column_mappings(
    headers: &[String],
    profile: &str,
    overrides: &std::collections::HashMap<String, String>,
) -> Vec<ColumnMapping> {
    headers
        .iter()
        .map(|header| {
            if let Some(canonical) = overrides.get(header) {
                return ColumnMapping {
                    source_header: header.clone(),
                    canonical_field: Some(canonical.clone()),
                    confidence: "user".to_string(),
                    reason: format!("User-selected mapping: '{header}' -> {canonical}"),
                };
            }
            match aliases::canonical_field(header) {
                Some(field) => {
                    let confidence = if profile == parser::PROFILE_CANADIAN_BANK_MULTI_SECTION {
                        "profile"
                    } else {
                        "alias"
                    };
                    ColumnMapping {
                        source_header: header.clone(),
                        canonical_field: Some(field.to_string()),
                        confidence: confidence.to_string(),
                        reason: format!("Matched alias registry: '{header}' -> {field}"),
                    }
                }
                None => ColumnMapping {
                    source_header: header.clone(),
                    canonical_field: None,
                    confidence: "unmapped".to_string(),
                    reason: format!("No known canonical field for '{header}'; ignored"),
                },
            }
        })
        .collect()
}

/// Marks the second and later occurrences of a resolved symbol within the
/// same import file as `Skip` (a "duplicate" row, per the design doc), never
/// overriding a row that's already `NeedsFix`.
fn mark_intra_file_duplicates(rows: &mut [NormalizedImportRow]) {
    let mut seen: HashSet<String> = HashSet::new();
    for row in rows.iter_mut() {
        if row.action == RowAction::NeedsFix {
            continue;
        }
        let Some(symbol) = row.resolved_symbol.clone() else {
            continue;
        };
        let key = symbol.to_uppercase();
        if seen.contains(&key) {
            row.action = RowAction::Skip;
            row.warnings.push(format!(
                "Duplicate symbol '{symbol}' already appears earlier in this file; skipped"
            ));
        } else {
            seen.insert(key);
        }
    }
}

/// Reclassifies `Create` rows as `Update` when a holding with the same
/// symbol already exists in the target account. Requires `account_id` to be
/// known — a not-yet-created account has no existing holdings to match
/// against, so every row stays `Create`.
async fn classify_against_existing(
    pool: &SqlitePool,
    rows: &mut [NormalizedImportRow],
    account_id: Option<&str>,
) -> Result<(), AppError> {
    let Some(account_id) = account_id else {
        return Ok(());
    };
    let existing = db::get_all_holdings(pool).await?;
    let existing_symbols: HashSet<String> = existing
        .iter()
        .filter(|h| h.account_id.as_deref() == Some(account_id))
        .map(|h| h.symbol.to_uppercase())
        .collect();
    for row in rows.iter_mut() {
        if row.action != RowAction::Create {
            continue;
        }
        if let Some(symbol) = row.resolved_symbol.as_deref() {
            if existing_symbols.contains(&symbol.to_uppercase()) {
                row.action = RowAction::Update;
            }
        }
    }
    Ok(())
}

fn count_actions(
    rows: &[NormalizedImportRow],
    cash_rows: &[NormalizedImportRow],
) -> (usize, usize, usize, usize, usize) {
    let mut create = 0;
    let mut update = 0;
    let mut skip = 0;
    let mut needs_fix = 0;
    let mut warning = 0;
    for row in rows.iter().chain(cash_rows.iter()) {
        match row.action {
            RowAction::Create => create += 1,
            RowAction::Update => update += 1,
            RowAction::Skip => skip += 1,
            RowAction::NeedsFix => needs_fix += 1,
            RowAction::Warning => warning += 1,
        }
    }
    (create, update, skip, needs_fix, warning)
}

/// Reads `file_path`, detects its format, normalizes every row, and
/// classifies each row's proposed action. Never writes to the DB.
pub async fn build_import_plan(
    pool: &SqlitePool,
    file_path: &str,
    context: &ImportContext,
) -> Result<ImportPlan, AppError> {
    let content = read_import_file(file_path)?;
    let detected = parser::detect_and_parse(&content).map_err(AppError::Validation)?;

    let total_rows = detected.holdings_section.rows.len()
        + detected
            .cash_section
            .as_ref()
            .map(|s| s.rows.len())
            .unwrap_or(0);
    if total_rows > crate::config::MAX_IMPORT_ROWS {
        return Err(AppError::Validation(format!(
            "Import file has {total_rows} data rows, which exceeds the limit of {}",
            crate::config::MAX_IMPORT_ROWS
        )));
    }

    let mut column_mappings = build_column_mappings(
        &detected.holdings_section.headers,
        detected.profile,
        &context.column_overrides,
    );
    if let Some(cash) = &detected.cash_section {
        for mapping in
            build_column_mappings(&cash.headers, detected.profile, &context.column_overrides)
        {
            if !column_mappings
                .iter()
                .any(|m| m.source_header == mapping.source_header)
            {
                column_mappings.push(mapping);
            }
        }
    }

    let mut rows: Vec<NormalizedImportRow> = detected
        .holdings_section
        .rows
        .iter()
        .map(|raw| normalize::normalize_row(raw, context))
        .collect();
    let mut cash_rows: Vec<NormalizedImportRow> = detected
        .cash_section
        .as_ref()
        .map(|section| {
            section
                .rows
                .iter()
                .map(|raw| normalize::normalize_cash_row(raw, context))
                .collect()
        })
        .unwrap_or_default();

    mark_intra_file_duplicates(&mut rows);
    mark_intra_file_duplicates(&mut cash_rows);

    classify_against_existing(pool, &mut rows, context.account_id.as_deref()).await?;
    classify_against_existing(pool, &mut cash_rows, context.account_id.as_deref()).await?;

    let (count_create, count_update, count_skip, count_needs_fix, count_warning) =
        count_actions(&rows, &cash_rows);

    Ok(ImportPlan {
        profile_detected: detected.profile.to_string(),
        column_mappings,
        rows,
        count_create,
        count_update,
        count_skip,
        count_needs_fix,
        count_warning,
        suggested_account_type: detected.suggested_account_type,
        suggested_account_number: detected.suggested_account_number,
        cash_rows,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::db;

    fn context(account_id: Option<&str>) -> ImportContext {
        ImportContext {
            account_type: "rrsp".to_string(),
            account_name: Some("TD RRSP".to_string()),
            account_id: account_id.map(str::to_string),
            source_profile: None,
            column_overrides: HashMap::new(),
        }
    }

    const SAMPLE: &str = "Portfolio report for RRSP account # 12345 as of 2026-01-01T09:08:10\n\
\n\
Cash Details\n\
Currency,Account Type,Settled Cash,Trade Cash\n\
CAD,CASH,89192.24,89192.24\n\
\n\
Holding Details\n\
Asset Class,Sector,Security Description,Symbol,Quantity,Average Cost,Average Cost Currency\n\
Equity,Information Tech.,APPLE INC,AAPL:US,100,135.5045,USD\n\
\n\
\n\
Exchange Rate: 1 CAD = 0.7126USD  1 USD = 1.4033CAD\n";

    #[tokio::test]
    async fn build_plan_from_multi_section_sample_produces_expected_counts() {
        let pool = db::open_test_db().await;
        let dir = std::env::temp_dir().join(format!("import-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("sample.csv");
        std::fs::write(&path, SAMPLE).unwrap();

        let plan = build_import_plan(&pool, path.to_str().unwrap(), &context(None))
            .await
            .expect("plan should build");

        assert_eq!(
            plan.profile_detected,
            parser::PROFILE_CANADIAN_BANK_MULTI_SECTION
        );
        assert_eq!(plan.rows.len(), 1);
        assert_eq!(plan.cash_rows.len(), 1);
        assert_eq!(plan.count_create, 2);
        assert_eq!(plan.suggested_account_type, Some("RRSP".to_string()));
        assert_eq!(plan.suggested_account_number, Some("12345".to_string()));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn existing_holding_in_target_account_is_classified_as_update() {
        let pool = db::open_test_db().await;
        let account_id = "acct-1";
        db::insert_account(&pool, account_id, "TD RRSP", "rrsp", None)
            .await
            .expect("insert account");
        db::insert_holding(
            &pool,
            crate::types::HoldingInput {
                symbol: "AAPL".to_string(),
                name: "Apple Inc.".to_string(),
                asset_type: crate::types::AssetType::Stock,
                account: crate::types::AccountType::Rrsp,
                account_id: Some(account_id.to_string()),
                quantity: 50.0,
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
        .expect("insert holding");

        let dir = std::env::temp_dir().join(format!("import-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("sample.csv");
        std::fs::write(&path, SAMPLE).unwrap();

        let plan = build_import_plan(&pool, path.to_str().unwrap(), &context(Some(account_id)))
            .await
            .expect("plan should build");

        assert_eq!(plan.rows[0].action, RowAction::Update);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn duplicate_symbol_within_file_is_marked_skip() {
        let pool = db::open_test_db().await;
        let content = "Symbol,Asset Class,Quantity,Average Cost,Currency\n\
                        AAPL,Equity,10,100,USD\n\
                        AAPL,Equity,5,110,USD\n";
        let dir = std::env::temp_dir().join(format!("import-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("sample.csv");
        std::fs::write(&path, content).unwrap();

        let plan = build_import_plan(&pool, path.to_str().unwrap(), &context(None))
            .await
            .expect("plan should build");

        assert_eq!(plan.rows.len(), 2);
        assert_eq!(plan.rows[0].action, RowAction::Create);
        assert_eq!(plan.rows[1].action, RowAction::Skip);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn nonexistent_file_path_returns_validation_error() {
        let pool = db::open_test_db().await;
        let err = build_import_plan(&pool, "/no/such/file.csv", &context(None))
            .await
            .expect_err("missing file should error");
        assert!(matches!(err, AppError::Validation(_)));
    }
}
