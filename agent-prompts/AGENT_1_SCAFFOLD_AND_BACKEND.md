# Agent 1: Scaffold + Rust Backend

## Role
You are building the foundation: project scaffold and the complete Rust backend for a Tauri v2 portfolio tracker app. Read CLAUDE.md first — it is the source of truth for types, architecture, and conventions.

## Step 1: Project Scaffold

Create a Tauri v2 project with React + TypeScript frontend using Vite.

```bash
npm create tauri-app@latest portfolio-tracker -- --template react-ts
cd portfolio-tracker
```

Install frontend dependencies:
```bash
npm install recharts react-router-dom lucide-react
npm install -D tailwindcss @tailwindcss/vite
```

Set up Rust dependencies in `src-tauri/Cargo.toml`:
```toml
[dependencies]
tauri = { version = "2", features = [] }
tauri-plugin-shell = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
reqwest = { version = "0.12", features = ["json"] }
tokio = { version = "1", features = ["full"] }
chrono = { version = "0.4", features = ["serde"] }
rusqlite = { version = "0.31", features = ["bundled"] }
uuid = { version = "1", features = ["v4", "serde"] }
```

Copy the CLAUDE.md into the project root. Configure `tauri.conf.json` per the architecture doc (window 1200×800, title "Portfolio Tracker").

Set up Tailwind with the CSS variables from CLAUDE.md in `src/index.css`. Add Google Fonts import to `index.html`.

## Step 2: types.rs

Create `src-tauri/src/types.rs` with all shared types. These MUST serialize to camelCase to match the TypeScript contract:

- `AssetType` enum: Stock, Etf, Crypto, Cash (serde lowercase)
- `Holding` struct: id, symbol, name, asset_type, quantity (f64), cost_basis (f64), currency, created_at, updated_at
- `HoldingInput` struct: same as Holding but without id, created_at, updated_at (for add command)
- `PriceData` struct: symbol, price (f64), currency, change (f64), change_percent (f64), updated_at
- `FxRate` struct: pair, rate (f64), updated_at
- `HoldingWithPrice` struct: all Holding fields + current_price, current_price_cad, market_value_cad, cost_value_cad, gain_loss, gain_loss_percent, weight, daily_change_percent
- `PortfolioSnapshot` struct: holdings (Vec<HoldingWithPrice>), total_value, total_cost, total_gain_loss, total_gain_loss_percent, daily_pnl, last_updated
- `StressScenario` struct: name, shocks (HashMap<String, f64>)
- `StressHoldingResult` struct: holding_id, symbol, name, current_value, stressed_value, impact, shock_applied
- `StressResult` struct: scenario, current_value, stressed_value, total_impact, total_impact_percent, holding_breakdown (Vec<StressHoldingResult>)

## Step 3: db.rs

SQLite database layer:

- `init_db(connection)` — create tables if not exist: holdings, price_cache, fx_rates (schemas in CLAUDE.md)
- `insert_holding(conn, input: HoldingInput) -> Result<Holding>` — generate UUID, set timestamps
- `update_holding(conn, holding: Holding) -> Result<Holding>` — update all fields, set updated_at
- `delete_holding(conn, id: &str) -> Result<bool>`
- `get_all_holdings(conn) -> Result<Vec<Holding>>`
- `upsert_price(conn, price: &PriceData) -> Result<()>`
- `get_cached_prices(conn) -> Result<Vec<PriceData>>`
- `upsert_fx_rate(conn, rate: &FxRate) -> Result<()>`
- `get_fx_rates(conn) -> Result<Vec<FxRate>>`

Use `?` operator for error handling, map rusqlite errors to String for Tauri compatibility.

## Step 4: price.rs

Yahoo Finance price fetching:

- `fetch_price(client: &reqwest::Client, symbol: &str) -> Result<PriceData>`
  - URL: `https://query1.finance.yahoo.com/v8/finance/chart/{symbol}?interval=1d&range=1d`
  - MUST set `User-Agent: Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)` header
  - Parse JSON: navigate to `chart.result[0].meta` for regularMarketPrice, previousClose, currency
  - Calculate change and changePercent from regularMarketPrice vs previousClose

- `fetch_all_prices(client: &reqwest::Client, symbols: Vec<String>) -> Vec<PriceData>`
  - Fetch concurrently with tokio::join or futures::join_all
  - Skip failures, log errors, return successful results

- For crypto: symbols are formatted as `BTC-CAD`, `ETH-CAD`, etc. on Yahoo Finance
- For cash holdings: no price fetch needed, value = quantity (already in stated currency)

## Step 5: fx.rs

FX rate fetching:

- `fetch_fx_rate(client: &reqwest::Client, from: &str) -> Result<FxRate>`
  - Symbol format: `{from}CAD=X` (e.g., `USDCAD=X`, `EURCAD=X`)
  - Same Yahoo Finance endpoint as prices
  - Return FxRate { pair: "USDCAD", rate, updated_at }

- `fetch_all_fx_rates(client: &reqwest::Client, currencies: Vec<String>) -> Vec<FxRate>`
  - Skip "CAD" (rate is 1.0)
  - Fetch concurrently

- `convert_to_cad(amount: f64, from_currency: &str, rates: &[FxRate]) -> f64`
  - If from_currency == "CAD", return amount
  - Find matching rate, multiply
  - If rate not found, log warning and return amount unchanged

## Step 6: stress.rs

Stress test engine:

- `run_stress_test(snapshot: &PortfolioSnapshot, scenario: &StressScenario) -> StressResult`
  - For each holding in snapshot:
    - Determine applicable shock: look up by asset_type (e.g., "stock" → scenario.shocks["stock"])
    - Also apply FX shock if holding currency != CAD (e.g., USD holding → apply scenario.shocks["fx_usd_cad"] to the FX rate)
    - stressed_value = current_market_value_cad × (1 + asset_shock) × (1 + fx_shock)
    - Track per-holding impact
  - Sum up totals, calculate delta and percent

## Step 7: commands.rs

Tauri commands (thin wrappers over domain logic):

```rust
#[tauri::command]
async fn get_portfolio(db: State<'_, DbState>, client: State<'_, HttpClient>) -> Result<PortfolioSnapshot, String>
```

Commands to implement:
- `get_portfolio` — get holdings, fetch/use cached prices, fetch/use cached FX, assemble snapshot
- `get_holdings` — raw holdings list from DB
- `add_holding` — insert, return created holding
- `update_holding` — update, return updated holding
- `delete_holding` — delete by id, return bool
- `refresh_prices` — fetch all prices + FX rates for held symbols/currencies, update cache, return prices
- `run_stress_test` — take StressScenario, build snapshot, run engine, return result
- `get_performance` — for v1, return mock data (array of {date, value} covering requested range). Add a TODO for real historical tracking.

State types:
```rust
pub struct DbState(pub Mutex<rusqlite::Connection>);
pub struct HttpClient(pub reqwest::Client);
```

## Step 8: main.rs

Wire everything together:

```rust
fn main() {
    let db_path = /* tauri app_data_dir */;
    let conn = Connection::open(db_path).expect("Failed to open DB");
    db::init_db(&conn).expect("Failed to init DB");

    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)")
        .build()
        .expect("Failed to create HTTP client");

    tauri::Builder::default()
        .manage(DbState(Mutex::new(conn)))
        .manage(HttpClient(client))
        .invoke_handler(tauri::generate_handler![
            commands::get_portfolio,
            commands::get_holdings,
            commands::add_holding,
            commands::update_holding,
            commands::delete_holding,
            commands::refresh_prices,
            commands::run_stress_test,
            commands::get_performance,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

## Verification
After building, run `cargo build` in `src-tauri/` — it should compile with zero errors. The frontend doesn't need to be functional yet, just the Vite dev server should start.
