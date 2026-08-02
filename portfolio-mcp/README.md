# portfolio-mcp

A standalone MCP (Model Context Protocol) server that exposes the Portfolio Tracker database over the stdio transport, letting AI assistants read and write your portfolio data.

## Tools

| Tool | Description |
|------|-------------|
| `list_holdings` | List all current holdings |
| `add_holding` | Add a new holding |
| `update_holding` | Full-replace update of an existing holding's editable fields |
| `delete_holding` | Soft-delete a holding by UUID |
| `list_accounts` | List all accounts (id, name, type, institution) |
| `create_account` | Create a new account |
| `list_dividends` | List dividend records, optionally filtered by holding UUID |
| `add_dividend` | Record a new dividend payment for a holding |
| `delete_dividend` | Soft-delete a dividend record by UUID |
| `list_transactions` | List all buy/sell transactions |
| `add_transaction` | Record a new transaction |
| `delete_transaction` | Soft-delete a transaction |
| `list_alerts` | List all price alerts |
| `add_alert` | Create a price alert |
| `delete_alert` | Delete an alert |
| `reset_alert` | Reset a triggered alert |
| `get_portfolio_snapshot` | Full snapshot with live prices, market values, G/L, weights |
| `run_stress_test` | Apply asset-class and FX shocks to the current portfolio (shocks validated at the tool boundary) |
| `get_config` | Read a config value (key must be on the allowlist; e.g. `base_currency`) |
| `set_config` | Write a config value (key and value both validated, mirroring the Tauri app's config layer) |

All write tools (`add_*`, `update_*`, `delete_*`, `reset_alert`, `create_account`) validate their input — including UUID format on every ID — using the same rules enforced by the Tauri desktop app, so data written through this server can't bypass validation the UI would otherwise apply. List tools are **not** paginated; they return the full result set.

## Build

```bash
. ~/.cargo/env
cargo build -p portfolio-mcp --release
```

The binary is at `target/release/portfolio-mcp`.

## Configuration in Claude Code

Add to `~/.claude/settings.json`:

```json
{
  "mcpServers": {
    "portfolio": {
      "command": "/path/to/target/release/portfolio-mcp",
      "env": {
        "PORTFOLIO_DB_PATH": "/Users/YOU/Library/Application Support/com.portfolio-tracker.app/portfolio.db"
      }
    }
  }
}
```

Replace `/path/to/target/release/portfolio-mcp` with the actual binary path (e.g. the absolute path from `pwd` inside the repo).

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `PORTFOLIO_DB_PATH` | `~/Library/Application Support/com.portfolio-tracker.app/portfolio.db` | Path to the SQLite DB created by the Tauri app |
| `RUST_LOG` | `portfolio_mcp=info` | Log level filter (logs go to stderr, never stdout) |

## Notes

- The server connects to the **existing** database created by the Tauri app.  It will not create a new database.
- Prices and FX rates are read from the cache populated by the Tauri app's refresh cycle.  The MCP server does not fetch live prices itself.
- `realized_gains` and `annual_dividend_income` in `get_portfolio_snapshot` are reported as `0` in the MCP context; for authoritative figures, use the Tauri app directly.
- Portfolio-snapshot, FX, and stress-test math come from the shared `portfolio-core` crate (also used by `src-tauri`), so this server and the desktop app always compute the same values.
- `src/validation.rs` mirrors the Tauri command layer's validation (field checks, UUID format, config key/value allowlist, stress-shock bounds), so writes made through this server are held to the same rules as the desktop app.
