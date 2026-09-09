//! Add the `tracked_masternodes` table (wallet-independent masternodes the
//! user follows).
//!
//! One row per (network, proTxHash). NOT wallet-scoped on purpose: a
//! tracked masternode belongs to no wallet, survives deleting any single
//! wallet, and is keyed by the network it lives on. `snapshot_json` is the
//! versioned cache of what the wallet layer has learned about the node
//! (its DML entry, Platform identity key hashes, registration details) —
//! PUBLIC material only, re-fetchable, decoded by
//! `platform_wallet::masternode::snapshot_from_json`. Keys a user attaches
//! to a tracked node live in the host's secure storage, never here.

pub fn migration() -> String {
    "CREATE TABLE tracked_masternodes (
        network TEXT NOT NULL CHECK (network IN ('mainnet', 'testnet', 'devnet', 'regtest')),
        pro_tx_hash BLOB NOT NULL CHECK (length(pro_tx_hash) = 32),
        label TEXT,
        added_at INTEGER NOT NULL,
        snapshot_json TEXT NOT NULL,
        PRIMARY KEY (network, pro_tx_hash)
    );"
    .to_string()
}
