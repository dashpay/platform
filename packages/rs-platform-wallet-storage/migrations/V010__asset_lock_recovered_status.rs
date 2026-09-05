//! Widen the `asset_locks.status` CHECK domain with
//! `recovered_from_chain` — the status the restore-scan reconstruction
//! assigns to asset locks rebuilt from chain-locked on-chain records
//! (Core finality proven, Platform-side consumption unknown).
//!
//! SQLite cannot alter a CHECK constraint in place, so this rebuilds
//! the table: create the widened twin, copy every row, drop the old
//! table, rename. `asset_locks` is a leaf table (it references
//! `wallets`; nothing references it), so the drop/rename is
//! safe under `PRAGMA foreign_keys = ON`, and the copied rows satisfy
//! the re-declared FK because they came from a table with the same
//! constraint.
//!
//! The status list below is FROZEN — like V001's, it must never track
//! the live `ASSET_LOCK_STATUS_LABELS` const, or a future variant
//! addition would rewrite this migration's generated SQL and break its
//! Refinery checksum on databases that already applied it. The
//! `asset_lock_status_labels_frozen_in_latest_migration` unit test in
//! `sqlite::schema::asset_locks` pins the live const to this list so a
//! new variant fails compilation of intent loudly: append V012+ with
//! another rebuild, never edit this file.

pub fn migration() -> String {
    "\
CREATE TABLE asset_locks_v4 (
    wallet_id BLOB NOT NULL,
    outpoint BLOB NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('built', 'broadcast', 'is_locked', 'chain_locked', 'consumed', 'recovered_from_chain')),
    account_index INTEGER NOT NULL,
    identity_index INTEGER NOT NULL,
    amount_duffs INTEGER NOT NULL,
    lifecycle_blob BLOB NOT NULL,
    PRIMARY KEY (wallet_id, outpoint),
    FOREIGN KEY (wallet_id) REFERENCES wallets(wallet_id) ON DELETE CASCADE
);

-- Orphan policy: a row whose wallet was deleted while FK enforcement
-- happened to be off is unreachable garbage (every read path keys
-- through wallets), but copying it into the FK-declared twin
-- under PRAGMA foreign_keys = ON would abort this whole migration with
-- 'FOREIGN KEY constraint failed'. Drop such rows explicitly — the
-- same outcome the declared ON DELETE CASCADE would have produced had
-- enforcement been on when the wallet was deleted.
DELETE FROM asset_locks
    WHERE wallet_id NOT IN (SELECT wallet_id FROM wallets);

INSERT INTO asset_locks_v4 (wallet_id, outpoint, status, account_index, identity_index, amount_duffs, lifecycle_blob)
    SELECT wallet_id, outpoint, status, account_index, identity_index, amount_duffs, lifecycle_blob FROM asset_locks;

DROP TABLE asset_locks;

ALTER TABLE asset_locks_v4 RENAME TO asset_locks;"
        .to_string()
}
