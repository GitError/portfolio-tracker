//! Row normalization: turns one raw source row into a `NormalizedImportRow`,
//! classifying it as `Create` (default; reclassified against the DB later),
//! `NeedsFix`, or `Warning`.

use std::collections::HashMap;

use crate::import_pipeline::aliases::{canonical_field, map_asset_class, resolve_symbol};
use crate::import_pipeline::parser::RawRow;
use crate::types::{ImportContext, NormalizedImportRow, RowAction};

/// Case-insensitive lookup of a raw row value by its original header text.
fn raw_get<'a>(row: &'a HashMap<String, String>, header: &str) -> Option<&'a str> {
    row.iter()
        .find(|(k, _)| k.trim().eq_ignore_ascii_case(header))
        .map(|(_, v)| v.as_str())
        .filter(|v| !v.trim().is_empty())
}

/// Builds a canonical-field -> value map from a row, walking `headers` in
/// their original column order (not `row`'s `HashMap` iteration order, which
/// is unordered) so that when two columns alias to the same canonical field,
/// the earlier-declared column deterministically wins rather than whichever
/// happened to be visited last. `overrides` (user-selected column mappings
/// from the review UI) take priority over the static alias registry.
fn build_canonical_map(
    row: &HashMap<String, String>,
    headers: &[String],
    overrides: &HashMap<String, String>,
) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for header in headers {
        let Some(value) = row.get(header) else {
            continue;
        };
        if value.trim().is_empty() {
            continue;
        }
        let field = overrides
            .get(header)
            .cloned()
            .or_else(|| canonical_field(header).map(str::to_string));
        if let Some(field) = field {
            map.entry(field).or_insert_with(|| value.clone());
        }
    }
    map
}

/// Parses a numeric field, rejecting non-finite results (`NaN`/`Infinity`,
/// including overflow like `1e400`) that would otherwise silently bypass the
/// `>= 0` sign checks below and the `holdings` table's `CHECK` constraints.
fn parse_f64(value: Option<&str>) -> Option<f64> {
    value
        .and_then(|v| v.trim().parse::<f64>().ok())
        .filter(|n| n.is_finite())
}

/// Derives `(cost_basis, cost_basis_source)`:
/// 1. Directly from `average_cost` (already per-unit).
/// 2. Otherwise from `book_value / quantity` when quantity is positive.
/// 3. Otherwise `(None, None)` — the caller marks the row `NeedsFix`.
fn derive_cost_basis(
    average_cost: Option<f64>,
    book_value: Option<f64>,
    quantity: Option<f64>,
) -> (Option<f64>, Option<String>) {
    if let Some(ac) = average_cost {
        return (Some(ac), Some("average_cost".to_string()));
    }
    if let (Some(bv), Some(q)) = (book_value, quantity) {
        if q > 0.0 {
            return (Some(bv / q), Some("derived:total_cost/qty".to_string()));
        }
    }
    (None, None)
}

/// Normalizes one Holding Details / generic-CSV row into a
/// `NormalizedImportRow`. Shared by both `CanadianBankMultiSection` and
/// generic-CSV profiles — the profile only affects section detection, not
/// this per-row algorithm.
pub fn normalize_row(
    raw: &RawRow,
    headers: &[String],
    context: &ImportContext,
) -> NormalizedImportRow {
    let canonical = build_canonical_map(&raw.values, headers, &context.column_overrides);

    let mut warnings = Vec::new();
    let mut errors = Vec::new();

    // ── Symbol ──
    let symbol_raw = canonical.get("symbol").cloned();
    let (resolved_symbol, symbol_warning) = match symbol_raw.as_deref() {
        Some(s) if !s.trim().is_empty() => {
            let (resolved, warning) = resolve_symbol(s);
            (Some(resolved), warning)
        }
        _ => (None, None),
    };
    if let Some(w) = symbol_warning {
        warnings.push(w);
    }
    if symbol_raw
        .as_deref()
        .map(str::trim)
        .unwrap_or("")
        .is_empty()
    {
        errors.push("Missing or unresolvable symbol".to_string());
    }

    // ── Name ──
    let name = canonical.get("name").cloned();

    // ── Asset type ──
    let asset_type_raw = canonical.get("asset_type").cloned().unwrap_or_default();
    let (asset_type, asset_type_warning) = map_asset_class(&asset_type_raw);
    if asset_type.is_none() {
        errors.push("Missing asset class".to_string());
    }
    if let Some(w) = asset_type_warning {
        warnings.push(w);
    }

    // ── Quantity ──
    let quantity = parse_f64(canonical.get("quantity").map(|s| s.as_str()));
    match quantity {
        None => errors.push("Missing or invalid quantity".to_string()),
        Some(0.0) => errors.push("Quantity is zero".to_string()),
        Some(q) if q < 0.0 => warnings.push(format!(
            "Negative quantity ({q}) — short positions are not modeled; row kept as-is"
        )),
        _ => {}
    }

    // ── Currency: Average Cost Currency > Settlement Currency > generic alias ──
    let average_cost_currency = raw_get(&raw.values, "Average Cost Currency");
    let settlement_currency = raw_get(&raw.values, "Settlement Currency");
    let currency = average_cost_currency
        .or(settlement_currency)
        .map(str::to_string)
        .or_else(|| canonical.get("currency").cloned());
    if currency.as_deref().map(str::trim).unwrap_or("").is_empty() {
        errors.push("Missing currency".to_string());
    }
    if let (Some(acc), Some(sc)) = (average_cost_currency, settlement_currency) {
        if !acc.eq_ignore_ascii_case(sc) {
            warnings.push(format!(
                "Settlement Currency ({sc}) does not match Average Cost Currency ({acc})"
            ));
        }
    }

    // ── Cost basis ──
    let average_cost = parse_f64(canonical.get("average_cost").map(|s| s.as_str()));
    let book_value = parse_f64(canonical.get("book_value").map(|s| s.as_str()));
    let (cost_basis, cost_basis_source) = derive_cost_basis(average_cost, book_value, quantity);
    match cost_basis {
        None => errors.push(
            "Could not determine cost basis (no Average Cost, and no Total Cost/Quantity to derive from)".to_string(),
        ),
        Some(cb) if cb < 0.0 => errors.push(format!("Cost basis cannot be negative ({cb})")),
        _ => {}
    }

    let market_value = parse_f64(canonical.get("market_value").map(|s| s.as_str()));
    let exchange = canonical.get("exchange").cloned();
    let target_weight = parse_f64(canonical.get("target_weight").map(|s| s.as_str()));
    let dividend_yield = parse_f64(canonical.get("dividend_yield").map(|s| s.as_str()));
    let annualized_income = parse_f64(canonical.get("annualized_income").map(|s| s.as_str()));
    let ex_dividend_date = canonical.get("ex_dividend_date").cloned();

    let action = if !errors.is_empty() {
        RowAction::NeedsFix
    } else if !warnings.is_empty() {
        RowAction::Warning
    } else {
        RowAction::Create
    };

    NormalizedImportRow {
        row_number: raw.row_number,
        action,
        symbol: symbol_raw,
        resolved_symbol,
        name,
        asset_type,
        quantity,
        cost_basis,
        cost_basis_source,
        currency,
        book_value,
        market_value,
        exchange,
        target_weight,
        account_type: context.account_type.clone(),
        account_name: context.account_name.clone(),
        warnings,
        errors,
        raw: raw.values.clone(),
        dividend_yield,
        annualized_income,
        ex_dividend_date,
    }
}

/// Normalizes one row from the Cash Details section
/// (`Currency, Account Type, Settled Cash, Trade Cash`) into a synthetic cash
/// holding candidate: symbol `{CURRENCY}-CASH`, cost basis fixed at 1.0.
pub fn normalize_cash_row(raw: &RawRow, context: &ImportContext) -> NormalizedImportRow {
    let mut warnings = Vec::new();
    let mut errors = Vec::new();

    let currency = raw_get(&raw.values, "Currency").map(str::to_string);
    if currency.is_none() {
        errors.push("Cash row is missing a Currency".to_string());
    }
    let quantity = parse_f64(raw_get(&raw.values, "Settled Cash"));
    if quantity.is_none() {
        errors.push("Cash row has a missing or invalid Settled Cash balance".to_string());
    } else if quantity == Some(0.0) {
        warnings.push("Settled Cash balance is zero".to_string());
    }

    let symbol = currency
        .as_ref()
        .map(|c| format!("{}-CASH", c.to_uppercase()));

    let action = if !errors.is_empty() {
        RowAction::NeedsFix
    } else if !warnings.is_empty() {
        RowAction::Warning
    } else {
        RowAction::Create
    };

    NormalizedImportRow {
        row_number: raw.row_number,
        action,
        symbol: symbol.clone(),
        resolved_symbol: symbol,
        name: currency.as_ref().map(|c| format!("{c} Cash")),
        asset_type: Some("Cash".to_string()),
        quantity,
        cost_basis: Some(1.0),
        cost_basis_source: Some("fixed:cash".to_string()),
        currency: currency.clone(),
        book_value: None,
        market_value: None,
        exchange: None,
        target_weight: None,
        account_type: context.account_type.clone(),
        account_name: context.account_name.clone(),
        warnings,
        errors,
        raw: raw.values.clone(),
        dividend_yield: None,
        annualized_income: None,
        ex_dividend_date: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context() -> ImportContext {
        ImportContext {
            account_type: "RRSP".to_string(),
            account_name: Some("TD RRSP".to_string()),
            account_id: None,
            source_profile: None,
            column_overrides: HashMap::new(),
        }
    }

    fn row(pairs: &[(&str, &str)], row_number: usize) -> RawRow {
        let mut values = HashMap::new();
        for (k, v) in pairs {
            values.insert(k.to_string(), v.to_string());
        }
        RawRow { row_number, values }
    }

    fn headers_of(pairs: &[(&str, &str)]) -> Vec<String> {
        pairs.iter().map(|(k, _)| k.to_string()).collect()
    }

    /// Builds a row from `pairs` (in the given order) and normalizes it,
    /// so column-order-sensitive behavior (e.g. alias-collision precedence)
    /// is exercised the same way production code builds `headers`.
    fn normalize(
        pairs: &[(&str, &str)],
        row_number: usize,
        ctx: &ImportContext,
    ) -> NormalizedImportRow {
        normalize_row(&row(pairs, row_number), &headers_of(pairs), ctx)
    }

    #[test]
    fn cost_basis_direct_from_average_cost() {
        let normalized = normalize(
            &[
                ("Symbol", "AAPL:US"),
                ("Asset Class", "Equity"),
                ("Quantity", "100"),
                ("Average Cost", "135.50"),
                ("Average Cost Currency", "USD"),
            ],
            8,
            &context(),
        );
        assert_eq!(normalized.cost_basis, Some(135.50));
        assert_eq!(
            normalized.cost_basis_source,
            Some("average_cost".to_string())
        );
        assert_eq!(normalized.action, RowAction::Create);
    }

    #[test]
    fn cost_basis_derived_from_total_cost_over_quantity() {
        let normalized = normalize(
            &[
                ("Symbol", "AAPL:US"),
                ("Asset Class", "Equity"),
                ("Quantity", "100"),
                ("Total Cost", "13550.0"),
                ("Currency", "USD"),
            ],
            8,
            &context(),
        );
        assert_eq!(normalized.cost_basis, Some(135.5));
        assert_eq!(
            normalized.cost_basis_source,
            Some("derived:total_cost/qty".to_string())
        );
    }

    #[test]
    fn needs_fix_when_cost_basis_undeterminable() {
        let normalized = normalize(
            &[
                ("Symbol", "AAPL:US"),
                ("Asset Class", "Equity"),
                ("Quantity", "100"),
                ("Currency", "USD"),
            ],
            8,
            &context(),
        );
        assert_eq!(normalized.action, RowAction::NeedsFix);
        assert!(normalized.cost_basis.is_none());
        assert!(normalized.errors.iter().any(|e| e.contains("cost basis")));
    }

    #[test]
    fn needs_fix_when_symbol_blank() {
        let normalized = normalize(
            &[
                ("Symbol", ""),
                ("Asset Class", "Equity"),
                ("Quantity", "100"),
                ("Average Cost", "10"),
                ("Currency", "USD"),
            ],
            8,
            &context(),
        );
        assert_eq!(normalized.action, RowAction::NeedsFix);
        assert!(normalized.symbol.is_none());
        assert!(normalized.errors.iter().any(|e| e.contains("symbol")));
    }

    #[test]
    fn warning_when_asset_class_is_fixed_income() {
        let normalized = normalize(
            &[
                ("Symbol", "GIC123"),
                ("Asset Class", "Fixed Income"),
                ("Quantity", "1"),
                ("Average Cost", "1000"),
                ("Currency", "CAD"),
            ],
            8,
            &context(),
        );
        assert_eq!(normalized.action, RowAction::Warning);
        assert_eq!(normalized.asset_type, Some("Other".to_string()));
        assert!(normalized
            .warnings
            .iter()
            .any(|w| w.contains("no live pricing")));
    }

    #[test]
    fn currency_priority_prefers_average_cost_currency_over_settlement_currency() {
        let normalized = normalize(
            &[
                ("Symbol", "AAPL:US"),
                ("Asset Class", "Equity"),
                ("Quantity", "100"),
                ("Average Cost", "135.50"),
                ("Average Cost Currency", "USD"),
                ("Settlement Currency", "CAD"),
            ],
            8,
            &context(),
        );
        assert_eq!(normalized.currency, Some("USD".to_string()));
        assert!(normalized
            .warnings
            .iter()
            .any(|w| w.contains("Settlement Currency")));
    }

    #[test]
    fn negative_quantity_is_a_warning_not_needs_fix() {
        let normalized = normalize(
            &[
                ("Symbol", "AAPL:US"),
                ("Asset Class", "Equity"),
                ("Quantity", "-5"),
                ("Average Cost", "135.50"),
                ("Currency", "USD"),
            ],
            8,
            &context(),
        );
        assert_eq!(normalized.action, RowAction::Warning);
        assert!(normalized.warnings.iter().any(|w| w.contains("Negative")));
    }

    #[test]
    fn zero_quantity_is_needs_fix() {
        let normalized = normalize(
            &[
                ("Symbol", "AAPL:US"),
                ("Asset Class", "Equity"),
                ("Quantity", "0"),
                ("Average Cost", "135.50"),
                ("Currency", "USD"),
            ],
            8,
            &context(),
        );
        assert_eq!(normalized.action, RowAction::NeedsFix);
    }

    #[test]
    fn non_finite_quantity_is_needs_fix_not_silently_accepted() {
        let normalized = normalize(
            &[
                ("Symbol", "AAPL:US"),
                ("Asset Class", "Equity"),
                ("Quantity", "NaN"),
                ("Average Cost", "135.50"),
                ("Currency", "USD"),
            ],
            8,
            &context(),
        );
        assert_eq!(normalized.action, RowAction::NeedsFix);
        assert!(normalized.quantity.is_none());
    }

    #[test]
    fn negative_cost_basis_is_needs_fix() {
        let normalized = normalize(
            &[
                ("Symbol", "AAPL:US"),
                ("Asset Class", "Equity"),
                ("Quantity", "10"),
                ("Average Cost", "-5"),
                ("Currency", "USD"),
            ],
            8,
            &context(),
        );
        assert_eq!(normalized.action, RowAction::NeedsFix);
        assert!(normalized.errors.iter().any(|e| e.contains("negative")));
    }

    #[test]
    fn column_override_maps_an_unrecognized_header_to_a_canonical_field() {
        let mut ctx = context();
        ctx.column_overrides
            .insert("My Custom Qty".to_string(), "quantity".to_string());
        let normalized = normalize(
            &[
                ("Symbol", "AAPL:US"),
                ("Asset Class", "Equity"),
                ("My Custom Qty", "42"),
                ("Average Cost", "135.50"),
                ("Currency", "USD"),
            ],
            8,
            &ctx,
        );
        assert_eq!(normalized.quantity, Some(42.0));
    }

    #[test]
    fn alias_collision_deterministically_prefers_earlier_declared_column() {
        // Both "Total Cost" and "Book Value" alias to `book_value`; the
        // earlier-declared column (by header order) must always win, not
        // whichever the HashMap happened to visit last.
        let a = normalize(
            &[
                ("Symbol", "AAPL:US"),
                ("Asset Class", "Equity"),
                ("Quantity", "10"),
                ("Total Cost", "100"),
                ("Book Value", "200"),
                ("Currency", "USD"),
            ],
            8,
            &context(),
        );
        let b = normalize(
            &[
                ("Symbol", "AAPL:US"),
                ("Asset Class", "Equity"),
                ("Quantity", "10"),
                ("Book Value", "200"),
                ("Total Cost", "100"),
                ("Currency", "USD"),
            ],
            8,
            &context(),
        );
        assert_eq!(a.book_value, Some(100.0));
        assert_eq!(b.book_value, Some(200.0));
    }

    #[test]
    fn exchange_and_target_weight_are_captured_from_canonical_columns() {
        let normalized = normalize(
            &[
                ("Symbol", "AAPL:US"),
                ("Asset Class", "Equity"),
                ("Quantity", "100"),
                ("Average Cost", "135.50"),
                ("Currency", "USD"),
                ("Exchange", "NASDAQ"),
                ("Target Weight", "12.5"),
            ],
            8,
            &context(),
        );
        assert_eq!(normalized.exchange, Some("NASDAQ".to_string()));
        assert_eq!(normalized.target_weight, Some(12.5));
    }

    #[test]
    fn exchange_and_target_weight_are_none_when_columns_absent() {
        let normalized = normalize(
            &[
                ("Symbol", "AAPL:US"),
                ("Asset Class", "Equity"),
                ("Quantity", "100"),
                ("Average Cost", "135.50"),
                ("Currency", "USD"),
            ],
            8,
            &context(),
        );
        assert_eq!(normalized.exchange, None);
        assert_eq!(normalized.target_weight, None);
    }

    #[test]
    fn cash_row_parses_from_cash_details_section() {
        let r = row(
            &[
                ("Currency", "CAD"),
                ("Account Type", "CASH"),
                ("Settled Cash", "89192.24"),
                ("Trade Cash", "89192.24"),
            ],
            5,
        );
        let normalized = normalize_cash_row(&r, &context());
        assert_eq!(normalized.symbol, Some("CAD-CASH".to_string()));
        assert_eq!(normalized.currency, Some("CAD".to_string()));
        assert_eq!(normalized.quantity, Some(89192.24));
        assert_eq!(normalized.cost_basis, Some(1.0));
        assert_eq!(normalized.action, RowAction::Create);
    }

    #[test]
    fn cash_row_needs_fix_when_settled_cash_missing() {
        let r = row(&[("Currency", "CAD"), ("Account Type", "CASH")], 5);
        let normalized = normalize_cash_row(&r, &context());
        assert_eq!(normalized.action, RowAction::NeedsFix);
    }
}
