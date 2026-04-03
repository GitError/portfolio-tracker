-- Normalize symbol to uppercase in symbol_cache and price_alerts so plain
-- B-tree indexes can be used instead of UPPER() function calls in WHERE clauses.
UPDATE symbol_cache SET symbol = UPPER(symbol);
UPDATE price_alerts SET symbol = UPPER(symbol);
