#![allow(clippy::field_reassign_with_default)]

//! Contacts + identity-keys rehydrate through the keyless `load()` path:
//! store → drop → reopen → load → assert the
//! `ClientWalletStartState.contacts` / `.identity_keys` slots carry the
//! persisted PUBLIC material bit-exact.

mod common;

use common::{ensure_wallet_meta, fresh_persister, wid};
use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::BinaryData;
use dpp::prelude::Identifier;
use platform_wallet::changeset::{
    ContactChangeSet, ContactRequestEntry, IdentityKeyEntry, IdentityKeysChangeSet,
    PlatformWalletChangeSet, PlatformWalletPersistence, ReceivedContactRequestKey,
    SentContactRequestKey,
};
use platform_wallet::wallet::identity::ContactRequest;

fn reopen(path: &std::path::Path) -> platform_wallet_storage::SqlitePersister {
    platform_wallet_storage::SqlitePersister::open(
        platform_wallet_storage::SqlitePersisterConfig::new(path),
    )
    .expect("reopen persister")
}

fn req(sender: u8, recipient: u8) -> ContactRequestEntry {
    ContactRequestEntry {
        request: ContactRequest {
            sender_id: Identifier::from([sender; 32]),
            recipient_id: Identifier::from([recipient; 32]),
            sender_key_index: 1,
            recipient_key_index: 2,
            account_reference: 3,
            encrypted_account_label: None,
            encrypted_public_key: vec![9, 9, 9],
            auto_accept_proof: None,
            core_height_created_at: 42,
            created_at: 7,
        },
    }
}

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

/// Contacts (sent + received) rehydrate bit-exact into the keyless
/// `ClientWalletStartState.contacts` slot.
#[test]
fn g_rt1_contacts_rehydrate_into_keyless_payload() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xC0);
    ensure_wallet_meta(&persister, &w);

    let sent_key = SentContactRequestKey {
        owner_id: Identifier::from([0x11; 32]),
        recipient_id: Identifier::from([0x22; 32]),
    };
    let recv_key = ReceivedContactRequestKey {
        owner_id: Identifier::from([0x11; 32]),
        sender_id: Identifier::from([0x33; 32]),
    };
    let sent_entry = req(0x11, 0x22);
    let recv_entry = req(0x33, 0x11);
    let mut sent = std::collections::BTreeMap::new();
    sent.insert(sent_key, sent_entry.clone());
    let mut recv = std::collections::BTreeMap::new();
    recv.insert(recv_key, recv_entry.clone());

    persister
        .store(
            w,
            PlatformWalletChangeSet {
                contacts: Some(ContactChangeSet {
                    sent_requests: sent,
                    incoming_requests: recv,
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    drop(persister);

    let p2 = reopen(&path);
    let state = p2.load().expect("load");
    let slice = state.wallets.get(&w).expect("wallet rehydrated");

    let got_sent = slice
        .contacts
        .sent_requests
        .get(&sent_key)
        .expect("sent request rehydrated");
    assert_eq!(
        got_sent.request.core_height_created_at,
        sent_entry.request.core_height_created_at
    );
    assert_eq!(
        got_sent.request.encrypted_public_key,
        sent_entry.request.encrypted_public_key
    );
    let got_recv = slice
        .contacts
        .incoming_requests
        .get(&recv_key)
        .expect("incoming request rehydrated");
    assert_eq!(got_recv.request.sender_id, recv_entry.request.sender_id);
    // The rehydration feed never carries deletes.
    assert!(slice.contacts.removed_sent.is_empty());
    assert!(slice.contacts.removed_incoming.is_empty());
}

/// Identity-key entries rehydrate bit-exact into the keyless
/// `ClientWalletStartState.identity_keys` slot.
#[test]
fn g_rt2_identity_keys_rehydrate_into_keyless_payload() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xC1);
    ensure_wallet_meta(&persister, &w);
    let id = Identifier::from([0x44; 32]);
    platform_wallet_storage::sqlite::schema::identities::ensure_exists(
        &persister.lock_conn_for_test(),
        &w,
        id.as_slice().try_into().unwrap(),
    )
    .unwrap();

    let e0 = key_entry(id, 0, 0xAA);
    let e1 = key_entry(id, 1, 0xBB);
    let mut keys = IdentityKeysChangeSet::default();
    keys.upserts.insert((id, 0), e0.clone());
    keys.upserts.insert((id, 1), e1.clone());
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                identity_keys: Some(keys),
                ..Default::default()
            },
        )
        .unwrap();
    drop(persister);

    let p2 = reopen(&path);
    let state = p2.load().expect("load");
    let slice = state.wallets.get(&w).expect("wallet rehydrated");
    assert_eq!(slice.identity_keys.upserts.len(), 2);
    assert_eq!(slice.identity_keys.upserts.get(&(id, 0)), Some(&e0));
    assert_eq!(slice.identity_keys.upserts.get(&(id, 1)), Some(&e1));
    assert!(slice.identity_keys.removed.is_empty());
}

/// A metadata-only wallet has empty (not error) contacts /
/// identity-keys slots.
#[test]
fn g_rt3_empty_slots_for_bare_wallet() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xC2);
    ensure_wallet_meta(&persister, &w);
    drop(persister);
    let p2 = reopen(&path);
    let state = p2.load().expect("load");
    let slice = state.wallets.get(&w).expect("wallet present");
    assert!(slice.contacts.sent_requests.is_empty());
    assert!(slice.contacts.incoming_requests.is_empty());
    assert!(slice.contacts.established.is_empty());
    assert!(slice.identity_keys.upserts.is_empty());
}
