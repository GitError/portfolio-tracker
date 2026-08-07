# Privacy & Local-Data Threat Model

Portfolio Tracker is a local-first, single-user desktop app. There is no account system,
no cloud sync, and no backend server operated by the app's developers. This document
describes what data is stored, where, and what protection it does (and doesn't) have.

---

## Trust model

- **Local user-profile only.** All data — holdings, transactions, dividends, alerts,
  accounts, config — belongs to the OS user profile running the app. Anyone with access
  to that profile (another local admin, malware running as that user, a stolen unlocked
  device) has the same access the app does.
- **No cloud sync.** The app never transmits portfolio data (holdings, quantities, cost
  basis, transactions) to any server. The only outbound network calls are to Yahoo
  Finance for price/FX quotes and symbol search (`src-tauri/src/price.rs`,
  `src-tauri/src/fx.rs`, `src-tauri/src/search.rs`) — these send symbols and currency
  pairs being priced, not portfolio contents.
- **No remote telemetry.** The app does not phone home usage data, crash reports, or
  analytics anywhere.
- **portfolio-mcp** (see `CLAUDE.md` / `portfolio-mcp/README.md`) reads and writes the
  same local database over stdio when explicitly configured into an MCP client (e.g.
  Claude Code). It never opens a network listener; access is gated entirely by whoever
  can launch the local binary with `PORTFOLIO_DB_PATH` set.

## Where data lives

| Store | Location | Contents |
|---|---|---|
| SQLite database | `app_data_dir/portfolio.db` — on macOS, `~/Library/Application Support/com.giterror.portfolio-tracker/portfolio.db` (see `src-tauri/tauri.conf.json`'s `identifier` and `src-tauri/src/lib.rs`'s `app.path().app_data_dir()`) | All holdings, transactions, dividends, alerts, accounts, and config: symbols, quantities, cost basis, account types, thresholds |
| WebView local storage (`localStorage`) | Managed by the OS WebView, inside the same app-data profile | See table below |

### `localStorage` keys

| Key | Purpose | Contents |
|---|---|---|
| `portfolio_snapshot_cache` | Offline fallback shown while the Tauri backend is briefly unreachable | **Totals only**: `totalValue`, `holdingCount`, `lastUpdated`, `baseCurrency`. No symbols, quantities, cost basis, or account data (`frontend/lib/portfolioCache.ts`) |
| `app-config` | Fallback for `app_theme` / `app_language` / `base_currency` / `cost_basis_method` etc. when running outside Tauri (`npm run dev` in a browser) — unused in the shipped desktop app, where config is persisted in SQLite via `get_config_cmd`/`set_config_cmd` | Display preferences only, no financial data |
| `app_theme`, `app_language` | Pre-mount theme/locale so the first paint isn't unstyled, before the async Tauri config call resolves | A theme name / locale code |
| `sidebar-expanded` | UI layout preference | A boolean |

Prior to the fix for #757, `portfolio_snapshot_cache` stored a full copy of the portfolio
snapshot **and** the raw holdings array (symbols, quantities, cost basis, account types) —
duplicating sensitive data from the SQLite database into a second, separately-cleared
plaintext store. It now stores only the four aggregate fields above; the live holdings
list is never cached, and the offline UI (`TopBar`, `Dashboard`, `Holdings`) is expected
to show a "reconnect to view your holdings" message rather than stale per-holding data
when offline (`isOffline` in `frontend/hooks/usePortfolio.ts`).

Users can clear `portfolio_snapshot_cache` at any time from **Settings → Data Management
→ Clear Local Cache** (`frontend/components/Settings.tsx`), which calls
`clearSnapshotCache()`.

## What is NOT protected

- **The SQLite database is not encrypted at rest.** Anything readable by the OS user
  account can open `portfolio.db` directly with any SQLite client and read all holdings,
  transactions, and cost-basis data in plaintext. Full-disk encryption (e.g. macOS
  FileVault) protects the file only while the disk is powered off / locked; it provides
  no protection against another process or user on an unlocked, logged-in machine.
- **`localStorage` is cleared on WebView data reset**, e.g. clearing the app's website
  data, resetting the Tauri WebView, or uninstalling/reinstalling the app — same as any
  other local-only cache. Losing it only degrades the brief offline fallback (see above);
  no user data is lost, since it was never the source of truth.
- **Backups are also unencrypted.** `backup_database` (`src-tauri/src/commands/backup.rs`)
  writes a plain copy of `portfolio.db` to a user-chosen location; that copy carries the
  same lack of at-rest protection as the live database, and is the user's responsibility
  to store securely once exported.
- **No secret separates one local user's data from another's file permissions** — the
  database is not access-controlled beyond normal OS file permissions on `app_data_dir`.

## Future direction (not implemented)

An OS-keystore-derived encryption mode for the SQLite database (e.g. deriving a SQLCipher
key from the macOS Keychain) is a plausible future hardening step, but it is **not**
currently implemented anywhere in this codebase. Treat any mention of "encrypted
database" elsewhere as aspirational until this document is updated to say otherwise.
