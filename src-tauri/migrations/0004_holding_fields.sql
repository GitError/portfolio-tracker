-- Add indicated annual dividend, dividend frequency, and maturity date to holdings
ALTER TABLE holdings ADD COLUMN indicated_annual_dividend          REAL;
ALTER TABLE holdings ADD COLUMN indicated_annual_dividend_currency TEXT;
ALTER TABLE holdings ADD COLUMN dividend_frequency                 TEXT
    CHECK (dividend_frequency IS NULL OR dividend_frequency IN
           ('monthly','quarterly','semi-annual','annual','irregular'));
ALTER TABLE holdings ADD COLUMN maturity_date TEXT;
