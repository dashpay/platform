//! Add the `core_locked_outpoints` table (outpoints a wallet keeps out of
//! coin selection).
//!
//! One row per `(wallet_id, outpoint)`, and the row is the lock: the
//! collateral of a registered masternode, locked when the wallet processes
//! its ProRegTx, or an outpoint locked by hand. Deleting the row unlocks it.
//! Spending a collateral would end its masternode registration, which is why
//! the wallet keeps it out of ordinary sends.
//!
//! A table of its own rather than a `core_utxos` column: a lock can exist
//! before its coin does (the ProRegTx can be processed before the collateral
//! it names arrives, and an outpoint can be locked by hand at any time), and
//! it outlives the coin being spent, so it cannot live on the coin's row.
//! `outpoint` carries the same encoding as `core_utxos.outpoint`.
//!
//! Purely additive: an upgraded database starts with no locks.

pub fn migration() -> String {
    "\
CREATE TABLE core_locked_outpoints (
    wallet_id BLOB NOT NULL,
    outpoint BLOB NOT NULL,
    PRIMARY KEY (wallet_id, outpoint),
    FOREIGN KEY (wallet_id) REFERENCES wallets(wallet_id) ON DELETE CASCADE
);
"
    .to_string()
}
