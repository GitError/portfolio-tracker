-- Research watchlist and market-analysis tab (#769).
-- Named watchlists hold research items (symbol + thesis/catalysts/risks/entry
-- range); each item's last-fetched market data snapshot is cached in its own
-- table so staleness/cooldown logic can reason about `retrieved_at`
-- independently of the 24h symbol_cache fundamentals TTL used elsewhere.

CREATE TABLE watchlists (
    id         TEXT PRIMARY KEY NOT NULL,
    name       TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE watchlist_items (
    id               TEXT PRIMARY KEY NOT NULL,
    watchlist_id     TEXT NOT NULL REFERENCES watchlists (id) ON DELETE CASCADE,
    symbol           TEXT NOT NULL,
    currency         TEXT NOT NULL,
    thesis           TEXT,
    catalysts        TEXT,
    risks            TEXT,
    entry_price_low  REAL,
    entry_price_high REAL,
    created_at       TEXT NOT NULL,
    updated_at       TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_watchlist_items_watchlist_id ON watchlist_items(watchlist_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_watchlist_items_unique_symbol ON watchlist_items(watchlist_id, symbol);

-- One-to-one with watchlist_items (PK = FK): a missing row means "never
-- fetched yet", distinct from a row with `error` set (last fetch failed).
CREATE TABLE watchlist_item_snapshots (
    watchlist_item_id  TEXT PRIMARY KEY NOT NULL REFERENCES watchlist_items (id) ON DELETE CASCADE,
    name                 TEXT,
    price               REAL,
    currency             TEXT,
    market_cap          REAL,
    fifty_two_week_low  REAL,
    fifty_two_week_high REAL,
    ytd_return           REAL,
    one_year_return      REAL,
    dividend_yield       REAL,
    pe_ratio             REAL,
    retrieved_at         TEXT NOT NULL,
    error                TEXT
);
