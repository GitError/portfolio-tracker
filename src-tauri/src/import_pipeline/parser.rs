//! Format detection and section splitting for the import pipeline.
//!
//! Two shapes are supported:
//! - `CanadianBankMultiSection`: a multi-section document (account header,
//!   Cash Details block, Holding Details block, Exchange Rate footer), as
//!   validated against TD Direct Investing RRSP/TFSA portfolio reports.
//! - Generic flat CSV: a single header row followed by data rows.

use std::collections::HashMap;

use ::csv::ReaderBuilder;

use crate::csv::detect_csv_delimiter;

pub const PROFILE_CANADIAN_BANK_MULTI_SECTION: &str = "CanadianBankMultiSection";
pub const PROFILE_GENERIC_CSV: &str = "GenericCSV";

/// One data row plus its 1-indexed absolute line number in the source file,
/// preserved for the preview's "source row number" field and error messages.
#[derive(Debug, Clone, PartialEq)]
pub struct RawRow {
    pub row_number: usize,
    pub values: HashMap<String, String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ParsedSection {
    pub headers: Vec<String>,
    pub rows: Vec<RawRow>,
}

#[derive(Debug, Clone)]
pub struct DetectedFile {
    pub profile: &'static str,
    pub suggested_account_type: Option<String>,
    pub suggested_account_number: Option<String>,
    pub cash_section: Option<ParsedSection>,
    pub holdings_section: ParsedSection,
}

/// Parses one CSV line into trimmed fields, honoring quoted commas via the
/// `csv` crate rather than a naive `split(',')`.
fn parse_line_as_fields(line: &str, delimiter: u8) -> Result<Vec<String>, String> {
    let mut reader = ReaderBuilder::new()
        .has_headers(false)
        .delimiter(delimiter)
        .from_reader(line.as_bytes());
    match reader.records().next() {
        Some(Ok(record)) => Ok(record.iter().map(|f| f.trim().to_string()).collect()),
        Some(Err(e)) => Err(e.to_string()),
        None => Ok(Vec::new()),
    }
}

fn build_row(headers: &[String], fields: &[String], row_number: usize) -> RawRow {
    let mut values = HashMap::with_capacity(headers.len());
    for (i, header) in headers.iter().enumerate() {
        values.insert(header.clone(), fields.get(i).cloned().unwrap_or_default());
    }
    RawRow { row_number, values }
}

/// Line 1 heuristic: `^Portfolio report for .+ account #`.
fn is_multi_section(first_line: &str) -> bool {
    first_line.starts_with("Portfolio report for") && first_line.contains("account #")
}

/// Extracts `(account_type, account_number)` from a line like
/// `Portfolio report for RRSP account # nnn as of 2026-01-01T00:00:00`.
fn extract_account_context(line: &str) -> Option<(String, String)> {
    let rest = line.strip_prefix("Portfolio report for ")?;
    let (account_type, rest) = rest.split_once(" account # ")?;
    let account_number = rest.split_whitespace().next()?;
    Some((account_type.trim().to_string(), account_number.to_string()))
}

/// Collects contiguous non-blank data lines starting at `start`, stopping at
/// the first blank line, the `Exchange Rate:` footer, or end of file.
/// Returns the collected lines and the number of lines consumed (including
/// any trailing blank line, so the caller can resume scanning after it).
fn collect_section_lines(lines: &[&str], start: usize) -> Vec<(usize, String)> {
    let mut collected = Vec::new();
    let mut idx = start;
    while idx < lines.len() {
        let line = lines[idx];
        if line.trim().is_empty() || line.trim_start().starts_with("Exchange Rate:") {
            break;
        }
        collected.push((idx, line.to_string()));
        idx += 1;
    }
    collected
}

fn parse_section(
    lines: &[&str],
    header_line_idx: usize,
    delimiter: u8,
) -> Result<ParsedSection, String> {
    let headers = parse_line_as_fields(lines[header_line_idx], delimiter)?;
    let data_lines = collect_section_lines(lines, header_line_idx + 1);
    let mut rows = Vec::with_capacity(data_lines.len());
    for (line_idx, line) in data_lines {
        let fields = parse_line_as_fields(&line, delimiter)?;
        if fields.iter().all(|f| f.trim().is_empty()) {
            continue;
        }
        // Absolute file line number, 1-indexed.
        rows.push(build_row(&headers, &fields, line_idx + 1));
    }
    Ok(ParsedSection { headers, rows })
}

fn find_line_index(lines: &[&str], exact: &str) -> Option<usize> {
    lines.iter().position(|l| l.trim() == exact)
}

fn parse_multi_section(content: &str) -> Result<DetectedFile, String> {
    let lines: Vec<&str> = content.lines().collect();
    let delimiter = b',';

    let first_line = lines
        .iter()
        .find(|l| !l.trim().is_empty())
        .copied()
        .unwrap_or_default();
    let (suggested_account_type, suggested_account_number) =
        match extract_account_context(first_line) {
            Some((t, n)) => (Some(t), Some(n)),
            None => (None, None),
        };

    let cash_section = match find_line_index(&lines, "Cash Details") {
        Some(idx) => {
            let header_idx = idx + 1;
            if header_idx >= lines.len() {
                None
            } else {
                Some(parse_section(&lines, header_idx, delimiter)?)
            }
        }
        None => None,
    };

    let holding_details_idx = find_line_index(&lines, "Holding Details")
        .ok_or_else(|| "Multi-section file is missing a 'Holding Details' section".to_string())?;
    let header_idx = holding_details_idx + 1;
    if header_idx >= lines.len() || !lines[header_idx].starts_with("Asset Class,") {
        return Err(
            "'Holding Details' section is missing its 'Asset Class,...' header row".to_string(),
        );
    }
    let holdings_section = parse_section(&lines, header_idx, delimiter)?;

    Ok(DetectedFile {
        profile: PROFILE_CANADIAN_BANK_MULTI_SECTION,
        suggested_account_type,
        suggested_account_number,
        cash_section,
        holdings_section,
    })
}

/// Generic flat CSV fallback: the first line with at least 3 non-empty
/// comma-separated fields is treated as the header row; every subsequent
/// non-blank line is a data row, read to end of file.
fn parse_generic_flat(content: &str) -> Result<DetectedFile, String> {
    let delimiter = detect_csv_delimiter(content);
    let lines: Vec<&str> = content.lines().collect();

    let header_idx = lines
        .iter()
        .position(|l| {
            if l.trim().is_empty() {
                return false;
            }
            match parse_line_as_fields(l, delimiter) {
                Ok(fields) => fields.iter().filter(|f| !f.trim().is_empty()).count() >= 3,
                Err(_) => false,
            }
        })
        .ok_or_else(|| {
            "No header row found (need at least 3 comma-separated columns)".to_string()
        })?;

    let headers = parse_line_as_fields(lines[header_idx], delimiter)?;
    let mut rows = Vec::new();
    for (i, line) in lines.iter().enumerate().skip(header_idx + 1) {
        if line.trim().is_empty() {
            continue;
        }
        let fields = parse_line_as_fields(line, delimiter)?;
        if fields.iter().all(|f| f.trim().is_empty()) {
            continue;
        }
        rows.push(build_row(&headers, &fields, i + 1));
    }

    Ok(DetectedFile {
        profile: PROFILE_GENERIC_CSV,
        suggested_account_type: None,
        suggested_account_number: None,
        cash_section: None,
        holdings_section: ParsedSection { headers, rows },
    })
}

/// Detects the source file's format and splits it into sections.
pub fn detect_and_parse(content: &str) -> Result<DetectedFile, String> {
    let content = content.trim_start_matches('\u{feff}');
    let first_line = content.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
    if is_multi_section(first_line) {
        parse_multi_section(content)
    } else {
        parse_generic_flat(content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn detects_multi_section_format() {
        let detected = detect_and_parse(SAMPLE).expect("should parse");
        assert_eq!(detected.profile, PROFILE_CANADIAN_BANK_MULTI_SECTION);
    }

    #[test]
    fn extracts_account_type_and_number_from_line_one() {
        let detected = detect_and_parse(SAMPLE).expect("should parse");
        assert_eq!(detected.suggested_account_type, Some("RRSP".to_string()));
        assert_eq!(detected.suggested_account_number, Some("12345".to_string()));
    }

    #[test]
    fn splits_cash_details_section() {
        let detected = detect_and_parse(SAMPLE).expect("should parse");
        let cash = detected
            .cash_section
            .expect("cash section should be present");
        assert_eq!(
            cash.headers,
            vec!["Currency", "Account Type", "Settled Cash", "Trade Cash"]
        );
        assert_eq!(cash.rows.len(), 1);
        assert_eq!(cash.rows[0].values.get("Currency").unwrap(), "CAD");
        assert_eq!(cash.rows[0].values.get("Settled Cash").unwrap(), "89192.24");
    }

    #[test]
    fn splits_holding_details_section() {
        let detected = detect_and_parse(SAMPLE).expect("should parse");
        let holdings = &detected.holdings_section;
        assert_eq!(holdings.headers[0], "Asset Class");
        assert_eq!(holdings.rows.len(), 1);
        assert_eq!(holdings.rows[0].values.get("Symbol").unwrap(), "AAPL:US");
        assert_eq!(holdings.rows[0].values.get("Quantity").unwrap(), "100");
    }

    #[test]
    fn blank_lines_between_sections_are_not_treated_as_data_rows() {
        let detected = detect_and_parse(SAMPLE).expect("should parse");
        // One cash row, one holding row — the blank separator lines must not
        // have been counted as (empty) data rows.
        assert_eq!(detected.cash_section.unwrap().rows.len(), 1);
        assert_eq!(detected.holdings_section.rows.len(), 1);
    }

    #[test]
    fn ignores_exchange_rate_footer() {
        let detected = detect_and_parse(SAMPLE).expect("should parse");
        // The footer line must never show up as a holdings/cash data row.
        for row in &detected.holdings_section.rows {
            for v in row.values.values() {
                assert!(!v.contains("Exchange Rate"));
            }
        }
    }

    #[test]
    fn row_numbers_reflect_absolute_file_line_numbers() {
        let detected = detect_and_parse(SAMPLE).expect("should parse");
        // "Equity,..." is line 9 (1-indexed) in SAMPLE.
        assert_eq!(detected.holdings_section.rows[0].row_number, 9);
        // "CAD,CASH,..." is line 5 (1-indexed).
        assert_eq!(detected.cash_section.unwrap().rows[0].row_number, 5);
    }

    #[test]
    fn multi_section_missing_holding_details_errors() {
        let content = "Portfolio report for TFSA account # 999 as of now\n\nCash Details\nCurrency,Account Type,Settled Cash,Trade Cash\nCAD,CASH,1.0,1.0\n";
        let err = detect_and_parse(content).expect_err("should error without Holding Details");
        assert!(err.contains("Holding Details"));
    }

    #[test]
    fn detects_generic_flat_csv_when_no_multi_section_header() {
        let content = "Symbol,Name,Quantity,Currency\nAAPL,Apple Inc.,10,USD\n";
        let detected = detect_and_parse(content).expect("should parse");
        assert_eq!(detected.profile, PROFILE_GENERIC_CSV);
        assert_eq!(detected.holdings_section.rows.len(), 1);
        assert!(detected.cash_section.is_none());
    }

    #[test]
    fn generic_flat_csv_supports_semicolon_delimiter() {
        let content = "Symbol;Name;Quantity;Currency\nAAPL;Apple Inc.;10;USD\n";
        let detected = detect_and_parse(content).expect("should parse");
        assert_eq!(
            detected.holdings_section.headers,
            vec!["Symbol", "Name", "Quantity", "Currency"]
        );
        assert_eq!(
            detected.holdings_section.rows[0]
                .values
                .get("Symbol")
                .unwrap(),
            "AAPL"
        );
    }

    #[test]
    fn generic_flat_csv_skips_leading_junk_lines_before_header() {
        let content = "Export generated 2026-01-01\n\nSymbol,Name,Quantity,Currency\nAAPL,Apple Inc.,10,USD\n";
        let detected = detect_and_parse(content).expect("should parse");
        assert_eq!(detected.holdings_section.headers[0], "Symbol");
        assert_eq!(detected.holdings_section.rows.len(), 1);
    }

    #[test]
    fn bom_prefixed_multi_section_file_is_handled() {
        let content = format!("\u{feff}{}", SAMPLE);
        let detected = detect_and_parse(&content).expect("should parse");
        assert_eq!(detected.profile, PROFILE_CANADIAN_BANK_MULTI_SECTION);
    }
}
