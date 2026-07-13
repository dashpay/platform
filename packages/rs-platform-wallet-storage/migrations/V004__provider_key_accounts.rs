//! Pre-derived platform-node keys of the EdDSA provider account
//! (dashpay/platform#4113).
//!
//! Additive-only: V001-V003 stay byte-identical so refinery's
//! applied-migration checksums never diverge on an existing store.
//!
//! `account_registrations` needs NO DDL change and deliberately gets none —
//! the provider key-material accounts (`ProviderOperatorKeys` /
//! `ProviderPlatformKeys`) ride its existing columns, whose
//! `CHECK (account_type IN (...))` already admits the `'provider_operator'`
//! and `'provider_platform'` labels (V001, generated from
//! `ACCOUNT_TYPE_LABELS`). Only the one-to-many that has nowhere to live in
//! a single row lands here: the batch of platform-node public keys
//! pre-derived at registration.
//!
//! Ed25519/SLIP-10 is hardened-only, so a watch-only wallet can never
//! re-derive this pool from the account xpub — persisting it verbatim is the
//! only way a restored wallet can list its node keys without re-prompting the
//! user for their recovery phrase. PUBLIC material only (`public_key`,
//! `node_id`); no signing material has a column here.

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
