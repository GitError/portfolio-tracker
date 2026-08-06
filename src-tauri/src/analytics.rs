use std::collections::{HashMap, VecDeque};

use portfolio_core::fx::convert_to_base;

use crate::types::{
    FxRate, HoldingId, RealizedGainsSummary, RealizedLot, Transaction, TransactionType,
};

/// Compute realized gains for a slice of transactions using the specified method.
///
/// `method` must be `"avco"` (average cost) or `"fifo"` (first-in, first-out).
/// Transactions are expected to be pre-sorted by `transacted_at` (oldest first).
pub fn compute_realized_gains(
    transactions: &[Transaction],
    method: &str,
) -> Result<RealizedGainsSummary, String> {
    match method {
        "fifo" => compute_fifo(transactions),
        "avco" => compute_avco(transactions),
        other => Err(format!(
            "Unrecognized cost-basis method {:?}; expected \"avco\" or \"fifo\"",
            other
        )),
    }
}

// ── AVCO ──────────────────────────────────────────────────────────────────────

fn compute_avco(transactions: &[Transaction]) -> Result<RealizedGainsSummary, String> {
    let mut avg_cost = 0.0f64;
    let mut total_qty = 0.0f64;
    let mut lots: Vec<RealizedLot> = Vec::new();
    let mut total_proceeds = 0.0f64;
    let mut total_cost_basis = 0.0f64;

    for tx in transactions {
        match tx.transaction_type {
            TransactionType::Buy => {
                let new_total_cost = avg_cost * total_qty + tx.price * tx.quantity;
                total_qty += tx.quantity;
                avg_cost = if total_qty > 0.0 {
                    new_total_cost / total_qty
                } else {
                    0.0
                };
            }
            TransactionType::Sell => {
                // Epsilon for float comparison; portfolio values are in dollars with 2 decimal places, so 1e-9 is safely sub-cent.
                if tx.quantity > total_qty + 1e-9 {
                    return Err(format!(
                        "Sell quantity {:.4} exceeds available inventory {:.4} at {}",
                        tx.quantity, total_qty, tx.transacted_at
                    ));
                }

                let sold_qty = tx.quantity;
                let proceeds = sold_qty * tx.price;
                let cost_basis = sold_qty * avg_cost;
                let gain_loss = proceeds - cost_basis;

                total_proceeds += proceeds;
                total_cost_basis += cost_basis;
                total_qty -= sold_qty;

                // Epsilon for float comparison; portfolio values are in dollars with 2 decimal places, so 1e-9 is safely sub-cent.
                if total_qty < 1e-9 {
                    total_qty = 0.0;
                    avg_cost = 0.0;
                }

                lots.push(RealizedLot {
                    sold_at: date_part(&tx.transacted_at)?,
                    quantity: sold_qty,
                    proceeds,
                    cost_basis,
                    gain_loss,
                });
            }
        }
    }

    let total_realized_gain = total_proceeds - total_cost_basis;

    Ok(RealizedGainsSummary {
        total_realized_gain,
        total_proceeds,
        total_cost_basis,
        lots,
    })
}

// ── FIFO ──────────────────────────────────────────────────────────────────────

fn compute_fifo(transactions: &[Transaction]) -> Result<RealizedGainsSummary, String> {
    // Queue entries: (quantity_remaining, buy_price)
    let mut buy_queue: VecDeque<(f64, f64)> = VecDeque::new();
    let mut lots: Vec<RealizedLot> = Vec::new();
    let mut total_proceeds = 0.0f64;
    let mut total_cost_basis = 0.0f64;

    for tx in transactions {
        match tx.transaction_type {
            TransactionType::Buy => {
                buy_queue.push_back((tx.quantity, tx.price));
            }
            TransactionType::Sell => {
                let available: f64 = buy_queue.iter().map(|(q, _)| q).sum();
                // Epsilon for float comparison; portfolio values are in dollars with 2 decimal places, so 1e-9 is safely sub-cent.
                if tx.quantity > available + 1e-9 {
                    return Err(format!(
                        "Sell quantity {:.4} exceeds available inventory {:.4} at {}",
                        tx.quantity, available, tx.transacted_at
                    ));
                }

                let mut remaining_sell = tx.quantity;
                let mut lot_cost_basis = 0.0f64;

                // Epsilon for float comparison; portfolio values are in dollars with 2 decimal places, so 1e-9 is safely sub-cent.
                while remaining_sell > 1e-9 {
                    let Some((ref mut lot_qty, lot_price)) = buy_queue.front_mut() else {
                        return Err(
                            "Sell quantity exceeds total buy quantity — check transaction history"
                                .into(),
                        );
                    };

                    let consumed = remaining_sell.min(*lot_qty);
                    lot_cost_basis += consumed * *lot_price;
                    *lot_qty -= consumed;
                    remaining_sell -= consumed;

                    // Epsilon for float comparison; portfolio values are in dollars with 2 decimal places, so 1e-9 is safely sub-cent.
                    if *lot_qty < 1e-9 {
                        buy_queue.pop_front();
                    }
                }

                let proceeds = tx.quantity * tx.price;
                let gain_loss = proceeds - lot_cost_basis;

                total_proceeds += proceeds;
                total_cost_basis += lot_cost_basis;

                lots.push(RealizedLot {
                    sold_at: date_part(&tx.transacted_at)?,
                    quantity: tx.quantity,
                    proceeds,
                    cost_basis: lot_cost_basis,
                    gain_loss,
                });
            }
        }
    }

    let total_realized_gain = total_proceeds - total_cost_basis;

    Ok(RealizedGainsSummary {
        total_realized_gain,
        total_proceeds,
        total_cost_basis,
        lots,
    })
}

/// Aggregate per-holding summaries into one combined summary.
pub fn aggregate_summaries(summaries: Vec<RealizedGainsSummary>) -> RealizedGainsSummary {
    let mut total_realized_gain = 0.0f64;
    let mut total_proceeds = 0.0f64;
    let mut total_cost_basis = 0.0f64;
    let mut lots: Vec<RealizedLot> = Vec::new();

    for s in summaries {
        total_realized_gain += s.total_realized_gain;
        total_proceeds += s.total_proceeds;
        total_cost_basis += s.total_cost_basis;
        lots.extend(s.lots);
    }

    // Sort lots by sold_at date, oldest first
    lots.sort_by(|a, b| a.sold_at.cmp(&b.sold_at));

    RealizedGainsSummary {
        total_realized_gain,
        total_proceeds,
        total_cost_basis,
        lots,
    }
}

/// Group transactions by holding_id, compute realized gains per group,
/// convert each group's totals into `base_currency`, then aggregate.
/// Uses the given method ("avco" | "fifo").
///
/// `holding_currencies` maps each holding to the currency its transaction
/// prices are denominated in (see `Transaction::price` doc comment — prices
/// are always in "the holding's original currency"). Without this conversion
/// step, gains from holdings in different currencies would be summed as if
/// they were the same currency (#754).
///
/// FX conversion uses the current/cached rate, not the historical rate at
/// time of sale — acceptable for a live dashboard snapshot figure, but not a
/// substitute for a proper tax/accounting record. If a holding's currency has
/// no cached rate (or maps to no known holding, e.g. a deleted holding), the
/// conversion falls back to a 1:1 rate rather than failing the whole snapshot.
pub fn compute_realized_gains_grouped(
    transactions: &[Transaction],
    method: &str,
    holding_currencies: &HashMap<HoldingId, String>,
    base_currency: &str,
    fx_rates: &[FxRate],
) -> Result<RealizedGainsSummary, String> {
    // Group by holding_id (transactions are already sorted by transacted_at)
    let mut by_holding: HashMap<&HoldingId, Vec<&Transaction>> = HashMap::new();
    for tx in transactions {
        by_holding.entry(&tx.holding_id).or_default().push(tx);
    }

    let mut summaries = Vec::new();
    for (holding_id, txs) in by_holding {
        // Each group is already sorted because get_all_transactions returns ASC order
        let owned: Vec<Transaction> = txs.iter().map(|t| (*t).clone()).collect();
        let summary = compute_realized_gains(&owned, method)?;

        let currency = holding_currencies
            .get(holding_id)
            .map(String::as_str)
            .unwrap_or(base_currency);
        let fx_rate =
            convert_to_base(1.0, currency, base_currency, fx_rates).unwrap_or_else(|| {
                tracing::warn!(
                    holding_id = %holding_id.0,
                    currency = %currency,
                    base = %base_currency,
                    "FX rate unavailable for realized-gains conversion; using 1:1 rate"
                );
                1.0
            });

        summaries.push(convert_summary_to_base(summary, fx_rate));
    }

    Ok(aggregate_summaries(summaries))
}

/// Scale every monetary field of a `RealizedGainsSummary` (totals and
/// per-lot amounts) by `fx_rate`. `fx_rate` is 1.0 when already in base
/// currency, so this is a no-op for the common case.
fn convert_summary_to_base(summary: RealizedGainsSummary, fx_rate: f64) -> RealizedGainsSummary {
    RealizedGainsSummary {
        total_realized_gain: summary.total_realized_gain * fx_rate,
        total_proceeds: summary.total_proceeds * fx_rate,
        total_cost_basis: summary.total_cost_basis * fx_rate,
        lots: summary
            .lots
            .into_iter()
            .map(|lot| RealizedLot {
                proceeds: lot.proceeds * fx_rate,
                cost_basis: lot.cost_basis * fx_rate,
                gain_loss: lot.gain_loss * fx_rate,
                ..lot
            })
            .collect(),
    }
}

/// Extract the YYYY-MM-DD portion from an ISO 8601 timestamp.
///
/// Uses `str::get` rather than byte-slicing: a fixed `&ts[..10]` panics if
/// byte offset 10 doesn't land on a char boundary (possible with a malformed,
/// non-ASCII `transacted_at` value). `get` returns `None` in that case instead
/// of panicking, which we surface as an error.
fn date_part(ts: &str) -> Result<String, String> {
    ts.get(..10)
        .map(str::to_string)
        .ok_or_else(|| format!("Malformed transacted_at timestamp: {:?}", ts))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{HoldingId, TransactionId, TransactionType};

    fn buy(quantity: f64, price: f64, date: &str) -> Transaction {
        Transaction {
            id: TransactionId(uuid::Uuid::new_v4().to_string()),
            holding_id: HoldingId("h1".to_string()),
            transaction_type: TransactionType::Buy,
            quantity,
            price,
            transacted_at: format!("{}T00:00:00Z", date),
            created_at: "2024-01-01T00:00:00Z".to_string(),
        }
    }

    fn sell(quantity: f64, price: f64, date: &str) -> Transaction {
        Transaction {
            id: TransactionId(uuid::Uuid::new_v4().to_string()),
            holding_id: HoldingId("h1".to_string()),
            transaction_type: TransactionType::Sell,
            quantity,
            price,
            transacted_at: format!("{}T00:00:00Z", date),
            created_at: "2024-01-01T00:00:00Z".to_string(),
        }
    }

    fn buy_for(holding_id: &str, quantity: f64, price: f64, date: &str) -> Transaction {
        Transaction {
            id: TransactionId(uuid::Uuid::new_v4().to_string()),
            holding_id: HoldingId(holding_id.to_string()),
            transaction_type: TransactionType::Buy,
            quantity,
            price,
            transacted_at: format!("{}T00:00:00Z", date),
            created_at: "2024-01-01T00:00:00Z".to_string(),
        }
    }

    fn sell_for(holding_id: &str, quantity: f64, price: f64, date: &str) -> Transaction {
        Transaction {
            id: TransactionId(uuid::Uuid::new_v4().to_string()),
            holding_id: HoldingId(holding_id.to_string()),
            transaction_type: TransactionType::Sell,
            quantity,
            price,
            transacted_at: format!("{}T00:00:00Z", date),
            created_at: "2024-01-01T00:00:00Z".to_string(),
        }
    }

    fn make_holding_currencies(pairs: &[(&str, &str)]) -> HashMap<HoldingId, String> {
        pairs
            .iter()
            .map(|(id, currency)| (HoldingId(id.to_string()), currency.to_string()))
            .collect()
    }

    fn make_fx_rate(pair: &str, rate: f64) -> FxRate {
        FxRate {
            pair: pair.to_string(),
            rate,
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        }
    }

    // ── AVCO tests ────────────────────────────────────────────────────────────

    #[test]
    fn avco_no_transactions_returns_zero_summary() {
        let summary = compute_realized_gains(&[], "avco").unwrap();
        assert_eq!(summary.lots.len(), 0);
        assert!((summary.total_realized_gain).abs() < 1e-9);
    }

    #[test]
    fn avco_only_buys_no_realized_gain() {
        let txs = vec![
            buy(10.0, 100.0, "2024-01-01"),
            buy(5.0, 120.0, "2024-02-01"),
        ];
        let summary = compute_realized_gains(&txs, "avco").unwrap();
        assert_eq!(summary.lots.len(), 0);
        assert!((summary.total_realized_gain).abs() < 1e-9);
    }

    #[test]
    fn avco_simple_gain() {
        // Buy 10 @ 100, sell 5 @ 150 → avg_cost=100, proceeds=750, cost=500, gain=250
        let txs = vec![
            buy(10.0, 100.0, "2024-01-01"),
            sell(5.0, 150.0, "2024-02-01"),
        ];
        let summary = compute_realized_gains(&txs, "avco").unwrap();
        assert_eq!(summary.lots.len(), 1);
        assert!((summary.total_proceeds - 750.0).abs() < 1e-9);
        assert!((summary.total_cost_basis - 500.0).abs() < 1e-9);
        assert!((summary.total_realized_gain - 250.0).abs() < 1e-9);
    }

    #[test]
    fn avco_simple_loss() {
        // Buy 10 @ 100, sell 5 @ 60 → avg_cost=100, proceeds=300, cost=500, gain=-200
        let txs = vec![
            buy(10.0, 100.0, "2024-01-01"),
            sell(5.0, 60.0, "2024-02-01"),
        ];
        let summary = compute_realized_gains(&txs, "avco").unwrap();
        assert!((summary.total_realized_gain - (-200.0)).abs() < 1e-9);
    }

    #[test]
    fn avco_running_average_updated_after_second_buy() {
        // Buy 10 @ 100 → avg=100
        // Buy 10 @ 200 → avg=150
        // Sell 5 @ 180 → cost=5*150=750, proceeds=900, gain=150
        let txs = vec![
            buy(10.0, 100.0, "2024-01-01"),
            buy(10.0, 200.0, "2024-02-01"),
            sell(5.0, 180.0, "2024-03-01"),
        ];
        let summary = compute_realized_gains(&txs, "avco").unwrap();
        assert!((summary.total_cost_basis - 750.0).abs() < 1e-9);
        assert!((summary.total_proceeds - 900.0).abs() < 1e-9);
        assert!((summary.total_realized_gain - 150.0).abs() < 1e-9);
    }

    #[test]
    fn avco_sell_exceeds_inventory_returns_error() {
        let txs = vec![
            buy(5.0, 100.0, "2024-01-01"),
            sell(10.0, 150.0, "2024-02-01"),
        ];
        assert!(compute_realized_gains(&txs, "avco").is_err());
    }

    #[test]
    fn avco_multiple_sells_accumulate_correctly() {
        // Buy 20 @ 50 → avg=50
        // Sell 5 @ 80 → gain = 5*(80-50) = 150
        // Sell 5 @ 40 → gain = 5*(40-50) = -50
        // total = 100
        let txs = vec![
            buy(20.0, 50.0, "2024-01-01"),
            sell(5.0, 80.0, "2024-02-01"),
            sell(5.0, 40.0, "2024-03-01"),
        ];
        let summary = compute_realized_gains(&txs, "avco").unwrap();
        assert_eq!(summary.lots.len(), 2);
        assert!((summary.total_realized_gain - 100.0).abs() < 1e-9);
    }

    // ── FIFO tests ────────────────────────────────────────────────────────────

    #[test]
    fn fifo_no_transactions_returns_zero_summary() {
        let summary = compute_realized_gains(&[], "fifo").unwrap();
        assert_eq!(summary.lots.len(), 0);
        assert!((summary.total_realized_gain).abs() < 1e-9);
    }

    #[test]
    fn fifo_simple_gain() {
        // Buy 10 @ 100, sell 5 @ 150 → first 5 units cost 100 each, proceeds=750, cost=500, gain=250
        let txs = vec![
            buy(10.0, 100.0, "2024-01-01"),
            sell(5.0, 150.0, "2024-02-01"),
        ];
        let summary = compute_realized_gains(&txs, "fifo").unwrap();
        assert!((summary.total_realized_gain - 250.0).abs() < 1e-9);
    }

    #[test]
    fn fifo_uses_oldest_lots_first() {
        // Buy 5 @ 100 (lot A), Buy 5 @ 200 (lot B)
        // Sell 5 @ 300 → should consume lot A (cost=500), proceeds=1500, gain=1000
        let txs = vec![
            buy(5.0, 100.0, "2024-01-01"),
            buy(5.0, 200.0, "2024-02-01"),
            sell(5.0, 300.0, "2024-03-01"),
        ];
        let summary = compute_realized_gains(&txs, "fifo").unwrap();
        assert!((summary.total_cost_basis - 500.0).abs() < 1e-9);
        assert!((summary.total_proceeds - 1500.0).abs() < 1e-9);
        assert!((summary.total_realized_gain - 1000.0).abs() < 1e-9);
    }

    #[test]
    fn fifo_spans_multiple_buy_lots() {
        // Buy 3 @ 100, Buy 3 @ 200 → Sell 5 @ 250
        // Consumes: 3 @ 100 (cost=300) + 2 @ 200 (cost=400) = 700 total cost
        // Proceeds = 5 * 250 = 1250, gain = 550
        let txs = vec![
            buy(3.0, 100.0, "2024-01-01"),
            buy(3.0, 200.0, "2024-02-01"),
            sell(5.0, 250.0, "2024-03-01"),
        ];
        let summary = compute_realized_gains(&txs, "fifo").unwrap();
        assert!((summary.total_cost_basis - 700.0).abs() < 1e-9);
        assert!((summary.total_realized_gain - 550.0).abs() < 1e-9);
    }

    #[test]
    fn fifo_sell_exceeds_inventory_returns_error() {
        let txs = vec![
            buy(3.0, 100.0, "2024-01-01"),
            sell(5.0, 150.0, "2024-02-01"),
        ];
        assert!(compute_realized_gains(&txs, "fifo").is_err());
    }

    // ── Aggregation tests ─────────────────────────────────────────────────────

    #[test]
    fn aggregate_empty_summaries_returns_zero() {
        let result = aggregate_summaries(vec![]);
        assert!((result.total_realized_gain).abs() < 1e-9);
        assert_eq!(result.lots.len(), 0);
    }

    #[test]
    fn aggregate_two_holdings_sums_gains() {
        let s1 = RealizedGainsSummary {
            total_realized_gain: 100.0,
            total_proceeds: 600.0,
            total_cost_basis: 500.0,
            lots: vec![RealizedLot {
                sold_at: "2024-02-01".to_string(),
                quantity: 5.0,
                proceeds: 600.0,
                cost_basis: 500.0,
                gain_loss: 100.0,
            }],
        };
        let s2 = RealizedGainsSummary {
            total_realized_gain: -50.0,
            total_proceeds: 200.0,
            total_cost_basis: 250.0,
            lots: vec![RealizedLot {
                sold_at: "2024-03-01".to_string(),
                quantity: 2.0,
                proceeds: 200.0,
                cost_basis: 250.0,
                gain_loss: -50.0,
            }],
        };
        let result = aggregate_summaries(vec![s1, s2]);
        assert!((result.total_realized_gain - 50.0).abs() < 1e-9);
        assert!((result.total_proceeds - 800.0).abs() < 1e-9);
        assert!((result.total_cost_basis - 750.0).abs() < 1e-9);
        assert_eq!(result.lots.len(), 2);
        // Lots should be sorted by date
        assert!(result.lots[0].sold_at <= result.lots[1].sold_at);
    }

    // ── Grouped multi-currency tests (#754) ──────────────────────────────────

    #[test]
    fn grouped_converts_each_holding_to_base_currency_before_aggregating() {
        // h1 is CAD: buy 10 @ 100, sell 5 @ 150 → gain 250 CAD
        // h2 is USD: buy 10 @ 100, sell 5 @ 150 → gain 250 USD → 337.5 CAD @ 1.35
        // A naive raw-currency sum would give 500; the correct base-currency
        // total is 250 + 337.5 = 587.5.
        let txs = vec![
            buy_for("h1", 10.0, 100.0, "2024-01-01"),
            sell_for("h1", 5.0, 150.0, "2024-02-01"),
            buy_for("h2", 10.0, 100.0, "2024-01-01"),
            sell_for("h2", 5.0, 150.0, "2024-02-01"),
        ];
        let currencies = make_holding_currencies(&[("h1", "CAD"), ("h2", "USD")]);
        let fx_rates = vec![make_fx_rate("USDCAD", 1.35)];

        let summary =
            compute_realized_gains_grouped(&txs, "avco", &currencies, "CAD", &fx_rates).unwrap();

        assert!(
            (summary.total_realized_gain - 587.5).abs() < 1e-6,
            "expected 587.5 (250 CAD + 250 USD * 1.35), got {}",
            summary.total_realized_gain
        );
    }

    #[test]
    fn grouped_same_currency_holdings_unaffected() {
        let txs = vec![
            buy_for("h1", 10.0, 100.0, "2024-01-01"),
            sell_for("h1", 5.0, 150.0, "2024-02-01"),
            buy_for("h2", 20.0, 50.0, "2024-01-01"),
            sell_for("h2", 5.0, 80.0, "2024-02-01"),
        ];
        let currencies = make_holding_currencies(&[("h1", "CAD"), ("h2", "CAD")]);

        let summary =
            compute_realized_gains_grouped(&txs, "avco", &currencies, "CAD", &[]).unwrap();

        // h1 gain: 250, h2 gain: 5*(80-50)=150 → total 400
        assert!((summary.total_realized_gain - 400.0).abs() < 1e-6);
    }

    #[test]
    fn grouped_missing_fx_rate_falls_back_to_unconverted_rate() {
        // h2 is EUR but no EUR rate is cached; conversion should fall back to
        // 1:1 rather than erroring — acceptable for a dashboard snapshot figure.
        let txs = vec![
            buy_for("h2", 10.0, 100.0, "2024-01-01"),
            sell_for("h2", 5.0, 150.0, "2024-02-01"),
        ];
        let currencies = make_holding_currencies(&[("h2", "EUR")]);

        let summary =
            compute_realized_gains_grouped(&txs, "avco", &currencies, "CAD", &[]).unwrap();

        assert!((summary.total_realized_gain - 250.0).abs() < 1e-6);
    }

    #[test]
    fn grouped_converts_lot_level_fields_too() {
        let txs = vec![
            buy_for("h2", 10.0, 100.0, "2024-01-01"),
            sell_for("h2", 5.0, 150.0, "2024-02-01"),
        ];
        let currencies = make_holding_currencies(&[("h2", "USD")]);
        let fx_rates = vec![make_fx_rate("USDCAD", 1.35)];

        let summary =
            compute_realized_gains_grouped(&txs, "avco", &currencies, "CAD", &fx_rates).unwrap();

        assert_eq!(summary.lots.len(), 1);
        assert!((summary.lots[0].proceeds - 750.0 * 1.35).abs() < 1e-6);
        assert!((summary.lots[0].cost_basis - 500.0 * 1.35).abs() < 1e-6);
        assert!((summary.lots[0].gain_loss - 250.0 * 1.35).abs() < 1e-6);
    }

    #[test]
    fn grouped_unknown_holding_id_falls_back_to_base_currency() {
        // Transactions referencing a holding absent from the currency map
        // (e.g. the holding was since deleted) should not error — fall back
        // to treating the amount as already in base currency.
        let txs = vec![
            buy_for("ghost", 10.0, 100.0, "2024-01-01"),
            sell_for("ghost", 5.0, 150.0, "2024-02-01"),
        ];
        let currencies = HashMap::new();

        let summary =
            compute_realized_gains_grouped(&txs, "avco", &currencies, "CAD", &[]).unwrap();

        assert!((summary.total_realized_gain - 250.0).abs() < 1e-6);
    }

    #[test]
    fn date_part_extracts_first_ten_chars() {
        assert_eq!(date_part("2024-03-15T12:00:00Z").unwrap(), "2024-03-15");
        assert_eq!(date_part("2024-03-15").unwrap(), "2024-03-15");
    }

    #[test]
    fn date_part_rejects_string_shorter_than_ten_bytes() {
        assert!(date_part("2024").is_err());
    }

    #[test]
    fn date_part_rejects_non_char_boundary_instead_of_panicking() {
        // 9 ASCII bytes followed by 'é' (2 UTF-8 bytes: 0xC3 0xA9) means byte
        // offset 10 lands in the middle of 'é' — `&s[..10]` would panic here.
        let malformed = "2024-03-1é";
        assert_eq!(malformed.len(), 11, "sanity check: 11 bytes, 10 chars");
        assert!(date_part(malformed).is_err());
    }
}
