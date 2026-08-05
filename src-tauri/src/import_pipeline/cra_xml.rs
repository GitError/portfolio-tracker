//! CRA T5 and T5008 XML import parser.
//!
//! Parses Canadian Revenue Agency information-return XML files downloaded from
//! a brokerage or the CRA My Account portal:
//!
//! * **T5008** — Statement of Securities Transactions (dispositions / sales).
//!   Returns [`T5008Disposition`] records keyed to the tax year in
//!   `T5008Summary/tx_yr`.
//!
//! * **T5** — Statement of Investment Income (dividends, interest, foreign
//!   income, royalties, capital-gains dividends). Returns [`T5IncomeRecord`]
//!   records, one per income-type box that contains a non-zero amount on each
//!   slip.
//!
//! The top-level entry point is [`parse_cra_xml`], which detects the form type
//! from the file content and dispatches to the appropriate inner parser.
//!
//! Reference: CRA 2026V4 schema.

use serde::Deserialize;

use crate::types::{CraXmlResult, T5008Disposition, T5IncomeRecord};

// ── Internal XML serde structures ────────────────────────────────────────────
// These mirror the CRA XML schema and are never exposed outside this module.

#[derive(Debug, Deserialize)]
struct CraReturn {
    #[serde(rename = "T5008")]
    t5008: Option<T5008Root>,
    #[serde(rename = "T5")]
    t5: Option<T5Root>,
}

// ── T5008 structures ──────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct T5008Root {
    #[serde(rename = "T5008Slip", default)]
    slips: Vec<T5008SlipXml>,
    #[serde(rename = "T5008Summary")]
    summary: Option<T5008SummaryXml>,
}

#[derive(Debug, Deserialize)]
struct T5008SlipXml {
    disp_record: Option<DispRecordXml>,
    ident_record: Option<IdentRecordXml>,
}

#[derive(Debug, Deserialize)]
struct DispRecordXml {
    #[serde(rename = "T5008_AMT")]
    amounts: Option<T5008AmtXml>,
    /// Security type code (Box 15): SHR, UNIT, MUT, BON, OPT, FUT, OTH.
    dsps_scty_tcd: Option<String>,
    /// Quantity disposed (Box 16).
    dsps_scty_cnt: Option<String>,
    /// CUSIP or ISIN (Box 18).
    dsps_cusip_nbr: Option<String>,
    /// Security description (Box 17).
    id_scty_dsps_txt: Option<String>,
    /// Currency ISO 4217 (Box 13).
    fgn_crcy_cd: Option<String>,
}

#[derive(Debug, Deserialize)]
struct T5008AmtXml {
    /// Book value (Box 20).
    cost_bok_val_amt: Option<String>,
    /// Proceeds of disposition (Box 21).
    dispn_amt: Option<String>,
}

#[derive(Debug, Deserialize)]
struct IdentRecordXml {
    /// Broker account number.
    rcpnt_acct_nbr: Option<String>,
}

#[derive(Debug, Deserialize)]
struct T5008SummaryXml {
    tx_yr: Option<String>,
}

// ── T5 structures ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct T5Root {
    #[serde(rename = "T5Slip", default)]
    slips: Vec<T5SlipXml>,
    #[serde(rename = "T5Summary")]
    summary: Option<T5SummaryXml>,
}

#[derive(Debug, Deserialize, Default)]
struct T5SlipXml {
    /// Box 10: actual non-eligible dividend amount.
    actl_dvnd_amt: Option<String>,
    /// Box 24: actual eligible dividend amount.
    actl_elg_dvamt: Option<String>,
    /// Box 13: Canadian interest income.
    cdn_int_amt: Option<String>,
    /// Box 14: other Canadian income.
    oth_cdn_incamt: Option<String>,
    /// Box 15: foreign income (dividends, etc.).
    fgn_incamt: Option<String>,
    /// Box 17: Canadian royalty income.
    cdn_royl_amt: Option<String>,
    /// Box 18: capital gains dividend (post 2024-06-25).
    cgain_dvnd_amt: Option<String>,
    /// Box 16: foreign tax paid (withholding tax).
    fgn_tx_pay_amt: Option<String>,
    /// Box 27: currency code (ISO 4217); blank for CAD.
    fgn_crcy_ind: Option<String>,
    /// Box 29: broker account number.
    rcpnt_fi_acct_nbr: Option<String>,
}

#[derive(Debug, Deserialize)]
struct T5SummaryXml {
    tx_yr: Option<String>,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Parse a decimal string (e.g. "1234.56") to f64, returning `None` on blank
/// or unparseable input.
fn parse_decimal(s: Option<&str>) -> Option<f64> {
    s.and_then(|v| {
        let trimmed = v.trim();
        if trimmed.is_empty() {
            None
        } else {
            trimmed.parse::<f64>().ok()
        }
    })
}

/// Map a CRA T5008 security type code to an app-level asset type string.
///
/// | Code   | CRA description        | App type  |
/// |--------|------------------------|-----------|
/// | `SHR`  | Shares (equities)      | `"stock"` |
/// | `UNIT` | Units (ETFs / trusts)  | `"etf"`   |
/// | `MUT`  | Mutual fund units      | `"etf"`   |
/// | `BON`  | Bonds / debentures     | `"other"` |
/// | `OPT`  | Options (skipped)      | `"other"` |
/// | `FUT`  | Futures (skipped)      | `"other"` |
/// | `OTH`  | Other                  | `"other"` |
fn map_t5008_asset_type(code: Option<&str>) -> &'static str {
    match code.map(str::trim).unwrap_or("") {
        "SHR" => "stock",
        "UNIT" | "MUT" => "etf",
        _ => "other",
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Read a CRA XML file from disk, detect its form type (T5 or T5008), and
/// return a typed [`CraXmlResult`].
///
/// Returns `Err(String)` on I/O errors, XML parse errors, or if the file does
/// not contain a recognisable T5 or T5008 root element.
pub fn parse_cra_xml_file(file_path: &str) -> Result<CraXmlResult, String> {
    let xml =
        std::fs::read_to_string(file_path).map_err(|e| format!("Cannot read CRA XML file: {e}"))?;
    parse_cra_xml(&xml)
}

/// Parse a CRA XML string and return a typed [`CraXmlResult`].
///
/// Exported for unit testing without touching the filesystem.
pub fn parse_cra_xml(xml: &str) -> Result<CraXmlResult, String> {
    let ret: CraReturn =
        quick_xml::de::from_str(xml).map_err(|e| format!("CRA XML parse error: {e}"))?;

    if let Some(t5008) = ret.t5008 {
        return Ok(CraXmlResult::T5008 {
            dispositions: map_t5008(t5008),
        });
    }
    if let Some(t5) = ret.t5 {
        return Ok(CraXmlResult::T5 { income: map_t5(t5) });
    }
    Err("CRA XML file does not contain a T5 or T5008 element".to_string())
}

/// Convert deserialized T5008 XML structures into public `T5008Disposition` records.
fn map_t5008(root: T5008Root) -> Vec<T5008Disposition> {
    let tax_year: u16 = root
        .summary
        .as_ref()
        .and_then(|s| s.tx_yr.as_deref())
        .and_then(|y| y.trim().parse().ok())
        .unwrap_or(0);

    root.slips
        .into_iter()
        .filter_map(|slip| {
            let disp = slip.disp_record?;
            let account_number = slip
                .ident_record
                .and_then(|ir| ir.rcpnt_acct_nbr)
                .filter(|s| !s.trim().is_empty());

            Some(T5008Disposition {
                name: disp
                    .id_scty_dsps_txt
                    .map(|s| s.trim().to_string())
                    .unwrap_or_default(),
                asset_type: map_t5008_asset_type(disp.dsps_scty_tcd.as_deref()).to_string(),
                quantity: parse_decimal(disp.dsps_scty_cnt.as_deref()),
                cusip_isin: disp
                    .dsps_cusip_nbr
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty()),
                currency: disp
                    .fgn_crcy_cd
                    .map(|s| s.trim().to_uppercase())
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| "CAD".to_string()),
                book_value: parse_decimal(
                    disp.amounts
                        .as_ref()
                        .and_then(|a| a.cost_bok_val_amt.as_deref()),
                ),
                proceeds: parse_decimal(disp.amounts.as_ref().and_then(|a| a.dispn_amt.as_deref())),
                tax_year,
                account_number,
            })
        })
        .collect()
}

/// Convert deserialized T5 XML structures into public `T5IncomeRecord`s.
///
/// Each non-zero income box on a slip produces one record so the caller can
/// display them individually and let the user associate each with a holding.
fn map_t5(root: T5Root) -> Vec<T5IncomeRecord> {
    let tax_year: u16 = root
        .summary
        .as_ref()
        .and_then(|s| s.tx_yr.as_deref())
        .and_then(|y| y.trim().parse().ok())
        .unwrap_or(0);

    let mut records = Vec::new();

    for slip in root.slips {
        let currency = slip
            .fgn_crcy_ind
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_uppercase)
            .unwrap_or_else(|| "CAD".to_string());
        let withholding_tax = parse_decimal(slip.fgn_tx_pay_amt.as_deref());
        let account_number = slip.rcpnt_fi_acct_nbr.filter(|s| !s.trim().is_empty());

        // Emit one record per income type that has a non-zero amount.
        let boxes: &[(&str, Option<&str>)] = &[
            ("dividend_non_eligible", slip.actl_dvnd_amt.as_deref()),
            ("dividend_eligible", slip.actl_elg_dvamt.as_deref()),
            ("interest", slip.cdn_int_amt.as_deref()),
            ("other_canadian", slip.oth_cdn_incamt.as_deref()),
            ("foreign_income", slip.fgn_incamt.as_deref()),
            ("royalty", slip.cdn_royl_amt.as_deref()),
            ("capital_gains_dividend", slip.cgain_dvnd_amt.as_deref()),
        ];

        for (income_type, raw) in boxes {
            if let Some(amount) = parse_decimal(*raw) {
                if amount.abs() > f64::EPSILON {
                    records.push(T5IncomeRecord {
                        income_type: income_type.to_string(),
                        amount,
                        currency: currency.clone(),
                        withholding_tax,
                        tax_year,
                        account_number: account_number.clone(),
                    });
                }
            }
        }
    }

    records
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const T5008_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Return>
  <T5008>
    <T5008Slip>
      <disp_record>
        <T5008_AMT>
          <cost_bok_val_amt>1500.00</cost_bok_val_amt>
          <dispn_amt>2000.00</dispn_amt>
        </T5008_AMT>
        <dsps_scty_tcd>SHR</dsps_scty_tcd>
        <dsps_scty_cnt>10.0000</dsps_scty_cnt>
        <dsps_cusip_nbr>037833100</dsps_cusip_nbr>
        <id_scty_dsps_txt>APPLE INC</id_scty_dsps_txt>
        <fgn_crcy_cd>USD</fgn_crcy_cd>
      </disp_record>
      <ident_record>
        <rcpnt_acct_nbr>ABC-12345</rcpnt_acct_nbr>
      </ident_record>
    </T5008Slip>
    <T5008Slip>
      <disp_record>
        <T5008_AMT>
          <cost_bok_val_amt>800.00</cost_bok_val_amt>
          <dispn_amt>950.00</dispn_amt>
        </T5008_AMT>
        <dsps_scty_tcd>UNIT</dsps_scty_tcd>
        <dsps_scty_cnt>20.0000</dsps_scty_cnt>
        <id_scty_dsps_txt>VANGUARD S&amp;P 500 ETF</id_scty_dsps_txt>
        <fgn_crcy_cd>CAD</fgn_crcy_cd>
      </disp_record>
    </T5008Slip>
    <T5008Summary>
      <tx_yr>2025</tx_yr>
    </T5008Summary>
  </T5008>
</Return>"#;

    const T5_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Return>
  <T5>
    <T5Slip>
      <actl_dvnd_amt>120.50</actl_dvnd_amt>
      <actl_elg_dvamt>85.00</actl_elg_dvamt>
      <fgn_incamt>45.00</fgn_incamt>
      <fgn_tx_pay_amt>6.75</fgn_tx_pay_amt>
      <fgn_crcy_ind>CAD</fgn_crcy_ind>
      <rcpnt_fi_acct_nbr>ABC-12345</rcpnt_fi_acct_nbr>
    </T5Slip>
    <T5Summary>
      <tx_yr>2025</tx_yr>
    </T5Summary>
  </T5>
</Return>"#;

    // ── parse_cra_xml rejects unrecognised content ─────────────────────────

    #[test]
    fn unknown_xml_returns_error() {
        let result = parse_cra_xml("<Return><T4><tx_yr>2025</tx_yr></T4></Return>");
        assert!(
            result.is_err(),
            "XML with no T5 or T5008 element should return Err"
        );
        assert!(result.unwrap_err().contains("T5"));
    }

    // ── T5008 parsing ──────────────────────────────────────────────────────

    #[test]
    fn t5008_parses_two_slips() {
        let result = parse_cra_xml(T5008_XML).expect("parse");
        let CraXmlResult::T5008 { dispositions } = result else {
            panic!("expected T5008 result");
        };
        assert_eq!(dispositions.len(), 2);
    }

    #[test]
    fn t5008_first_slip_fields_are_correct() {
        let CraXmlResult::T5008 { dispositions } = parse_cra_xml(T5008_XML).expect("parse") else {
            panic!()
        };
        let d = &dispositions[0];
        assert_eq!(d.name, "APPLE INC");
        assert_eq!(d.asset_type, "stock");
        assert_eq!(d.currency, "USD");
        assert_eq!(d.tax_year, 2025);
        assert!((d.proceeds.unwrap() - 2_000.0).abs() < 0.01);
        assert!((d.book_value.unwrap() - 1_500.0).abs() < 0.01);
        assert!((d.quantity.unwrap() - 10.0).abs() < 0.01);
        assert_eq!(d.cusip_isin.as_deref(), Some("037833100"));
        assert_eq!(d.account_number.as_deref(), Some("ABC-12345"));
    }

    #[test]
    fn t5008_etf_slip_mapped_correctly() {
        let CraXmlResult::T5008 { dispositions } = parse_cra_xml(T5008_XML).expect("parse") else {
            panic!()
        };
        let d = &dispositions[1];
        assert_eq!(d.asset_type, "etf"); // UNIT → etf
        assert_eq!(d.currency, "CAD");
        assert!(d.account_number.is_none()); // no ident_record
    }

    #[test]
    fn t5008_security_type_mapping() {
        assert_eq!(map_t5008_asset_type(Some("SHR")), "stock");
        assert_eq!(map_t5008_asset_type(Some("UNIT")), "etf");
        assert_eq!(map_t5008_asset_type(Some("MUT")), "etf");
        assert_eq!(map_t5008_asset_type(Some("BON")), "other");
        assert_eq!(map_t5008_asset_type(Some("OPT")), "other");
        assert_eq!(map_t5008_asset_type(Some("FUT")), "other");
        assert_eq!(map_t5008_asset_type(Some("OTH")), "other");
        assert_eq!(map_t5008_asset_type(None), "other");
    }

    // ── T5 parsing ─────────────────────────────────────────────────────────

    #[test]
    fn t5_parses_three_income_records() {
        let CraXmlResult::T5 { income } = parse_cra_xml(T5_XML).expect("parse") else {
            panic!("expected T5 result")
        };
        // 3 non-zero boxes: actl_dvnd_amt, actl_elg_dvamt, fgn_incamt
        assert_eq!(income.len(), 3);
    }

    #[test]
    fn t5_non_eligible_dividend_record() {
        let CraXmlResult::T5 { income } = parse_cra_xml(T5_XML).expect("parse") else {
            panic!()
        };
        let rec = income
            .iter()
            .find(|r| r.income_type == "dividend_non_eligible")
            .expect("non-eligible dividend record");
        assert!((rec.amount - 120.50).abs() < 0.01);
        assert_eq!(rec.currency, "CAD");
        assert!((rec.withholding_tax.unwrap() - 6.75).abs() < 0.01);
        assert_eq!(rec.tax_year, 2025);
        assert_eq!(rec.account_number.as_deref(), Some("ABC-12345"));
    }

    #[test]
    fn t5_eligible_dividend_record() {
        let CraXmlResult::T5 { income } = parse_cra_xml(T5_XML).expect("parse") else {
            panic!()
        };
        let rec = income
            .iter()
            .find(|r| r.income_type == "dividend_eligible")
            .expect("eligible dividend record");
        assert!((rec.amount - 85.0).abs() < 0.01);
    }

    #[test]
    fn t5_foreign_income_record() {
        let CraXmlResult::T5 { income } = parse_cra_xml(T5_XML).expect("parse") else {
            panic!()
        };
        let rec = income
            .iter()
            .find(|r| r.income_type == "foreign_income")
            .expect("foreign income record");
        assert!((rec.amount - 45.0).abs() < 0.01);
    }

    #[test]
    fn t5_zero_amount_boxes_are_omitted() {
        // interest and royalty are not in the fixture XML at all — quick-xml
        // leaves them as None, so they must not appear in the output.
        let CraXmlResult::T5 { income } = parse_cra_xml(T5_XML).expect("parse") else {
            panic!()
        };
        let types: Vec<&str> = income.iter().map(|r| r.income_type.as_str()).collect();
        assert!(!types.contains(&"interest"));
        assert!(!types.contains(&"royalty"));
    }

    // ── parse_decimal helper ───────────────────────────────────────────────

    #[test]
    fn parse_decimal_handles_edge_cases() {
        assert_eq!(parse_decimal(None), None);
        assert_eq!(parse_decimal(Some("")), None);
        assert_eq!(parse_decimal(Some("  ")), None);
        assert!((parse_decimal(Some("1234.56")).unwrap() - 1234.56).abs() < 0.001);
        assert_eq!(parse_decimal(Some("not-a-number")), None);
    }
}
