-- This migration adds CHECK constraints and NOT NULL constraints.
-- SQLite does not support ALTER TABLE ADD CONSTRAINT, so we use
-- table rebuilds for tables where this is safe.

PRAGMA foreign_keys = OFF;

-- Add CHECK constraints to price_alerts (safe: can rebuild)
CREATE TABLE price_alerts_new (
    id         TEXT PRIMARY KEY NOT NULL,
    symbol     TEXT NOT NULL,
    direction  TEXT NOT NULL CHECK (direction IN ('above', 'below')),
    threshold  REAL NOT NULL CHECK (threshold != 0),
    note       TEXT NOT NULL DEFAULT '',
    triggered  INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    currency   TEXT NOT NULL DEFAULT 'USD'
);

INSERT INTO price_alerts_new
    SELECT id, symbol, direction, threshold, note, triggered, created_at, currency
    FROM price_alerts;

DROP TABLE price_alerts;
ALTER TABLE price_alerts_new RENAME TO price_alerts;

CREATE INDEX IF NOT EXISTS idx_price_alerts_symbol ON price_alerts(symbol);

-- Add CHECK constraints to holdings (quantity >= 0, cost_basis >= 0, asset_type enum)
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
    target_weight                    REAL NOT NULL DEFAULT 0,
    indicated_annual_dividend        REAL,
    indicated_annual_dividend_currency TEXT,
    dividend_frequency               TEXT CHECK (
                                         dividend_frequency IS NULL OR
                                         dividend_frequency IN ('monthly', 'quarterly', 'semi-annual', 'annual', 'irregular')
                                     ),
    maturity_date                    TEXT,
    created_at                       TEXT NOT NULL,
    updated_at                       TEXT NOT NULL,
    FOREIGN KEY (account_id) REFERENCES accounts(id) ON DELETE SET NULL
);

INSERT INTO holdings_new
    SELECT id, symbol, name, asset_type, account, account_id, quantity, cost_basis,
           currency, exchange, target_weight, indicated_annual_dividend,
           indicated_annual_dividend_currency, dividend_frequency, maturity_date,
           created_at, updated_at
    FROM holdings;

DROP TABLE holdings;
ALTER TABLE holdings_new RENAME TO holdings;

CREATE INDEX IF NOT EXISTS idx_holdings_symbol ON holdings(symbol);
CREATE INDEX IF NOT EXISTS idx_holdings_account_id ON holdings(account_id);

PRAGMA foreign_keys = ON;
