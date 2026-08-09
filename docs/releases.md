# Releases

## post-v0.2.0 — 2026-08-08 to 2026-08-09

A batch of correctness fixes, security improvements, and feature completions shipped after v0.2.0.

**Added**
- **Color Schemes** — Settings → Display now offers a Color Scheme picker alongside the existing Dark/Light/System toggle: **Dracula**, **SynthWave '84**, **Nord** (dark), and **Warm Light** (light). Applied instantly via a `[data-scheme="..."]` CSS override through a new `useColorScheme` hook, no reload required. Persisted via a new `app_color_scheme` config key, allowlisted and value-validated in `portfolio-core` and shared by both the Tauri app and the MCP server. The Theme toggle disables with an explanatory note while a non-default scheme is active. All palettes verified WCAG AA (≥4.5:1). Closes #778. PR #797.
- **Research Watchlist** — new "Research" tab for tracking investment ideas before adding them to the portfolio. Named watchlists hold per-symbol research notes (thesis, catalysts, risks, entry-price range) alongside a cached Yahoo Finance market-data snapshot (price, market cap, 52-week range, YTD/1Y return, dividend yield, P/E). Every snapshot shows its retrieval time; stale data (>15 min) is flagged visually and never hidden. Refresh All / per-row refresh with a 5-minute per-symbol cooldown. "Add to Holdings" pre-fills the existing modal without auto-importing. 3 new DB tables (`watchlists`, `watchlist_items`, `watchlist_item_snapshots`), 9 Tauri commands, translated into all 8 supported locales. PR #773.
- **Export to PDF** — "Export PDF" button in the Holdings toolbar. Generates a portfolio summary PDF (header with export timestamp and base currency, per-holding table with alternating row shading, allocation breakdown by asset class) and auto-saves to `~/Downloads/portfolio-YYYY-MM-DD.pdf`. Built with the `genpdf` Rust crate and IBM Plex Sans embedded fonts. No dialog plugin required — the path is fully computed server-side. PR #765. Gated behind `import.meta.env.DEV` and hidden from production builds until table formatting is production-ready; re-exports now append `_1`, `_2`, ... instead of silently overwriting a same-day file. Closes #785. PRs #782, #792.
- **MCP write access opt-in** — `portfolio-mcp` is now read-only by default. Set `PORTFOLIO_MCP_WRITE_ENABLED=true` to register write and delete tools. Startup log shows which mode is active. PR #762.
- **localStorage privacy** — Reduced localStorage footprint: only the last portfolio snapshot (market values and metadata, no sensitive personal data) is persisted for offline fallback. Documented in `docs/privacy.md`. PR #763.

**Fixed**
- **Yahoo Finance watchlist authentication** — `v7/finance/quote` (used by the Research Watchlist snapshot fetch) now requires a session cookie + crumb token; requests with only a `User-Agent` header were rejected with HTTP 401. Adds cookie+crumb acquisition ahead of the request. The main price-refresh path (`v8/finance/chart`) was unaffected. PR #795.
- **Import wizard file selection** — Tauri v2 `File` objects never expose a real filesystem `.path` (that only existed in Tauri v1), so selecting a file in Step 1 always failed with "Could not resolve a filesystem path for this file." Replaced with `tauri-plugin-dialog`'s native `open()` picker, which returns a real OS path directly. Closes #776. PR #777.
- **Import wizard commit validation** — `commit_import_rows` only checked for negative quantity before writing holdings, letting zero/NaN/infinite quantity, invalid cost basis, malformed currency codes, and out-of-range target weight slip past the wizard. Now routes every committed row through the same shared `portfolio_core::validation::validate_holding_fields` validators the regular holding commands use. PR #783.
- **CSV import aggregate cash rows** — Brokerage CSV exports sometimes include an aggregate/summary cash line (e.g. "Total Cash") alongside individual positions, which was previously imported as a real holding and double-counted. `normalize_row` now detects and skips these rows, with an explanatory note in the import plan. PR #791.
- **Import wizard UI polish** — the account-picker dropdown collapsed shorter than other populated dropdowns when no option was selected (empty inline child collapsing the flex trigger's line box); now falls back to a non-breaking space. The wizard's final-step title was renamed from "Import Plus Insights" to "Import Summary" across all locale files. PR #793.
- **Performance view first-snapshot state** — immediately after a fresh import + refresh, the Performance chart looked flat/misleading because `perfIsEmpty` only special-cased zero snapshots, not the normal one-snapshot state. Now shows an explicit first-snapshot state instead. Closes #787. PR #788.
- **Dashboard layout and price-refresh UX** — added a Dismiss control to the price-refresh failure banner alongside Retry (dismissing hides the banner without clearing `failedSymbols` or triggering another refresh); moved the summary panel (Positions, Best/Worst Performer, Cash Position, Realized Gains) to the top of the Dashboard; increased Top Movers from 3 to 6 rows per side; limited Concentration to the top 5 holdings by weight. PR #794.
- **Sidebar de-duplication** — removed the embedded portfolio value / daily P&L block from the sidebar so it's a clean, navigation-only panel again; those stats remain fully visible on the Dashboard. Closes #790. PR #796.
- **Missing-FX realized gains fallback** — `compute_realized_gains_grouped` now surfaces an explicit error/unavailable state when no FX rate is cached for a non-base holding, instead of silently using a 1:1 rate and displaying a USD amount as CAD. PR #771.
- **Realized-gains cache invalidation** — `RealizedGainsCacheState` is now invalidated when a holding's currency is updated or a holding is soft-deleted. Historical transactions for soft-deleted holdings are correctly converted using the holding's original currency rather than defaulting to the base currency. PR #772.
- **MCP default database path** — `portfolio-mcp` default DB path corrected from `com.portfolio-tracker.app` to `com.giterror.portfolio-tracker` to match the Tauri app identifier. Launching without `PORTFOLIO_DB_PATH` now opens the correct database. PR #770.
- **Multi-currency realized gains** (`compute_realized_gains_grouped`) — gains and proceeds are now converted to base currency per holding before aggregation. Mixed CAD/USD portfolios were previously showing incorrect totals. PR #760.
- **CI workspace coverage** — CI now runs `cargo test --workspace`, `cargo clippy --workspace`, and `cargo fmt --all` from the workspace root. Path filters include `portfolio-core/**`, `portfolio-mcp/**`, `Cargo.toml`, and `Cargo.lock`. Previously only `src-tauri/**` triggered Rust CI, leaving the two library crates uncovered. PR #761.

**Refactored**
- **Centralized validation** — Shared domain validation (UUID format, currency codes, account types, config keys/values, field bounds) moved from duplicated implementations in `src-tauri` and `portfolio-mcp` into a single `portfolio-core::validation` module. Both layers now delegate to the same source of truth. PR #764.

**Docs / housekeeping**
- Roadmap rewritten: near-term reordered around financial correctness, speculative features moved to ❄️ icebox, Recently Shipped updated.
- README updated to reflect cross-platform support (macOS Apple Silicon + Intel, Windows, Linux). Keyboard shortcuts include `Ctrl` equivalents for Windows/Linux.
- CRA XML ts-rs bindings (`CraXmlResult`, `T5008Disposition`, `T5IncomeRecord`) committed.

---

## v0.2.0 — 2026-08-05

The headline feature of this release is the **Import Plus Insights wizard** — a guided multi-step flow that replaces the old strict CSV importer. The wizard accepts CSV, XLSX, and CRA XML files, infers column roles from broker-specific header aliases, previews an import plan row-by-row before committing, and shows post-import insights after the commit.

**Added**
- **Import Plus Insights wizard** — CSV/XLSX file upload → account context selection → broker-alias column inference → import plan with per-row status (create / update / skip / needs_fix / warning) → commit → post-import insights summary.
- **XLSX import** — spreadsheet files supported alongside CSV; first sheet is parsed via the `calamine` Rust crate.
- **CRA XML import** — T5008 (Statement of Securities Transactions) and T5 (Statement of Investment Income) XML from CRA My Account are parsed server-side via `quick-xml` + serde and fed into the wizard's import plan. T5008 slips map to `T5008Disposition`; T5 slips map to `T5IncomeRecord`.
- **Broker-alias column registry** — maps common header variants (`Ticker`, `Security`, `Symbol`, `Qty`, `Shares`, `Units`, …) to canonical fields so files from Questrade, Wealthsimple, RBC, TD, and other brokers import without manual header editing.

**Fixed**
- Analytics N+1 eliminated — `useAnalytics` now batch-fetches fundamentals from the cache in a single pass instead of one round-trip per holding.
- Import row validation tightened: empty symbol, zero/negative quantity, and missing cost basis are caught at the `needs_fix` stage rather than failing silently on commit.
- Duplicate detection uses symbol + account + quantity fingerprinting so re-importing an unchanged file produces zero `create` or `update` rows.
- Import error messages surfaced per-row in the plan view rather than aborting the whole import on first error.
- `parse_cra_xml_cmd` tolerates missing optional fields in T5008 slips (`book_value`, `security_type`) and continues processing remaining slips instead of returning an error on the first incomplete record.

**Changed**
- Old strict CSV importer removed; the Import wizard is now the only import path.
- Holdings toolbar **Import** button opens the wizard modal.

---

## 2026-08-02 — Housekeeping Pass

A broad housekeeping pass across the whole workspace (roughly PRs #599–#715, since the prior 2026-07-28 documentation snapshot). Not yet tagged as a new version — `package.json`, `src-tauri/tauri.conf.json`, and `src-tauri/Cargo.toml` are still at `0.1.0-8`; see `CHANGELOG.md` for the itemized per-PR history.

**Added**
- **`portfolio-core`** — a new shared workspace crate holding portfolio-snapshot, FX, and stress-test math. Both `src-tauri` and `portfolio-mcp` depend on it instead of duplicating the logic, so the desktop app and the MCP server always agree on computed values.
- **MCP server tools** — `update_holding`, `list_accounts`, `create_account`, `list_dividends`, `add_dividend`, `delete_dividend`. `portfolio-mcp` now covers accounts and dividends in addition to holdings, transactions, and alerts (20 tools total).
- **Frontend `ErrorBoundary`** wraps the app root so a component crash shows a fallback screen instead of a blank window.
- **`AppError::NotFound` / `AppError::Conflict`** — Tauri commands can now surface not-found and conflict states distinctly instead of collapsing everything into a generic validation error.
- Migration `0013_dividends_pay_date_index.sql` — indexes `dividends.pay_date`, the column the annual-dividend-income query filters on.

**Fixed**
- Config key allowlist and per-key value validation (`app_theme`, `app_language`, `base_currency`, `cost_basis_method`, `holdings_hidden_columns`) applied consistently on both the Tauri (`commands/config.rs`) and MCP (`validation.rs`) layers, including a `cost_basis_method` avco/fifo check with a graceful fallback on read.
- UUID format validation added to every MCP delete/reset tool, matching the Tauri command layer — a malformed ID is now rejected instead of silently affecting zero rows.
- Stress-test shocks validated at the command boundary on both the Tauri and MCP layers.
- `backup_database`/`restore_database` hardened: a concurrency guard (`BackupLockState`) rejects an overlapping backup/restore instead of racing the same file, WAL is checkpointed before the file is copied, and stale backup staging files left over from a previous run are cleaned up on startup.
- A background WAL checkpoint task plus a bounded pool-close timeout on app shutdown prevent unbounded WAL growth and hangs on quit.
- `sanitize_str` UTF-8 safety fix in the CSV layer; symbol input is now validated before being used to build Yahoo Finance request URLs.
- Locale-aware formatting completed across the frontend — `formatPercent`, `formatTargetWeight`, and `formatCompact` (now using the user's base currency) all respect the active locale, and `useActionInsights` recommendations are fully wired to i18next.
- `useLanguage.setLanguage` is now properly awaited with error handling and rolls back on failure instead of firing and forgetting.
- The Alerts and Dividends views keep a persistent error banner on load failure instead of silently falling back to an empty list.

---

## Download

Published installers are attached to GitHub Releases:

- Latest release: https://github.com/GitError/portfolio-tracker/releases/latest
- All releases: https://github.com/GitError/portfolio-tracker/releases

Draft releases are created automatically from tags that match `v*.*.*`.

## Versioning

Keep these three files in sync for every release:

- `package.json`
- `src-tauri/tauri.conf.json`
- `src-tauri/Cargo.toml`

Use `./scripts/bump-version.sh X.Y.Z` to update them together.

## Release flow

1. Update `CHANGELOG.md`
2. Run `./scripts/bump-version.sh X.Y.Z`
3. Commit the version bump
4. Tag the release with `vX.Y.Z`
5. Push the branch and tag
6. GitHub Actions builds installers and creates a draft release
7. Review the draft release and publish it manually

## Code signing

### macOS

Unsigned macOS builds work for testing, but users will see Gatekeeper warnings.
To enable Developer ID signing and notarization, configure these repository secrets:

- `APPLE_CERTIFICATE`
- `APPLE_CERTIFICATE_PASSWORD`
- `APPLE_SIGNING_IDENTITY`
- `APPLE_ID`
- `APPLE_PASSWORD`
- `APPLE_TEAM_ID`

The release workflow is already wired to use those environment variables when they are present.

### Windows

Windows signing is optional for now. Without a signing certificate, SmartScreen may warn on first launch.

## Expected artifacts

- macOS Apple Silicon: `.dmg`
- macOS Intel: `.dmg`
- Windows: `.exe`, `.msi`
- Linux: `.AppImage`, `.deb`
