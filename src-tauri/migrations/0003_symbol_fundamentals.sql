-- Add fundamentals columns to symbol_cache
ALTER TABLE symbol_cache ADD COLUMN sector                  TEXT;
ALTER TABLE symbol_cache ADD COLUMN industry                TEXT;
ALTER TABLE symbol_cache ADD COLUMN country                 TEXT;
ALTER TABLE symbol_cache ADD COLUMN beta                    REAL;
ALTER TABLE symbol_cache ADD COLUMN pe_ratio                REAL;
ALTER TABLE symbol_cache ADD COLUMN dividend_yield          REAL;
ALTER TABLE symbol_cache ADD COLUMN eps                     REAL;
ALTER TABLE symbol_cache ADD COLUMN market_cap              REAL;
ALTER TABLE symbol_cache ADD COLUMN fundamentals_updated_at TEXT;
