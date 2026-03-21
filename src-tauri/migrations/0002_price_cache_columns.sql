-- Add open price, previous close, and volume to price_cache
ALTER TABLE price_cache ADD COLUMN open           REAL;
ALTER TABLE price_cache ADD COLUMN previous_close REAL;
ALTER TABLE price_cache ADD COLUMN volume         INTEGER;
