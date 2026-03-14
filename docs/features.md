# Feature Guide

A walkthrough of every view in Portfolio Tracker, what it shows, and how to use it.

---

## Dashboard

The Dashboard is the home screen. It gives you a live snapshot of your entire portfolio at a glance.

### Total Value & Daily P&L

The top bar shows:
- **Total Portfolio Value** — the sum of all holdings converted to CAD at current FX rates.
- **Daily P&L** — how much the portfolio has moved today in absolute CAD and as a percentage. Calculated as `Σ (current price − previous close) × quantity × FX rate` across all holdings.
- **Last Refreshed** — the timestamp of the most recent price fetch. Click **Refresh** (top-right) or press the refresh button to pull fresh quotes from Yahoo Finance.

### Allocation Donut

The donut chart breaks your portfolio into four asset classes: **Stock**, **ETF**, **Crypto**, and **Cash**. Each slice is sized by market value as a share of the total. Hover a slice to see the exact CAD value and weight.

### Currency Exposure

Shows what percentage of your portfolio is denominated in each currency (USD, CAD, EUR, GBP, etc.) before FX conversion. Useful for understanding FX risk.

### Top Movers

A ranked list of holdings sorted by absolute daily change (largest gainers and losers first). Each row shows the symbol, daily change in percent, and the absolute CAD impact on the portfolio.

---

## Holdings

The Holdings view is where you manage your positions.

### Adding a Holding

Click **Add Holding** (top-right of the Holdings view). Fill in:

| Field | Description |
|-------|-------------|
| Symbol | Ticker symbol — see formats below |
| Name | Display name (e.g. "Apple Inc.") |
| Asset Type | Stock, ETF, Crypto, or Cash |
| Quantity | Number of units (shares, coins, or 1 for cash) |
| Cost Basis | Price per unit in the holding's native currency |
| Currency | ISO code of the holding's currency (e.g. `USD`, `CAD`) |

#### Symbol Formats

| Asset Type | Format | Examples |
|------------|--------|---------|
| Stocks | Exchange ticker | `AAPL`, `SHOP.TO`, `ENB.TO` |
| ETFs | Exchange ticker | `VFV.TO`, `QQQ`, `XEF.TO` |
| Crypto | `{COIN}-{CURRENCY}` | `BTC-CAD`, `ETH-USD`, `SOL-USD` |
| Cash | Any label | Leave symbol blank or use currency code |

> **Toronto Stock Exchange:** append `.TO` to Canadian tickers (e.g. `TD.TO`, `RY.TO`).

### Editing and Deleting

Click any row in the holdings table to open the edit modal. Update any field and save. To delete, click the trash icon in the row or inside the edit modal.

### Sorting

Click any column header to sort by that column. Click again to reverse the order. Default sort is by market value descending.

### Gain / Loss Calculation

```
Cost Value (CAD)   = quantity × cost_basis × fx_rate_at_cost_currency
Market Value (CAD) = quantity × current_price × current_fx_rate
Gain / Loss        = Market Value − Cost Value
Gain / Loss %      = Gain / Loss ÷ Cost Value × 100
```

FX conversion uses live rates fetched from Yahoo Finance at the time of the last refresh.

---

## Performance

The Performance view shows how your portfolio has grown over time.

> **v1 note:** Historical data in v1 is generated from simulated snapshots seeded from your current holdings and cost basis. Real daily snapshot tracking is planned for v2 (see [Roadmap](roadmap.md)).

### Time Range

Use the range buttons at the top to select the window:

| Range | Description |
|-------|-------------|
| 1W | Last 7 days |
| 1M | Last 30 days |
| 3M | Last 90 days |
| 6M | Last 6 months |
| 1Y | Last 12 months |
| ALL | Full history |

### Area Chart

The main chart plots portfolio value over the selected range. The shaded area represents cumulative value. Hover anywhere on the chart to see the exact value on that date.

### Daily Returns Bar Chart

Below the area chart, a bar chart shows the day-over-day change for each session. Green bars are gains, red bars are losses.

### Stats

| Stat | Definition |
|------|-----------|
| Total Return | `(current value − initial cost) ÷ initial cost` |
| Max Drawdown | Largest peak-to-trough decline over the period |
| Volatility | Annualized standard deviation of daily returns |
| Best Day | Highest single-day return in the period |
| Worst Day | Lowest single-day return in the period |

---

## Stress Test

The Stress Test view simulates how your portfolio would perform under adverse market conditions.

### What Is a Stress Scenario?

A scenario is a set of **percentage shocks** applied to each asset class and/or FX rate. For example, a "Bear Market" scenario might apply −20% to stocks and −40% to crypto simultaneously, letting you see the combined dollar impact on your portfolio.

### Preset Scenarios

| Scenario | Shocks Applied |
|----------|---------------|
| Mild Correction | Stocks −5%, ETFs −5%, Crypto −10% |
| Bear Market | Stocks −20%, ETFs −20%, Crypto −40%, USD/CAD −5% |
| Crypto Winter | Crypto −50% |
| CAD Crash | USD/CAD +15%, EUR/CAD +10%, GBP/CAD +10% |
| Stagflation | Stocks −15%, ETFs −12%, Crypto −20%, USD/CAD +8% |

### Custom Scenario

Use the sliders to set your own shocks per asset class and FX pair. Values range from −80% to +80%. The results update live as you drag.

### Reading the Results

- **Total Impact** — the dollar (CAD) change and percentage change applied to your current portfolio value.
- **Waterfall Chart** — shows the contribution of each asset class to the total impact. Bars extending left are losses; right are gains.
- **Breakdown Table** — per-holding detail: current value, stressed value, shock applied, and dollar impact.

### FX Shock Direction

A **positive** USD/CAD shock means the CAD weakens relative to USD (e.g. +15% means 1 USD now buys 15% more CAD). For USD-denominated holdings this is a **gain** in CAD terms, not a loss. The breakdown table reflects this correctly.

---

## Multi-Currency

Portfolio Tracker supports holdings in any currency that Yahoo Finance covers. All values are normalized to **CAD** for display.

### How FX Conversion Works

1. On each price refresh, FX rates are fetched from Yahoo Finance for all currency pairs present in your holdings (e.g. `USDCAD=X`, `EURCAD=X`, `GBPCAD=X`).
2. Each holding's market price is multiplied by the corresponding rate to get the CAD value.
3. CAD-denominated holdings have a rate of 1.0 and are passed through unchanged.

### Supported Currencies

Any currency pair available on Yahoo Finance is supported. Commonly used pairs:

- `USD` → `USDCAD=X`
- `EUR` → `EURCAD=X`
- `GBP` → `GBPCAD=X`
- `CHF` → `CHFCAD=X`
- `AUD` → `AUDCAD=X`
- `JPY` → `JPYCAD=X`

If a rate cannot be fetched, the holding's original amount is used as-is (no conversion applied) and a warning is logged.
