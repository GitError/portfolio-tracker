# Feature Guide

A walkthrough of every view in Portfolio Tracker, what it shows, and how to use it.

---

## Dashboard

The Dashboard is the home screen. It gives you a live snapshot of your entire portfolio at a glance.

### Total Value & Daily P&L

The top bar shows:
- **Total Portfolio Value** — the sum of all holdings converted to your base currency at current FX rates.
- **Daily P&L** — how much the portfolio has moved today in absolute and percentage terms. Calculated as `Σ (current price − previous close) × quantity × FX rate` across all holdings. Holdings bought today are excluded from the daily P&L calculation (no prior-day close to compare against).
- **Last Refreshed** — the timestamp of the most recent price fetch. Click **Refresh** (top-right) or press `⌘R` to pull fresh quotes from Yahoo Finance.
- **Auto-refresh countdown** — when auto-refresh is enabled in Settings, a countdown timer appears next to the refresh button.

### Allocation Donut

The donut chart breaks your portfolio into four asset classes: **Stock**, **ETF**, **Crypto**, and **Cash**. Each slice is sized by market value as a share of the total. Hover a slice to see the exact value and weight.

### Currency Exposure

Shows what percentage of your portfolio is denominated in each currency before FX conversion. Useful for understanding FX risk.

### Top Movers

A ranked list of holdings sorted by absolute daily change (largest gainers and losers first). Each row shows the symbol, daily change in percent, and the absolute impact on the portfolio.

---

## Holdings

The Holdings view is where you manage your positions.

### Adding a Holding

Click **Add Holding** (top-right) or press `⌘N`. Fill in:

| Field | Description |
|-------|-------------|
| Symbol | Ticker — type to search and autocomplete via Yahoo Finance |
| Name | Display name (e.g. "Apple Inc.") — auto-filled from symbol search |
| Asset Type | Stock, ETF, Crypto, or Cash |
| Account | TFSA, RRSP, Taxable, or Cash |
| Quantity | Number of units (shares, coins, or amount for cash) |
| Cost Basis | Price per unit in the holding's native currency |
| Currency | ISO code of the holding's currency (e.g. `USD`, `CAD`) |
| Target Weight | Optional — your target allocation percentage for rebalancing |

#### Symbol Search

Start typing a symbol or company name in the Symbol field. Results are fetched from Yahoo Finance and cached locally. Selecting a result auto-fills the Name, Asset Type, and Currency fields.

#### Symbol Formats

| Asset Type | Format | Examples |
|------------|--------|---------|
| Stocks | Exchange ticker | `AAPL`, `SHOP.TO`, `ENB.TO` |
| ETFs | Exchange ticker | `VFV.TO`, `QQQ`, `XEF.TO` |
| Crypto | `{COIN}-{CURRENCY}` | `BTC-CAD`, `ETH-USD`, `SOL-USD` |
| Cash | Any label or leave blank | Auto-generated as `{CURRENCY}-CASH` |

> **Toronto Stock Exchange:** append `.TO` to Canadian tickers (e.g. `TD.TO`, `RY.TO`).

#### Account Types

| Account | Description |
|---------|-------------|
| TFSA | Tax-Free Savings Account |
| RRSP | Registered Retirement Savings Plan |
| Taxable | Non-registered brokerage account |
| Cash | Cash position (automatically set for Cash asset type) |

### Editing and Deleting

Click any row in the holdings table to open the edit modal. Update any field and save. To delete, use the trash icon in the edit modal or the row action menu.

### CSV Import

Click **Import** to bulk-load holdings from a CSV file.

**Required columns:** `symbol`, `type`, `quantity`, `cost_basis`, `currency`

**Optional columns:** `name`, `account`, `exchange`, `target_weight`

A preview screen shows each row's validation status (ready, duplicate, invalid symbol, or currency mismatch) before you commit the import. Rows that would be duplicates are skipped automatically.

**Symbol notation:** You can use `SYMBOL:COUNTRY` format for automatic exchange suffix resolution (e.g. `BMO:CA` → `BMO.TO`, `BARC:GB` → `BARC.L`).

Click **Export** or press `⌘E` to download your current holdings as a CSV, which can be re-imported later.

### Sorting and Filtering

Click any column header to sort by that column. Click again to reverse the order. Use the account filter dropdown to show only holdings in a specific account type.

### Gain / Loss Calculation

```
Cost Value   = quantity × cost_basis × fx_rate
Market Value = quantity × current_price × current_fx_rate
Gain / Loss  = Market Value − Cost Value
Gain / Loss% = Gain / Loss ÷ Cost Value × 100
```

FX conversion uses live rates fetched from Yahoo Finance at the time of the last refresh.

---

## Performance

The Performance view shows how your portfolio has grown over time using snapshots recorded on every price refresh.

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

> **Note:** Performance data accumulates from the first time you refresh prices. If you have just set up the app, trigger a refresh (`⌘R`) to record the first snapshot.

### Area Chart

The main chart plots portfolio value over the selected range. Hover anywhere on the chart to see the exact value on that date.

### Daily Returns Bar Chart

Below the area chart, a bar chart shows the day-over-day change for each session. Green bars are gains, red bars are losses.

### Benchmark Overlay

Use the benchmark selector to overlay a reference index (S&P 500, NASDAQ 100, TSX, or Bitcoin) on the area chart. Both series are normalized to the same starting value so relative performance is directly comparable.

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

A scenario is a set of **percentage shocks** applied to each asset class and/or FX rate. For example, "Bear Market" applies −20% to stocks and −40% to crypto simultaneously, showing the combined dollar impact on your portfolio.

### Preset Scenarios

| Scenario | Shocks Applied |
|----------|---------------|
| Mild Correction | Stocks −5%, ETFs −5%, Crypto −10% |
| Bear Market | Stocks −20%, ETFs −20%, Crypto −40%, USD/CAD −5% |
| Crypto Winter | Crypto −50% |
| Base Currency Drop | Base currency weakens −15% vs USD, −10% vs EUR and GBP |
| Stagflation | Stocks −15%, ETFs −12%, Crypto −20%, USD/CAD +8% |
| AI Correction | Stocks −25%, ETFs −20%, Crypto −30% |
| Tech Drawdown | Stocks −35%, ETFs −25%, Crypto −20% |
| Mild Recession | Stocks −10%, ETFs −8%, Crypto −15% |
| Inflation Shock | Stocks −18%, ETFs −15%, USD/CAD +5% |
| CAD Weakness | USD/CAD +12%, EUR/CAD +8%, GBP/CAD +8% |
| Commodity Rally | Stocks +5%, ETFs +4%, CAD/USD +6% |

> The "Base Currency Drop" scenario adjusts dynamically based on your currently selected base currency.

### Custom Scenario

Use the sliders to set your own shocks per asset class and FX pair. Values range from −80% to +80%. Results update live as you drag.

### Reading the Results

- **Total Impact** — the dollar and percentage change applied to your current portfolio value.
- **Waterfall Chart** — contribution of each asset class to the total impact. Bars extending left are losses; right are gains.
- **Breakdown Table** — per-holding detail: current value, stressed value, shock applied, and dollar impact.

### FX Shock Direction

A **positive** USD/CAD shock means CAD weakens relative to USD (e.g. +15% means 1 USD buys 15% more CAD). For USD-denominated holdings this is a **gain** in CAD terms. The breakdown table reflects this correctly.

---

## Rebalancing

The Rebalance view helps you bring your portfolio back to your target allocation.

### Setting Target Weights

Assign a target weight (%) to each holding when adding or editing it in the Holdings view. Weights are stored per holding and can be updated at any time. The total of all target weights must not exceed 100%.

### Reading the Rebalance View

| Column | Description |
|--------|-------------|
| Current Weight | The holding's actual share of the portfolio today |
| Target Weight | Your intended allocation |
| Drift | Current minus target — how far off you are |
| Target Value | What the holding should be worth at your target weight |
| Delta | How much to buy (+) or sell (−) to reach target |

### Deployable Cash

Cash positions with a target weight below their current weight are treated as deployable capital. The view shows how much cash is available to deploy toward underweight positions.

---

## Price Alerts

The Alerts view lets you set price threshold notifications for any symbol.

### Creating an Alert

Click **Add Alert** and fill in:

| Field | Description |
|-------|-------------|
| Symbol | The ticker to watch (e.g. `AAPL`, `BTC-CAD`) |
| Direction | **Above** — fires when price rises above threshold; **Below** — fires when price falls below threshold |
| Threshold | The price level to watch |
| Note | Optional label for the alert |

### How Alerts Fire

Alerts are checked automatically each time prices are refreshed. When a price crosses the threshold, the alert is marked **triggered** and highlighted in the Alerts view.

To watch the same level again, click **Reset** on the triggered alert. To remove it entirely, click **Delete**.

---

## Dividend Tracking

The Dividends view lets you record and review dividend payments for your holdings.

### Recording a Dividend

Click **Add Dividend** and fill in:

| Field | Description |
|-------|-------------|
| Holding | Select the holding that paid the dividend |
| Amount per Unit | Dividend amount per share/unit in the holding's currency |
| Currency | Payment currency |
| Ex-Dividend Date | The ex-date (ownership must precede this date to qualify) |
| Pay Date | The date payment is received |

### Summary and History

The **Summary** grid shows total dividends received per symbol across all recorded payments. The **History** table lists every individual payment with full detail.

> Dividend amounts are recorded as entered and are not converted to base currency automatically.

---

## Settings

Open Settings from the sidebar or press `⌘,`.

### Display

**Base Currency** — select the currency all portfolio values are displayed in. Changing the base currency immediately triggers a price refresh so FX conversions update. Supported: CAD, USD, EUR, GBP, AUD, CHF, JPY.

### Data

**Auto-refresh** — automatically refresh prices in the background on a fixed interval. Options: Disabled (default), 1 minute, 5 minutes, 15 minutes, 30 minutes, 1 hour. A countdown timer in the TopBar shows time until the next refresh.

### Calculations

**Cost Basis Method** — controls how average cost is calculated when you hold multiple lots of the same asset:

| Method | Description |
|--------|-------------|
| AVCO (Average Cost) | Uses the weighted average cost across all purchases. Default and most common for Canadian tax purposes. |
| FIFO (First In, First Out) | Uses the cost of the oldest lot first. Common in the US. |

> The cost basis method setting is stored and surfaced here; full lot-level tracking is planned for a future release.

---

## Multi-Currency

Portfolio Tracker supports holdings in any currency that Yahoo Finance covers. All values are normalized to your **base currency** for display.

### Changing the Base Currency

Use the currency picker in the TopBar (top-right) or go to **Settings → Display → Base Currency**. A price refresh is triggered automatically on change so all conversions update immediately.

### How FX Conversion Works

1. On each price refresh, FX rates are fetched from Yahoo Finance for all currency pairs present in your holdings (e.g. `USDCAD=X`, `EURCAD=X`, `GBPCAD=X`).
2. Each holding's market price is multiplied by the corresponding rate to get the base-currency value.
3. Holdings already denominated in the base currency have a rate of 1.0 and are passed through unchanged.
4. Rates are cached locally and reused between refreshes.

### Supported Currencies

Any currency pair available on Yahoo Finance is supported. Commonly used pairs:

| Currency | Yahoo Finance pair (base = CAD) |
|----------|---------------------------------|
| USD | `USDCAD=X` |
| EUR | `EURCAD=X` |
| GBP | `GBPCAD=X` |
| CHF | `CHFCAD=X` |
| AUD | `AUDCAD=X` |
| JPY | `JPYCAD=X` |

If a rate cannot be fetched, the most recently cached rate is used and a warning banner appears in the TopBar.

---

## Transaction History

The Transaction History view shows every buy and sell transaction recorded for each holding, used to calculate cost basis under the selected AVCO or FIFO method.

### Logging a Transaction

Click **Add Transaction** and fill in:

| Field | Description |
|-------|-------------|
| Holding | The holding this transaction belongs to |
| Type | Buy or Sell |
| Quantity | Number of units transacted |
| Price | Price per unit in the holding's native currency |
| Date | Transaction date and time |

Transactions feed directly into the cost basis calculations shown in the Holdings table. The AVCO method averages all purchase prices; FIFO uses the oldest lot first on each sale.

---

## Analytics

The Analytics view provides portfolio-level risk and composition metrics derived from Yahoo Finance fundamental data.

### Sector Breakdown

A donut chart and table showing what percentage of your portfolio (by market value) falls into each GICS sector (Technology, Financials, Energy, etc.). Cash and unclassified holdings are grouped as "Other".

### Country Exposure

Breakdown of portfolio weight by the country of domicile of each holding. Useful for understanding geographic concentration risk.

### Risk Metrics

| Metric | Description |
|--------|-------------|
| Weighted Beta | Portfolio-level beta, weighted by market value |
| Portfolio Yield | Weighted dividend yield across all holdings |
| P/E Ratio | Weighted price-to-earnings ratio |
| Largest Position | Weight of the single largest holding |
| HHI Concentration | Herfindahl–Hirschman Index — higher = more concentrated |
| Top Sector | Sector with the greatest portfolio weight |

### Realized Gains

Displays the cumulative realized gain/loss from all completed sell transactions, calculated using the configured cost basis method (AVCO or FIFO).

> Analytics data is fetched from Yahoo Finance on first visit and cached for up to 24 hours so subsequent navigation is instant.

---

## Accounts

The Accounts feature lets you organize holdings by named account (TFSA, RRSP, FHSA, Taxable, Crypto, Other). Each account has a name, type, and optional notes.

### Managing Accounts

Open the **Accounts** modal from the Holdings toolbar. You can:
- Create named accounts (e.g. "Questrade TFSA", "Wealthsimple RRSP")
- Delete accounts that have no holdings assigned

Holdings are assigned to an account when you add or edit them. The Holdings table can be filtered by account using the account dropdown.

---

## Action Center

The Action Center is a quick-access side panel that surfaces recent price alert triggers and provides a fast path to log transactions without navigating away from your current view.

### Alert Triggers

Recent alert triggers appear in a list with symbol, threshold crossed, and timestamp. Triggered alerts can be reset directly from the Action Center.

### Fast Transaction Entry

The Action Center includes a compact transaction form. Enter symbol, type (buy/sell), quantity, and price to log a transaction immediately.

---

## Keyboard Shortcuts

Press `?` at any time to open the shortcuts overlay. Full list:

| Shortcut | Action |
|----------|--------|
| `1` / `⌘1` | Navigate to Dashboard |
| `2` / `⌘2` | Navigate to Holdings |
| `3` / `⌘3` | Navigate to Performance |
| `4` / `⌘4` | Navigate to Stress Test |
| `⌘R` | Refresh prices |
| `⌘N` | Open Add Holding modal |
| `⌘E` | Export holdings to CSV |
| `⌘,` | Open Settings |
| `?` | Toggle keyboard shortcuts overlay |
| `Esc` | Close overlays / modals |

> Shortcuts do not fire when focus is inside an input, textarea, or select field.

---

## Backup and Restore

Use **Export Data** (in the Holdings menu) to download a full JSON backup — holdings, alerts, transactions, dividends, and settings. Use **Import Data** to restore from that file. Import replaces all current data — export first if you want to preserve existing data.

For a CSV-based backup of holdings only, use **Export CSV** (`⌘E`) from the Holdings view.
