//! Add the `core_dismissed_txids` table (the transactions a wallet dropped
//! as dead, kept across restarts).
//!
//! key-wallet remembers the unconfirmed transactions it dropped as dead: an
//! abandoned payment, the loser of an InstantSend-locked conflict. A peer
//! delivering one of them again is ignored. That memory lived only in the
//! process, so after a restart the same transaction was counted again.
//! Each change to it (a dismissal, a forgetting when the transaction is
//! mined or locked, an eviction past the bound) is written here in the same
//! transaction as the removal that caused it, and the load hands the rows
//! back oldest first. `id` keeps that order. `abandoned` tells the user's
//! abandons from swept conflict losers, as key-wallet keeps them in two sets:
//! a transaction in both has a row of each, and each kind has its own bound,
//! so sweeps never push an abandon out.
//!
//! `IF NOT EXISTS`: a database that already has the table keeps it and its
//! rows. One with the table as it was before `abandoned` has it rebuilt, its
//! rows (but for those of a deleted wallet and those past a wallet's newest
//! 4,096) kept and read as abandoned; one with another shape is refused
//! (`migrations::v024`). Otherwise an upgraded database starts with no
//! dismissals.

pub fn migration() -> String {
    "\
CREATE TABLE IF NOT EXISTS core_dismissed_txids (
    id INTEGER PRIMARY KEY,
    wallet_id BLOB NOT NULL,
    txid BLOB NOT NULL,
    abandoned INTEGER NOT NULL DEFAULT 0 CHECK (abandoned IN (0, 1)),
    UNIQUE (wallet_id, txid, abandoned),
    FOREIGN KEY (wallet_id) REFERENCES wallets(wallet_id) ON DELETE CASCADE
);
"
    .to_string()
}
