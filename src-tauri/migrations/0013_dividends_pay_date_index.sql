-- get_annual_dividend_income (db.rs) filters dividends by `d.pay_date >= $1`
-- over the full table on every portfolio snapshot; index the column it scans.
CREATE INDEX IF NOT EXISTS idx_dividends_pay_date ON dividends(pay_date);
