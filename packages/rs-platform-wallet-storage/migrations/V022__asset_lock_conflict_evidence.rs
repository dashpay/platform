//! Retain operation bindings through conflict and order recovery snapshots.
//! Lifecycle blobs are copied unchanged; revision is an explicit sidecar.
pub fn migration() -> String {
    "CREATE TABLE asset_locks_v22 (
        wallet_id BLOB NOT NULL,
        outpoint BLOB NOT NULL,
        status TEXT NOT NULL CHECK (status IN ('built', 'broadcast', 'is_locked', 'chain_locked', 'consumed', 'recovered_from_chain', 'conflicted')),
        account_index INTEGER NOT NULL,
        identity_index INTEGER NOT NULL,
        amount_duffs INTEGER NOT NULL,
        lifecycle_blob BLOB NOT NULL,
        operation_id BLOB CHECK (operation_id IS NULL OR (typeof(operation_id) = 'blob' AND length(operation_id) = 16)),
        operation_revision INTEGER NOT NULL DEFAULT 0 CHECK (typeof(operation_revision) = 'integer' AND operation_revision >= 0),
        PRIMARY KEY (wallet_id, outpoint),
        FOREIGN KEY (wallet_id) REFERENCES wallets(wallet_id) ON DELETE CASCADE
    );
    INSERT INTO asset_locks_v22 (wallet_id, outpoint, status, account_index, identity_index, amount_duffs, lifecycle_blob, operation_id)
        SELECT wallet_id, outpoint, status, account_index, identity_index, amount_duffs, lifecycle_blob, operation_id FROM asset_locks;
    DROP TABLE asset_locks;
    ALTER TABLE asset_locks_v22 RENAME TO asset_locks;
    CREATE UNIQUE INDEX asset_locks_wallet_operation_id ON asset_locks(wallet_id, operation_id) WHERE operation_id IS NOT NULL;
    CREATE TRIGGER asset_locks_operation_id_immutable BEFORE UPDATE OF operation_id ON asset_locks
        WHEN OLD.operation_id IS NOT NULL AND NEW.operation_id IS NOT OLD.operation_id
        BEGIN SELECT RAISE(ABORT, 'asset lock operation ID cannot change'); END;".to_owned()
}
