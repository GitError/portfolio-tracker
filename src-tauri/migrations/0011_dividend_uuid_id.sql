-- Migrate dividends.id from an auto-incrementing integer to a UUID string,
-- matching every other entity (holdings, transactions, price_alerts, accounts).
-- SQLite has no ALTER COLUMN TYPE, so the table is rebuilt.
CREATE TABLE dividends_new (
    id              TEXT PRIMARY KEY,
    holding_id      TEXT NOT NULL REFERENCES holdings (id) ON DELETE CASCADE,
    amount_per_unit REAL NOT NULL,
    currency        TEXT NOT NULL,
    ex_date         TEXT NOT NULL,
    pay_date        TEXT NOT NULL,
    created_at      TEXT NOT NULL,
    deleted_at      TIMESTAMP NULL DEFAULT NULL
);

INSERT INTO dividends_new (id, holding_id, amount_per_unit, currency, ex_date, pay_date, created_at, deleted_at)
SELECT
    lower(
        hex(randomblob(4)) || '-' ||
        hex(randomblob(2)) || '-4' ||
        substr(hex(randomblob(2)), 2) || '-' ||
        substr('89ab', abs(random()) % 4 + 1, 1) ||
        substr(hex(randomblob(2)), 2) || '-' ||
        hex(randomblob(6))
    ),
    holding_id, amount_per_unit, currency, ex_date, pay_date, created_at, deleted_at
FROM dividends;

DROP TABLE dividends;
ALTER TABLE dividends_new RENAME TO dividends;

CREATE INDEX IF NOT EXISTS idx_dividends_holding_id
    ON dividends (holding_id);
