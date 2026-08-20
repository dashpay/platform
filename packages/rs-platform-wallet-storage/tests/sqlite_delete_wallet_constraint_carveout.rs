#![allow(clippy::field_reassign_with_default)]

//! QA (Marvin) — `delete_wallet`'s pre-flush carve-out
//! (`persister.rs`, `Err(e) if e.persistence_kind() == PersistenceErrorKind::Constraint`)
//! is scoped to the `Constraint` *kind*, not to the two new
//! `IdentityIndexConflict` / `WalletlessIdentityIndex` variants it was
//! introduced for. Any native SQLite `ConstraintViolation` (FK, CHECK,
//! UNIQUE, NOT NULL — see `WalletStorageError::persistence_kind`) in a
//! wallet's drained pre-flush buffer is classified `Constraint` too, so
//! it is ALSO silently dropped-with-a-warn-log and the delete proceeds,
//! not just an identity-index collision. Before this PR any such failure
//! hard-aborted `delete_wallet` (buffer restored, error returned).
//!
//! This test reaches that carve-out via an ordinary FK violation on
//! `identity_keys.identity_id -> identities.identity_id` that has
//! nothing to do with identity-index uniqueness, to show the carve-out's
//! blast radius is broader than the requirement it was written for.

mod common;

use common::{ensure_wallet_meta, fresh_persister_with_mode};

use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::BinaryData;
use dpp::prelude::Identifier;
use platform_wallet::changeset::{
    IdentityKeyEntry, IdentityKeysChangeSet, PlatformWalletChangeSet, PlatformWalletPersistence,
};
use platform_wallet_storage::FlushMode;
use rusqlite::{params, OptionalExtension};

/// A buffered `identity_keys` upsert whose `identity_id` has NO
/// `identities` row (never `ensure_exists`-ed, never stored) is a plain
/// FK violation — nothing to do with `(wallet_id, identity_index)`
/// uniqueness. `delete_wallet`'s Constraint-kind carve-out swallows it
/// exactly like it swallows an identity-index conflict, and the wallet
/// is deleted with NO error surfaced to the caller.
#[test]
fn delete_wallet_silently_drops_an_unrelated_fk_violation_too() {
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

    // If this FK violation instead hard-aborted `delete_wallet` (the
    // pre-PR behavior for ANY apply failure), this `expect` would panic.
    // It doesn't: the carve-out treats it exactly like an identity-index
    // conflict and silently drops it.
    let report = p
        .delete_wallet_skip_backup(w)
        .expect("QA: FK violation unrelated to identity-index is ALSO silently swallowed");

    assert_eq!(report.wallet_id, w);
    let conn = p.lock_conn_for_test();
    let wallets: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM wallet_metadata WHERE wallet_id = ?1",
            params![w.as_slice()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        wallets, 0,
        "the wallet is gone — no trace of the dropped write, no error"
    );
    let key_row: Option<i64> = conn
        .query_row(
            "SELECT 1 FROM identity_keys WHERE identity_id = ?1",
            params![identity_id.as_slice()],
            |row| row.get(0),
        )
        .optional()
        .unwrap();
    assert_eq!(
        key_row, None,
        "the FK-violating row never landed, as expected"
    );
}
