CREATE INDEX IF NOT EXISTS idx_holdings_account_id ON holdings(account_id);
CREATE INDEX IF NOT EXISTS idx_holdings_symbol ON holdings(symbol);
CREATE INDEX IF NOT EXISTS idx_transactions_holding_id ON transactions(holding_id);
CREATE INDEX IF NOT EXISTS idx_price_cache_symbol ON price_cache(symbol);
