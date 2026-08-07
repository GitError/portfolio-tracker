# portfolio-mcp

A standalone MCP (Model Context Protocol) server that exposes the Portfolio Tracker database over the stdio transport, letting AI assistants read your portfolio data — and, if explicitly opted into, write and delete it too.

## Access modes

The server is **read-only by default**. Mutating tools aren't just denied when called — they aren't registered at all, so a client's `tools/list` won't even show them unless the corresponding mode is enabled:

| Env var | Default | Registers |
|---------|---------|-----------|
| `PORTFOLIO_MCP_WRITE_ENABLED` | `false` | Non-destructive write tools: `add_holding`, `update_holding`, `create_account`, `add_dividend`, `add_transaction`, `add_alert`, `reset_alert`, `set_config` |
| `PORTFOLIO_MCP_DESTRUCTIVE_WRITE_ENABLED` | `false` | Destructive tools: `delete_holding`, `delete_dividend`, `delete_transaction`, `delete_alert` |

Set a variable to the literal string `true` to enable it; any other value (or leaving it unset) keeps that category disabled. The two flags are independent — you can allow data entry without allowing deletion, or vice versa. All read-only tools (`list_*`, `get_portfolio_snapshot`, `run_stress_test`, `get_config`) are always available regardless of these flags.

If a client calls a tool that isn't enabled, the server returns an error explaining which env var to set rather than silently failing. The active mode is logged once at startup (to stderr — see `RUST_LOG`).

```json
{
  "mcpServers": {
    "portfolio": {
      "command": "/path/to/target/release/portfolio-mcp",
      "env": {
        "PORTFOLIO_DB_PATH": "/Users/YOU/Library/Application Support/com.portfolio-tracker.app/portfolio.db",
        "PORTFOLIO_MCP_WRITE_ENABLED": "true",
        "PORTFOLIO_MCP_DESTRUCTIVE_WRITE_ENABLED": "false"
      }
    }
  }
}
```

## Tools

| Tool | Mode required | Description |
|------|----------------|-------------|
| `list_holdings` | read-only | List all current holdings |
| `add_holding` | write | Add a new holding |
| `update_holding` | write | Full-replace update of an existing holding's editable fields |
| `delete_holding` | destructive | Soft-delete a holding by UUID |
| `list_accounts` | read-only | List all accounts (id, name, type, institution) |
| `create_account` | write | Create a new account |
| `list_dividends` | read-only | List dividend records, optionally filtered by holding UUID |
| `add_dividend` | write | Record a new dividend payment for a holding |
| `delete_dividend` | destructive | Soft-delete a dividend record by UUID |
| `list_transactions` | read-only | List all buy/sell transactions |
| `add_transaction` | write | Record a new transaction |
| `delete_transaction` | destructive | Soft-delete a transaction |
| `list_alerts` | read-only | List all price alerts |
| `add_alert` | write | Create a price alert |
| `delete_alert` | destructive | Delete an alert |
| `reset_alert` | write | Reset a triggered alert |
| `get_portfolio_snapshot` | read-only | Full snapshot with live prices, market values, G/L, weights |
| `run_stress_test` | read-only | Apply asset-class and FX shocks to the current portfolio (shocks validated at the tool boundary; simulation only, no writes) |
| `get_config` | read-only | Read a config value (key must be on the allowlist; e.g. `base_currency`) |
| `set_config` | write | Write a config value (key and value both validated, mirroring the Tauri app's config layer) |

All write tools (`add_*`, `update_*`, `delete_*`, `reset_alert`, `create_account`) validate their input — including UUID format on every ID — using the same rules enforced by the Tauri desktop app, so data written through this server can't bypass validation the UI would otherwise apply. List tools are **not** paginated; they return the full result set.

## Build

```bash
. ~/.cargo/env
cargo build -p portfolio-mcp --release
```

The binary is at `target/release/portfolio-mcp`.

## Configuration in Claude Code

Add to `~/.claude/settings.json`. This example runs in read-only mode — omit `PORTFOLIO_MCP_WRITE_ENABLED` / `PORTFOLIO_MCP_DESTRUCTIVE_WRITE_ENABLED` entirely, or set them per the [Access modes](#access-modes) section above, to change that:

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
| `PORTFOLIO_MCP_WRITE_ENABLED` | `false` | Set to `true` to register non-destructive write tools — see [Access modes](#access-modes) |
| `PORTFOLIO_MCP_DESTRUCTIVE_WRITE_ENABLED` | `false` | Set to `true` to register `delete_*` tools — see [Access modes](#access-modes) |
| `RUST_LOG` | `portfolio_mcp=info` | Log level filter (logs go to stderr, never stdout) |

## Notes

- The server connects to the **existing** database created by the Tauri app.  It will not create a new database.
- Prices and FX rates are read from the cache populated by the Tauri app's refresh cycle.  The MCP server does not fetch live prices itself.
- `realized_gains` and `annual_dividend_income` in `get_portfolio_snapshot` are reported as `0` in the MCP context; for authoritative figures, use the Tauri app directly.
- Portfolio-snapshot, FX, and stress-test math come from the shared `portfolio-core` crate (also used by `src-tauri`), so this server and the desktop app always compute the same values.
- `src/validation.rs` mirrors the Tauri command layer's validation (field checks, UUID format, config key/value allowlist, stress-shock bounds), so writes made through this server are held to the same rules as the desktop app.
