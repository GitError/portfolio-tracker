//! Column alias registry, asset-class mapping, and symbol resolution for the
//! import pipeline. See docs/superpowers/specs/2026-05-24-import-plus-insights-design.md.

/// Country codes with a known Yahoo Finance exchange-suffix mapping in
/// `crate::csv::normalize_symbol_for_import`. Anything else is passed through
/// unsuffixed and flagged with a warning rather than silently guessed.
const KNOWN_COUNTRY_CODES: &[&str] = &["US", "CA", "GB", "DE", "FR", "AU", "JP", "HK"];

/// Maps a source CSV header to a canonical holding field, using a static
/// curated alias registry. Matching is case-insensitive and trims whitespace.
/// Returns `None` for headers with no known mapping (preserved in preview
/// metadata but otherwise ignored).
pub fn canonical_field(header: &str) -> Option<&'static str> {
    match header.trim().to_lowercase().as_str() {
        "symbol" | "ticker" | "security id" | "cusip" | "isin" => Some("symbol"),
        "security description"
        | "description"
        | "security name"
        | "investment name"
        | "holding name"
        | "name" => Some("name"),
        "quantity" | "shares" | "units" | "qty" | "open qty" | "number of shares"
        | "units held" | "position quantity" => Some("quantity"),
        "average cost"
        | "avg cost"
        | "book cost per share"
        | "cost per unit"
        | "average book cost"
        | "book value per unit"
        | "unit cost"
        | "average buy price"
        | "book cost per unit" => Some("average_cost"),
        "total cost" | "book value" | "book cost" | "adjusted cost base" | "acb"
        | "total book value" | "cost basis" => Some("book_value"),
        "average cost currency" | "settlement currency" | "currency" | "ccy" | "denomination" => {
            Some("currency")
        }
        "market value"
        | "current value"
        | "total market value"
        | "current market value"
        | "value" => Some("market_value"),
        "asset class" | "asset type" | "type" | "security type" | "category"
        | "investment type" | "product type" => Some("asset_type"),
        "exchange" | "market" | "listing exchange" | "market code" => Some("exchange"),
        "target weight" | "target weight (%)" | "target allocation" | "target %"
        | "target_weight" => Some("target_weight"),
        "settled cash" | "trade cash" | "cash balance" | "cash" | "cash position" | "balance" => {
            Some("cash_balance")
        }
        "dividend yield (%)" | "dividend yield" => Some("dividend_yield"),
        "annualized income" => Some("annualized_income"),
        "ex-dividend date" | "ex dividend date" => Some("ex_dividend_date"),
        _ => None,
    }
}

/// Maps a source `Asset Class` value to an app asset type string
/// ("Stock" | "ETF" | "Crypto" | "Cash" | "Other"), plus an optional warning.
/// A blank value returns `(None, None)`; the caller marks the row `NeedsFix`.
///
/// Note: the app's `AssetType` enum only has four variants (stock/etf/crypto/
/// cash) — there is no persisted "Other" asset type today. Rows that resolve
/// to "Other" are shown in the plan with a warning (matching the design doc),
/// but `commit_import` cannot write them to the holdings table and reports
/// them as a skipped-with-error row instead of silently miscategorizing them.
pub fn map_asset_class(raw: &str) -> (Option<String>, Option<String>) {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return (None, None);
    }
    match trimmed.to_lowercase().as_str() {
        "equity" => (Some("Stock".to_string()), None),
        "etf" | "mutual fund" => (Some("ETF".to_string()), None),
        "crypto" | "cryptocurrency" => (Some("Crypto".to_string()), None),
        "cash" => (Some("Cash".to_string()), None),
        "fixed income" | "gic" | "money market" | "bond" => (
            Some("Other".to_string()),
            Some(format!(
                "Asset class '{trimmed}' has no live pricing support and is imported as Other"
            )),
        ),
        _ => (
            Some("Other".to_string()),
            Some(format!(
                "Unrecognized asset class '{trimmed}'; imported as Other (no live pricing)"
            )),
        ),
    }
}

/// Resolves a `SYMBOL:COUNTRY` (or plain) symbol to a Yahoo Finance symbol,
/// reusing the app's existing country-suffix logic
/// (`crate::csv::normalize_symbol_for_import`). Returns the resolved symbol
/// plus an optional warning when the country code isn't one of the app's
/// known exchange suffixes (the symbol is still returned, unsuffixed).
pub fn resolve_symbol(raw: &str) -> (String, Option<String>) {
    let resolved = crate::csv::normalize_symbol_for_import(raw);
    let warning = raw.trim().split_once(':').and_then(|(_, country)| {
        let cc = country.trim().to_uppercase();
        if cc.is_empty() || KNOWN_COUNTRY_CODES.contains(&cc.as_str()) {
            None
        } else {
            Some(format!(
                "Unrecognized country code '{}' in symbol '{}'; kept without an exchange suffix",
                cc,
                raw.trim()
            ))
        }
    });
    (resolved, warning)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── canonical_field: at least 10 alias cases across different fields ──

    #[test]
    fn canonical_field_matches_symbol_aliases() {
        assert_eq!(canonical_field("Symbol"), Some("symbol"));
        assert_eq!(canonical_field("Ticker"), Some("symbol"));
        assert_eq!(canonical_field("CUSIP"), Some("symbol"));
        assert_eq!(canonical_field("isin"), Some("symbol"));
    }

    #[test]
    fn canonical_field_matches_name_aliases() {
        assert_eq!(canonical_field("Security Description"), Some("name"));
        assert_eq!(canonical_field("Investment Name"), Some("name"));
    }

    #[test]
    fn canonical_field_matches_quantity_aliases() {
        assert_eq!(canonical_field("Quantity"), Some("quantity"));
        assert_eq!(canonical_field("Open Qty"), Some("quantity"));
        assert_eq!(canonical_field("Number of Shares"), Some("quantity"));
    }

    #[test]
    fn canonical_field_matches_average_cost_aliases() {
        assert_eq!(canonical_field("Average Cost"), Some("average_cost"));
        assert_eq!(canonical_field("Avg Cost"), Some("average_cost"));
        assert_eq!(canonical_field("Average Buy Price"), Some("average_cost"));
    }

    #[test]
    fn canonical_field_matches_book_value_aliases() {
        assert_eq!(canonical_field("Total Cost"), Some("book_value"));
        assert_eq!(canonical_field("Adjusted Cost Base"), Some("book_value"));
        assert_eq!(canonical_field("ACB"), Some("book_value"));
    }

    #[test]
    fn canonical_field_matches_currency_aliases() {
        assert_eq!(canonical_field("Average Cost Currency"), Some("currency"));
        assert_eq!(canonical_field("Settlement Currency"), Some("currency"));
        assert_eq!(canonical_field("CCY"), Some("currency"));
    }

    #[test]
    fn canonical_field_matches_market_value_aliases() {
        assert_eq!(canonical_field("Market Value"), Some("market_value"));
        assert_eq!(canonical_field("Current Value"), Some("market_value"));
    }

    #[test]
    fn canonical_field_matches_asset_type_aliases() {
        assert_eq!(canonical_field("Asset Class"), Some("asset_type"));
        assert_eq!(canonical_field("Security Type"), Some("asset_type"));
    }

    #[test]
    fn canonical_field_matches_exchange_aliases() {
        assert_eq!(canonical_field("Exchange"), Some("exchange"));
        assert_eq!(canonical_field("Listing Exchange"), Some("exchange"));
    }

    #[test]
    fn canonical_field_matches_target_weight_aliases() {
        assert_eq!(canonical_field("Target Weight"), Some("target_weight"));
        assert_eq!(canonical_field("Target Weight (%)"), Some("target_weight"));
        assert_eq!(canonical_field("Target Allocation"), Some("target_weight"));
        assert_eq!(canonical_field("target_weight"), Some("target_weight"));
    }

    #[test]
    fn canonical_field_matches_cash_balance_aliases() {
        assert_eq!(canonical_field("Settled Cash"), Some("cash_balance"));
        assert_eq!(canonical_field("Trade Cash"), Some("cash_balance"));
    }

    #[test]
    fn canonical_field_matches_insight_metadata_aliases() {
        assert_eq!(
            canonical_field("Dividend Yield (%)"),
            Some("dividend_yield")
        );
        assert_eq!(
            canonical_field("Annualized Income"),
            Some("annualized_income")
        );
        assert_eq!(
            canonical_field("Ex-Dividend Date"),
            Some("ex_dividend_date")
        );
    }

    #[test]
    fn canonical_field_is_case_and_whitespace_insensitive() {
        assert_eq!(canonical_field("  SYMBOL  "), Some("symbol"));
        assert_eq!(canonical_field("aVeRaGe CoSt"), Some("average_cost"));
    }

    #[test]
    fn canonical_field_returns_none_for_unknown_header() {
        assert_eq!(canonical_field("Some Random Column"), None);
    }

    // ── SYMBOL:COUNTRY resolution ──────────────────────────────────────────

    #[test]
    fn resolve_symbol_us_has_no_suffix_and_no_warning() {
        let (resolved, warning) = resolve_symbol("AAPL:US");
        assert_eq!(resolved, "AAPL");
        assert!(warning.is_none());
    }

    #[test]
    fn resolve_symbol_ca_adds_to_suffix_and_no_warning() {
        let (resolved, warning) = resolve_symbol("TD:CA");
        assert_eq!(resolved, "TD.TO");
        assert!(warning.is_none());
    }

    #[test]
    fn resolve_symbol_gb_adds_l_suffix_and_no_warning() {
        let (resolved, warning) = resolve_symbol("BARC:GB");
        assert_eq!(resolved, "BARC.L");
        assert!(warning.is_none());
    }

    #[test]
    fn resolve_symbol_de_fr_au_known_suffixes_have_no_warning() {
        assert_eq!(resolve_symbol("SAP:DE"), ("SAP.DE".to_string(), None));
        assert_eq!(resolve_symbol("AIR:FR"), ("AIR.PA".to_string(), None));
        assert_eq!(resolve_symbol("CBA:AU"), ("CBA.AX".to_string(), None));
    }

    #[test]
    fn resolve_symbol_unknown_country_code_flags_warning_but_keeps_symbol() {
        let (resolved, warning) = resolve_symbol("XYZ:ZZ");
        assert_eq!(resolved, "XYZ");
        let warning = warning.expect("unknown country code should produce a warning");
        assert!(warning.contains("ZZ"));
        assert!(warning.contains("XYZ"));
    }

    #[test]
    fn resolve_symbol_plain_symbol_has_no_warning() {
        let (resolved, warning) = resolve_symbol("AAPL");
        assert_eq!(resolved, "AAPL");
        assert!(warning.is_none());
    }

    // ── Asset class mapping ─────────────────────────────────────────────────

    #[test]
    fn map_asset_class_equity_maps_to_stock() {
        assert_eq!(map_asset_class("Equity"), (Some("Stock".to_string()), None));
    }

    #[test]
    fn map_asset_class_etf_and_mutual_fund_map_to_etf() {
        assert_eq!(map_asset_class("ETF"), (Some("ETF".to_string()), None));
        assert_eq!(
            map_asset_class("Mutual Fund"),
            (Some("ETF".to_string()), None)
        );
    }

    #[test]
    fn map_asset_class_crypto_maps_to_crypto() {
        assert_eq!(
            map_asset_class("Crypto"),
            (Some("Crypto".to_string()), None)
        );
        assert_eq!(
            map_asset_class("Cryptocurrency"),
            (Some("Crypto".to_string()), None)
        );
    }

    #[test]
    fn map_asset_class_cash_maps_to_cash() {
        assert_eq!(map_asset_class("Cash"), (Some("Cash".to_string()), None));
    }

    #[test]
    fn map_asset_class_fixed_income_maps_to_other_with_warning() {
        let (asset_type, warning) = map_asset_class("Fixed Income");
        assert_eq!(asset_type, Some("Other".to_string()));
        let warning = warning.expect("Fixed Income should produce a warning");
        assert!(warning.contains("Fixed Income"));
        assert!(warning.contains("no live pricing"));
    }

    #[test]
    fn map_asset_class_gic_and_money_market_map_to_other_with_warning() {
        assert!(map_asset_class("GIC").1.is_some());
        assert!(map_asset_class("Money Market").1.is_some());
        assert_eq!(map_asset_class("GIC").0, Some("Other".to_string()));
    }

    #[test]
    fn map_asset_class_blank_returns_none() {
        assert_eq!(map_asset_class(""), (None, None));
        assert_eq!(map_asset_class("   "), (None, None));
    }

    #[test]
    fn map_asset_class_unrecognized_value_maps_to_other_with_warning() {
        let (asset_type, warning) = map_asset_class("Structured Note");
        assert_eq!(asset_type, Some("Other".to_string()));
        assert!(warning.is_some());
    }
}
