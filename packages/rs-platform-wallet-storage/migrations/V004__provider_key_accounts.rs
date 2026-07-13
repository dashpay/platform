//! Pre-derived platform-node keys of the EdDSA provider account (#4113).
//!
//! `account_registrations` deliberately gets no DDL change: its `account_type`
//! CHECK already admits both provider labels, so only the one-to-many that
//! cannot live in a single row lands here. PUBLIC material only.

pub fn migration() -> String {
    "\
CREATE TABLE provider_platform_node_keys (
    wallet_id BLOB NOT NULL,
    account_type TEXT NOT NULL,
    -- The parent PK is six columns wide, so referencing it takes all six.
    -- Provider key accounts carry no index / key-class / DashPay axis, so
    -- these always hold the same sentinels the parent row does.
    account_index INTEGER NOT NULL DEFAULT 0,
    key_class INTEGER NOT NULL DEFAULT 0,
    user_identity_id BLOB NOT NULL DEFAULT (zeroblob(32)),
    friend_identity_id BLOB NOT NULL DEFAULT (zeroblob(32)),
    -- Hardened index within the platform-node pool; `index` is reserved SQL.
    key_index INTEGER NOT NULL,
    public_key BLOB NOT NULL,
    node_id BLOB NOT NULL,
    PRIMARY KEY (wallet_id, account_type, key_index),
    FOREIGN KEY (wallet_id, account_type, account_index, key_class, user_identity_id, friend_identity_id)
        REFERENCES account_registrations(wallet_id, account_type, account_index, key_class, user_identity_id, friend_identity_id)
        ON DELETE CASCADE
);
"
    .to_string()
}
