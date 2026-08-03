//! Integration tests for the Import Plus Insights pipeline
//! (`src-tauri/src/import_pipeline/`), exercised through its public API
//! (`portfolio_tracker_lib::import_pipeline::build_import_plan`) against a
//! realistic multi-section TD Direct Investing RRSP export fixture.
//!
//! See docs/superpowers/specs/2026-05-24-import-plus-insights-design.md.

use std::collections::HashMap;

use portfolio_tracker_lib::import_pipeline::{build_import_plan, parser};
use portfolio_tracker_lib::types::{ImportContext, RowAction};
use sqlx::sqlite::SqlitePool;

const FIXTURE: &str = include_str!("fixtures/td_rrsp_sample.csv");

fn context() -> ImportContext {
    ImportContext {
        account_type: "rrsp".to_string(),
        account_name: Some("TD RRSP".to_string()),
        // Every test here leaves `account_id` unset. `build_import_plan` only
        // queries the DB (via `classify_against_existing`) when an account id
        // is present, so a pool that's never actually queried is fine —
        // no migrations needed for these tests.
        account_id: None,
        source_profile: None,
        column_overrides: HashMap::new(),
    }
}

/// An unconnected-until-used in-memory pool. Never queried by these tests
/// since `context().account_id` is always `None`.
async fn unused_pool() -> SqlitePool {
    SqlitePool::connect("sqlite::memory:")
        .await
        .expect("in-memory sqlite pool")
}

async fn write_fixture(dir: &std::path::Path) -> std::path::PathBuf {
    let path = dir.join("td_rrsp_sample.csv");
    std::fs::write(&path, FIXTURE).expect("write fixture");
    path
}

#[tokio::test]
async fn test_detect_multi_section_format() {
    let pool = unused_pool().await;
    let dir = std::env::temp_dir().join(format!("import-it-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = write_fixture(&dir).await;

    let plan = build_import_plan(&pool, path.to_str().unwrap(), &context())
        .await
        .expect("plan should build");

    assert_eq!(
        plan.profile_detected,
        parser::PROFILE_CANADIAN_BANK_MULTI_SECTION
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn test_cash_rows_parsed() {
    let pool = unused_pool().await;
    let dir = std::env::temp_dir().join(format!("import-it-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = write_fixture(&dir).await;

    let plan = build_import_plan(&pool, path.to_str().unwrap(), &context())
        .await
        .expect("plan should build");

    assert_eq!(plan.cash_rows.len(), 2);

    let cad = plan
        .cash_rows
        .iter()
        .find(|r| r.currency.as_deref() == Some("CAD"))
        .expect("CAD cash row");
    assert_eq!(cad.resolved_symbol, Some("CAD-CASH".to_string()));
    assert_eq!(cad.quantity, Some(4521.90));
    assert_eq!(cad.action, RowAction::Create);

    let usd = plan
        .cash_rows
        .iter()
        .find(|r| r.currency.as_deref() == Some("USD"))
        .expect("USD cash row");
    assert_eq!(usd.resolved_symbol, Some("USD-CASH".to_string()));
    assert_eq!(usd.quantity, Some(1875.33));
    assert_eq!(usd.action, RowAction::Create);
}

#[tokio::test]
async fn test_symbol_country_resolution() {
    let pool = unused_pool().await;
    let dir = std::env::temp_dir().join(format!("import-it-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = write_fixture(&dir).await;

    let plan = build_import_plan(&pool, path.to_str().unwrap(), &context())
        .await
        .expect("plan should build");

    let aapl = plan
        .rows
        .iter()
        .find(|r| r.symbol.as_deref() == Some("AAPL:US"))
        .expect("AAPL row");
    assert_eq!(aapl.resolved_symbol, Some("AAPL".to_string()));

    let ry = plan
        .rows
        .iter()
        .find(|r| r.symbol.as_deref() == Some("RY:CA"))
        .expect("RY row");
    assert_eq!(ry.resolved_symbol, Some("RY.TO".to_string()));

    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn test_average_cost_is_per_unit() {
    let pool = unused_pool().await;
    let dir = std::env::temp_dir().join(format!("import-it-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = write_fixture(&dir).await;

    let plan = build_import_plan(&pool, path.to_str().unwrap(), &context())
        .await
        .expect("plan should build");

    let aapl = plan
        .rows
        .iter()
        .find(|r| r.symbol.as_deref() == Some("AAPL:US"))
        .expect("AAPL row");
    assert_eq!(aapl.cost_basis, Some(148.22));
    assert_eq!(aapl.cost_basis_source, Some("average_cost".to_string()));

    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn test_derive_cost_from_total_cost() {
    let pool = unused_pool().await;
    let dir = std::env::temp_dir().join(format!("import-it-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = write_fixture(&dir).await;

    let plan = build_import_plan(&pool, path.to_str().unwrap(), &context())
        .await
        .expect("plan should build");

    let shop = plan
        .rows
        .iter()
        .find(|r| r.symbol.as_deref() == Some("SHOP:CA"))
        .expect("SHOP row — Average Cost blank, Total Cost/Quantity present");
    assert_eq!(shop.cost_basis, Some(100.0));
    assert_eq!(
        shop.cost_basis_source,
        Some("derived:total_cost/qty".to_string())
    );
    assert_eq!(shop.action, RowAction::Create);

    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn test_blank_symbol_is_needs_fix() {
    let pool = unused_pool().await;
    let dir = std::env::temp_dir().join(format!("import-it-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = write_fixture(&dir).await;

    let plan = build_import_plan(&pool, path.to_str().unwrap(), &context())
        .await
        .expect("plan should build");

    let blank = plan
        .rows
        .iter()
        .find(|r| r.name.as_deref() == Some("UNKNOWN POSITION"))
        .expect("blank-symbol row");
    assert_eq!(blank.action, RowAction::NeedsFix);
    assert!(blank.symbol.is_none());
    assert!(!blank.errors.is_empty());
    assert!(blank.errors.iter().any(|e| e.contains("symbol")));

    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn test_fixed_income_is_warning() {
    let pool = unused_pool().await;
    let dir = std::env::temp_dir().join(format!("import-it-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = write_fixture(&dir).await;

    let plan = build_import_plan(&pool, path.to_str().unwrap(), &context())
        .await
        .expect("plan should build");

    let bond = plan
        .rows
        .iter()
        .find(|r| r.symbol.as_deref() == Some("CA135087J546"))
        .expect("bond row");
    assert_eq!(bond.action, RowAction::Warning);
    assert_eq!(bond.asset_type, Some("Other".to_string()));
    assert!(bond.warnings.iter().any(|w| w.contains("pricing")));

    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn test_column_mappings_surfaced() {
    let pool = unused_pool().await;
    let dir = std::env::temp_dir().join(format!("import-it-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = write_fixture(&dir).await;

    let plan = build_import_plan(&pool, path.to_str().unwrap(), &context())
        .await
        .expect("plan should build");

    let mapped_fields: Vec<&str> = plan
        .column_mappings
        .iter()
        .filter_map(|m| m.canonical_field.as_deref())
        .collect();

    assert!(mapped_fields.contains(&"symbol"), "{mapped_fields:?}");
    assert!(mapped_fields.contains(&"name"), "{mapped_fields:?}");
    assert!(mapped_fields.contains(&"quantity"), "{mapped_fields:?}");
    assert!(mapped_fields.contains(&"currency"), "{mapped_fields:?}");
    assert!(mapped_fields.contains(&"asset_type"), "{mapped_fields:?}");
    // The canonical registry has no standalone "cost_basis" field — per-unit
    // cost basis is fed by the "average_cost" mapping (see
    // `import_pipeline::normalize::derive_cost_basis`), which is what this
    // asserts is surfaced in the plan's column mappings.
    assert!(mapped_fields.contains(&"average_cost"), "{mapped_fields:?}");

    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn test_full_plan_counts() {
    let pool = unused_pool().await;
    let dir = std::env::temp_dir().join(format!("import-it-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = write_fixture(&dir).await;

    let plan = build_import_plan(&pool, path.to_str().unwrap(), &context())
        .await
        .expect("plan should build");

    // 6 create (4 clean holdings + 2 cash rows), 1 warning (Fixed Income
    // bond), 2 needs_fix (blank symbol, and Market-Value-only with no
    // derivable cost basis). No updates/skips — nothing pre-exists in the DB
    // and there are no intra-file duplicate symbols.
    assert_eq!(plan.count_create, 6);
    assert_eq!(plan.count_warning, 1);
    assert_eq!(plan.count_needs_fix, 2);
    assert_eq!(plan.count_update, 0);
    assert_eq!(plan.count_skip, 0);

    std::fs::remove_dir_all(&dir).ok();
}

/// Edge case audit: Market Value must never be used to derive cost basis,
/// even when it's the only value-like column present on the row.
#[tokio::test]
async fn test_market_value_never_used_for_cost_basis() {
    let pool = unused_pool().await;
    let dir = std::env::temp_dir().join(format!("import-it-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = write_fixture(&dir).await;

    let plan = build_import_plan(&pool, path.to_str().unwrap(), &context())
        .await
        .expect("plan should build");

    let mystery = plan
        .rows
        .iter()
        .find(|r| r.symbol.as_deref() == Some("MYST:US"))
        .expect("Market-Value-only row");
    assert_eq!(mystery.market_value, Some(500.0));
    assert!(mystery.cost_basis.is_none());
    assert_eq!(mystery.action, RowAction::NeedsFix);
    assert!(mystery.errors.iter().any(|e| e.contains("cost basis")));

    std::fs::remove_dir_all(&dir).ok();
}

/// Edge case audit: blank lines between sections must never be counted as
/// data rows, and the `Exchange Rate:` footer must be silently ignored
/// rather than surfaced as an error or an extra row.
#[tokio::test]
async fn test_blank_lines_and_exchange_rate_footer_do_not_produce_rows_or_errors() {
    let pool = unused_pool().await;
    let dir = std::env::temp_dir().join(format!("import-it-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = write_fixture(&dir).await;

    let plan = build_import_plan(&pool, path.to_str().unwrap(), &context())
        .await
        .expect("plan should build");

    assert_eq!(plan.rows.len(), 7, "exactly the 7 holding data rows");
    assert_eq!(plan.cash_rows.len(), 2, "exactly the 2 cash data rows");
    for row in plan.rows.iter().chain(plan.cash_rows.iter()) {
        for v in row.raw.values() {
            assert!(!v.contains("Exchange Rate"));
        }
        assert!(row
            .errors
            .iter()
            .all(|e| !e.to_lowercase().contains("exchange rate")));
    }

    std::fs::remove_dir_all(&dir).ok();
}

/// Edge case audit: a mismatched Settlement Currency vs Average Cost
/// Currency must be a `warning`, not a `needs_fix` — the row still commits.
#[tokio::test]
async fn test_settlement_currency_mismatch_is_warning_not_needs_fix() {
    let pool = unused_pool().await;
    let content =
        "Symbol,Asset Class,Quantity,Average Cost,Average Cost Currency,Settlement Currency\n\
                   AAPL,Equity,10,150.00,USD,CAD\n";
    let dir = std::env::temp_dir().join(format!("import-it-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("mismatch.csv");
    std::fs::write(&path, content).unwrap();

    let plan = build_import_plan(&pool, path.to_str().unwrap(), &context())
        .await
        .expect("plan should build");

    assert_eq!(plan.rows.len(), 1);
    assert_eq!(plan.rows[0].action, RowAction::Warning);
    assert!(plan.rows[0]
        .warnings
        .iter()
        .any(|w| w.contains("Settlement Currency")));

    std::fs::remove_dir_all(&dir).ok();
}

/// Edge case audit: a blank Asset Class must be `needs_fix`, not `warning` —
/// unlike an unrecognized-but-present asset class value.
#[tokio::test]
async fn test_blank_asset_class_is_needs_fix_not_warning() {
    let pool = unused_pool().await;
    let content = "Symbol,Asset Class,Quantity,Average Cost,Currency\n\
                   AAPL,,10,150.00,USD\n";
    let dir = std::env::temp_dir().join(format!("import-it-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("blank_asset_class.csv");
    std::fs::write(&path, content).unwrap();

    let plan = build_import_plan(&pool, path.to_str().unwrap(), &context())
        .await
        .expect("plan should build");

    assert_eq!(plan.rows.len(), 1);
    assert_eq!(plan.rows[0].action, RowAction::NeedsFix);
    assert!(plan.rows[0].asset_type.is_none());
    assert!(plan.rows[0]
        .errors
        .iter()
        .any(|e| e.contains("asset class")));

    std::fs::remove_dir_all(&dir).ok();
}
