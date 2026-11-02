//! Stable correlation for idempotent wallet-drain recovery. Legacy blobs stay
//! byte-identical; old rows receive NULL and retain their previous behavior.
pub fn migration() -> String {
    "ALTER TABLE asset_locks ADD COLUMN operation_id BLOB \
     CHECK (operation_id IS NULL OR (typeof(operation_id) = 'blob' AND length(operation_id) = 16)); \
     CREATE UNIQUE INDEX asset_locks_wallet_operation_id \
     ON asset_locks(wallet_id, operation_id) WHERE operation_id IS NOT NULL; \
     CREATE TRIGGER asset_locks_operation_id_immutable \
     BEFORE UPDATE OF operation_id ON asset_locks \
     WHEN OLD.operation_id IS NOT NULL AND NEW.operation_id IS NOT OLD.operation_id \
     BEGIN SELECT RAISE(ABORT, 'asset lock operation ID cannot change'); END;".to_owned()
}
