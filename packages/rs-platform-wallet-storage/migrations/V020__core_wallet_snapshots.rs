//! Complete managed Core state, stored atomically with its projected history.

pub fn migration() -> String {
    "CREATE TABLE core_wallet_snapshots (
        wallet_id BLOB PRIMARY KEY NOT NULL REFERENCES wallets(wallet_id) ON DELETE CASCADE,
        format_version INTEGER NOT NULL,
            layout_marker BLOB NOT NULL,
        snapshot_blob BLOB NOT NULL
    );".to_string()
}
