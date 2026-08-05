//! Converts an XLSX workbook's first sheet into CSV text so it can be fed
//! through the existing text-based `parser::detect_and_parse` pipeline
//! unchanged — the multi-section marker-line detection, generic-CSV
//! fallback, and row normalization all stay CSV-shaped either way.

use calamine::{open_workbook, Data, Reader, Xlsx};

use crate::error::AppError;

/// Reads the first sheet of the XLSX file at `path` and reconstructs it as
/// CSV text, one line per row. Trailing empty cells on each row are dropped
/// before writing, so a marker line with only its first cell populated (e.g.
/// `Cash Details`) round-trips as a bare single-field line rather than
/// `Cash Details,,,,,,` — matching what `parser::detect_and_parse`'s exact
/// line-equality checks expect from a real CSV export.
pub fn read_xlsx_as_csv(path: &std::path::Path) -> Result<String, AppError> {
    let mut workbook: Xlsx<_> = open_workbook(path)
        .map_err(|e| AppError::Validation(format!("Cannot read XLSX file: {e}")))?;
    let sheet_name = workbook
        .sheet_names()
        .first()
        .cloned()
        .ok_or_else(|| AppError::Validation("XLSX file has no sheets".to_string()))?;
    let range = workbook
        .worksheet_range(&sheet_name)
        .map_err(|e| AppError::Validation(format!("Cannot read XLSX sheet '{sheet_name}': {e}")))?;

    // `flexible(true)`: unlike a CSV file's data rows, a reconstructed sheet
    // legitimately has different field counts per line — a marker line like
    // `Cash Details` has one field, a header row has several, and a blank
    // separator row has zero. The default writer rejects a varying field
    // count as malformed; this pipeline's multi-section format depends on it.
    let mut writer = ::csv::WriterBuilder::new()
        .flexible(true)
        .from_writer(vec![]);
    for row in range.rows() {
        let mut fields: Vec<String> = row.iter().map(cell_to_string).collect();
        while fields.last().is_some_and(|f| f.is_empty()) {
            fields.pop();
        }
        writer
            .write_record(&fields)
            .map_err(|e| AppError::Validation(format!("Cannot convert XLSX row to CSV: {e}")))?;
    }
    let bytes = writer.into_inner().map_err(|e| {
        AppError::Validation(format!("Cannot finalize XLSX-to-CSV conversion: {e}"))
    })?;
    String::from_utf8(bytes)
        .map_err(|e| AppError::Validation(format!("XLSX file contains invalid UTF-8 text: {e}")))
}

fn cell_to_string(cell: &Data) -> String {
    match cell {
        Data::Empty => String::new(),
        Data::String(s) => s.clone(),
        Data::Int(i) => i.to_string(),
        Data::Float(f) => format_float(*f),
        Data::Bool(b) => b.to_string(),
        Data::DateTime(dt) => dt
            .as_datetime()
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_default(),
        Data::DateTimeIso(s) | Data::DurationIso(s) => s.clone(),
        Data::Error(e) => format!("{e:?}"),
    }
}

/// Renders a whole-numbered float without a trailing ".0" (`"100"` rather
/// than `"100.0"`), matching how quantities/prices appear in a real CSV
/// export. Downstream `str::parse::<f64>` accepts either form, so this is a
/// readability choice, not a correctness requirement.
fn format_float(f: f64) -> String {
    if f.fract() == 0.0 && f.abs() < 1e15 {
        (f as i64).to_string()
    } else {
        f.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whole_number_float_renders_without_decimal() {
        assert_eq!(format_float(100.0), "100");
    }

    #[test]
    fn fractional_float_renders_with_decimal() {
        assert_eq!(format_float(135.5045), "135.5045");
    }

    #[test]
    fn string_cell_passes_through_unchanged() {
        assert_eq!(
            cell_to_string(&Data::String("AAPL:US".to_string())),
            "AAPL:US"
        );
    }

    #[test]
    fn empty_cell_becomes_empty_string() {
        assert_eq!(cell_to_string(&Data::Empty), "");
    }

    #[test]
    fn int_cell_renders_as_plain_integer() {
        assert_eq!(cell_to_string(&Data::Int(100)), "100");
    }
}
