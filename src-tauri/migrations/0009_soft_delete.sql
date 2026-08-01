-- Add soft-delete columns
ALTER TABLE holdings     ADD COLUMN deleted_at TIMESTAMP NULL DEFAULT NULL;
ALTER TABLE transactions ADD COLUMN deleted_at TIMESTAMP NULL DEFAULT NULL;
ALTER TABLE dividends    ADD COLUMN deleted_at TIMESTAMP NULL DEFAULT NULL;

-- NOTE (#675): as of this migration, holdings are never hard-deleted — deletion
-- goes through this soft-delete column instead (see db.rs delete_holding). This
-- means the `ON DELETE CASCADE` on transactions.holding_id and
-- dividends.holding_id (declared in 0001_initial_schema.sql) is dead: a real
-- DELETE FROM holdings that would trigger the cascade never happens in
-- application code. Soft-deleting a holding does NOT cascade to its
-- transactions/dividends — callers that need cascaded soft-deletes must do so
-- explicitly. The FK constraint itself is left in place (harmless, and would
-- correctly cascade if a hard delete were ever introduced), but it should not
-- be relied upon for cleanup today.
