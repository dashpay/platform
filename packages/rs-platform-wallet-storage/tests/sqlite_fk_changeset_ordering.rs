#![allow(clippy::field_reassign_with_default)]

//! FK parent-before-child ordering inside a single immediate-FK
//! transaction, exercised through the production `store()` -> flush path.
//! Two contracts hold:
//!
//! 1. A child whose FK parent is neither in the same payload nor on disk
//!    aborts the flush with a `Constraint`-kind `PersistenceError` and
//!    wipes the buffer (non-transient => no retry): the caller must
//!    include the parent in the same `store()` or write it first.
//! 2. A changeset carrying parent and child together commits — the fixed
//!    dispatch order writes the parent first for every FK edge.

mod common;

use common::{ensure_wallet_meta, fresh_persister, fresh_persister_with_mode, wid};

use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::BinaryData;
use dpp::prelude::Identifier;
use platform_wallet::changeset::{
    IdentityChangeSet, IdentityEntry, IdentityKeyEntry, IdentityKeysChangeSet,
    PersistenceErrorKind, PlatformWalletChangeSet, PlatformWalletPersistence,
};
use platform_wallet::wallet::identity::IdentityStatus;
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet_storage::sqlite::schema::identity_keys;
use platform_wallet_storage::FlushMode;

fn key_entry(identity: Identifier, key_id: u32, byte: u8) -> IdentityKeyEntry {
    IdentityKeyEntry {
        identity_id: identity,
        key_id,
        public_key: IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: key_id,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: BinaryData::new(vec![byte; 33]),
            disabled_at: None,
        }),
        public_key_hash: [byte; 20],
        wallet_id: None,
        derivation_indices: None,
    }
}

fn identity_entry(id: Identifier, wallet_id: Option<WalletId>) -> IdentityEntry {
    IdentityEntry {
        id,
        balance: 0,
        revision: 0,
        identity_index: Some(0),
        last_updated_balance_block_time: None,
        last_synced_keys_block_time: None,
        dpns_names: Vec::new(),
        contested_dpns_names: Vec::new(),
        status: IdentityStatus::Unknown,
        wallet_id,
        dashpay_profile: None,
        dashpay_payments: Default::default(),
        contact_profiles: Default::default(),
        ignored_senders: Default::default(),
    }
}

fn keys_changeset(identity: Identifier, key_id: u32, byte: u8) -> IdentityKeysChangeSet {
    let mut keys = IdentityKeysChangeSet::default();
    keys.upserts
        .insert((identity, key_id), key_entry(identity, key_id, byte));
    keys
}

fn identities_changeset(id: Identifier, wallet_id: Option<WalletId>) -> IdentityChangeSet {
    let mut identities = std::collections::BTreeMap::new();
    identities.insert(id, identity_entry(id, wallet_id));
    IdentityChangeSet {
        identities,
        removed: Default::default(),
    }
}

/// A changeset carrying `identity_keys` for an identity whose
/// `identities` parent is absent from both the payload and the DB (the
/// `wallets` parent is present, isolating the failure) aborts the flush
/// with a `Constraint`-kind `PersistenceError` carrying the constraint
/// class — not a panic or a raw-string-only error.
#[test]
fn identity_keys_without_parent_identity_aborts_with_constraint_kind() {
    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0xA1);
    ensure_wallet_meta(&persister, &w); // wallet parent present; identity parent absent
    let orphan_identity = Identifier::from([0x33; 32]);

    let err = persister
        .store(
            w,
            PlatformWalletChangeSet {
                identity_keys: Some(keys_changeset(orphan_identity, 0, 0x11)),
                ..Default::default()
            },
        )
        .expect_err("child-without-parent flush must fail, not silently succeed");

    assert_eq!(
        err.kind(),
        Some(PersistenceErrorKind::Constraint),
        "an immediate-FK abort must surface as a Constraint-kind PersistenceError, got {err:?}"
    );
    // The underlying rusqlite source must be walkable to the real FK
    // violation — the typed wrapper preserves it rather than flattening
    // to a lossy string.
    let source_chain = {
        use std::error::Error;
        let mut s = String::new();
        let mut cur: Option<&dyn Error> = Some(&err);
        while let Some(e) = cur {
            s.push_str(&e.to_string());
            s.push('\n');
            cur = e.source();
        }
        s
    };
    assert!(
        source_chain.contains("FOREIGN KEY"),
        "the FK violation must be reachable via Error::source(), got chain:\n{source_chain}"
    );
}

/// The constraint abort wipes the buffer: a follow-up `flush()` is a
/// clean no-op and nothing reached disk for the orphaned identity.
#[test]
fn constraint_abort_wipes_buffer_no_silent_retry() {
    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0xA2);
    ensure_wallet_meta(&persister, &w);
    let orphan_identity = Identifier::from([0x44; 32]);

    let _ = persister
        .store(
            w,
            PlatformWalletChangeSet {
                identity_keys: Some(keys_changeset(orphan_identity, 0, 0x55)),
                ..Default::default()
            },
        )
        .expect_err("must fail");

    // Buffer wiped: the next flush finds nothing to write and is a no-op.
    PlatformWalletPersistence::flush(&persister, w).expect("post-abort flush is a clean no-op");

    // And nothing was committed for the orphan identity.
    let on_disk =
        identity_keys::load_state(&persister.lock_conn_for_test(), &w).expect("load identity_keys");
    assert!(
        on_disk.upserts.is_empty(),
        "no identity_keys row may have been committed for the orphaned identity"
    );
}

/// The same contract in Manual flush mode: `store` only buffers, the
/// abort surfaces from the explicit `flush`, and the buffer is wiped so
/// the failed write is dropped (not silently re-attempted forever).
#[test]
fn manual_mode_child_without_parent_aborts_on_flush_and_drops_buffer() {
    let (persister, _tmp, _path) = fresh_persister_with_mode(FlushMode::Manual);
    let w = wid(0xA3);
    ensure_wallet_meta(&persister, &w);
    let orphan_identity = Identifier::from([0x66; 32]);

    // Manual mode: store buffers without touching SQL.
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                identity_keys: Some(keys_changeset(orphan_identity, 0, 0x77)),
                ..Default::default()
            },
        )
        .expect("manual-mode store only buffers");

    let err = PlatformWalletPersistence::flush(&persister, w)
        .expect_err("explicit flush must surface the FK abort");
    assert_eq!(
        err.kind(),
        Some(PersistenceErrorKind::Constraint),
        "manual-mode flush abort must also be Constraint-kind, got {err:?}"
    );

    // Buffer wiped on the fatal classification: a second flush is a no-op.
    PlatformWalletPersistence::flush(&persister, w).expect("second flush is a clean no-op");
}

/// The recovery contract: a COMPLETE changeset carrying the `identities`
/// parent AND its `identity_keys` child in the SAME `store()` commits.
/// This proves the fixed dispatch order writes `identities` before
/// `identity_keys` so the immediate FK is satisfied at the child insert
/// — the parent-before-child invariant for the `identity_id` FK edge.
#[test]
fn parent_and_child_in_same_changeset_commits() {
    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0xB1);
    ensure_wallet_meta(&persister, &w);
    let identity = Identifier::from([0x88; 32]);

    persister
        .store(
            w,
            PlatformWalletChangeSet {
                identities: Some(identities_changeset(identity, Some(w))),
                identity_keys: Some(keys_changeset(identity, 0, 0x99)),
                ..Default::default()
            },
        )
        .expect("parent+child in one changeset must commit under the fixed dispatch order");

    let on_disk =
        identity_keys::load_state(&persister.lock_conn_for_test(), &w).expect("load identity_keys");
    assert_eq!(
        on_disk.upserts.len(),
        1,
        "the child identity_keys row must be committed alongside its parent identity"
    );
    assert!(
        on_disk.upserts.contains_key(&(identity, 0)),
        "the committed row must be the one we wrote"
    );
}

/// The same edge from the wallets side: a complete changeset that carries
/// the `wallets` root anchor (via `wallet_metadata`) AND a `wallet_id`-FK
/// child (`identity_keys`, also needing its identity parent) commits in
/// one flush. `wallets` is dispatched first, so the child's
/// `wallet_id -> wallets` FK is satisfied even when the wallets row did
/// not pre-exist.
#[test]
fn wallets_anchor_and_children_in_same_changeset_commits() {
    use key_wallet::Network;
    use platform_wallet::changeset::WalletMetadataEntry;

    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0xB2); // deliberately NOT pre-seeded — the changeset carries it
    let identity = Identifier::from([0xAB; 32]);

    persister
        .store(
            w,
            PlatformWalletChangeSet {
                wallet_metadata: Some(WalletMetadataEntry {
                    network: Network::Testnet,
                    wallet_group_id: [0u8; 32],
                    birth_height: 0,
                }),
                identities: Some(identities_changeset(identity, Some(w))),
                identity_keys: Some(keys_changeset(identity, 0, 0xCD)),
                ..Default::default()
            },
        )
        .expect("wallets anchor + children in one changeset must commit");

    let on_disk =
        identity_keys::load_state(&persister.lock_conn_for_test(), &w).expect("load identity_keys");
    assert_eq!(
        on_disk.upserts.len(),
        1,
        "the wallet_id-FK child must commit because wallets is dispatched first"
    );
}
