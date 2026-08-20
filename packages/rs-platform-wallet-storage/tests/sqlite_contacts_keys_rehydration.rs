#![allow(clippy::field_reassign_with_default)]

//! Pre-keyed rehydration: identities load from SQLite already carrying
//! their own public keys + contact state, with nothing layered on
//! afterwards. store → drop → reopen → `load()` → assert on the
//! `ManagedIdentity` fields directly (TC-1..TC-10 of the #3968 spec).

mod common;

use std::collections::{BTreeMap, HashSet};

use common::{ensure_wallet_meta, fresh_persister, wid};
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::BinaryData;
use dpp::prelude::Identifier;
use platform_wallet::changeset::{
    ContactChangeSet, ContactRequestEntry, IdentityChangeSet, IdentityEntry, IdentityKeyEntry,
    IdentityKeysChangeSet, PersistenceError, PlatformWalletChangeSet, PlatformWalletPersistence,
    ReceivedContactRequestKey, SentContactRequestKey,
};
use platform_wallet::wallet::identity::{ContactRequest, EstablishedContact, IdentityStatus};
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet_storage::WalletStorageError;

fn reopen(path: &std::path::Path) -> platform_wallet_storage::SqlitePersister {
    platform_wallet_storage::SqlitePersister::open(
        platform_wallet_storage::SqlitePersisterConfig::new(path),
    )
    .expect("reopen persister")
}

/// A wallet-owned identity entry — lands in `wallet_identities[w][index]`.
fn wallet_identity_entry(id: Identifier, w: WalletId, index: u32) -> IdentityEntry {
    IdentityEntry {
        id,
        balance: 1_000,
        revision: 1,
        identity_index: Some(index),
        last_updated_balance_block_time: None,
        last_synced_keys_block_time: None,
        dpns_names: Vec::new(),
        contested_dpns_names: Vec::new(),
        status: IdentityStatus::Unknown,
        wallet_id: Some(w),
        dashpay_profile: None,
        dashpay_payments: Default::default(),
        contact_profiles: Default::default(),
        ignored_senders: Default::default(),
    }
}

/// An out-of-wallet identity entry (`identity_index = None`) — lands in
/// `out_of_wallet_identities`.
fn out_of_wallet_entry(id: Identifier, w: WalletId) -> IdentityEntry {
    IdentityEntry {
        identity_index: None,
        ..wallet_identity_entry(id, w, 0)
    }
}

fn id_changeset(entries: impl IntoIterator<Item = IdentityEntry>) -> IdentityChangeSet {
    let mut cs = IdentityChangeSet::default();
    for e in entries {
        cs.identities.insert(e.id, e);
    }
    cs
}

fn key_entry(id: Identifier, key_id: u32, byte: u8, security: SecurityLevel) -> IdentityKeyEntry {
    IdentityKeyEntry {
        identity_id: id,
        key_id,
        public_key: IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: key_id,
            purpose: Purpose::AUTHENTICATION,
            security_level: security,
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

fn keys_changeset(entries: impl IntoIterator<Item = IdentityKeyEntry>) -> IdentityKeysChangeSet {
    let mut cs = IdentityKeysChangeSet::default();
    for e in entries {
        cs.upserts.insert((e.identity_id, e.key_id), e);
    }
    cs
}

fn contact_request(sender: Identifier, recipient: Identifier) -> ContactRequest {
    ContactRequest {
        sender_id: sender,
        recipient_id: recipient,
        sender_key_index: 1,
        recipient_key_index: 2,
        account_reference: 3,
        encrypted_account_label: None,
        encrypted_public_key: vec![9, 9, 9],
        auto_accept_proof: None,
        core_height_created_at: 42,
        created_at: 7,
    }
}

fn established(owner: Identifier, contact: Identifier) -> EstablishedContact {
    EstablishedContact {
        contact_identity_id: contact,
        outgoing_request: contact_request(owner, contact),
        incoming_request: contact_request(contact, owner),
        alias: Some("friend".into()),
        note: Some("met at conf".into()),
        is_hidden: false,
        accepted_accounts: vec![0, 3, 7],
        payment_channel_broken: false,
        contact_account_label: None,
        external_account_reference: None,
    }
}

/// TC-1 — a freshly-loaded identity carries its persisted keys in
/// `public_keys()` immediately, bit-exact, with no sync.
#[test]
fn tc1_identity_keys_populate_public_keys_on_load() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xC1);
    ensure_wallet_meta(&persister, &w);
    let id = Identifier::from([0x44; 32]);
    let e0 = key_entry(id, 0, 0xAA, SecurityLevel::HIGH);
    let e1 = key_entry(id, 1, 0xBB, SecurityLevel::HIGH);
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                identities: Some(id_changeset([wallet_identity_entry(id, w, 0)])),
                identity_keys: Some(keys_changeset([e0.clone(), e1.clone()])),
                ..Default::default()
            },
        )
        .unwrap();
    drop(persister);

    let state = reopen(&path).load().expect("load");
    let managed = &state.wallets[&w].identity_manager.wallet_identities[&w][&0];
    let pks = managed.identity.public_keys();
    assert_eq!(pks.len(), 2);
    assert_eq!(pks.get(&0), Some(&e0.public_key));
    assert_eq!(pks.get(&1), Some(&e1.public_key));
}

/// TC-2 (load-bearing) — the freshly-loaded AUTHENTICATION/CRITICAL key is
/// immediately selectable via the exact `get_first_public_key_matching`
/// predicate the Platform signing path uses, before any sync.
#[test]
fn tc2_loaded_auth_key_is_selectable_for_signing() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xC2);
    ensure_wallet_meta(&persister, &w);
    let id = Identifier::from([0x55; 32]);
    let key = key_entry(id, 0, 0xCC, SecurityLevel::CRITICAL);
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                identities: Some(id_changeset([wallet_identity_entry(id, w, 0)])),
                identity_keys: Some(keys_changeset([key.clone()])),
                ..Default::default()
            },
        )
        .unwrap();
    drop(persister);

    let state = reopen(&path).load().expect("load");
    let managed = &state.wallets[&w].identity_manager.wallet_identities[&w][&0];
    // Exact call shape from wallet/identity/network/dpns.rs:207 and friends.
    let selected = managed
        .identity
        .get_first_public_key_matching(
            Purpose::AUTHENTICATION,
            HashSet::from([SecurityLevel::HIGH, SecurityLevel::CRITICAL]),
            HashSet::from([KeyType::ECDSA_SECP256K1]),
            false,
        )
        .expect("auth signing key selectable immediately after load, no sync");
    assert_eq!(selected.id(), 0);
    assert_eq!(selected, &key.public_key);
}

/// TC-3 — an established contact restores onto the identity, bit-exact.
#[test]
fn tc3_established_contact_restores_onto_identity() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xC3);
    ensure_wallet_meta(&persister, &w);
    let owner = Identifier::from([0x11; 32]);
    let contact = Identifier::from([0x22; 32]);
    let contact_state = established(owner, contact);
    let mut established_map = BTreeMap::new();
    established_map.insert(
        SentContactRequestKey {
            owner_id: owner,
            recipient_id: contact,
        },
        contact_state.clone(),
    );
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                identities: Some(id_changeset([wallet_identity_entry(owner, w, 0)])),
                contacts: Some(ContactChangeSet {
                    established: established_map,
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    drop(persister);

    let state = reopen(&path).load().expect("load");
    let managed = &state.wallets[&w].identity_manager.wallet_identities[&w][&0];
    assert_eq!(managed.dashpay().established_contacts().len(), 1);
    assert_eq!(
        managed.dashpay().established_contacts().get(&contact),
        Some(&contact_state)
    );
    assert!(managed.dashpay().sent_contact_requests().is_empty());
    assert!(managed.dashpay().incoming_contact_requests().is_empty());
}

/// TC-4 — pending sent and incoming requests restore with correct
/// directionality and map keys.
#[test]
fn tc4_sent_and_incoming_requests_restore_directionally() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xC4);
    ensure_wallet_meta(&persister, &w);
    let owner = Identifier::from([0x11; 32]);
    let recipient = Identifier::from([0x22; 32]);
    let sender = Identifier::from([0x33; 32]);

    let mut sent = BTreeMap::new();
    sent.insert(
        SentContactRequestKey {
            owner_id: owner,
            recipient_id: recipient,
        },
        ContactRequestEntry {
            request: contact_request(owner, recipient),
        },
    );
    let mut incoming = BTreeMap::new();
    incoming.insert(
        ReceivedContactRequestKey {
            owner_id: owner,
            sender_id: sender,
        },
        ContactRequestEntry {
            request: contact_request(sender, owner),
        },
    );
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                identities: Some(id_changeset([wallet_identity_entry(owner, w, 0)])),
                contacts: Some(ContactChangeSet {
                    sent_requests: sent,
                    incoming_requests: incoming,
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    drop(persister);

    let state = reopen(&path).load().expect("load");
    let managed = &state.wallets[&w].identity_manager.wallet_identities[&w][&0];
    let s = managed
        .dashpay()
        .sent_contact_requests()
        .get(&recipient)
        .expect("sent restored, keyed by recipient");
    assert_eq!(s.sender_id, owner);
    assert_eq!(s.recipient_id, recipient);
    let i = managed
        .dashpay()
        .incoming_contact_requests()
        .get(&sender)
        .expect("incoming restored, keyed by sender");
    assert_eq!(i.sender_id, sender);
    assert_eq!(i.recipient_id, owner);
    assert!(managed.dashpay().established_contacts().is_empty());
}

/// TC-5 — two identities in one wallet with the same numeric `KeyID` keep
/// disjoint key maps; the group-by must not misattribute.
#[test]
fn tc5_no_cross_identity_key_leakage() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xC5);
    ensure_wallet_meta(&persister, &w);
    let a = Identifier::from([0xA0; 32]);
    let b = Identifier::from([0xB0; 32]);
    let a0 = key_entry(a, 0, 0x0A, SecurityLevel::HIGH);
    let a1 = key_entry(a, 1, 0x1A, SecurityLevel::HIGH);
    let b0 = key_entry(b, 0, 0x0B, SecurityLevel::HIGH); // same KeyID 0, different data
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                identities: Some(id_changeset([
                    wallet_identity_entry(a, w, 0),
                    wallet_identity_entry(b, w, 1),
                ])),
                identity_keys: Some(keys_changeset([a0.clone(), a1.clone(), b0.clone()])),
                ..Default::default()
            },
        )
        .unwrap();
    drop(persister);

    let state = reopen(&path).load().expect("load");
    let wallet = &state.wallets[&w].identity_manager.wallet_identities[&w];
    let managed_a = &wallet[&0];
    let managed_b = &wallet[&1];
    assert_eq!(managed_a.identity.public_keys().len(), 2);
    assert_eq!(managed_b.identity.public_keys().len(), 1);
    assert_eq!(
        managed_a.identity.public_keys().get(&0),
        Some(&a0.public_key)
    );
    assert_eq!(
        managed_b.identity.public_keys().get(&0),
        Some(&b0.public_key)
    );
    assert_ne!(
        managed_a.identity.public_keys().get(&0),
        managed_b.identity.public_keys().get(&0),
        "same KeyID on different identities must not collapse"
    );
}

/// TC-6 — contact state does not leak across identities in one wallet.
#[test]
fn tc6_no_cross_identity_contact_leakage() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xC6);
    ensure_wallet_meta(&persister, &w);
    let a = Identifier::from([0xA0; 32]);
    let b = Identifier::from([0xB0; 32]);
    let c = Identifier::from([0xCC; 32]);
    let d = Identifier::from([0xDD; 32]);

    let mut established_map = BTreeMap::new();
    established_map.insert(
        SentContactRequestKey {
            owner_id: a,
            recipient_id: c,
        },
        established(a, c),
    );
    let mut sent = BTreeMap::new();
    sent.insert(
        SentContactRequestKey {
            owner_id: b,
            recipient_id: d,
        },
        ContactRequestEntry {
            request: contact_request(b, d),
        },
    );
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                identities: Some(id_changeset([
                    wallet_identity_entry(a, w, 0),
                    wallet_identity_entry(b, w, 1),
                ])),
                contacts: Some(ContactChangeSet {
                    established: established_map,
                    sent_requests: sent,
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    drop(persister);

    let state = reopen(&path).load().expect("load");
    let wallet = &state.wallets[&w].identity_manager.wallet_identities[&w];
    let managed_a = &wallet[&0];
    let managed_b = &wallet[&1];
    assert!(managed_a.dashpay().established_contacts().contains_key(&c));
    assert!(managed_a.dashpay().sent_contact_requests().is_empty());
    assert!(managed_b.dashpay().sent_contact_requests().contains_key(&d));
    assert!(managed_b.dashpay().established_contacts().is_empty());
}

/// TC-7 — an identity with zero persisted keys loads fine with an empty
/// key map and its scalar fields intact.
#[test]
fn tc7_identity_with_zero_keys_loads_empty() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xC7);
    ensure_wallet_meta(&persister, &w);
    let id = Identifier::from([0x77; 32]);
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                identities: Some(id_changeset([wallet_identity_entry(id, w, 0)])),
                ..Default::default()
            },
        )
        .unwrap();
    drop(persister);

    let state = reopen(&path).load().expect("load succeeds");
    let managed = &state.wallets[&w].identity_manager.wallet_identities[&w][&0];
    assert!(managed.identity.public_keys().is_empty());
    assert_eq!(managed.identity.balance(), 1_000);
    assert_eq!(managed.identity.revision(), 1);
    assert!(managed.dashpay().established_contacts().is_empty());
}

/// TC-8 — an out-of-wallet identity (no `identity_index`) with zero keys
/// and contacts loads into `out_of_wallet_identities`, empty.
///
/// Note: the SQLite `load_state`/`managed_identity_from_entry` path always
/// fills `wallet_id` with the load scope, so a row read under wallet `w`
/// carries `wallet_id = Some(w)` even in the out-of-wallet bucket (the
/// spec's `wallet_id: None` is unreachable through the wallet-scoped
/// reader). We assert the bucket placement and emptiness — the join's
/// concern — not that field.
#[test]
fn tc8_out_of_wallet_identity_loads_empty() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xC8);
    ensure_wallet_meta(&persister, &w);
    let id = Identifier::from([0x88; 32]);
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                identities: Some(id_changeset([out_of_wallet_entry(id, w)])),
                ..Default::default()
            },
        )
        .unwrap();
    drop(persister);

    let state = reopen(&path).load().expect("load succeeds");
    let im = &state.wallets[&w].identity_manager;
    let managed = im
        .out_of_wallet_identities
        .get(&id)
        .expect("out-of-wallet identity present");
    assert!(managed.identity.public_keys().is_empty());
    assert!(managed.dashpay().established_contacts().is_empty());
    assert!(managed.dashpay().sent_contact_requests().is_empty());
    assert!(managed.dashpay().incoming_contact_requests().is_empty());
}

/// TC-9 — a tombstoned identity's orphaned key/contact rows are the one
/// orphan class recovery mode forgives. Strict refuses them: "the owner is
/// gone" is exactly the state that silently discards live key material, so
/// only an operator who asked for a best-effort load gets the old skip.
#[test]
fn tc9_tombstoned_identity_orphan_rows_load_only_in_recovery() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xC9);
    ensure_wallet_meta(&persister, &w);
    let id = Identifier::from([0x99; 32]);
    let contact = Identifier::from([0xAB; 32]);
    let mut established_map = BTreeMap::new();
    established_map.insert(
        SentContactRequestKey {
            owner_id: id,
            recipient_id: contact,
        },
        established(id, contact),
    );
    // Store identity + key + contact.
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                identities: Some(id_changeset([wallet_identity_entry(id, w, 0)])),
                identity_keys: Some(keys_changeset([key_entry(
                    id,
                    0,
                    0x99,
                    SecurityLevel::HIGH,
                )])),
                contacts: Some(ContactChangeSet {
                    established: established_map,
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    // Tombstone the identity only; its key/contact rows are left behind.
    let mut removed = IdentityChangeSet::default();
    removed.removed.insert(id);
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                identities: Some(removed),
                ..Default::default()
            },
        )
        .unwrap();
    drop(persister);

    let strict = reopen(&path)
        .load()
        .expect_err("orphan rows of a tombstoned owner must abort a strict load");
    let PersistenceError::Backend { source, .. } = strict else {
        panic!("expected a typed backend error, got {strict:?}");
    };
    assert!(
        matches!(
            source.downcast_ref::<WalletStorageError>(),
            Some(WalletStorageError::OrphanedIdentityEntry { .. })
        ),
        "expected OrphanedIdentityEntry, got {source:?}"
    );

    let recovery = platform_wallet_storage::SqlitePersister::open(
        platform_wallet_storage::SqlitePersisterConfig::new(&path)
            .with_load_policy(platform_wallet_storage::LoadPolicy::Recovery),
    )
    .expect("reopen in recovery mode");
    let state = recovery
        .load()
        .expect("recovery must complete the load despite orphan rows");
    assert_eq!(
        recovery
            .last_load_degradation()
            .by_site
            .get(&platform_wallet_storage::LoadSite::TombstonedIdentityOrphan)
            .copied(),
        // One per leftover row: the key row and the established-contact row.
        Some(2),
        "both leftover collections must be counted"
    );
    let im = &state.wallets[&w].identity_manager;
    let present = im
        .wallet_identities
        .values()
        .flat_map(|m| m.values())
        .any(|m| m.identity.id() == id)
        || im.out_of_wallet_identities.contains_key(&id);
    assert!(
        !present,
        "tombstoned identity must be absent from both buckets"
    );
}

/// Several leftover rows of one tombstoned owner in one collection must
/// count several times: every other `LoadSite` counts occurrences, so a
/// rescue operator reading `by_site` would otherwise be unable to tell
/// three lost keys from three lost collections.
#[test]
fn tombstoned_identity_orphan_rows_are_counted_per_row() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xCA);
    ensure_wallet_meta(&persister, &w);
    let id = Identifier::from([0x9A; 32]);
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                identities: Some(id_changeset([wallet_identity_entry(id, w, 0)])),
                identity_keys: Some(keys_changeset([
                    key_entry(id, 0, 0x91, SecurityLevel::HIGH),
                    key_entry(id, 1, 0x92, SecurityLevel::HIGH),
                    key_entry(id, 2, 0x93, SecurityLevel::HIGH),
                ])),
                ..Default::default()
            },
        )
        .unwrap();
    // Tombstone the owner; its three key rows are left behind.
    let mut removed = IdentityChangeSet::default();
    removed.removed.insert(id);
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                identities: Some(removed),
                ..Default::default()
            },
        )
        .unwrap();
    drop(persister);

    let recovery = platform_wallet_storage::SqlitePersister::open(
        platform_wallet_storage::SqlitePersisterConfig::new(&path)
            .with_load_policy(platform_wallet_storage::LoadPolicy::Recovery),
    )
    .expect("reopen in recovery mode");
    recovery
        .load()
        .expect("recovery must complete the load despite orphan rows");
    assert_eq!(
        recovery
            .last_load_degradation()
            .by_site
            .get(&platform_wallet_storage::LoadSite::TombstonedIdentityOrphan)
            .copied(),
        Some(3),
        "each leftover key row must be counted"
    );
}

/// TC-10 — cross-wallet scoping is preserved: two wallets each holding an
/// identity with the same `KeyID` get only their own key.
#[test]
fn tc10_cross_wallet_scoping_preserved() {
    let (persister, _tmp, path) = fresh_persister();
    let wa = wid(0xAA);
    let wb = wid(0xBB);
    ensure_wallet_meta(&persister, &wa);
    ensure_wallet_meta(&persister, &wb);
    let ia = Identifier::from([0x1A; 32]);
    let ib = Identifier::from([0x1B; 32]);
    persister
        .store(
            wa,
            PlatformWalletChangeSet {
                identities: Some(id_changeset([wallet_identity_entry(ia, wa, 0)])),
                identity_keys: Some(keys_changeset([key_entry(
                    ia,
                    0,
                    0xAA,
                    SecurityLevel::HIGH,
                )])),
                ..Default::default()
            },
        )
        .unwrap();
    persister
        .store(
            wb,
            PlatformWalletChangeSet {
                identities: Some(id_changeset([wallet_identity_entry(ib, wb, 0)])),
                identity_keys: Some(keys_changeset([key_entry(
                    ib,
                    0,
                    0xBB,
                    SecurityLevel::HIGH,
                )])),
                ..Default::default()
            },
        )
        .unwrap();
    drop(persister);

    let state = reopen(&path).load().expect("load");
    let managed_a = &state.wallets[&wa].identity_manager.wallet_identities[&wa][&0];
    let managed_b = &state.wallets[&wb].identity_manager.wallet_identities[&wb][&0];
    assert_eq!(managed_a.identity.public_keys().len(), 1);
    assert_eq!(managed_b.identity.public_keys().len(), 1);
    assert_eq!(
        managed_a
            .identity
            .public_keys()
            .get(&0)
            .unwrap()
            .data()
            .as_slice(),
        &[0xAA; 33]
    );
    assert_eq!(
        managed_b
            .identity
            .public_keys()
            .get(&0)
            .unwrap()
            .data()
            .as_slice(),
        &[0xBB; 33]
    );
}
