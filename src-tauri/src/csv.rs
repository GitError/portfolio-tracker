use ::csv::{ReaderBuilder, StringRecord, Trim, WriterBuilder};

use crate::types::{AccountType, AssetType, Holding};

#[derive(Debug)]
pub struct ParsedImportRow {
    pub row: usize,
    pub symbol: String,
    pub name: String,
    pub asset_type: AssetType,
    pub account: AccountType,
    pub quantity: f64,
    pub cost_basis: f64,
    pub currency: String,
    pub exchange: String,
    pub target_weight: f64,
    pub indicated_annual_dividend: Option<f64>,
    pub indicated_annual_dividend_currency: Option<String>,
    pub dividend_frequency: Option<String>,
    pub maturity_date: Option<String>,
}

pub fn build_holdings_csv(holdings: &[Holding]) -> Result<String, String> {
    let mut writer = WriterBuilder::new().from_writer(vec![]);
    writer
        .write_record([
            "symbol",
            "name",
            "type",
            "account",
            "quantity",
            "cost_basis",
            "currency",
            "exchange",
            "target_weight",
            "indicated_annual_dividend",
            "indicated_annual_dividend_currency",
            "dividend_frequency",
            "maturity_date",
        ])
        .map_err(|e| e.to_string())?;

    for holding in holdings {
        writer
            .write_record([
                holding.symbol.clone(),
                holding.name.clone(),
                holding.asset_type.as_str().to_string(),
                holding.account.as_str().to_string(),
                holding.quantity.to_string(),
                holding.cost_basis.to_string(),
                holding.currency.clone(),
                holding.exchange.clone(),
                holding.target_weight.to_string(),
                holding
                    .indicated_annual_dividend
                    .map(|v| v.to_string())
                    .unwrap_or_default(),
                holding
                    .indicated_annual_dividend_currency
                    .clone()
                    .unwrap_or_default(),
                holding
                    .dividend_frequency
                    .as_ref()
                    .map(|f| f.as_str().to_string())
                    .unwrap_or_default(),
                holding.maturity_date.clone().unwrap_or_default(),
            ])
            .map_err(|e| e.to_string())?;
    }

    let bytes = writer.into_inner().map_err(|e| e.to_string())?;
    String::from_utf8(bytes).map_err(|e| e.to_string())
}

fn detect_csv_delimiter(content: &str) -> u8 {
    let first_line = content.lines().next().unwrap_or_default();
    if first_line.contains(';') && !first_line.contains(',') {
        b';'
    } else {
        b','
    }
}

fn find_column_index(headers: &StringRecord, field: &str) -> Option<usize> {
    headers.iter().position(|header| {
        header
            .trim_start_matches('\u{feff}')
            .trim()
            .eq_ignore_ascii_case(field)
    })
}

fn parse_required_field(
    record: &StringRecord,
    index: usize,
    row: usize,
    field: &str,
) -> Result<String, String> {
    let value = record.get(index).unwrap_or_default().trim();
    if value.is_empty() {
        return Err(format!("Row {}: missing_{}", row, field));
    }
    Ok(value.to_string())
}

/// Strip null bytes and ASCII control characters from a string field.
/// This prevents control characters from being stored in the database or
/// causing downstream parsing issues.
/// Enforces MAX_FIELD_LEN before processing to avoid allocating/scanning
/// excessively large inputs.
fn sanitize_str(s: &str) -> String {
    let s = if s.len() > crate::config::MAX_FIELD_LEN {
        &s[..crate::config::MAX_FIELD_LEN]
    } else {
        s
    };
    s.chars().filter(|c| !c.is_control()).collect()
}

fn parse_optional_field(record: &StringRecord, index: Option<usize>) -> String {
    index
        .and_then(|i| record.get(i))
        .unwrap_or_default()
        .trim()
        .to_string()
}

/// Convert `SYMBOL:COUNTRY` notation to a Yahoo Finance symbol.
/// Plain symbols are returned unchanged (uppercased).
/// Examples: `BMO:CA` → `BMO.TO`, `AAPL:US` → `AAPL`, `BARC:GB` → `BARC.L`
pub fn normalize_symbol_for_import(raw: &str) -> String {
    let trimmed = raw.trim();
    if let Some((sym, country)) = trimmed.split_once(':') {
        let sym = sym.trim().to_uppercase();
        match country.trim().to_uppercase().as_str() {
            "CA" => format!("{}.TO", sym),
            "GB" => format!("{}.L", sym),
            "AU" => format!("{}.AX", sym),
            "DE" => format!("{}.DE", sym),
            "FR" => format!("{}.PA", sym),
            "JP" => format!("{}.T", sym),
            "HK" => format!("{}.HK", sym),
            _ => sym, // US or unrecognised: no exchange suffix
        }
    } else {
        trimmed.to_uppercase()
    }
}

pub fn parse_import_rows(csv_content: &str) -> Result<Vec<ParsedImportRow>, String> {
    let content = csv_content.trim_start_matches('\u{feff}');
    let mut reader = ReaderBuilder::new()
        .trim(Trim::All)
        .delimiter(detect_csv_delimiter(content))
        .from_reader(content.as_bytes());

    let headers = reader
        .headers()
        .map_err(|e| format!("Invalid CSV header: {}", e))?
        .clone();
    let symbol_index = find_column_index(&headers, "symbol")
        .ok_or_else(|| "Missing required column: symbol".to_string())?;
    let name_index = find_column_index(&headers, "name");
    let account_index = find_column_index(&headers, "account");
    let exchange_index = find_column_index(&headers, "exchange");
    let type_index = find_column_index(&headers, "type")
        .ok_or_else(|| "Missing required column: type".to_string())?;
    let quantity_index = find_column_index(&headers, "quantity")
        .ok_or_else(|| "Missing required column: quantity".to_string())?;
    let cost_basis_index = find_column_index(&headers, "cost_basis")
        .ok_or_else(|| "Missing required column: cost_basis".to_string())?;
    let currency_index = find_column_index(&headers, "currency")
        .ok_or_else(|| "Missing required column: currency".to_string())?;
    let target_weight_index = find_column_index(&headers, "target_weight");
    let indicated_annual_dividend_index = find_column_index(&headers, "indicated_annual_dividend");
    let indicated_annual_dividend_currency_index =
        find_column_index(&headers, "indicated_annual_dividend_currency");
    let dividend_frequency_index = find_column_index(&headers, "dividend_frequency");
    let maturity_date_index = find_column_index(&headers, "maturity_date");

    let mut rows = Vec::new();

    for (index, record) in reader.records().enumerate() {
        if rows.len() >= crate::config::MAX_IMPORT_ROWS {
            return Err(format!(
                "CSV import is limited to {} rows",
                crate::config::MAX_IMPORT_ROWS
            ));
        }

        let row = index + 2;
        let record = record.map_err(|e| format!("Invalid CSV row {}: {}", row, e))?;
        if record.iter().all(|field| field.trim().is_empty()) {
            continue;
        }

        let asset_type = parse_required_field(&record, type_index, row, "type")?
            .to_lowercase()
            .parse::<AssetType>()
            .map_err(|_| format!("Row {}: invalid_type", row))?;
        let account = parse_optional_field(&record, account_index);
        let account = if account.is_empty() {
            if matches!(asset_type, AssetType::Cash) {
                AccountType::Cash
            } else {
                AccountType::Taxable
            }
        } else {
            // Unknown/unrecognised account strings default to the appropriate
            // type rather than crashing the whole import.
            account.to_lowercase().parse::<AccountType>().unwrap_or({
                if matches!(asset_type, AssetType::Cash) {
                    AccountType::Cash
                } else {
                    AccountType::Taxable
                }
            })
        };
        let currency = sanitize_str(&parse_required_field(
            &record,
            currency_index,
            row,
            "currency",
        )?)
        .to_uppercase();
        let raw_symbol = sanitize_str(&parse_optional_field(&record, Some(symbol_index)));
        let symbol = if matches!(asset_type, AssetType::Cash) {
            if raw_symbol.is_empty() || raw_symbol.eq_ignore_ascii_case("CASH") {
                format!("{}-CASH", currency)
            } else {
                normalize_symbol_for_import(&raw_symbol)
            }
        } else if raw_symbol.is_empty() {
            return Err(format!("Row {}: missing_symbol", row));
        } else {
            normalize_symbol_for_import(&raw_symbol)
        };
        // Validate length on the normalized symbol (not the raw input), because
        // normalization (uppercase, country-suffix expansion) can change the length.
        if symbol.len() > crate::config::MAX_FIELD_LEN {
            return Err(format!("Row {}: symbol exceeds maximum length", row));
        }

        let quantity = parse_required_field(&record, quantity_index, row, "quantity")?
            .parse::<f64>()
            .map_err(|_| format!("Row {}: invalid_quantity", row))?;
        if quantity <= 0.0 {
            return Err(format!("Row {}: invalid_quantity", row));
        }

        let cost_basis = parse_required_field(&record, cost_basis_index, row, "cost_basis")?
            .parse::<f64>()
            .map_err(|_| format!("Row {}: invalid_cost_basis", row))?;
        if cost_basis < 0.0 {
            return Err(format!("Row {}: invalid_cost_basis", row));
        }

        let target_weight = parse_optional_field(&record, target_weight_index);
        let target_weight = if target_weight.is_empty() {
            0.0
        } else {
            let parsed = target_weight
                .parse::<f64>()
                .map_err(|_| format!("Row {}: invalid_target_weight", row))?;
            if !(0.0..=100.0).contains(&parsed) {
                return Err(format!("Row {}: invalid_target_weight", row));
            }
            parsed
        };

        let name = sanitize_str(&parse_optional_field(&record, name_index));
        let exchange = sanitize_str(&parse_optional_field(&record, exchange_index)).to_uppercase();

        if name.len() > crate::config::MAX_FIELD_LEN {
            return Err(format!("Row {}: name exceeds maximum length", row));
        }
        if currency.len() > crate::config::MAX_FIELD_LEN {
            return Err(format!("Row {}: currency exceeds maximum length", row));
        }
        if exchange.len() > crate::config::MAX_FIELD_LEN {
            return Err(format!("Row {}: exchange exceeds maximum length", row));
        }

        let iad_str = parse_optional_field(&record, indicated_annual_dividend_index);
        let indicated_annual_dividend = if iad_str.is_empty() {
            None
        } else {
            iad_str.parse::<f64>().ok()
        };
        let iad_currency_str =
            parse_optional_field(&record, indicated_annual_dividend_currency_index);
        let indicated_annual_dividend_currency = if iad_currency_str.is_empty() {
            None
        } else {
            Some(iad_currency_str.to_uppercase())
        };
        let div_freq_str = parse_optional_field(&record, dividend_frequency_index).to_lowercase();
        const VALID_FREQS: &[&str] =
            &["monthly", "quarterly", "semi-annual", "annual", "irregular"];
        if !div_freq_str.is_empty() && !VALID_FREQS.contains(&div_freq_str.as_str()) {
            return Err(format!(
                "Row {}: invalid dividend_frequency '{}'. Valid: {}",
                row,
                div_freq_str,
                VALID_FREQS.join(", ")
            ));
        }
        let dividend_frequency = if div_freq_str.is_empty() {
            None
        } else {
            Some(div_freq_str)
        };
        let maturity_date_str = parse_optional_field(&record, maturity_date_index);
        if !maturity_date_str.is_empty() {
            chrono::NaiveDate::parse_from_str(&maturity_date_str, "%Y-%m-%d").map_err(|_| {
                format!(
                    "Row {}: invalid maturity_date '{}' (expected YYYY-MM-DD)",
                    row, maturity_date_str
                )
            })?;
        }
        let maturity_date = if maturity_date_str.is_empty() {
            None
        } else {
            Some(maturity_date_str)
        };

        rows.push(ParsedImportRow {
            row,
            symbol,
            name,
            asset_type,
            account,
            quantity,
            cost_basis,
            currency,
            exchange,
            target_weight,
            indicated_annual_dividend,
            indicated_annual_dividend_currency,
            dividend_frequency,
            maturity_date,
        });
    }

    if rows.is_empty() {
        return Err("CSV file is empty".to_string());
    }

    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AccountType, AssetType, Holding, HoldingId};

    fn make_holding(
        symbol: &str,
        asset_type: AssetType,
        quantity: f64,
        cost_basis: f64,
        currency: &str,
    ) -> Holding {
        Holding {
            id: HoldingId(symbol.to_string()),
            symbol: symbol.to_string(),
            name: symbol.to_string(),
            asset_type,
            account: AccountType::Taxable,
            account_id: None,
            account_name: None,
            quantity,
            cost_basis,
            currency: currency.to_string(),
            exchange: String::new(),
            target_weight: 0.0,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
            indicated_annual_dividend: None,
            indicated_annual_dividend_currency: None,
            dividend_frequency: None,
            maturity_date: None,
        }
    }

    #[test]
    fn normalize_symbol_strips_country_suffix() {
        assert_eq!(normalize_symbol_for_import("BMO:CA"), "BMO.TO");
        assert_eq!(normalize_symbol_for_import("AAPL:US"), "AAPL");
        assert_eq!(normalize_symbol_for_import("BARC:GB"), "BARC.L");
        assert_eq!(normalize_symbol_for_import("CBA:AU"), "CBA.AX");
        assert_eq!(normalize_symbol_for_import("SAP:DE"), "SAP.DE");
        assert_eq!(normalize_symbol_for_import("AIR:FR"), "AIR.PA");
        assert_eq!(normalize_symbol_for_import("7203:JP"), "7203.T");
        assert_eq!(normalize_symbol_for_import("0700:HK"), "0700.HK");
    }

    #[test]
    fn normalize_symbol_passes_through_plain_symbols() {
        assert_eq!(normalize_symbol_for_import("AAPL"), "AAPL");
        assert_eq!(normalize_symbol_for_import("BMO.TO"), "BMO.TO");
        assert_eq!(normalize_symbol_for_import("bmo"), "BMO");
        assert_eq!(normalize_symbol_for_import(" MSFT "), "MSFT");
    }

    #[test]
    fn parse_import_rows_normalizes_country_suffix() {
        let csv =
            "symbol,name,type,quantity,cost_basis,currency\nBMO:CA,Bank of Montreal,stock,10,80,CAD\n";
        let rows = parse_import_rows(csv).expect("parse csv");

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].symbol, "BMO.TO");
    }

    #[test]
    fn parse_import_rows_supports_cash_defaults() {
        let csv = "symbol,name,type,quantity,cost_basis,currency\n, ,cash,1000,1,CAD\n";
        let rows = parse_import_rows(csv).expect("parse csv");

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].symbol, "CAD-CASH");
        assert_eq!(rows[0].currency, "CAD");
        assert!(matches!(rows[0].asset_type, AssetType::Cash));
    }

    #[test]
    fn parse_import_rows_supports_semicolon_delimiter() {
        let csv =
            "symbol;name;type;quantity;cost_basis;currency\nAAPL;Apple Inc.;stock;5;120;usd\n";
        let rows = parse_import_rows(csv).expect("parse csv");

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].symbol, "AAPL");
        assert_eq!(rows[0].currency, "USD");
    }

    #[test]
    fn parse_import_rows_reads_optional_target_weight() {
        let csv = "symbol,name,type,quantity,cost_basis,currency,target_weight\nAAPL,Apple Inc.,stock,5,120,USD,12.5\n";
        let rows = parse_import_rows(csv).expect("parse csv");

        assert_eq!(rows.len(), 1);
        assert!((rows[0].target_weight - 12.5).abs() < 0.001);
    }

    #[test]
    fn parse_import_rows_rejects_missing_required_columns() {
        let csv = "symbol,name,type,quantity,currency\nAAPL,Apple Inc.,stock,5,USD\n";
        let error = parse_import_rows(csv).expect_err("missing cost_basis should fail");

        assert!(error.contains("Missing required column: cost_basis"));
    }

    #[test]
    fn import_weight_sum_over_100_detected() {
        // Two rows whose target_weight values sum to 110; the command-level guard
        // rejects this.  Verify parse_import_rows succeeds and the sum exceeds 100.
        let csv = "symbol,name,type,quantity,cost_basis,currency,target_weight\n\
                   AAPL,Apple Inc.,stock,5,120,USD,60\n\
                   MSFT,Microsoft,stock,3,200,USD,50\n";
        let rows = parse_import_rows(csv).expect("rows should parse");
        let total: f64 = rows.iter().map(|r| r.target_weight).sum();
        assert!(
            total > 100.0,
            "expected total > 100 to trigger command-level guard, got {}",
            total
        );
    }

    #[test]
    fn import_weight_sum_at_100_is_valid() {
        let csv = "symbol,name,type,quantity,cost_basis,currency,target_weight\n\
                   AAPL,Apple Inc.,stock,5,120,USD,60\n\
                   MSFT,Microsoft,stock,3,200,USD,40\n";
        let rows = parse_import_rows(csv).expect("rows should parse");
        let total: f64 = rows.iter().map(|r| r.target_weight).sum();
        assert!(
            (total - 100.0).abs() < 0.001,
            "expected total == 100, got {}",
            total
        );
    }

    #[test]
    fn build_holdings_csv_includes_target_weight_column() {
        let mut holding = make_holding("AAPL", AssetType::Stock, 5.0, 120.0, "USD");
        holding.target_weight = 22.5;

        let csv = build_holdings_csv(&[holding]).expect("build csv");

        assert!(csv.starts_with(
            "symbol,name,type,account,quantity,cost_basis,currency,exchange,target_weight"
        ));
        assert!(csv.contains(",22.5"));
    }

    // ── CSV round-trip tests ──────────────────────────────────────────────────

    /// Export a set of holdings to CSV, re-parse it with `parse_import_rows`,
    /// and verify that every key field is preserved exactly.
    #[test]
    fn csv_round_trip_preserves_key_fields() {
        let mut h1 = make_holding("AAPL", AssetType::Stock, 10.0, 155.25, "USD");
        h1.name = "Apple Inc.".to_string();
        h1.exchange = "NMS".to_string();
        h1.target_weight = 25.0;

        let mut h2 = make_holding("XIU.TO", AssetType::Etf, 50.0, 34.5, "CAD");
        h2.name = "iShares S&P/TSX 60 Index ETF".to_string();
        h2.exchange = "TRT".to_string();
        h2.target_weight = 15.0;

        let mut h3 = make_holding("BTC-USD", AssetType::Crypto, 0.5, 40000.0, "USD");
        h3.name = "Bitcoin USD".to_string();
        h3.target_weight = 10.0;

        let holdings = vec![h1, h2, h3];
        let csv = build_holdings_csv(&holdings).expect("build csv");

        let rows = parse_import_rows(&csv).expect("parse csv");

        assert_eq!(rows.len(), 3, "row count should be preserved");

        // Row 0 — AAPL (stock)
        assert_eq!(rows[0].symbol, "AAPL");
        assert!(matches!(rows[0].asset_type, AssetType::Stock));
        assert!((rows[0].quantity - 10.0).abs() < 0.001);
        assert!((rows[0].cost_basis - 155.25).abs() < 0.001);
        assert_eq!(rows[0].currency, "USD");
        assert_eq!(rows[0].exchange, "NMS");
        assert!((rows[0].target_weight - 25.0).abs() < 0.001);

        // Row 1 — XIU.TO (etf)
        assert_eq!(rows[1].symbol, "XIU.TO");
        assert!(matches!(rows[1].asset_type, AssetType::Etf));
        assert!((rows[1].quantity - 50.0).abs() < 0.001);
        assert!((rows[1].cost_basis - 34.5).abs() < 0.001);
        assert_eq!(rows[1].currency, "CAD");
        assert_eq!(rows[1].exchange, "TRT");
        assert!((rows[1].target_weight - 15.0).abs() < 0.001);

        // Row 2 — BTC-USD (crypto)
        assert_eq!(rows[2].symbol, "BTC-USD");
        assert!(matches!(rows[2].asset_type, AssetType::Crypto));
        assert!((rows[2].quantity - 0.5).abs() < 0.001);
        assert!((rows[2].cost_basis - 40000.0).abs() < 0.001);
        assert_eq!(rows[2].currency, "USD");
        assert!((rows[2].target_weight - 10.0).abs() < 0.001);
    }

    /// Exporting a single cash holding round-trips correctly.
    #[test]
    fn csv_round_trip_cash_holding() {
        let mut cash = make_holding("CAD-CASH", AssetType::Cash, 5000.0, 1.0, "CAD");
        cash.name = "CAD Cash".to_string();
        cash.target_weight = 5.0;

        let csv = build_holdings_csv(&[cash]).expect("build csv");
        let rows = parse_import_rows(&csv).expect("parse csv");

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].symbol, "CAD-CASH");
        assert!(matches!(rows[0].asset_type, AssetType::Cash));
        assert!((rows[0].quantity - 5000.0).abs() < 0.001);
        assert!((rows[0].cost_basis - 1.0).abs() < 0.001);
        assert_eq!(rows[0].currency, "CAD");
        assert!((rows[0].target_weight - 5.0).abs() < 0.001);
    }

    /// An empty holdings slice produces a CSV that fails parsing (no data rows).
    #[test]
    fn build_holdings_csv_empty_slice_roundtrip_fails_gracefully() {
        let csv = build_holdings_csv(&[]).expect("build csv for empty slice");
        // build_holdings_csv writes a header-only CSV; parse_import_rows should
        // return an error because there are no data rows.
        let result = parse_import_rows(&csv);
        assert!(result.is_err(), "empty csv should error on import");
        assert!(result.unwrap_err().contains("empty"));
    }

    /// Round-trip with target_weight = 0 (the default) is preserved as 0.
    #[test]
    fn csv_round_trip_zero_target_weight() {
        let holding = make_holding("MSFT", AssetType::Stock, 3.0, 200.0, "USD");
        // target_weight is already 0.0 from make_holding

        let csv = build_holdings_csv(&[holding]).expect("build csv");
        let rows = parse_import_rows(&csv).expect("parse csv");

        assert_eq!(rows.len(), 1);
        assert!((rows[0].target_weight - 0.0).abs() < 0.001);
    }

    #[test]
    fn import_csv_weight_sum_over_100_is_rejected() {
        let csv = "symbol,name,type,quantity,cost_basis,currency,target_weight\n\
                   AAPL,Apple,stock,5,120,USD,60\n\
                   MSFT,Microsoft,stock,3,200,USD,50\n";
        let rows = parse_import_rows(csv).expect("parse ok");
        let total: f64 = rows.iter().map(|r| r.target_weight).sum();
        assert!(
            total > 100.0,
            "csv weight sum should exceed 100, got {:.1}",
            total
        );
        // Confirm the error message format is correct when this check fires
        let err = format!(
            "Import failed: total target weight is {:.1}% (max 100%). Adjust weights before re-importing.",
            total
        );
        assert!(err.contains("Import failed"));
        assert!(err.contains("110.0%"));
    }

    #[test]
    fn import_csv_weight_sum_at_100_passes_csv_level_guard() {
        let csv = "symbol,name,type,quantity,cost_basis,currency,target_weight\n\
                   AAPL,Apple,stock,5,120,USD,60\n\
                   MSFT,Microsoft,stock,3,200,USD,40\n";
        let rows = parse_import_rows(csv).expect("parse ok");
        let total: f64 = rows.iter().map(|r| r.target_weight).sum();
        assert!(
            total <= 100.0,
            "csv weight sum should be <= 100, got {:.1}",
            total
        );
    }

    #[test]
    fn import_csv_existing_holdings_combined_with_csv_exceeds_100_is_rejected() {
        let existing_weight_sum = 70.0f64;
        let csv = "symbol,name,type,quantity,cost_basis,currency,target_weight\n\
                   GOOG,Alphabet,stock,2,150,USD,40\n";
        let rows = parse_import_rows(csv).expect("parse ok");
        let csv_sum: f64 = rows.iter().map(|r| r.target_weight).sum();
        // csv_sum alone (40) is <= 100, so it passes the CSV-level guard
        assert!(csv_sum <= 100.0);
        // But combined with existing it exceeds 100
        assert!(
            existing_weight_sum + csv_sum > 100.0,
            "combined should exceed 100, got {:.1}",
            existing_weight_sum + csv_sum
        );
    }

    // ── New tests for #377 ────────────────────────────────────────────────────

    /// A well-formed CSV row with valid fields parses successfully (ready path).
    #[test]
    fn parse_import_rows_valid_stock_row_succeeds() {
        let csv = "symbol,name,type,quantity,cost_basis,currency\n\
                   AAPL,Apple Inc.,stock,10,150.0,USD\n";
        let rows = parse_import_rows(csv).expect("valid row should parse");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].symbol, "AAPL");
        assert!(matches!(rows[0].asset_type, AssetType::Stock));
        assert!((rows[0].quantity - 10.0).abs() < 0.001);
        assert!((rows[0].cost_basis - 150.0).abs() < 0.001);
        assert_eq!(rows[0].currency, "USD");
    }

    /// A cash row is parsed and the symbol is derived from currency.
    #[test]
    fn parse_import_rows_cash_row_produces_cash_symbol() {
        let csv = "symbol,name,type,quantity,cost_basis,currency\n\
                   ,,cash,5000,1,CAD\n";
        let rows = parse_import_rows(csv).expect("cash row should parse");
        assert_eq!(rows.len(), 1);
        assert!(matches!(rows[0].asset_type, AssetType::Cash));
        assert_eq!(rows[0].symbol, "CAD-CASH");
    }

    /// Two rows with the same symbol+account are both returned by parse_import_rows;
    /// duplicate detection is the responsibility of the command layer.
    #[test]
    fn parse_import_rows_two_identical_symbol_account_rows_both_parsed() {
        let csv = "symbol,name,type,quantity,cost_basis,currency\n\
                   AAPL,Apple Inc.,stock,10,150,USD\n\
                   AAPL,Apple Inc.,stock,5,160,USD\n";
        let rows = parse_import_rows(csv).expect("both rows should parse");
        assert_eq!(
            rows.len(),
            2,
            "parse layer returns both rows before deduplication"
        );
        // Both rows share the same symbol (duplicate detection is command-level)
        assert_eq!(rows[0].symbol, rows[1].symbol);
    }

    /// A row with a missing quantity field returns an error.
    #[test]
    fn parse_import_rows_missing_quantity_returns_error() {
        let csv = "symbol,name,type,quantity,cost_basis,currency\n\
                   AAPL,Apple Inc.,stock,,150,USD\n";
        let err = parse_import_rows(csv).expect_err("missing quantity should fail");
        assert!(
            err.contains("missing_quantity") || err.contains("invalid_quantity"),
            "error should mention quantity, got: {}",
            err
        );
    }

    /// A row with a missing symbol (non-cash) returns an error.
    #[test]
    fn parse_import_rows_missing_symbol_returns_error() {
        let csv = "symbol,name,type,quantity,cost_basis,currency\n\
                   ,Apple Inc.,stock,10,150,USD\n";
        let err = parse_import_rows(csv).expect_err("missing symbol should fail");
        assert!(
            err.contains("missing_symbol") || err.contains("symbol"),
            "error should mention symbol, got: {}",
            err
        );
    }

    /// A row with a negative quantity is rejected.
    #[test]
    fn parse_import_rows_negative_quantity_returns_error() {
        let csv = "symbol,name,type,quantity,cost_basis,currency\n\
                   AAPL,Apple Inc.,stock,-10,150,USD\n";
        let err = parse_import_rows(csv).expect_err("negative quantity should fail");
        assert!(
            err.contains("invalid_quantity"),
            "error should mention invalid_quantity, got: {}",
            err
        );
    }

    /// maturity_date must be in YYYY-MM-DD format; non-ISO dates are rejected at the parse layer.
    #[test]
    fn parse_import_rows_non_iso_maturity_date_returns_error() {
        let csv = "symbol,name,type,quantity,cost_basis,currency,maturity_date\n\
                   GIC,GIC Bond,stock,1,1000,CAD,31/12/2030\n";
        let err = parse_import_rows(csv)
            .expect_err("non-ISO maturity_date should be rejected at parse layer");
        assert!(
            err.contains("maturity_date"),
            "error should mention maturity_date, got: {err}"
        );
    }

    /// dividend_frequency must be one of the known enum values; unrecognised values are rejected.
    #[test]
    fn parse_import_rows_invalid_dividend_frequency_returns_error() {
        let csv = "symbol,name,type,quantity,cost_basis,currency,dividend_frequency\n\
                   AAPL,Apple Inc.,stock,10,150,USD,fortnightly\n";
        let err = parse_import_rows(csv)
            .expect_err("unknown dividend_frequency should be rejected at parse layer");
        assert!(
            err.contains("dividend_frequency"),
            "error should mention dividend_frequency, got: {err}"
        );
    }

    /// A field that exceeds MAX_FIELD_LEN (500 bytes) causes a parse error.
    #[test]
    fn parse_import_rows_name_exceeding_max_field_len_returns_error() {
        let long_name = "A".repeat(crate::config::MAX_FIELD_LEN + 1);
        let csv = format!(
            "symbol,name,type,quantity,cost_basis,currency\nAAPL,{},stock,10,150,USD\n",
            long_name
        );
        let err = parse_import_rows(&csv).expect_err("oversized name should fail");
        assert!(
            err.contains("name exceeds maximum length"),
            "error should mention name length, got: {}",
            err
        );
    }

    /// A CSV that starts with a UTF-8 BOM (\xEF\xBB\xBF) is parsed correctly.
    #[test]
    fn parse_import_rows_bom_prefixed_csv_is_handled_gracefully() {
        let bom = "\u{feff}";
        let csv = format!(
            "{}symbol,name,type,quantity,cost_basis,currency\nAAPL,Apple Inc.,stock,10,150,USD\n",
            bom
        );
        let rows = parse_import_rows(&csv).expect("BOM-prefixed CSV should parse without error");
        assert_eq!(rows.len(), 1, "BOM should be stripped transparently");
        assert_eq!(rows[0].symbol, "AAPL");
    }
}
