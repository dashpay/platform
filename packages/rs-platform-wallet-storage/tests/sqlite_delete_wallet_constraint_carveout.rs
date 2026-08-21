#![allow(clippy::field_reassign_with_default)]

//! Boundary of `delete_wallet`'s pre-flush constraint carve-out.
//!
//! The carve-out exists for one state: pending identity writes that can
//! never be persisted must not make a wallet undeletable. It names the
//! two variants that describe that state
//! (`IdentityIndexConflict` / `WalletlessIdentityIndex`). Every OTHER
//! constraint failure in the drained pre-flush — a native SQLite FK,
//! CHECK, UNIQUE or NOT NULL violation — is a corruption signal that
//! must abort the delete with the buffer intact, not be swallowed at the
//! one moment an operator is removing state.

mod common;

use common::{ensure_identity, ensure_wallet_meta, fresh_persister_with_mode};

use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::BinaryData;
use dpp::prelude::Identifier;
use platform_wallet::changeset::{
    IdentityKeyEntry, IdentityKeysChangeSet, PersistenceErrorKind, PlatformWalletChangeSet,
    PlatformWalletPersistence,
};
use platform_wallet_storage::{FlushMode, WalletStorageError};
use rusqlite::{params, OptionalExtension};

/// A buffered `identity_keys` upsert whose `identity_id` has no
/// `identities` row is an FK violation — nothing to do with
/// `(wallet_id, identity_index)` uniqueness, whether it surfaces raw or
/// wrapped in `IdentityKeyWalletMismatch`. It is `Constraint`-KIND all
/// the same, so a kind-scoped carve-out would swallow it; the delete
/// must instead fail loudly and keep the pending write.
#[test]
fn delete_wallet_aborts_on_a_constraint_failure_outside_the_carve_out() {
    let (p, _tmp, _path) = fresh_persister_with_mode(FlushMode::Manual);
    let w = common::wid(0x5B);
    ensure_wallet_meta(&p, &w);

    // Deliberately NOT seeded into `identities` — the FK target is
    // missing.
    let identity_id = Identifier::from([0xAB; 32]);
    let public_key = IdentityPublicKey::V0(IdentityPublicKeyV0 {
        id: 0,
        purpose: Purpose::AUTHENTICATION,
        security_level: SecurityLevel::HIGH,
        contract_bounds: None,
        key_type: KeyType::ECDSA_SECP256K1,
        read_only: false,
        data: BinaryData::new(vec![2u8; 33]),
        disabled_at: None,
    });
    let entry = IdentityKeyEntry {
        identity_id,
        key_id: 1,
        public_key,
        public_key_hash: [9u8; 20],
        wallet_id: Some(w),
        derivation_indices: None,
    };
    let mut keys = IdentityKeysChangeSet::default();
    keys.upserts.insert((identity_id, 1), entry);
    p.store(
        w,
        PlatformWalletChangeSet {
            identity_keys: Some(keys),
            ..Default::default()
        },
    )
    .expect("Manual mode only buffers — no FK check happens here");

    let err = p
        .delete_wallet_skip_backup(w)
        .expect_err("an FK violation is not the state the carve-out covers");

    assert!(
        !matches!(
            err,
            WalletStorageError::IdentityIndexConflict { .. }
                | WalletStorageError::WalletlessIdentityIndex { .. }
        ),
        "the carve-out names two variants and this FK violation is neither, got `{err:?}`"
    );
    assert_eq!(
        err.persistence_kind(),
        PersistenceErrorKind::Constraint,
        "the kind is Constraint — which is exactly why kind-scoped tolerance was too wide"
    );

    let wallets: i64 = {
        let conn = p.lock_conn_for_test();
        conn.query_row(
            "SELECT COUNT(*) FROM wallets WHERE wallet_id = ?1",
            params![w.as_slice()],
            |row| row.get(0),
        )
        .expect("count wallets")
    };
    assert_eq!(wallets, 1, "the delete aborted — the wallet is still here");

    // The pending write was restored, not dropped: give the FK its
    // target and the same buffered changeset flushes cleanly.
    ensure_identity(&p, &[0xAB; 32], Some(&w));
    p.flush(w)
        .expect("the restored changeset is still flushable");
    let conn = p.lock_conn_for_test();
    let key_row: Option<i64> = conn
        .query_row(
            "SELECT 1 FROM identity_keys WHERE identity_id = ?1",
            params![identity_id.as_slice()],
            |row| row.get(0),
        )
        .optional()
        .expect("query identity_keys");
    assert_eq!(key_row, Some(1), "the buffered write survived the abort");
}
