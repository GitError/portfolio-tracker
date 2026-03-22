-- Recreate accounts table to add 'cash' to the type CHECK constraint.
-- SQLite does not support ALTER TABLE to modify CHECK constraints,
-- so we use the table-rebuild approach.

PRAGMA foreign_keys = OFF;

CREATE TABLE accounts_new (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    type        TEXT NOT NULL DEFAULT 'other'
                CHECK (type IN ('tfsa', 'rrsp', 'fhsa', 'taxable', 'crypto', 'cash', 'other')),
    institution TEXT,
    created_at  TEXT NOT NULL
);

INSERT INTO accounts_new SELECT id, name, type, institution, created_at FROM accounts;

DROP TABLE accounts;

ALTER TABLE accounts_new RENAME TO accounts;

PRAGMA foreign_keys = ON;
