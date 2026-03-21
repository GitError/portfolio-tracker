-- Initial schema migration
-- Converts the inline DDL from init_db() into versioned SQLx migrations

CREATE TABLE IF NOT EXISTS holdings (
    id            TEXT PRIMARY KEY,
    symbol        TEXT NOT NULL,
    name          TEXT NOT NULL,
    asset_type    TEXT NOT NULL,
    account       TEXT NOT NULL DEFAULT 'taxable',
    account_id    TEXT,
    quantity      REAL NOT NULL,
    cost_basis    REAL NOT NULL,
    currency      TEXT NOT NULL,
    exchange      TEXT NOT NULL DEFAULT '',
    target_weight REAL NOT NULL DEFAULT 0,
    created_at    TEXT NOT NULL,
    updated_at    TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS price_cache (
    symbol         TEXT PRIMARY KEY,
    price          REAL NOT NULL,
    currency       TEXT NOT NULL,
    change         REAL NOT NULL DEFAULT 0,
    change_percent REAL NOT NULL DEFAULT 0,
    updated_at     TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS fx_rates (
    pair       TEXT PRIMARY KEY,
    rate       REAL NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS symbol_cache (
    symbol     TEXT PRIMARY KEY,
    name       TEXT NOT NULL,
    asset_type TEXT NOT NULL,
    exchange   TEXT NOT NULL DEFAULT '',
    currency   TEXT NOT NULL DEFAULT 'USD',
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS app_config (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS portfolio_snapshots (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    total_value REAL NOT NULL,
    total_cost  REAL NOT NULL,
    gain_loss   REAL NOT NULL,
    recorded_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_snapshots_recorded_at
    ON portfolio_snapshots (recorded_at);

CREATE TABLE IF NOT EXISTS price_alerts (
    id        TEXT PRIMARY KEY,
    symbol    TEXT NOT NULL,
    direction TEXT NOT NULL,
    threshold REAL NOT NULL,
    note      TEXT NOT NULL DEFAULT '',
    triggered INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS transactions (
    id               TEXT PRIMARY KEY,
    holding_id       TEXT NOT NULL,
    transaction_type TEXT NOT NULL,
    quantity         REAL NOT NULL,
    price            REAL NOT NULL,
    transacted_at    TEXT NOT NULL,
    created_at       TEXT NOT NULL,
    FOREIGN KEY (holding_id) REFERENCES holdings (id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_transactions_holding_id
    ON transactions (holding_id);

CREATE INDEX IF NOT EXISTS idx_transactions_transacted_at
    ON transactions (transacted_at);

CREATE TABLE IF NOT EXISTS dividends (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    holding_id      TEXT NOT NULL REFERENCES holdings (id) ON DELETE CASCADE,
    amount_per_unit REAL NOT NULL,
    currency        TEXT NOT NULL,
    ex_date         TEXT NOT NULL,
    pay_date        TEXT NOT NULL,
    created_at      TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_dividends_holding_id
    ON dividends (holding_id);

CREATE TABLE IF NOT EXISTS accounts (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    type        TEXT NOT NULL DEFAULT 'other'
                CHECK (type IN ('tfsa', 'rrsp', 'fhsa', 'taxable', 'crypto', 'other')),
    institution TEXT,
    created_at  TEXT NOT NULL
);
