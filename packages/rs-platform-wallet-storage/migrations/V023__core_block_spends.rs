//! Add `core_block_spends` (the wallet's coins a processed block spent) and
//! `core_block_spends_migrated` (wallets whose earlier block spends were
//! recovered).
//!
//! The wallet notes a block's spend of a coin only in memory, and drops the
//! note once the block is final. A coin whose only spender is a transaction
//! the wallet keeps no record of (a foreign spend of a coin it claimed) then
//! had no durable trace of the spend, and a later release of the coin handed
//! it back as unspent although a block had spent it. A row here outlives
//! that: the sweep release never sets a coin with a row unspent, and the load
//! never hands one back as unspent. A row goes only when a chain truncation
//! replaces its block, when its block hash no longer matches the header
//! chain at its height, when the host rewinds or resets the wallet's chain
//! data, or with its wallet. Never at a ChainLock.
//!
//! `core_block_spends_migrated` marks a wallet whose block spends from before
//! this table were recovered (a one-time rescan by the host). No row means
//! the recovery is still owed. A reset of the wallet's chain data keeps it; a
//! wallet delete removes it.
//!
//! `outpoint` carries the same encoding as `core_utxos.outpoint`;
//! `block_hash` and `spender_txid` are the 32 raw hash bytes.
//!
//! Idempotent (`IF NOT EXISTS`): a wallet file written by a downstream build
//! that shipped these exact tables and index before this migration existed
//! adopts them in place, rows included. The definitions are that build's,
//! byte for byte, so an adopted file and a fresh one have the same shape.
//!
//! Purely additive: an upgraded database starts with no block spends, and
//! every wallet owes the recovery.

pub fn migration() -> String {
    "\
CREATE TABLE IF NOT EXISTS core_block_spends (
    wallet_id BLOB NOT NULL,
    outpoint BLOB NOT NULL,
    height INTEGER NOT NULL,
    block_hash BLOB NOT NULL,
    spender_txid BLOB NOT NULL,
    PRIMARY KEY (wallet_id, outpoint),
    FOREIGN KEY (wallet_id) REFERENCES wallets(wallet_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_core_block_spends_height
    ON core_block_spends(wallet_id, height);

CREATE TABLE IF NOT EXISTS core_block_spends_migrated (
    wallet_id BLOB NOT NULL PRIMARY KEY,
    migrated_at INTEGER NOT NULL DEFAULT (unixepoch()),
    FOREIGN KEY (wallet_id) REFERENCES wallets(wallet_id) ON DELETE CASCADE
);"
    .to_string()
}
