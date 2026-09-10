#![allow(clippy::field_reassign_with_default)]

//! Write-path coverage for the `IdentityChangeSet.removed` tombstone
//! branch. The tombstone runs a wallet-scoped, NULL-safe
//! `UPDATE identities SET tombstoned = 1 WHERE identity_id = ?1 AND
//! wallet_id IS ?2`, mirroring the upsert's per-entry wallet cross-check.
//! These tests pin that a tombstoned identity is excluded from the
//! per-wallet `load_state` and that a foreign wallet's `removed` set
//! cannot tombstone this wallet's identity.

mod common;

use std::collections::{BTreeMap, BTreeSet};

use common::{ensure_wallet_meta, fresh_persister, wid};
use dpp::identity::accessors::IdentityGettersV0;
use dpp::prelude::Identifier;
use platform_wallet::changeset::{
    IdentityChangeSet, IdentityEntry, PlatformWalletChangeSet, PlatformWalletPersistence,
};
use platform_wallet::wallet::identity::IdentityStatus;
use platform_wallet_storage::sqlite::schema::identities;

fn reopen(path: &std::path::Path) -> platform_wallet_storage::SqlitePersister {
    platform_wallet_storage::SqlitePersister::open(
        platform_wallet_storage::SqlitePersisterConfig::new(path),
    )
    .expect("reopen persister")
}

/// Build an `IdentityEntry` parented to a specific wallet (so the upsert
/// cross-check passes and the typed `wallet_id` column is populated).
///
/// The derivation slot is keyed off `id` because `(wallet_id,
/// identity_index)` names exactly one identity: two identities of one
/// wallet sharing a slot is the corruption `store` refuses to write.
fn entry_for(id: u8, wallet_id: [u8; 32]) -> IdentityEntry {
    IdentityEntry {
        id: Identifier::from([id; 32]),
        balance: u64::from(id),
        revision: 1,
        identity_index: Some(u32::from(id)),
        last_updated_balance_block_time: None,
        last_synced_keys_block_time: None,
        dpns_names: Vec::new(),
        contested_dpns_names: Vec::new(),
        status: IdentityStatus::Active,
        wallet_id: Some(wallet_id),
        dashpay_profile: None,
        dashpay_payments: Default::default(),
        contact_profiles: Default::default(),
        ignored_senders: Default::default(),
    }
}

/// An identity routed through `IdentityChangeSet.removed` is tombstoned
/// and disappears from the per-wallet `load_state` while a sibling,
/// non-removed identity survives.
#[test]
fn qa_tomb1_removed_identity_excluded_from_load() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xD0);
    ensure_wallet_meta(&persister, &w);

    let keep = entry_for(0x01, w);
    let drop_me = entry_for(0x02, w);
    let mut idents: BTreeMap<Identifier, IdentityEntry> = BTreeMap::new();
    idents.insert(keep.id, keep.clone());
    idents.insert(drop_me.id, drop_me.clone());

    // First flush: insert both.
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                identities: Some(IdentityChangeSet {
                    identities: idents,
                    removed: Default::default(),
                }),
                ..Default::default()
            },
        )
        .unwrap();

    // Second flush: tombstone drop_me.
    let mut removed: BTreeSet<Identifier> = BTreeSet::new();
    removed.insert(drop_me.id);
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                identities: Some(IdentityChangeSet {
                    identities: Default::default(),
                    removed,
                }),
                ..Default::default()
            },
        )
        .unwrap();
    drop(persister);

    let p2 = reopen(&path);
    let conn = p2.lock_conn_for_test();

    // The tombstoned row is still physically present (logical delete).
    let total: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM identities WHERE wallet_id = ?1",
            rusqlite::params![w.as_slice()],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(total, 2, "tombstone is a logical delete; row stays on disk");

    let tombstoned: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM identities WHERE wallet_id = ?1 AND tombstoned = 1",
            rusqlite::params![w.as_slice()],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(tombstoned, 1, "exactly one identity tombstoned");

    // load_state must skip the tombstoned identity and keep the other.
    let state = identities::load_state(&conn, &w).unwrap();
    drop(conn);
    let wallet_idents = state.wallet_identities.get(&w).expect("wallet bucket");
    assert_eq!(
        wallet_idents.len(),
        1,
        "load_state must surface only the non-tombstoned identity"
    );
    let surviving_ids: Vec<Identifier> = wallet_idents.values().map(|m| m.identity.id()).collect();
    assert!(
        surviving_ids.contains(&keep.id),
        "kept identity must survive load"
    );
    assert!(
        !surviving_ids.contains(&drop_me.id),
        "tombstoned identity must NOT appear in load"
    );
}

/// Re-upserting a tombstoned identity clears the tombstone (the upsert
/// sets `tombstoned = 0`) — the resurrection path the writer relies on.
#[test]
fn qa_tomb2_reupsert_clears_tombstone() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xD1);
    ensure_wallet_meta(&persister, &w);

    let e = entry_for(0x05, w);
    let mut idents: BTreeMap<Identifier, IdentityEntry> = BTreeMap::new();
    idents.insert(e.id, e.clone());
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                identities: Some(IdentityChangeSet {
                    identities: idents.clone(),
                    removed: Default::default(),
                }),
                ..Default::default()
            },
        )
        .unwrap();

    let mut removed: BTreeSet<Identifier> = BTreeSet::new();
    removed.insert(e.id);
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                identities: Some(IdentityChangeSet {
                    identities: Default::default(),
                    removed,
                }),
                ..Default::default()
            },
        )
        .unwrap();

    // Re-upsert resurrects.
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                identities: Some(IdentityChangeSet {
                    identities: idents,
                    removed: Default::default(),
                }),
                ..Default::default()
            },
        )
        .unwrap();
    drop(persister);

    let p2 = reopen(&path);
    let conn = p2.lock_conn_for_test();
    let tombstoned: i64 = conn
        .query_row(
            "SELECT tombstoned FROM identities WHERE identity_id = ?1",
            rusqlite::params![e.id.as_slice()],
            |r| r.get(0),
        )
        .unwrap();
    let state = identities::load_state(&conn, &w).unwrap();
    drop(conn);
    assert_eq!(tombstoned, 0, "re-upsert must clear the tombstone flag");
    assert_eq!(
        state
            .wallet_identities
            .get(&w)
            .map(|m| m.len())
            .unwrap_or(0),
        1,
        "resurrected identity must reappear in load"
    );
}

/// The tombstone UPDATE is scoped by `wallet_id`: a `removed` entry
/// naming an identity parented to a different wallet is a no-op against
/// that wallet's row (NULL-safe `wallet_id IS ?2` predicate). An
/// identity_id is globally unique to one wallet, so this is
/// defense-in-depth enforcing the isolation the data model assumes.
#[test]
fn qa_tomb3_tombstone_update_is_wallet_scoped() {
    let (persister, _tmp, path) = fresh_persister();
    let wa = wid(0xE0);
    let wb = wid(0xE1);
    ensure_wallet_meta(&persister, &wa);
    ensure_wallet_meta(&persister, &wb);

    // Identity 0x07 is parented to wallet B.
    let b_ident = entry_for(0x07, wb);
    let mut b_map: BTreeMap<Identifier, IdentityEntry> = BTreeMap::new();
    b_map.insert(b_ident.id, b_ident.clone());
    persister
        .store(
            wb,
            PlatformWalletChangeSet {
                identities: Some(IdentityChangeSet {
                    identities: b_map,
                    removed: Default::default(),
                }),
                ..Default::default()
            },
        )
        .unwrap();

    // Wallet A flushes a `removed` set naming wallet B's identity id.
    let mut removed: BTreeSet<Identifier> = BTreeSet::new();
    removed.insert(b_ident.id);
    persister
        .store(
            wa,
            PlatformWalletChangeSet {
                identities: Some(IdentityChangeSet {
                    identities: Default::default(),
                    removed,
                }),
                ..Default::default()
            },
        )
        .unwrap();
    drop(persister);

    let p2 = reopen(&path);
    let conn = p2.lock_conn_for_test();
    let tombstoned: i64 = conn
        .query_row(
            "SELECT tombstoned FROM identities WHERE identity_id = ?1",
            rusqlite::params![b_ident.id.as_slice()],
            |r| r.get(0),
        )
        .unwrap();
    let b_state = identities::load_state(&conn, &wb).unwrap();
    drop(conn);

    // Cross-wallet isolation: wallet A's `removed` set names wallet B's
    // identity, but the wallet-scoped tombstone UPDATE leaves B's row
    // untouched, so B's load still surfaces the identity.
    assert_eq!(
        tombstoned, 0,
        "wallet-scoped tombstone: A's removed set must NOT affect B's identity"
    );
    assert_eq!(
        b_state
            .wallet_identities
            .get(&wb)
            .map(|m| m.len())
            .unwrap_or(0),
        1,
        "B's identity must survive A's unrelated tombstone"
    );
}
