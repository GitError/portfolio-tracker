# Releases

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
