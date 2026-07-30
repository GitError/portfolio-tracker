-- target_weight was REAL NOT NULL DEFAULT 0, so "no target set" and "target
-- explicitly set to 0%" were indistinguishable, causing the rebalance engine
-- to either always skip explicit-zero targets (bug) or always full-sell
-- untouched holdings (regression). Make the column nullable so NULL means
-- "no target set" and 0.0 means "explicitly targeted at 0%".
--
-- Existing rows with target_weight = 0.0 are reset to NULL: prior to this
-- migration every holding the user never touched already defaulted to 0.0,
-- so treating those as "unset" restores the original (pre-regression)
-- behavior for existing data. SQLite has no ALTER COLUMN, so the table is
-- rebuilt.
CREATE TABLE holdings_new (
    id                               TEXT PRIMARY KEY NOT NULL,
    symbol                           TEXT NOT NULL,
    name                             TEXT NOT NULL,
    asset_type                       TEXT NOT NULL CHECK (asset_type IN ('stock', 'etf', 'crypto', 'cash')),
    account                          TEXT NOT NULL DEFAULT 'taxable',
    account_id                       TEXT,
    quantity                         REAL NOT NULL CHECK (quantity >= 0),
    cost_basis                       REAL NOT NULL CHECK (cost_basis >= 0),
    currency                         TEXT NOT NULL,
    exchange                         TEXT NOT NULL DEFAULT '',
    target_weight                    REAL,
    indicated_annual_dividend        REAL,
    indicated_annual_dividend_currency TEXT,
    dividend_frequency               TEXT CHECK (
                                         dividend_frequency IS NULL OR
                                         dividend_frequency IN ('monthly', 'quarterly', 'semi-annual', 'annual', 'irregular')
                                     ),
    maturity_date                    TEXT,
    created_at                       TEXT NOT NULL,
    updated_at                       TEXT NOT NULL,
    deleted_at                       TIMESTAMP NULL DEFAULT NULL,
    FOREIGN KEY (account_id) REFERENCES accounts(id) ON DELETE SET NULL
);

INSERT INTO holdings_new
    SELECT id, symbol, name, asset_type, account, account_id, quantity, cost_basis,
           currency, exchange,
           CASE WHEN target_weight = 0.0 THEN NULL ELSE target_weight END,
           indicated_annual_dividend, indicated_annual_dividend_currency,
           dividend_frequency, maturity_date, created_at, updated_at, deleted_at
    FROM holdings;

DROP TABLE holdings;
ALTER TABLE holdings_new RENAME TO holdings;

CREATE INDEX IF NOT EXISTS idx_holdings_symbol ON holdings(symbol);
CREATE INDEX IF NOT EXISTS idx_holdings_account_id ON holdings(account_id);
