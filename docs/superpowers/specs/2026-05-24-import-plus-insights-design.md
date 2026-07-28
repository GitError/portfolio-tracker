# Import Plus Insights Design

## Context

Portfolio Tracker already has a broad MVP: holdings, accounts, transactions, dividends, alerts, analytics, stress tests, rebalancing, CSV import/export, and local SQLite persistence. The highest-value next improvement is not another calculated field or cosmetic pass. The app needs to ingest real portfolio data with much less manual shaping, then turn the import result into immediate reviewable insight.

The current importer is too strict for real brokerage exports. It accepts CSV only, expects canonical app headers, requires fields such as `type`, `currency`, and `cost_basis`, and treats the account as a row-level CSV concern. In practice, the user should be able to upload a brokerage CSV or XLSX, choose the account once, and let the app infer holdings from common broker columns.

## Goals

- Replace the current schema-shaped CSV import with a guided import workflow.
- Support account context at import time: account type and account name apply to all rows unless the file provides stronger account information.
- Infer canonical holding fields from messy brokerage column names using deterministic aliases and broker profiles.
- Support derived values such as per-unit cost basis from book value divided by quantity.
- Produce a reviewable import plan before committing changes.
- Show post-import insights that explain what changed and what needs attention.
- Keep the first version local and deterministic; no brokerage API, cloud sync, or LLM inference.

## Non-Goals

- Direct Questrade, Interactive Brokers, Wealthsimple, or other brokerage API integration.
- Full transaction-history reconstruction from statement exports.
- Tax-ready adjusted cost base reporting.
- Automatic deletion of holdings that are missing from an import.
- AI-based column inference.
- A complete redesign of every portfolio screen.

## User Workflow

### 1. Upload And Account Context

The user starts an import from Holdings or a dedicated Import view. They choose a CSV or XLSX file, then provide import context:

- `accountType`: TFSA, RRSP, FHSA, Taxable, Cash, Crypto, or Other.
- `accountName`: a named account such as "Wealthsimple TFSA" or "IBKR Margin".
- Optional `source`: broker/source label such as Wealthsimple, Questrade, TD, IBKR, generic CSV.

The account context applies to every imported row by default. If the file contains account columns and those columns map confidently, the preview can show that the file is overriding or refining the chosen context.

### 2. Column Inference

The backend reads file headers and maps source columns to canonical fields. The first version should use deterministic rules:

- Exact canonical header match.
- Alias match from a static registry.
- Source-specific broker profile match.
- User-selected mapping from the review UI.
- Derived mapping, such as `cost_basis = book_value / quantity`.

The alias registry starts as a curated static module. It can grow over time as real exports are encountered. The registry should group aliases by canonical field and optionally by source profile.

Initial canonical holding fields:

- `symbol`
- `name`
- `quantity`
- `currency`
- `market_value`
- `book_value`
- `average_cost`
- `asset_type`
- `exchange`
- `target_weight`
- `cash_balance`

Transaction reconstruction is deliberately excluded from this design. If an export contains transaction-like columns, the first version should ignore them unless they are needed to infer current holdings.

Representative aliases:

- `symbol`: symbol, ticker, security, instrument, product, security id.
- `name`: name, description, security name, investment name, holding name.
- `quantity`: quantity, shares, units, qty, position quantity.
- `currency`: currency, ccy, settlement currency.
- `market_value`: market value, current value, value, market val.
- `book_value`: book value, cost, total cost, cost basis, adjusted cost base, acb.
- `average_cost`: average cost, avg cost, book cost per share, cost per unit.
- `asset_type`: asset type, type, security type, category.
- `exchange`: exchange, market, listing exchange.
- `cash_balance`: cash, cash balance, balance.

Unknown columns are preserved in preview metadata but ignored by default.

### 3. Row Normalization

The importer converts source rows into normalized holding candidates. It should infer missing fields when confidence is high:

- `cost_basis` from `average_cost` if provided.
- `cost_basis` from `book_value / quantity` when quantity is positive and per-unit cost is absent.
- `asset_type` from explicit column, symbol pattern, cash pattern, or symbol validation.
- `currency`, `name`, and `exchange` from symbol validation when missing.
- Cash holdings from cash-like rows or currency balances.
- Account assignment from import context.

The importer must not silently guess critical low-confidence values. Rows with missing or ambiguous symbol, quantity, currency, or cost basis become `needs_fix`.

### 4. Import Plan Preview

The preview should be an import plan, not just a parsed table. Each row receives an action and supporting metadata:

- `create`: add a new holding.
- `update`: update an existing holding in the selected account.
- `skip`: duplicate or no-op row.
- `needs_fix`: cannot commit until the user resolves a required issue.
- `warning`: can commit, but includes inferred or suspicious data.

Preview rows should show:

- Source row number.
- Original symbol and resolved symbol.
- Name.
- Account.
- Asset type.
- Currency.
- Quantity.
- Cost basis and derivation reason.
- Proposed action.
- Warnings and errors.

The user can commit clean rows while leaving `needs_fix` rows unimported. The first version does not need inline editing for every field, but it should identify exactly what is blocking a row.

### 5. Commit And Post-Import Insights

After commit, the app returns an import summary and a concise insight panel:

- Created holdings.
- Updated holdings.
- Skipped rows.
- Rows needing attention.
- New positions.
- Changed quantities.
- Account holdings missing from the imported file, shown as review candidates only.
- Biggest allocation changes.
- Drift from target weights after import.
- Stale or unpriced symbols.
- Cash balance changes.

This panel should answer, "What changed and what should I look at?" The import flow becomes a power-user review workflow rather than a one-way data entry tool.

## Backend Design

Add a new import pipeline rather than overloading the existing `parse_import_rows` path:

```text
read file
  -> detect format
  -> extract sheet/table
  -> infer columns
  -> normalize rows
  -> build import plan
  -> commit selected clean rows
  -> return summary and insights
```

Core types:

- `ImportContext`: account type, account name/account id, source profile, and user mapping overrides.
- `ColumnMapping`: source header to canonical field, confidence, and reason.
- `NormalizedImportRow`: canonical interpretation of a source row, inferred values, raw values, warnings, and errors.
- `ImportPlan`: preview payload containing mappings, row actions, counts, and unresolved issues.
- `ImportCommitRequest`: selected row identifiers or a "commit clean rows" mode.
- `ImportCommitResult`: created/updated/skipped rows plus insight deltas.

The existing CSV importer can remain temporarily as a compatibility path, but new UI work should target the new pipeline.

## Frontend Design

Replace the current single import modal with a wizard-style import surface:

1. Upload file and choose account context.
2. Review inferred column mappings.
3. Review import plan.
4. Commit and review post-import insights.

Use a dedicated import route rather than a larger modal. The mapping table, preview table, row filters, and post-import summary are dense enough that the workflow should not be cramped inside the existing holdings modal.

The UI should use compact, power-user-oriented controls: file picker, account selectors, mapping table, status badges, filters for row actions, and a clear "commit clean rows" action.

## Testing

Backend tests should cover:

- Header alias matching.
- Source-specific mappings.
- CSV delimiter detection.
- XLSX extraction if XLSX support is included in the first implementation.
- Cost basis derivation from book value and quantity.
- Cash row detection.
- Account context application.
- Duplicate/no-op/update/create decisions.
- Rows that must become `needs_fix`.

Frontend tests should cover:

- Wizard step transitions.
- Account context submission.
- Mapping review rendering.
- Preview status counts.
- Commit clean rows behavior.
- Post-import summary rendering.

End-to-end tests should cover a representative import file and verify that holdings are created or updated with the selected account context.

## Implementation Notes

The first implementation should prioritize deterministic correctness over breadth. A small set of well-tested aliases and broker profiles is better than a large fuzzy matcher that silently imports bad data.

The first implementation should target both CSV and XLSX. If XLSX parsing creates dependency or packaging problems, the implementation plan may split XLSX into a follow-up, but the designed user workflow includes spreadsheet uploads.

The alias registry should be easy to extend. A static Rust module is acceptable initially; a JSON/TOML registry can follow if broker profiles grow quickly.

Existing holdings should not be overwritten silently. The plan preview should show `update` rows explicitly, and the default "commit clean rows" action may include updates only when the row has no blocking issues and the action is visible in the summary counts.

Initial source profiles should include a generic profile plus seeded aliases for the user's real exports as they become available. Do not invent untested broker-specific mappings beyond common column names unless a sample export validates them.
