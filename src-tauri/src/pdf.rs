use chrono::Utc;
use genpdf::elements::{Break, CellDecorator, LinearLayout, Paragraph, StyledElement, TableLayout};
use genpdf::error::Error as GenPdfError;
use genpdf::fonts::{FontData, FontFamily};
use genpdf::render::Area;
use genpdf::style::{Color, Style};
use genpdf::{
    Alignment, Context, Document, Element, Margins, Mm, PageDecorator, PaperSize, Position,
};

use crate::types::{AssetType, PortfolioSnapshot};

const FONT_REGULAR: &[u8] = include_bytes!("../assets/fonts/IBMPlexSans-Regular.ttf");
const FONT_BOLD: &[u8] = include_bytes!("../assets/fonts/IBMPlexSans-Bold.ttf");

const COLOR_GAIN: Color = Color::Rgb(0, 212, 170);
const COLOR_LOSS: Color = Color::Rgb(255, 71, 87);
const COLOR_MUTED: Color = Color::Greyscale(110);
const COLOR_BORDER: Color = Color::Greyscale(190);
const COLOR_ROW_SHADE: Color = Color::Greyscale(242);

/// Builds a point-in-time PDF snapshot of the portfolio: header, summary, a
/// paginated holdings table, and an asset-class allocation breakdown, with a
/// repeating footer on every page.
pub fn build_portfolio_pdf(snapshot: &PortfolioSnapshot) -> Result<Vec<u8>, String> {
    let font_family = load_font_family()?;
    let mut doc = Document::new(font_family);
    doc.set_title("Portfolio Tracker Export");
    doc.set_paper_size(PaperSize::A4);
    doc.set_page_decorator(FooterPageDecorator::new());

    doc.push(build_header(snapshot));
    doc.push(Break::new(1.5));

    doc.push(section_title("Summary"));
    doc.push(Break::new(0.4));
    doc.push(build_summary(snapshot)?);
    doc.push(Break::new(1.5));

    doc.push(section_title("Holdings"));
    doc.push(Break::new(0.4));
    doc.push(build_holdings_table(snapshot)?);
    doc.push(Break::new(1.5));

    doc.push(section_title("Allocation"));
    doc.push(Break::new(0.4));
    doc.push(build_allocation(snapshot)?);

    let mut buffer = Vec::new();
    doc.render(&mut buffer)
        .map_err(|e| format!("Failed to render PDF: {e}"))?;
    Ok(buffer)
}

/// `genpdf` requires embedded font data (no system-font fallback). Only Regular and
/// Bold TTFs are bundled, so the italic/bold-italic slots reuse them — this document
/// never requests italic text.
fn load_font_family() -> Result<FontFamily<FontData>, String> {
    let regular = FontData::new(FONT_REGULAR.to_vec(), None)
        .map_err(|e| format!("Failed to load embedded regular font: {e}"))?;
    let bold = FontData::new(FONT_BOLD.to_vec(), None)
        .map_err(|e| format!("Failed to load embedded bold font: {e}"))?;
    Ok(FontFamily {
        regular: regular.clone(),
        italic: regular,
        bold: bold.clone(),
        bold_italic: bold,
    })
}

fn build_header(snapshot: &PortfolioSnapshot) -> LinearLayout {
    let title_style = Style::new().bold().with_font_size(20);
    let meta_style = Style::new().with_font_size(9).with_color(COLOR_MUTED);
    let exported_at = Utc::now().format("%Y-%m-%d %H:%M UTC");

    LinearLayout::vertical()
        .element(cell("Portfolio Tracker", title_style, Alignment::Left))
        .element(Break::new(0.3))
        .element(cell(
            &format!(
                "Exported {exported_at} \u{b7} Base currency: {}",
                snapshot.base_currency
            ),
            meta_style,
            Alignment::Left,
        ))
}

fn section_title(title: &str) -> StyledElement<Paragraph> {
    cell(
        title,
        Style::new().bold().with_font_size(12),
        Alignment::Left,
    )
}

fn build_summary(snapshot: &PortfolioSnapshot) -> Result<TableLayout, String> {
    let label_style = Style::new().with_font_size(8).with_color(COLOR_MUTED);
    let value_style = Style::new().bold().with_font_size(13);

    let mut table = TableLayout::new(vec![1, 1, 1]);
    push_row(
        &mut table,
        [
            summary_cell(
                "Total Value",
                &format_currency(snapshot.total_value, &snapshot.base_currency),
                label_style,
                value_style,
            ),
            summary_cell(
                "Daily P&L",
                &format_currency(snapshot.daily_pnl, &snapshot.base_currency),
                label_style,
                value_style.with_color(pnl_color(snapshot.daily_pnl)),
            ),
            summary_cell(
                "Total Gain/Loss",
                &format!(
                    "{} ({})",
                    format_currency(snapshot.total_gain_loss, &snapshot.base_currency),
                    format_signed_percent(snapshot.total_gain_loss_percent)
                ),
                label_style,
                value_style.with_color(pnl_color(snapshot.total_gain_loss)),
            ),
        ],
    )?;

    Ok(table)
}

fn summary_cell(label: &str, value: &str, label_style: Style, value_style: Style) -> LinearLayout {
    LinearLayout::vertical()
        .element(cell(label, label_style, Alignment::Left))
        .element(Break::new(0.2))
        .element(cell(value, value_style, Alignment::Left))
}

fn build_holdings_table(snapshot: &PortfolioSnapshot) -> Result<TableLayout, String> {
    let mut table = TableLayout::new(vec![2, 3, 2, 2, 2, 2, 3, 3, 2]);
    table.set_cell_decorator(AlternatingRowDecorator);

    let header_style = Style::new().bold().with_font_size(8);
    push_row(
        &mut table,
        [
            cell("Symbol", header_style, Alignment::Left),
            cell("Name", header_style, Alignment::Left),
            cell("Account", header_style, Alignment::Left),
            cell("Qty", header_style, Alignment::Right),
            cell("Cost Basis", header_style, Alignment::Right),
            cell("Price", header_style, Alignment::Right),
            cell("Market Value", header_style, Alignment::Right),
            cell("Gain/Loss", header_style, Alignment::Right),
            cell("G/L %", header_style, Alignment::Right),
        ],
    )?;

    let body_style = Style::new().with_font_size(8);
    for holding in &snapshot.holdings {
        let gain_style = body_style.with_color(pnl_color(holding.gain_loss));
        push_row(
            &mut table,
            [
                cell(&holding.symbol, body_style, Alignment::Left),
                cell(&truncate(&holding.name, 28), body_style, Alignment::Left),
                cell(
                    &holding.account.as_str().to_uppercase(),
                    body_style,
                    Alignment::Left,
                ),
                cell(
                    &format_quantity(holding.quantity),
                    body_style,
                    Alignment::Right,
                ),
                cell(
                    &format_currency(holding.cost_basis, &holding.currency),
                    body_style,
                    Alignment::Right,
                ),
                cell(
                    &format_currency(holding.current_price, &holding.currency),
                    body_style,
                    Alignment::Right,
                ),
                cell(
                    &format_currency(holding.market_value_cad, &snapshot.base_currency),
                    body_style,
                    Alignment::Right,
                ),
                cell(
                    &format_currency(holding.gain_loss, &snapshot.base_currency),
                    gain_style,
                    Alignment::Right,
                ),
                cell(
                    &format_signed_percent(holding.gain_loss_percent),
                    gain_style,
                    Alignment::Right,
                ),
            ],
        )?;
    }

    Ok(table)
}

fn build_allocation(snapshot: &PortfolioSnapshot) -> Result<TableLayout, String> {
    let mut table = TableLayout::new(vec![2, 3, 2]);
    table.set_cell_decorator(AlternatingRowDecorator);

    let header_style = Style::new().bold().with_font_size(9);
    push_row(
        &mut table,
        [
            cell("Asset Class", header_style, Alignment::Left),
            cell("Market Value", header_style, Alignment::Right),
            cell("Weight", header_style, Alignment::Right),
        ],
    )?;

    let body_style = Style::new().with_font_size(9);
    for (asset_type, value, weight) in compute_allocation(snapshot) {
        push_row(
            &mut table,
            [
                cell(asset_type_label(&asset_type), body_style, Alignment::Left),
                cell(
                    &format_currency(value, &snapshot.base_currency),
                    body_style,
                    Alignment::Right,
                ),
                cell(&format_percent(weight), body_style, Alignment::Right),
            ],
        )?;
    }

    Ok(table)
}

/// Sums market value (in base currency) and portfolio weight per asset class, in
/// display order. `AssetType` has no `PartialEq`/`Hash` derive, so this accumulates
/// into four named buckets instead of a `HashMap`.
fn compute_allocation(snapshot: &PortfolioSnapshot) -> [(AssetType, f64, f64); 4] {
    let (mut stock, mut etf, mut crypto, mut cash) =
        ((0.0, 0.0), (0.0, 0.0), (0.0, 0.0), (0.0, 0.0));
    for holding in &snapshot.holdings {
        let bucket = match holding.asset_type {
            AssetType::Stock => &mut stock,
            AssetType::Etf => &mut etf,
            AssetType::Crypto => &mut crypto,
            AssetType::Cash => &mut cash,
        };
        bucket.0 += holding.market_value_cad;
        bucket.1 += holding.weight;
    }
    [
        (AssetType::Stock, stock.0, stock.1),
        (AssetType::Etf, etf.0, etf.1),
        (AssetType::Crypto, crypto.0, crypto.1),
        (AssetType::Cash, cash.0, cash.1),
    ]
}

fn asset_type_label(asset_type: &AssetType) -> &'static str {
    match asset_type {
        AssetType::Stock => "Stock",
        AssetType::Etf => "ETF",
        AssetType::Crypto => "Crypto",
        AssetType::Cash => "Cash",
    }
}

fn pnl_color(value: f64) -> Color {
    if value > 0.0 {
        COLOR_GAIN
    } else if value < 0.0 {
        COLOR_LOSS
    } else {
        COLOR_MUTED
    }
}

/// Builds one left/right-aligned, explicitly-styled table cell. Wrapping in
/// `.styled(style)` (rather than only styling the string) matters: `Paragraph`
/// computes its line height from the *outer* style argument passed to `render`,
/// not from each string's own style, so an unwrapped cell would reserve the
/// ambient default line height instead of its own font size.
fn cell(text: &str, style: Style, alignment: Alignment) -> StyledElement<Paragraph> {
    Paragraph::new(text.to_string())
        .aligned(alignment)
        .styled(style)
}

/// Appends one row of `N` identically-typed cells to `table`. Shared by every
/// table in this document since `TableLayoutRow` requires the row's element
/// count to match the table's column count.
fn push_row<E: Element + 'static, const N: usize>(
    table: &mut TableLayout,
    cells: [E; N],
) -> Result<(), String> {
    let mut row = table.row();
    for c in cells {
        row.push_element(c);
    }
    row.push().map_err(|e| e.to_string())
}

fn format_amount(value: f64) -> String {
    let sign = if value < 0.0 { "-" } else { "" };
    let formatted = format!("{:.2}", value.abs());
    let (int_part, dec_part) = formatted
        .split_once('.')
        .unwrap_or((formatted.as_str(), "00"));
    let grouped: String = int_part
        .as_bytes()
        .rchunks(3)
        .rev()
        .map(|chunk| std::str::from_utf8(chunk).unwrap_or(""))
        .collect::<Vec<_>>()
        .join(",");
    format!("{sign}{grouped}.{dec_part}")
}

fn format_currency(value: f64, currency: &str) -> String {
    format!("{} {currency}", format_amount(value))
}

fn format_percent(value: f64) -> String {
    format!("{value:.2}%")
}

fn format_signed_percent(value: f64) -> String {
    format!("{value:+.2}%")
}

fn format_quantity(value: f64) -> String {
    if value.fract().abs() < 1e-9 {
        format!("{value:.0}")
    } else {
        let s = format!("{value:.8}");
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

fn truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars.saturating_sub(1)).collect();
        format!("{truncated}\u{2026}")
    }
}

/// Draws a bottom border under the header row and a light background under every
/// other data row. `genpdf`'s public `Area` API exposes only stroked lines
/// (`draw_line`), not a filled-rectangle primitive, so the "shading" is
/// approximated with closely-spaced overlapping horizontal strokes.
struct AlternatingRowDecorator;

impl CellDecorator for AlternatingRowDecorator {
    fn decorate_cell(
        &mut self,
        _column: usize,
        row: usize,
        _has_more: bool,
        area: Area<'_>,
        _style: Style,
    ) {
        if row == 0 {
            draw_bottom_border(&area);
        } else if row % 2 == 1 {
            shade_area(&area);
        }
    }
}

fn draw_bottom_border(area: &Area<'_>) {
    let size = area.size();
    area.draw_line(
        vec![
            Position::new(0.0, size.height),
            Position::new(size.width, size.height),
        ],
        Style::new().with_color(COLOR_BORDER),
    );
}

fn shade_area(area: &Area<'_>) {
    let size = area.size();
    let style = Style::new().with_color(COLOR_ROW_SHADE);
    let step = Mm::from(0.35_f32);
    let mut y = Mm::from(0.0_f32);
    while y < size.height {
        area.draw_line(
            vec![Position::new(0.0, y), Position::new(size.width, y)],
            style,
        );
        y += step;
    }
}

/// Applies page margins and reserves a bottom strip for a repeating footer
/// ("Generated by Portfolio Tracker" + export timestamp). `SimplePageDecorator`
/// only supports margins and a top header, so a custom `PageDecorator` is needed
/// for a footer.
struct FooterPageDecorator {
    margins: Margins,
    footer_height: Mm,
}

impl FooterPageDecorator {
    fn new() -> Self {
        FooterPageDecorator {
            margins: Margins::trbl(18.0, 15.0, 12.0, 15.0),
            footer_height: Mm::from(8.0_f32),
        }
    }
}

impl PageDecorator for FooterPageDecorator {
    fn decorate_page<'a>(
        &mut self,
        context: &Context,
        mut area: Area<'a>,
        _style: Style,
    ) -> Result<Area<'a>, GenPdfError> {
        area.add_margins(self.margins);
        let size = area.size();

        let mut footer_area = area.clone();
        footer_area.add_offset(Position::new(0.0, size.height - self.footer_height));
        footer_area.set_height(self.footer_height);

        let footer_style = Style::new().with_font_size(7).with_color(COLOR_MUTED);
        let footer_text = format!(
            "Generated by Portfolio Tracker \u{b7} {}",
            Utc::now().format("%Y-%m-%d %H:%M UTC")
        );
        cell(&footer_text, footer_style, Alignment::Center).render(
            context,
            footer_area,
            footer_style,
        )?;

        area.set_height(size.height - self.footer_height - Mm::from(2.0_f32));
        Ok(area)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::make_holding;
    use crate::types::{AccountType, HoldingWithPrice};

    fn make_holding_with_price(
        symbol: &str,
        asset_type: AssetType,
        quantity: f64,
        cost_basis: f64,
        currency: &str,
        market_value_cad: f64,
        weight: f64,
    ) -> HoldingWithPrice {
        let mut holding = make_holding(symbol, asset_type, quantity, cost_basis, currency);
        holding.account = AccountType::Taxable;
        HoldingWithPrice {
            holding,
            current_price: cost_basis,
            current_price_cad: cost_basis,
            market_value_cad,
            cost_value_cad: cost_basis * quantity,
            gain_loss: market_value_cad - cost_basis * quantity,
            gain_loss_percent: 0.0,
            weight,
            target_value: 0.0,
            target_delta_value: 0.0,
            target_delta_percent: 0.0,
            daily_change_percent: 0.0,
            fx_stale: false,
            price_is_stale: false,
        }
    }

    fn make_snapshot(holdings: Vec<HoldingWithPrice>) -> PortfolioSnapshot {
        let total_value = holdings.iter().map(|h| h.market_value_cad).sum();
        PortfolioSnapshot {
            holdings,
            total_value,
            total_cost: 0.0,
            total_gain_loss: 0.0,
            total_gain_loss_percent: 0.0,
            daily_pnl: 0.0,
            last_updated: "2024-01-01T00:00:00Z".to_string(),
            base_currency: "CAD".to_string(),
            total_target_weight: 0.0,
            target_cash_delta: 0.0,
            realized_gains: 0.0,
            annual_dividend_income: 0.0,
            requires_cost_basis_selection: false,
        }
    }

    #[test]
    fn build_portfolio_pdf_produces_valid_pdf_for_populated_snapshot() {
        let holdings = vec![
            make_holding_with_price("AAPL", AssetType::Stock, 10.0, 150.0, "USD", 2500.0, 60.0),
            make_holding_with_price("XIU.TO", AssetType::Etf, 20.0, 30.0, "CAD", 700.0, 20.0),
            make_holding_with_price(
                "BTC-USD",
                AssetType::Crypto,
                0.1,
                40000.0,
                "USD",
                5500.0,
                20.0,
            ),
        ];
        let snapshot = make_snapshot(holdings);

        let pdf = build_portfolio_pdf(&snapshot).expect("PDF should build successfully");

        assert!(
            pdf.starts_with(b"%PDF-"),
            "output must start with the PDF magic header"
        );
        assert!(
            pdf.len() > 1000,
            "a populated snapshot should produce a non-trivial PDF"
        );
    }

    #[test]
    fn build_portfolio_pdf_produces_valid_pdf_for_empty_holdings() {
        let snapshot = make_snapshot(vec![]);

        let pdf = build_portfolio_pdf(&snapshot)
            .expect("PDF should build successfully even with no holdings");

        assert!(
            pdf.starts_with(b"%PDF-"),
            "output must start with the PDF magic header"
        );
    }

    #[test]
    fn compute_allocation_sums_market_value_and_weight_per_asset_class() {
        let holdings = vec![
            make_holding_with_price("AAPL", AssetType::Stock, 10.0, 150.0, "USD", 1000.0, 40.0),
            make_holding_with_price("MSFT", AssetType::Stock, 5.0, 200.0, "USD", 500.0, 20.0),
            make_holding_with_price("XIU.TO", AssetType::Etf, 20.0, 30.0, "CAD", 700.0, 28.0),
        ];
        let snapshot = make_snapshot(holdings);

        let allocation = compute_allocation(&snapshot);

        let stock = allocation
            .iter()
            .find(|(t, _, _)| matches!(t, AssetType::Stock))
            .unwrap();
        assert!((stock.1 - 1500.0).abs() < 0.001);
        assert!((stock.2 - 60.0).abs() < 0.001);

        let etf = allocation
            .iter()
            .find(|(t, _, _)| matches!(t, AssetType::Etf))
            .unwrap();
        assert!((etf.1 - 700.0).abs() < 0.001);
        assert!((etf.2 - 28.0).abs() < 0.001);

        let crypto = allocation
            .iter()
            .find(|(t, _, _)| matches!(t, AssetType::Crypto))
            .unwrap();
        assert_eq!(crypto.1, 0.0);
    }

    #[test]
    fn format_amount_adds_thousands_separators_and_handles_negatives() {
        assert_eq!(format_amount(1234567.891), "1,234,567.89");
        assert_eq!(format_amount(-1234.5), "-1,234.50");
        assert_eq!(format_amount(0.0), "0.00");
    }

    #[test]
    fn format_quantity_trims_trailing_zeros_for_fractional_values() {
        assert_eq!(format_quantity(10.0), "10");
        assert_eq!(format_quantity(0.5), "0.5");
        assert_eq!(format_quantity(0.12345678), "0.12345678");
    }

    #[test]
    fn truncate_appends_ellipsis_only_when_over_the_limit() {
        assert_eq!(truncate("Apple Inc.", 28), "Apple Inc.");
        assert_eq!(
            truncate("A Company With A Very Long Legal Name Indeed", 20),
            "A Company With A Ve\u{2026}"
        );
    }
}
