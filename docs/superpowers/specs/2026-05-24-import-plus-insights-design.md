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

- `symbol`: symbol, ticker, security, instrument, product, security id, cusip, isin.
- `name`: name, description, security name, investment name, holding name, security description.
- `quantity`: quantity, shares, units, qty, position quantity, number of shares, units held.
- `currency`: currency, ccy, settlement currency, average cost currency, foreign currency, currency code.
- `market_value`: market value, current value, value, market val, total market value, current market value.
- `book_value`: book value, total cost, cost basis, adjusted cost base, acb, book cost, total book value, book cost value.
- `average_cost`: average cost, avg cost, book cost per share, cost per unit, average book cost, book value per unit, unit cost.
- `asset_type`: asset class, asset type, type, security type, category, investment type, product type.
- `exchange`: exchange, market, listing exchange, market code.
- `cash_balance`: settled cash, trade cash, cash, cash balance, balance, cash position.

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
- Multi-section CSV section splitting (Cash Details / Holding Details / Exchange Rate footer).
- `SYMBOL:COUNTRY` format resolution.

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

**Known limitation:** upgrading to TypeScript 7+ is blocked until `typescript-eslint` adds support ([typescript-eslint#10940](https://github.com/typescript-eslint/typescript-eslint/issues/10940)).

---

## Canadian Brokerage Formats

### Overview

Canadian banks and brokerages export portfolio data in two fundamentally different shapes, and the importer must handle both correctly.

**Holdings snapshot exports** are what the user will most commonly import: a point-in-time table of current positions with columns for symbol, quantity, book value, market value, and currency. All major Canadian brokerages (TD Direct Investing, RBC Direct Investing, BMO InvestorLine, CIBC Investor's Edge, Questrade, Wealthsimple Trade, NBDB, Desjardins Online Brokerage) offer a CSV or spreadsheet download of current holdings. The column names vary by broker but are semantically consistent.

**CRA information return exports** are tax documents that brokerages file with the Canada Revenue Agency. The two most relevant are the T5008 (Statement of Securities Transactions) and the T5 (Statement of Investment Income). The CRA defines an XML schema for these; some brokerages make client-downloadable versions available. T5008 and T5 data can be imported into the app for dividend history enrichment, but they are **not** holdings snapshots — see the section below for the distinction.

### Multi-Section CSV Format (Validated)

Some Canadian broker exports are not flat CSV files — they are multi-section documents with metadata blocks, section headers, and a footer, all mixed into the same file. The parser must detect and handle this structure before column inference runs.

**Validated structure (TD Direct Investing RRSP/TFSA portfolio report):**

```
Line 1:   Portfolio report for {ACCOUNT_TYPE} account # {ACCOUNT_NUM} as of {TIMESTAMP}
Line 2:   (blank)
Line 3:   Cash Details
Line 4:   Currency,Account Type,Settled Cash,Trade Cash
Line 5+:  cash data rows (one per currency)
Line N:   (blank)
Line N+1: (blank)
Line N+2: Holding Details
Line N+3: Asset Class,Sector,Security Description,Symbol,... (43-column header)
Line N+4+: holdings data rows
...
Line M:   (blank)
Line M+1: (blank)
Line M+2: Exchange Rate: 1 CAD = {RATE}USD  1 USD = {RATE}CAD
```

**Sample (real data, values anonymized):**

```
Portfolio report for RRSP account # nnn as of 202n-0n-n0T0n:n8:1n

Cash Details
Currency,Account Type,Settled Cash,Trade Cash
CAD,CASH,89192.24,89192.24

Holding Details
Asset Class,Sector,Security Description,Symbol,Quantity,Average Cost,Average Cost Currency,...
Equity,Information Tech.,APPLE INC,AAPL:US,100,135.5045,USD,13550.45,USD,333.74,USD,...


Exchange Rate: 1 CAD = 0.7126USD  1 USD = 1.4033CAD
```

**Detection heuristic:** If line 1 matches `^Portfolio report for .+ account #` and the file contains `Holding Details` followed by a line beginning with `Asset Class,`, classify as `CanadianBankMultiSection` and apply the structured parser. This format is confirmed for TD Direct Investing RRSP and TFSA accounts.

**Multi-section parser behavior:**

1. Extract account type and account number from line 1 if present; surface as import context suggestions (user can override in the wizard).
2. Parse the Cash Details block. Each row (`Currency, Account Type, Settled Cash, Trade Cash`) becomes a Cash holding candidate using `Settled Cash` as the balance and `Currency` as the denomination (e.g. `CAD-CASH`).
3. Parse the Holding Details block as the primary holdings table.
4. Ignore the Exchange Rate footer for import purposes; optionally surface as context in the post-import insights panel.
5. Blank rows between sections must not be treated as data rows.

**Full 43-column holdings header (validated, TD portfolio report):**

`Asset Class`, `Sector`, `Security Description`, `Symbol`, `Quantity`, `Average Cost`, `Average Cost Currency`, `Total Cost`, `Total Cost Currency`, `Current Price`, `Current Price Currency`, `Market Value`, `Market Value Currency`, `Unrealized Gain/Loss $`, `Unrealized Gain/Loss $ Currency`, `Unrealized Gain/Loss %`, `Account %`, `FX Rate`, `Previous Close`, `Previous Close Currency`, `Maturity Date`, `Days to Maturity`, `Annualized Income`, `Annualized Income Currency`, `Dividend Yield (%)`, `Indicated Annual Dividend`, `Indicated Annual Dividend Currency`, `Dividend Frequency`, `Ex-Dividend Date`, `Settlement Currency`, `Bid Lots`, `Bid`, `Bid Currency`, `Ask`, `Ask Currency`, `Ask Lots`, `Chg $`, `Chg %`, `Open`, `Open Currency`, `Volume`, `Beta`, `P/E Ratio`, `EPS`

**Column mapping for holdings import (all others ignored):**

| Source column | App field | Notes |
|---|---|---|
| `Symbol` | `symbol` | `SYMBOL:COUNTRY` format — resolve with existing country-suffix logic |
| `Security Description` | `name` | Full name; fallback if `Symbol` is ambiguous |
| `Asset Class` | `asset_type` | See asset class mapping table below |
| `Quantity` | `quantity` | Numeric; flag negative as `warning` (short position) |
| `Average Cost` | `cost_basis` | Per-unit cost in `Average Cost Currency`; primary source |
| `Average Cost Currency` | `currency` | Native currency of the holding (e.g. USD for AAPL) |
| `Total Cost` | `book_value` | Total position cost; used only when `Average Cost` is absent |
| `Settlement Currency` | *(cross-check)* | Must match `Average Cost Currency`; flag mismatch as `warning` |
| `Dividend Yield (%)` | insight metadata | Not imported; surface in post-import insights only |
| `Annualized Income` | insight metadata | Not imported; surface in post-import insights only |
| `Ex-Dividend Date` | insight metadata | Not imported; surface in post-import insights only |

**Important currency note:** `Market Value` in this format is expressed in the account's base currency (CAD) after FX conversion — **not** the holding's native currency. Do not use `Market Value` for cost basis derivation. Use `Average Cost` × `Quantity` to reconstruct total cost when needed, both in the holding's native currency.

**Asset Class to app asset_type mapping (validated):**

| Source `Asset Class` | App `asset_type` |
|---|---|
| Equity | Stock |
| ETF | ETF |
| Mutual Fund | ETF |
| Fixed Income | Other (flag as `warning` — no live price via Yahoo Finance) |
| GIC | Other (flag as `warning`) |
| Money Market | Other (flag as `warning`) |
| Cash | Cash |
| Crypto | Crypto |
| (blank) | `needs_fix` |

**Symbol format — `SYMBOL:COUNTRY` resolution:**

The `Symbol` column in this format uses the `SYMBOL:COUNTRY` notation the app already supports for manual entry. The same resolution logic applies:

- `:US` → no suffix (e.g. `AAPL:US` → `AAPL`)
- `:CA` → `.TO` (e.g. `TD:CA` → `TD.TO`)
- `:GB` → `.L`
- `:DE` → `.DE`
- Other country codes → attempt resolution; flag as `warning` if Yahoo Finance lookup fails

### Canadian Bank Holdings CSV Column Reference

This table documents validated and expected column names from real Canadian brokerage exports. The alias registry above should prefer the validated names.

| Canonical app field | Validated (TD) | Expected elsewhere |
|---|---|---|
| `symbol` | `Symbol` | Ticker, Security, CUSIP, ISIN, Security ID |
| `name` | `Security Description` | Description, Security Name, Investment Name |
| `quantity` | `Quantity` | Shares, Units, Qty, # Units, Number of Shares |
| `currency` | `Average Cost Currency`, `Settlement Currency` | Currency, CCY, Denomination |
| `book_value` | `Total Cost` | Book Value, Adjusted Cost Base, ACB, Book Cost, Total Book Value |
| `market_value` | `Market Value` | Current Value, Total Market Value |
| `average_cost` | `Average Cost` | Avg Cost, Book Cost Per Share, Average Book Cost, Cost Per Unit |
| `asset_type` | `Asset Class` | Asset Type, Type, Security Type, Category, Investment Type |
| `cash_balance` | `Settled Cash` (Cash Details section) | Trade Cash, Cash Balance, Cash Position |
| `account_type` | extracted from line 1 header | Account Type, Registration Type |
| `account_number` | extracted from line 1 header | Account #, Account Number |

**Canadian-specific normalization rules:**

- `Average Cost Currency` is the native currency of the holding. Use it as `currency`, not `Market Value Currency` (which is the base/display currency after FX).
- `Average Cost` is per-unit and directly maps to `cost_basis`. Do not divide.
- `Total Cost` is the total position book value. Derive per-unit cost as `Total Cost / Quantity` only when `Average Cost` is absent.
- Cash rows in the Cash Details section have no symbol. Generate the symbol as `{CURRENCY}-CASH` (e.g. `CAD-CASH`) and set `cost_basis = 1.0`, `currency = {CURRENCY}`.
- Rows with `Asset Class` of "Fixed Income", "GIC", or "Money Market" can be imported as `Other` type holdings with a `warning` that live pricing may not be available.
- Negative quantity rows represent short positions. Flag as `warning`; the app does not model short positions but can store the row with a note.

### Broker Profiles

Profiles are applied based on file detection. Only add a profile when a real export has been validated against it.

**`CanadianBankMultiSection`** — *validated against TD Direct Investing RRSP/TFSA*

Triggered by the line 1 pattern `^Portfolio report for .+ account #`. Uses the structured multi-section parser. Symbol format is `SYMBOL:COUNTRY`. Cost basis comes directly from `Average Cost`. Cash from the Cash Details section.

**`GenericCanadianCSV`** — *applies when no specific profile matches but headers suggest a Canadian broker*

Flat CSV, no section headers. Uses the full alias registry. Attempts `SYMBOL:COUNTRY` resolution when `:` appears in the symbol column. Derives cost basis from `Average Cost` if present, then from `Total Cost / Quantity`.

**`Questrade`** — *to be validated against a real export*

Suspected columns: `Symbol`, `Description`, `Open Qty`, `Current Price`, `Book Cost`, `Market Value`, `Currency`. Do not activate until confirmed.

**`Wealthsimple`** — *to be validated against a real export*

Suspected columns: `Symbol`, `Name`, `Shares`, `Average Buy Price`, `Market Value`, `Currency`. Do not activate until confirmed.

Do not invent profiles. Promote a broker's guessed profile to active only after a real sample export validates the column names.

### CRA T5008 — Statement of Securities Transactions

**Reference:** CRA 2026V4 schema, updated 2026-01-30  
**URL:** https://www.canada.ca/en/revenue-agency/services/e-services/filing-information-returns-electronically-t4-t5-other-types-returns-overview/t619-2026/t5008-2026.html

T5008 records securities **dispositions** (sales, redemptions, exchanges) for a tax year — not current holdings. A brokerage files one T5008 slip per client per disposition event or per tax year per security.

**What T5008 is useful for in this app:**

T5008 cannot be used to directly create holdings. Its value is:

1. The CRA field names are the canonical reference for what Canadian brokerages call their columns.
2. A future "transaction history" feature could parse T5008 records to reconstruct realized gain/loss history.
3. CUSIP and ISIN numbers can resolve ambiguous symbols in holdings CSVs.

**T5008 slip field mapping:**

| App field | T5008 XML field | Box | Notes |
|---|---|---|---|
| `name` | `id_scty_dsps_txt` | 17 | Security description, up to 60 chars |
| `asset_type` | `dsps_scty_tcd` | 15 | 3-char type code; see codes below |
| `quantity` | `dsps_scty_cnt` | 16 | Units disposed; up to 4 decimal places |
| `cusip_isin` | `dsps_cusip_nbr` | 18 | CUSIP (9 chars) or ISIN (12 chars) |
| `cusip_isin_type` | `dsps_cusip_cd` | 18 | 1=none, 2=CUSIP, 3=ISIN |
| `currency` | `fgn_crcy_cd` | 13 | ISO 4217; CAD for domestic |
| `book_value` | `cost_bok_val_amt` | 20 | Total cost or book value (not per-unit) |
| `proceeds` | `dispn_amt` | 21 | Proceeds of disposition |
| `disposition_date` | `DISPN_DT/dy` + `mo` | 14 | Day + month; year from `tx_yr` in summary |
| `account_number` | `rcpnt_acct_nbr` | — | Broker-assigned account number |
| `face_value` | `fval_amt` | 19 | Face amount (bonds/debentures) |

**T5008 security type codes:**

| Code | Description | App `asset_type` |
|---|---|---|
| `SHR` | Shares (equities) | Stock |
| `UNIT` | Units (ETFs, trusts, REITs) | ETF |
| `MUT` | Mutual fund units | ETF |
| `BON` | Bonds or debentures | Other |
| `OPT` | Options | (skip) |
| `FUT` | Futures | (skip) |
| `OTH` | Other | Other |

**T5008 XML structure summary:**

```xml
<Return>
  <T5008>
    <T5008Slip>
      <disp_record>
        <DISPN_DT><dy/><mo/></DISPN_DT>
        <T5008_AMT>
          <fval_amt/>          <!-- box 19: face amount -->
          <cost_bok_val_amt/>  <!-- box 20: cost or book value -->
          <dispn_amt/>         <!-- box 21: proceeds -->
        </T5008_AMT>
        <dsps_scty_tcd/>       <!-- box 15: security type code -->
        <dsps_scty_cnt/>       <!-- box 16: quantity -->
        <dsps_cusip_nbr/>      <!-- box 18: CUSIP or ISIN -->
        <dsps_cusip_cd/>       <!-- box 18: CUSIP/ISIN indicator -->
        <id_scty_dsps_txt/>    <!-- box 17: security description -->
        <fgn_crcy_cd/>         <!-- box 13: currency -->
      </disp_record>
      <ident_record>
        <rcpnt_acct_nbr/>      <!-- broker account number -->
        <dispn_trans_cnt/>     <!-- number of transactions -->
        <sttl_amt/>            <!-- total proceeds for recipient -->
      </ident_record>
    </T5008Slip>
    <T5008Summary>
      <tx_yr/>                 <!-- taxation year -->
      <tot_sttl_amt/>          <!-- total proceeds -->
      <slp_cnt/>               <!-- slip count -->
    </T5008Summary>
  </T5008>
</Return>
```

### CRA T5 — Statement of Investment Income

**Reference:** CRA 2026V4 schema, updated 2026-01-30  
**URL:** https://www.canada.ca/en/revenue-agency/services/e-services/filing-information-returns-electronically-t4-t5-other-types-returns-overview/t619-2026/t5-2026.html

T5 records investment income for a tax year: dividends, interest, foreign income, royalties, capital gains dividends. One slip per client per issuer per year.

**T5 slip field mapping:**

| App dividend field | T5 XML field | Box | Notes |
|---|---|---|---|
| `amount` (non-eligible dividend) | `actl_dvnd_amt` | 10 | Small-corp / non-eligible dividends |
| `amount` (eligible dividend) | `actl_elg_dvamt` | 24 | Large public corp eligible dividends |
| `amount` (interest) | `cdn_int_amt` | 13 | Interest from Canadian sources |
| `foreign_income` | `fgn_incamt` | 15 | Foreign dividends and income |
| `withholding_tax` | `fgn_tx_pay_amt` | 16 | Foreign tax paid |
| `currency` | `fgn_crcy_ind` | 27 | ISO 4217; CAD if domestic |
| `account_number` | `rcpnt_fi_acct_nbr` | 29 | Broker account number |

**Income type mapping:**

| Box | Field | App interpretation |
|---|---|---|
| 10 | `actl_dvnd_amt` | Dividend (non-eligible) |
| 24 | `actl_elg_dvamt` | Dividend (eligible) |
| 13 | `cdn_int_amt` | Interest |
| 14 | `oth_cdn_incamt` | Other Canadian income |
| 15 | `fgn_incamt` | Foreign income / dividend |
| 17 | `cdn_royl_amt` | Royalty |
| 18 | `cgain_dvnd_amt` | Capital gains dividend (post 2024-06-25) |

Note: T5 does not carry a per-security symbol — it summarizes income from a single payer for the year. On T5 import, prompt the user to associate the record with a holding or log it as unattributed.

### Dividend Import from CRA Tax Slips

When the user imports a T5 document for dividend history:

1. Parse income amounts and currency from the T5 slip fields.
2. Prompt the user to associate the record with a holding (no automatic attribution).
3. Map income type to the app's `dividend_type` field using the table above.
4. Use `tx_yr` from the T5 summary as the approximate year; exact ex-date and pay-date are not available from T5 alone.
5. `fgn_tx_pay_amt` (box 16) maps to a withholding tax annotation on the dividend record.

This flow is optional in the first implementation. If T5/T5008 XML parsing adds meaningful complexity, defer it. The multi-section CSV parser and alias registry are the higher-value first deliverable.
