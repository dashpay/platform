//! Add the `core_locked_outpoints` table (outpoints the wallet keeps out of
//! coin selection — masternode collateral locked by a ProRegTx).
//!
//! key-wallet reports a lock once, when it processes the registration, and
//! expects the store to hand it back on load. The block is not processed
//! again after a restart, so without this table the collateral comes back
//! spendable and a max send can deregister the masternode.
//!
//! No coin row is required behind a lock: key-wallet locks a named
//! collateral before the coin arrives, and a spent coin keeps its lock.
//! Purely additive: an upgraded database gains an empty table.

pub fn migration() -> String {
    "CREATE TABLE core_locked_outpoints (
        wallet_id BLOB NOT NULL,
        outpoint BLOB NOT NULL,
        PRIMARY KEY (wallet_id, outpoint),
        FOREIGN KEY (wallet_id) REFERENCES wallets(wallet_id) ON DELETE CASCADE
    );"
    .to_string()
}
