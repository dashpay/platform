#![allow(clippy::field_reassign_with_default)]

//! `load()` reconstructs the wired-up subset of client start-state.
//!
//! The wallet-level fields (`wallets[*].utxos` / `.unused_asset_locks`)
//! are blocked on upstream `Wallet::from_persisted` — the persister
//! stores the data (verified via direct SQL probes) but cannot
//! reconstruct the `Wallet` + `ManagedWalletInfo` pair that
//! `ClientWalletStartState` requires. The unwired fields are listed in
//! `persister::LOAD_UNIMPLEMENTED` and surfaced via a `tracing::warn!`
//! on every `load`.

mod common;

use common::{ensure_wallet_meta, fresh_persister, wid};
use dash_sdk::platform::address_sync::AddressFunds;
use key_wallet::PlatformP2PKHAddress;
use platform_wallet::changeset::{
    PlatformAddressBalanceEntry, PlatformAddressChangeSet, PlatformWalletChangeSet,
    PlatformWalletPersistence,
};
use platform_wallet_storage::WalletStorageError;

fn entry(
    wallet_id: [u8; 32],
    account_index: u32,
    address_index: u32,
    byte: u8,
) -> PlatformAddressBalanceEntry {
    PlatformAddressBalanceEntry {
        wallet_id,
        account_index,
        address_index,
        address: PlatformP2PKHAddress::new([byte; 20]),
        funds: AddressFunds {
            balance: address_index as u64 * 100,
            nonce: address_index,
            as_of_height: address_index as u64 * 1_000,
        },
    }
}

/// load() reconstructs platform_addresses per wallet.
#[test]
fn tc040_load_platform_addresses() {
    let (persister, _tmp, _path) = fresh_persister();
    let a = wid(0xAA);
    let b = wid(0xBB);
    ensure_wallet_meta(&persister, &a);
    ensure_wallet_meta(&persister, &b);
    let mut cs_a = PlatformWalletChangeSet::default();
    cs_a.platform_addresses = Some(PlatformAddressChangeSet {
        addresses: vec![entry(a, 0, 0, 0x11), entry(a, 0, 1, 0x12)],
        sync_height: Some(10),
        ..Default::default()
    });
    let mut cs_b = PlatformWalletChangeSet::default();
    cs_b.platform_addresses = Some(PlatformAddressChangeSet {
        addresses: vec![entry(b, 0, 0, 0x21)],
        sync_height: Some(20),
        ..Default::default()
    });
    persister.store(a, cs_a).unwrap();
    persister.store(b, cs_b).unwrap();
    drop(persister);
    let tmp_dir = _tmp;
    let path = tmp_dir.path().join("wallet.db");
    let p2 = platform_wallet_storage::SqlitePersister::open(
        platform_wallet_storage::SqlitePersisterConfig::new(&path),
    )
    .unwrap();
    let state = p2.load().unwrap();
    assert_eq!(state.platform_addresses.len(), 2);
    assert_eq!(state.platform_addresses[&a].sync_height, 10);
    assert_eq!(state.platform_addresses[&b].sync_height, 20);
}

/// A deterministic test xpub built from a fixed serialized form so the
/// per-account reconstruction round-trip is reproducible.
fn test_xpub() -> key_wallet::bip32::ExtendedPubKey {
    key_wallet::bip32::ExtendedPubKey::decode(&hex::decode(
        "0488B21E000000000000000000873DFF81C02F525623FD1FE5167EAC3A55A049DE3D314BB42EE227FFED37D5080339A36013301597DAEF41FBE593A02CC513D0B55527EC2DF1050E2E8FF49C85C2",
    ).unwrap()).unwrap()
}

/// A `platform_payment` registration + platform_addresses for an account
/// reconstruct into `load_state().per_account` keyed by the account
/// index. Addresses for the account seed the per-account state.
#[test]
fn load_state_reconstructs_per_account_from_registration_and_addresses() {
    use platform_wallet::changeset::AccountRegistrationEntry;

    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0x64);
    ensure_wallet_meta(&persister, &w);

    let account_index = 7u32;
    let reg = AccountRegistrationEntry {
        account_type: key_wallet::account::AccountType::PlatformPayment {
            account: account_index,
            key_class: 0,
        },
        account_xpub: test_xpub(),
    };
    let mut cs = PlatformWalletChangeSet::default();
    cs.account_registrations = vec![reg];
    cs.platform_addresses = Some(PlatformAddressChangeSet {
        addresses: vec![
            entry(w, account_index, 0, 0xB0),
            entry(w, account_index, 1, 0xB1),
        ],
        sync_height: Some(5),
        ..Default::default()
    });
    persister.store(w, cs).unwrap();
    drop(persister);

    let p2 = reopen(&path);
    let conn = p2.lock_conn_for_test();
    let state = platform_wallet_storage::sqlite::schema::platform_addrs::load_state(&conn, &w)
        .expect("load_state");
    drop(conn);

    assert_eq!(
        state.per_account.len(),
        1,
        "exactly one platform_payment account should be reconstructed"
    );
    let account = state
        .per_account
        .get(&account_index)
        .expect("per_account must be keyed by the registered account index");

    // The xpub round-tripped from the registration row.
    assert_eq!(
        account.extended_public_key(),
        &test_xpub(),
        "reconstructed account must carry the registered xpub"
    );

    // Both seeded address rows landed in the index<->address bimap.
    let addr0 = PlatformP2PKHAddress::new([0xB0; 20]);
    let addr1 = PlatformP2PKHAddress::new([0xB1; 20]);
    assert_eq!(account.addresses().len(), 2);
    assert_eq!(account.addresses().get_by_left(&0), Some(&addr0));
    assert_eq!(account.addresses().get_by_left(&1), Some(&addr1));

    // The funds map mirrors what `entry()` seeded for each address.
    assert_eq!(
        account.found().get(&addr0),
        Some(&AddressFunds {
            balance: 0,
            nonce: 0,
            as_of_height: 0,
        }),
        "address 0 funds must match the seeded entry"
    );
    assert_eq!(
        account.found().get(&addr1),
        Some(&AddressFunds {
            balance: 100,
            nonce: 1,
            as_of_height: 1_000,
        }),
        "address 1 funds must match the seeded entry"
    );

    assert_eq!(state.sync_height, 5);
}

/// A registration with no addresses still populates `per_account` (the
/// xpub alone is enough to track the account); the wallet is therefore
/// surfaced by `load()` even with zero addresses and zero watermarks.
#[test]
fn registration_without_addresses_still_surfaces_in_load() {
    use platform_wallet::changeset::AccountRegistrationEntry;

    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0x65);
    ensure_wallet_meta(&persister, &w);

    let reg = AccountRegistrationEntry {
        account_type: key_wallet::account::AccountType::PlatformPayment {
            account: 3,
            key_class: 0,
        },
        account_xpub: test_xpub(),
    };
    let mut cs = PlatformWalletChangeSet::default();
    cs.account_registrations = vec![reg];
    persister.store(w, cs).unwrap();
    drop(persister);

    let p2 = reopen(&path);
    let state = p2.load().unwrap();
    let restored = state
        .platform_addresses
        .get(&w)
        .expect("wallet with a platform_payment registration must surface in load()");
    assert!(
        restored.per_account.contains_key(&3),
        "the registered account must be present in per_account"
    );
}

/// A wallet with no platform state at all — no registrations, no
/// addresses, all watermarks zero — is omitted from `load()`.
#[test]
fn wallet_without_platform_state_is_omitted_from_load() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0x66);
    ensure_wallet_meta(&persister, &w);
    // No platform-payment registration, no addresses, no sync row.
    drop(persister);

    let p2 = reopen(&path);
    let state = p2.load().unwrap();
    assert!(
        !state.platform_addresses.contains_key(&w),
        "a wallet with no platform state must be omitted from load()"
    );
}

/// non-wired-up sub-areas are written to disk (verified by
/// direct SQL probes) but do not surface in the load result.
///
/// Constructs non-empty `ContactChangeSet` and `TokenBalanceChangeSet`
/// payloads — `is_empty()` returns false on either, so the buffer
/// flushes them — then asserts both the `contacts` and `token_balances`
/// rows are present in SQLite after a reopen, while
/// `ClientStartState.platform_addresses` stays empty for the wallet
/// (no platform-address activity was stored).
#[test]
fn tc043_non_wired_up_persisted_but_not_returned() {
    use dpp::prelude::Identifier;
    use platform_wallet::changeset::{
        ContactChangeSet, ContactRequestEntry, SentContactRequestKey, TokenBalanceChangeSet,
    };
    use platform_wallet::wallet::identity::ContactRequest;

    let (persister, tmp, path) = fresh_persister();
    let w = wid(0xCC);
    let owner = Identifier::from([0x11; 32]);
    let recipient = Identifier::from([0x22; 32]);
    let token = Identifier::from([0x33; 32]);
    ensure_wallet_meta(&persister, &w);
    // token_balances FK targets identities(identity_id), so the owner
    // identity must exist before any token-balance row is written.
    // contacts is wallet-scoped, so it doesn't need an identity row.
    common::ensure_identity(&persister, owner.as_bytes(), Some(&w));
    let mut sent_requests = std::collections::BTreeMap::new();
    sent_requests.insert(
        SentContactRequestKey {
            owner_id: owner,
            recipient_id: recipient,
        },
        ContactRequestEntry {
            request: ContactRequest {
                sender_id: owner,
                recipient_id: recipient,
                sender_key_index: 0,
                recipient_key_index: 0,
                account_reference: 0,
                encrypted_account_label: None,
                encrypted_public_key: Vec::new(),
                auto_accept_proof: None,
                core_height_created_at: 0,
                created_at: 0,
            },
        },
    );
    let mut balances = std::collections::BTreeMap::new();
    balances.insert((owner, token), 42u64);
    let cs = PlatformWalletChangeSet {
        contacts: Some(ContactChangeSet {
            sent_requests,
            ..Default::default()
        }),
        token_balances: Some(TokenBalanceChangeSet {
            balances,
            ..Default::default()
        }),
        ..Default::default()
    };
    persister.store(w, cs).unwrap();
    drop(persister);

    // Reopen against the same DB and confirm the rows are durable on
    // disk + the load result is platform-address-empty for this wallet.
    let p2 = platform_wallet_storage::SqlitePersister::open(
        platform_wallet_storage::SqlitePersisterConfig::new(&path),
    )
    .unwrap();
    let state = p2.load().unwrap();
    assert!(
        !state.platform_addresses.contains_key(&w),
        "no platform-address activity was stored — wallet must be absent"
    );
    drop(p2);

    let conn = common::ro_conn(&path);
    let sent: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM contacts \
             WHERE wallet_id = ?1 AND owner_id = ?2 AND contact_id = ?3 AND state = 'sent'",
            rusqlite::params![w.as_slice(), owner.as_slice(), recipient.as_slice()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(sent, 1, "contacts 'sent' row missing after reopen");
    let tokens: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM token_balances WHERE identity_id = ?1 AND token_id = ?2",
            rusqlite::params![owner.as_slice(), token.as_slice()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(tokens, 1, "token_balances row missing after reopen");
    drop(tmp);
}

// ---------------------------------------------------------------------------
// P4 — functional load() readers
// ---------------------------------------------------------------------------

use dpp::prelude::Identifier;
use platform_wallet::changeset::{
    ContactChangeSet, ContactRequestEntry, IdentityChangeSet, IdentityEntry,
    ReceivedContactRequestKey, SentContactRequestKey,
};
use platform_wallet::wallet::identity::{ContactRequest, EstablishedContact, IdentityStatus};

fn reopen(path: &std::path::Path) -> platform_wallet_storage::SqlitePersister {
    platform_wallet_storage::SqlitePersister::open(
        platform_wallet_storage::SqlitePersisterConfig::new(path),
    )
    .expect("reopen persister")
}

fn identity_entry(id: u8, idx: Option<u32>) -> IdentityEntry {
    IdentityEntry {
        id: Identifier::from([id; 32]),
        balance: u64::from(id),
        revision: 1,
        identity_index: idx,
        last_updated_balance_block_time: None,
        last_synced_keys_block_time: None,
        dpns_names: Vec::new(),
        contested_dpns_names: Vec::new(),
        status: IdentityStatus::Active,
        wallet_id: None,
        dashpay_profile: None,
        dashpay_payments: Default::default(),
        contact_profiles: Default::default(),
        ignored_senders: Default::default(),
    }
}

fn contact_request_entry(sender: u8, recipient: u8) -> ContactRequestEntry {
    ContactRequestEntry {
        request: ContactRequest {
            sender_id: Identifier::from([sender; 32]),
            recipient_id: Identifier::from([recipient; 32]),
            sender_key_index: 0,
            recipient_key_index: 0,
            account_reference: 0,
            encrypted_account_label: None,
            encrypted_public_key: Vec::new(),
            auto_accept_proof: None,
            core_height_created_at: 100,
            created_at: 0,
        },
    }
}

/// identities reader round-trips per wallet, exact equality
/// on `id`s.
///
/// `persister.load()` no longer surfaces the identities slot (the
/// `ClientStartState` revert dropped it), so this exercises the
/// hardened dormant reader `schema::identities::load_state` directly —
/// keeping its fail-hard behaviour genuinely covered.
#[test]
fn tc_p4_003_load_identities_two_wallets() {
    use platform_wallet_storage::sqlite::schema::identities;
    use std::collections::BTreeMap;
    let (persister, _tmp, path) = fresh_persister();
    let a = wid(0xAA);
    let b = wid(0xBB);
    ensure_wallet_meta(&persister, &a);
    ensure_wallet_meta(&persister, &b);

    let mut identities_a: BTreeMap<Identifier, IdentityEntry> = BTreeMap::new();
    let e_a1 = identity_entry(0x01, Some(0));
    let e_a2 = identity_entry(0x02, Some(1));
    identities_a.insert(e_a1.id, e_a1.clone());
    identities_a.insert(e_a2.id, e_a2.clone());
    let cs_a = PlatformWalletChangeSet {
        identities: Some(IdentityChangeSet {
            identities: identities_a,
            removed: Default::default(),
        }),
        ..Default::default()
    };

    let mut identities_b: BTreeMap<Identifier, IdentityEntry> = BTreeMap::new();
    let e_b1 = identity_entry(0x10, Some(0));
    identities_b.insert(e_b1.id, e_b1.clone());
    let cs_b = PlatformWalletChangeSet {
        identities: Some(IdentityChangeSet {
            identities: identities_b,
            removed: Default::default(),
        }),
        ..Default::default()
    };

    persister.store(a, cs_a).unwrap();
    persister.store(b, cs_b).unwrap();
    drop(persister);

    let p2 = reopen(&path);
    let conn = p2.lock_conn_for_test();
    let a_state = identities::load_state(&conn, &a).expect("load_state A");
    let b_state = identities::load_state(&conn, &b).expect("load_state B");
    drop(conn);

    // Both stored under identity_index 0 and 1 — wallet bucket.
    let bucket_a = a_state.wallet_identities.get(&a).expect("bucket A");
    assert_eq!(bucket_a.len(), 2);
    let mut got_ids: Vec<_> = bucket_a.values().map(|m| m.identity.id()).collect();
    got_ids.sort();
    use dpp::identity::accessors::IdentityGettersV0;
    let mut expect_ids = vec![e_a1.id, e_a2.id];
    expect_ids.sort();
    assert_eq!(got_ids, expect_ids);

    let bucket_b = b_state.wallet_identities.get(&b).expect("bucket B");
    assert_eq!(bucket_b.len(), 1);
    assert_eq!(bucket_b.values().next().unwrap().identity.id(), e_b1.id);
}

/// contacts round-trip per wallet, exact equality on the
/// contact-request key + entry.
#[test]
fn tc_p4_004_load_contacts_two_wallets() {
    use std::collections::BTreeMap;
    let (persister, _tmp, path) = fresh_persister();
    let a = wid(0xCA);
    let b = wid(0xCB);
    ensure_wallet_meta(&persister, &a);
    ensure_wallet_meta(&persister, &b);
    let key_a = SentContactRequestKey {
        owner_id: Identifier::from([0x11; 32]),
        recipient_id: Identifier::from([0x12; 32]),
    };
    let entry_a = contact_request_entry(0x11, 0x12);
    let mut sent_a = BTreeMap::new();
    sent_a.insert(key_a, entry_a.clone());
    persister
        .store(
            a,
            PlatformWalletChangeSet {
                contacts: Some(ContactChangeSet {
                    sent_requests: sent_a,
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();

    let key_b = SentContactRequestKey {
        owner_id: Identifier::from([0x21; 32]),
        recipient_id: Identifier::from([0x22; 32]),
    };
    let entry_b = contact_request_entry(0x21, 0x22);
    let mut sent_b = BTreeMap::new();
    sent_b.insert(key_b, entry_b.clone());
    persister
        .store(
            b,
            PlatformWalletChangeSet {
                contacts: Some(ContactChangeSet {
                    sent_requests: sent_b,
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    drop(persister);

    let p2 = reopen(&path);
    let conn = p2.lock_conn_for_test();
    let a_state = platform_wallet_storage::sqlite::schema::contacts::load_state_for_test(&conn, &a)
        .expect("contacts load_state A");
    let b_state = platform_wallet_storage::sqlite::schema::contacts::load_state_for_test(&conn, &b)
        .expect("contacts load_state B");
    drop(conn);
    // Exact reconstruction: each wallet sees only its own request, with
    // the full `ContactRequest` surviving the round-trip.
    assert_eq!(a_state.sent_requests.len(), 1);
    assert_eq!(a_state.sent_requests.get(&key_a), Some(&entry_a));
    assert!(a_state.incoming_requests.is_empty() && a_state.established.is_empty());
    assert_eq!(b_state.sent_requests.len(), 1);
    assert_eq!(b_state.sent_requests.get(&key_b), Some(&entry_b));
    assert!(b_state.incoming_requests.is_empty() && b_state.established.is_empty());
}

/// Round-trip one changeset through the persister and read the wallet's
/// `contacts` back via the hardened reader. Shared by the lifecycle-state
/// tests below.
fn contacts_round_trip(
    wallet_byte: u8,
    cs: ContactChangeSet,
) -> platform_wallet_storage::sqlite::schema::contacts::ContactsRecords {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(wallet_byte);
    ensure_wallet_meta(&persister, &w);
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                contacts: Some(cs),
                ..Default::default()
            },
        )
        .unwrap();
    drop(persister);

    let p2 = reopen(&path);
    let conn = p2.lock_conn_for_test();
    platform_wallet_storage::sqlite::schema::contacts::load_state_for_test(&conn, &w)
        .expect("contacts load_state")
}

/// A fully-populated [`EstablishedContact`] so the round-trip exercises
/// every metadata column (`alias`, `note`, `is_hidden`,
/// `accepted_accounts`, `payment_channel_broken`) plus both request blobs.
fn established_contact(owner: u8, contact: u8) -> EstablishedContact {
    EstablishedContact {
        contact_identity_id: Identifier::from([contact; 32]),
        outgoing_request: contact_request_entry(owner, contact).request,
        incoming_request: contact_request_entry(contact, owner).request,
        alias: Some("best friend".to_string()),
        note: Some("met at conf".to_string()),
        is_hidden: true,
        accepted_accounts: vec![1, 7, 42],
        // Non-default so the round-trip test pins the new
        // `payment_channel_broken` column through write + read.
        payment_channel_broken: true,
        // This backend has no column for the system-derived account label,
        // so the read path always reconstructs it as `None`; keep the
        // fixture `None` to match through the round-trip.
        contact_account_label: None,
        // This backend has no column for the rotation self-heal marker, so
        // the read path always reconstructs it as `None`; keep the fixture
        // `None` to match through the round-trip.
        external_account_reference: None,
    }
}

/// a sent-only request round-trips into `sent_requests` and
/// nowhere else.
#[test]
fn tc_p4_004a_sent_only_round_trip() {
    let key = SentContactRequestKey {
        owner_id: Identifier::from([0x31; 32]),
        recipient_id: Identifier::from([0x32; 32]),
    };
    let entry = contact_request_entry(0x31, 0x32);
    let mut sent = std::collections::BTreeMap::new();
    sent.insert(key, entry.clone());
    let state = contacts_round_trip(
        0xA1,
        ContactChangeSet {
            sent_requests: sent,
            ..Default::default()
        },
    );
    assert_eq!(state.sent_requests.get(&key), Some(&entry));
    assert!(state.incoming_requests.is_empty());
    assert!(state.established.is_empty());
}

/// a received-only request round-trips into `incoming_requests`
/// and nowhere else.
#[test]
fn tc_p4_004b_received_only_round_trip() {
    let key = ReceivedContactRequestKey {
        owner_id: Identifier::from([0x41; 32]),
        sender_id: Identifier::from([0x42; 32]),
    };
    // sender → owner (the request comes FROM the sender TO our owner).
    let entry = contact_request_entry(0x42, 0x41);
    let mut incoming = std::collections::BTreeMap::new();
    incoming.insert(key, entry.clone());
    let state = contacts_round_trip(
        0xB2,
        ContactChangeSet {
            incoming_requests: incoming,
            ..Default::default()
        },
    );
    assert_eq!(state.incoming_requests.get(&key), Some(&entry));
    assert!(state.sent_requests.is_empty());
    assert!(state.established.is_empty());
}

/// an established contact round-trips into `established` with
/// both request blobs and all four metadata columns intact.
#[test]
fn tc_p4_004c_established_round_trip() {
    let key = SentContactRequestKey {
        owner_id: Identifier::from([0x51; 32]),
        recipient_id: Identifier::from([0x52; 32]),
    };
    let contact = established_contact(0x51, 0x52);
    let mut est = std::collections::BTreeMap::new();
    est.insert(key, contact.clone());
    let state = contacts_round_trip(
        0xC3,
        ContactChangeSet {
            established: est,
            ..Default::default()
        },
    );
    assert_eq!(state.established.get(&key), Some(&contact));
    assert!(state.sent_requests.is_empty());
    assert!(state.incoming_requests.is_empty());
}

/// `removed_sent` / `removed_incoming` delete the matching
/// pending row — an explicit cancellation, not an auto-establishment.
#[test]
fn tc_p4_004d_removal_deletes_pending_rows() {
    let sent_key = SentContactRequestKey {
        owner_id: Identifier::from([0x61; 32]),
        recipient_id: Identifier::from([0x62; 32]),
    };
    let recv_key = ReceivedContactRequestKey {
        owner_id: Identifier::from([0x61; 32]),
        sender_id: Identifier::from([0x63; 32]),
    };
    let mut sent = std::collections::BTreeMap::new();
    sent.insert(sent_key, contact_request_entry(0x61, 0x62));
    let mut incoming = std::collections::BTreeMap::new();
    incoming.insert(recv_key, contact_request_entry(0x63, 0x61));

    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xD4);
    ensure_wallet_meta(&persister, &w);
    // First store both pending rows.
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                contacts: Some(ContactChangeSet {
                    sent_requests: sent,
                    incoming_requests: incoming,
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    // Then cancel both.
    let mut removed_sent = std::collections::BTreeSet::new();
    removed_sent.insert(sent_key);
    let mut removed_incoming = std::collections::BTreeSet::new();
    removed_incoming.insert(recv_key);
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                contacts: Some(ContactChangeSet {
                    removed_sent,
                    removed_incoming,
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    drop(persister);

    let p2 = reopen(&path);
    let conn = p2.lock_conn_for_test();
    let state = platform_wallet_storage::sqlite::schema::contacts::load_state_for_test(&conn, &w)
        .expect("contacts load_state");
    drop(conn);
    assert!(state.sent_requests.is_empty(), "removed_sent left a row");
    assert!(
        state.incoming_requests.is_empty(),
        "removed_incoming left a row"
    );
    assert!(state.established.is_empty());
}

/// auto-establishment. An `established` upsert over a prior
/// pending row for the same `(owner, contact)` pair collapses to a
/// single established row — load returns it under `established`, never
/// `sent` / `incoming`.
#[test]
fn tc_p4_004e_auto_establishment_collapses_pending() {
    let key = SentContactRequestKey {
        owner_id: Identifier::from([0x71; 32]),
        recipient_id: Identifier::from([0x72; 32]),
    };
    let recv_key = ReceivedContactRequestKey {
        owner_id: Identifier::from([0x71; 32]),
        sender_id: Identifier::from([0x72; 32]),
    };
    let contact = established_contact(0x71, 0x72);

    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xE5);
    ensure_wallet_meta(&persister, &w);
    // Seed both a sent AND a received pending row for the same pair.
    let mut sent = std::collections::BTreeMap::new();
    sent.insert(key, contact_request_entry(0x71, 0x72));
    let mut incoming = std::collections::BTreeMap::new();
    incoming.insert(recv_key, contact_request_entry(0x72, 0x71));
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                contacts: Some(ContactChangeSet {
                    sent_requests: sent,
                    incoming_requests: incoming,
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    // Now establish — must collapse the pending row, not add a sibling.
    let mut est = std::collections::BTreeMap::new();
    est.insert(key, contact.clone());
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                contacts: Some(ContactChangeSet {
                    established: est,
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    drop(persister);

    // Exactly one row on disk for the pair, in the 'established' state.
    let p2 = reopen(&path);
    {
        let conn = p2.lock_conn_for_test();
        let n: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM contacts \
                 WHERE wallet_id = ?1 AND owner_id = ?2 AND contact_id = ?3",
                rusqlite::params![
                    w.as_slice(),
                    key.owner_id.as_slice(),
                    key.recipient_id.as_slice()
                ],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(n, 1, "auto-establishment must collapse to a single row");
    }
    let conn = p2.lock_conn_for_test();
    let state = platform_wallet_storage::sqlite::schema::contacts::load_state_for_test(&conn, &w)
        .expect("contacts load_state");
    drop(conn);
    assert_eq!(state.established.get(&key), Some(&contact));
    assert!(
        state.sent_requests.is_empty(),
        "establishment must not leave a 'sent' bucket entry"
    );
    assert!(
        state.incoming_requests.is_empty(),
        "establishment must not leave an 'incoming' bucket entry"
    );
}

/// A pending `sent` followed by the matching `incoming` for the same
/// pair promotes the row to `established` even without an explicit
/// `established` upsert — both request blobs are now stored, so the
/// reader reconstructs it under `established`, never `sent`/`incoming`.
#[test]
fn sent_then_matching_incoming_promotes_to_established() {
    let sent_key = SentContactRequestKey {
        owner_id: Identifier::from([0x81; 32]),
        recipient_id: Identifier::from([0x82; 32]),
    };
    let recv_key = ReceivedContactRequestKey {
        owner_id: Identifier::from([0x81; 32]),
        sender_id: Identifier::from([0x82; 32]),
    };
    let est_key = SentContactRequestKey {
        owner_id: Identifier::from([0x81; 32]),
        recipient_id: Identifier::from([0x82; 32]),
    };

    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xF6);
    ensure_wallet_meta(&persister, &w);

    let mut sent = std::collections::BTreeMap::new();
    sent.insert(sent_key, contact_request_entry(0x81, 0x82));
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                contacts: Some(ContactChangeSet {
                    sent_requests: sent,
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();

    let mut incoming = std::collections::BTreeMap::new();
    incoming.insert(recv_key, contact_request_entry(0x82, 0x81));
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                contacts: Some(ContactChangeSet {
                    incoming_requests: incoming,
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    drop(persister);

    let p2 = reopen(&path);
    let conn = p2.lock_conn_for_test();
    let state = platform_wallet_storage::sqlite::schema::contacts::load_state_for_test(&conn, &w)
        .expect("contacts load_state");
    drop(conn);
    assert!(
        state.established.contains_key(&est_key),
        "matching opposite request must promote the row to established"
    );
    assert!(
        state.sent_requests.is_empty(),
        "promotion must clear the 'sent' bucket"
    );
    assert!(
        state.incoming_requests.is_empty(),
        "promotion must clear the 'incoming' bucket"
    );
}

/// Symmetric to [`sent_then_matching_incoming_promotes_to_established`]:
/// a pending `incoming` followed by the matching `sent` for the same
/// pair promotes the row to `established`.
#[test]
fn received_then_matching_sent_promotes_to_established() {
    let recv_key = ReceivedContactRequestKey {
        owner_id: Identifier::from([0x91; 32]),
        sender_id: Identifier::from([0x92; 32]),
    };
    let sent_key = SentContactRequestKey {
        owner_id: Identifier::from([0x91; 32]),
        recipient_id: Identifier::from([0x92; 32]),
    };

    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xF7);
    ensure_wallet_meta(&persister, &w);

    let mut incoming = std::collections::BTreeMap::new();
    incoming.insert(recv_key, contact_request_entry(0x92, 0x91));
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                contacts: Some(ContactChangeSet {
                    incoming_requests: incoming,
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();

    let mut sent = std::collections::BTreeMap::new();
    sent.insert(sent_key, contact_request_entry(0x91, 0x92));
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                contacts: Some(ContactChangeSet {
                    sent_requests: sent,
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    drop(persister);

    let p2 = reopen(&path);
    let conn = p2.lock_conn_for_test();
    let state = platform_wallet_storage::sqlite::schema::contacts::load_state_for_test(&conn, &w)
        .expect("contacts load_state");
    drop(conn);
    assert!(
        state.established.contains_key(&sent_key),
        "matching opposite request must promote the row to established"
    );
    assert!(state.sent_requests.is_empty());
    assert!(state.incoming_requests.is_empty());
}

/// asset locks bucketed by (wallet, account, outpoint).
#[test]
fn tc_p4_005_load_asset_locks_bucketed() {
    use dashcore::hashes::Hash;
    use dashcore::{OutPoint, Transaction, Txid};
    use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;
    use platform_wallet::changeset::{AssetLockChangeSet, AssetLockEntry};
    use platform_wallet::wallet::asset_lock::tracked::AssetLockStatus;
    let (persister, _tmp, path) = fresh_persister();
    let a = wid(0xAA);
    let b = wid(0xBB);
    ensure_wallet_meta(&persister, &a);
    ensure_wallet_meta(&persister, &b);

    let mk_entry = |op: OutPoint, account: u32| AssetLockEntry {
        out_point: op,
        transaction: Transaction {
            version: 3,
            lock_time: 0,
            input: vec![],
            output: vec![],
            special_transaction_payload: None,
        },
        account_index: account,
        funding_type: AssetLockFundingType::IdentityTopUp,
        identity_index: 0,
        amount_duffs: 1000,
        status: AssetLockStatus::Built,
        proof: None,
    };
    let op_a0_1 = OutPoint {
        txid: Txid::from_byte_array([0x10; 32]),
        vout: 0,
    };
    let op_a0_2 = OutPoint {
        txid: Txid::from_byte_array([0x11; 32]),
        vout: 0,
    };
    let op_a5 = OutPoint {
        txid: Txid::from_byte_array([0x20; 32]),
        vout: 0,
    };
    let op_b0 = OutPoint {
        txid: Txid::from_byte_array([0x30; 32]),
        vout: 0,
    };
    let mut locks_a = AssetLockChangeSet::default();
    locks_a.asset_locks.insert(op_a0_1, mk_entry(op_a0_1, 0));
    locks_a.asset_locks.insert(op_a0_2, mk_entry(op_a0_2, 0));
    locks_a.asset_locks.insert(op_a5, mk_entry(op_a5, 5));
    persister
        .store(
            a,
            PlatformWalletChangeSet {
                asset_locks: Some(locks_a),
                ..Default::default()
            },
        )
        .unwrap();
    let mut locks_b = AssetLockChangeSet::default();
    locks_b.asset_locks.insert(op_b0, mk_entry(op_b0, 0));
    persister
        .store(
            b,
            PlatformWalletChangeSet {
                asset_locks: Some(locks_b),
                ..Default::default()
            },
        )
        .unwrap();
    drop(persister);

    let p2 = reopen(&path);
    let conn = p2.lock_conn_for_test();
    let a_buckets = platform_wallet_storage::sqlite::schema::asset_locks::load_state(&conn, &a)
        .expect("asset_locks load_state A");
    let b_buckets = platform_wallet_storage::sqlite::schema::asset_locks::load_state(&conn, &b)
        .expect("asset_locks load_state B");
    drop(conn);
    assert_eq!(a_buckets.len(), 2, "expected 2 account buckets for A");
    assert_eq!(a_buckets[&0].len(), 2);
    assert_eq!(a_buckets[&5].len(), 1);
    assert_eq!(b_buckets[&0].len(), 1);
}

/// empty wallets emit `wallets_pending_rehydration = N`
/// and `wallets` slot stays empty.
#[tracing_test::traced_test]
#[test]
fn tc_p4_006_pending_rehydration_count() {
    let (persister, _tmp, path) = fresh_persister();
    ensure_wallet_meta(&persister, &wid(0x01));
    ensure_wallet_meta(&persister, &wid(0x02));
    ensure_wallet_meta(&persister, &wid(0x03));
    drop(persister);
    let p2 = reopen(&path);
    let state = p2.load().unwrap();
    assert!(state.wallets.is_empty());
    assert!(logs_contain("wallets_pending_rehydration=3"));
    assert!(logs_contain("wallets_rehydrated=0"));
}

/// load() summary carries every counter, including zeros.
#[tracing_test::traced_test]
#[test]
fn tc_p4_007_summary_log_counters() {
    let (persister, _tmp, path) = fresh_persister();
    ensure_wallet_meta(&persister, &wid(0x10));
    ensure_wallet_meta(&persister, &wid(0x11));
    drop(persister);
    let p2 = reopen(&path);
    let _ = p2.load().unwrap();
    for field in [
        "wallets_seen=2",
        "addresses_loaded=0",
        "wallets_rehydrated=0",
        "wallets_pending_rehydration=2",
    ] {
        assert!(logs_contain(field), "missing structured field: {field}");
    }
}

/// a corrupted blob is a HARD failure. The hardened reader
/// returns `Err` for the corrupt wallet (no silent skip) while the
/// second, intact wallet still decodes cleanly.
#[test]
fn tc_p4_008_corruption_is_hard_error() {
    use platform_wallet_storage::sqlite::schema::identities;
    use std::collections::BTreeMap;
    let (persister, _tmp, path) = fresh_persister();
    let a = wid(0xCA);
    let b = wid(0xCB);
    ensure_wallet_meta(&persister, &a);
    ensure_wallet_meta(&persister, &b);
    let mut id_a = BTreeMap::new();
    id_a.insert(Identifier::from([0x01; 32]), identity_entry(0x01, Some(0)));
    persister
        .store(
            a,
            PlatformWalletChangeSet {
                identities: Some(IdentityChangeSet {
                    identities: id_a,
                    removed: Default::default(),
                }),
                ..Default::default()
            },
        )
        .unwrap();
    let mut id_b = BTreeMap::new();
    id_b.insert(Identifier::from([0x02; 32]), identity_entry(0x02, Some(0)));
    persister
        .store(
            b,
            PlatformWalletChangeSet {
                identities: Some(IdentityChangeSet {
                    identities: id_b,
                    removed: Default::default(),
                }),
                ..Default::default()
            },
        )
        .unwrap();
    // Truncate A's blob to a single zero byte so bincode bails out.
    {
        let conn = persister.lock_conn_for_test();
        conn.execute(
            "UPDATE identities SET entry_blob = X'00' WHERE wallet_id = ?1",
            rusqlite::params![a.as_slice()],
        )
        .unwrap();
    }
    drop(persister);
    let p2 = reopen(&path);
    let conn = p2.lock_conn_for_test();
    let a_result = identities::load_state(&conn, &a);
    let b_state = identities::load_state(&conn, &b).expect("B must decode cleanly");
    drop(conn);
    assert!(
        matches!(a_result, Err(WalletStorageError::BincodeDecode { .. })),
        "corrupt identity blob must be a typed BincodeDecode error, not a \
         silent skip; got {a_result:?}"
    );
    assert_eq!(b_state.wallet_identities.get(&b).map(|m| m.len()), Some(1));
}

/// 008b: `contacts::load_state` is fail-hard. A garbage
/// `outgoing_request` blob yields a typed `BincodeDecode`; a non-32-byte
/// id column yields a typed `BlobDecode`. Neither is silently skipped,
/// and an intact wallet still decodes cleanly.
#[test]
fn tc_p4_008b_contacts_corruption_is_hard_error() {
    use platform_wallet_storage::sqlite::schema::contacts;
    let (persister, _tmp, path) = fresh_persister();
    let bad_blob = wid(0xD1);
    let bad_id = wid(0xD2);
    let good = wid(0xD3);
    ensure_wallet_meta(&persister, &bad_blob);
    ensure_wallet_meta(&persister, &bad_id);
    ensure_wallet_meta(&persister, &good);
    {
        let conn = persister.lock_conn_for_test();
        // bad_blob: well-formed 32-byte ids, undecodable request blob.
        conn.execute(
            "INSERT INTO contacts (wallet_id, owner_id, contact_id, state, outgoing_request) \
             VALUES (?1, ?2, ?3, 'sent', X'00')",
            rusqlite::params![bad_blob.as_slice(), &[0x11u8; 32][..], &[0x12u8; 32][..]],
        )
        .unwrap();
        // bad_id: owner_id is only 10 bytes — fails the 32-byte check.
        conn.execute(
            "INSERT INTO contacts (wallet_id, owner_id, contact_id, state, outgoing_request) \
             VALUES (?1, ?2, ?3, 'sent', X'00')",
            rusqlite::params![bad_id.as_slice(), &[0xAAu8; 10][..], &[0x12u8; 32][..]],
        )
        .unwrap();
    }
    let mut sent_good = std::collections::BTreeMap::new();
    sent_good.insert(
        SentContactRequestKey {
            owner_id: Identifier::from([0x21; 32]),
            recipient_id: Identifier::from([0x22; 32]),
        },
        contact_request_entry(0x21, 0x22),
    );
    persister
        .store(
            good,
            PlatformWalletChangeSet {
                contacts: Some(ContactChangeSet {
                    sent_requests: sent_good,
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    drop(persister);

    let p2 = reopen(&path);
    let conn = p2.lock_conn_for_test();
    let blob_result = contacts::load_state_for_test(&conn, &bad_blob);
    let id_result = contacts::load_state_for_test(&conn, &bad_id);
    let good_state =
        contacts::load_state_for_test(&conn, &good).expect("intact wallet must decode");
    drop(conn);

    assert!(
        matches!(blob_result, Err(WalletStorageError::BincodeDecode { .. })),
        "garbage contacts entry_blob must be a typed BincodeDecode; got {blob_result:?}"
    );
    assert!(
        matches!(id_result, Err(WalletStorageError::BlobDecode { .. })),
        "non-32-byte contacts id column must be a typed BlobDecode; got {id_result:?}"
    );
    assert_eq!(good_state.sent_requests.len(), 1);
}

/// 008c: `asset_locks::load_state` is fail-hard. A garbage
/// `lifecycle_blob` yields a typed `BincodeDecode`; a malformed
/// `outpoint` column yields a typed decode error. An intact wallet
/// still decodes cleanly.
#[test]
fn tc_p4_008c_asset_locks_corruption_is_hard_error() {
    use dashcore::hashes::Hash;
    use dashcore::{OutPoint, Transaction, Txid};
    use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;
    use platform_wallet::changeset::{AssetLockChangeSet, AssetLockEntry};
    use platform_wallet::wallet::asset_lock::tracked::AssetLockStatus;
    use platform_wallet_storage::sqlite::schema::asset_locks;

    let (persister, _tmp, path) = fresh_persister();
    let bad_blob = wid(0xE1);
    let bad_op = wid(0xE2);
    let good = wid(0xE3);
    ensure_wallet_meta(&persister, &bad_blob);
    ensure_wallet_meta(&persister, &bad_op);
    ensure_wallet_meta(&persister, &good);
    {
        let conn = persister.lock_conn_for_test();
        // bad_blob: decodable outpoint key, undecodable lifecycle_blob.
        conn.execute(
            "INSERT INTO asset_locks \
                (wallet_id, outpoint, status, account_index, identity_index, amount_duffs, lifecycle_blob) \
             VALUES (?1, ?2, 'built', 0, 0, 0, X'00')",
            rusqlite::params![bad_blob.as_slice(), &[0x01u8; 36][..]],
        )
        .unwrap();
        // bad_op: outpoint column is 4 bytes — too short to bincode-decode
        // a full OutPoint, so it fails as a typed decode error.
        conn.execute(
            "INSERT INTO asset_locks \
                (wallet_id, outpoint, status, account_index, identity_index, amount_duffs, lifecycle_blob) \
             VALUES (?1, ?2, 'built', 0, 0, 0, X'00')",
            rusqlite::params![bad_op.as_slice(), &[0x01u8; 4][..]],
        )
        .unwrap();
    }
    let op_good = OutPoint {
        txid: Txid::from_byte_array([0x40; 32]),
        vout: 0,
    };
    let mut locks_good = AssetLockChangeSet::default();
    locks_good.asset_locks.insert(
        op_good,
        AssetLockEntry {
            out_point: op_good,
            transaction: Transaction {
                version: 3,
                lock_time: 0,
                input: vec![],
                output: vec![],
                special_transaction_payload: None,
            },
            account_index: 0,
            funding_type: AssetLockFundingType::IdentityTopUp,
            identity_index: 0,
            amount_duffs: 1000,
            status: AssetLockStatus::Built,
            proof: None,
        },
    );
    persister
        .store(
            good,
            PlatformWalletChangeSet {
                asset_locks: Some(locks_good),
                ..Default::default()
            },
        )
        .unwrap();
    drop(persister);

    let p2 = reopen(&path);
    let conn = p2.lock_conn_for_test();
    let blob_result = asset_locks::load_state(&conn, &bad_blob);
    let op_result = asset_locks::load_state(&conn, &bad_op);
    let good_state = asset_locks::load_state(&conn, &good).expect("intact wallet must decode");
    drop(conn);

    assert!(
        matches!(blob_result, Err(WalletStorageError::BincodeDecode { .. })),
        "garbage asset_locks lifecycle_blob must be a typed BincodeDecode; got {blob_result:?}"
    );
    // A 4-byte outpoint is too short for the 32-byte txid prefix, so
    // bincode fails deterministically with BincodeDecode (UnexpectedEnd)
    // before the trailing-bytes check.
    assert!(
        matches!(op_result, Err(WalletStorageError::BincodeDecode { .. })),
        "malformed asset_locks outpoint must be a typed BincodeDecode; got {op_result:?}"
    );
    assert_eq!(good_state[&0].len(), 1);
}

/// 008d: `wallet_meta::list_ids` is fail-hard on a malformed
/// stored `wallet_id`. This is the code path where a non-32-byte id
/// actually surfaces (the per-area `load_state` readers take a typed
/// `&WalletId`, so the length check belongs here). A 10-byte
/// `wallet_metadata.wallet_id` yields a typed `InvalidWalletIdLength`.
#[test]
fn tc_p4_008d_list_ids_rejects_non_32_byte_wallet_id() {
    use platform_wallet_storage::sqlite::schema::wallet_meta;
    let (persister, _tmp, path) = fresh_persister();
    {
        let conn = persister.lock_conn_for_test();
        conn.execute(
            "INSERT INTO wallet_metadata (wallet_id, network, birth_height) \
             VALUES (?1, 'testnet', 0)",
            rusqlite::params![&[0xAAu8; 10][..]],
        )
        .unwrap();
    }
    drop(persister);

    let p2 = reopen(&path);
    let conn = p2.lock_conn_for_test();
    let result = wallet_meta::list_ids(&conn);
    drop(conn);
    assert!(
        matches!(
            result,
            Err(WalletStorageError::InvalidWalletIdLength { actual: 10 })
        ),
        "non-32-byte stored wallet_id must be a typed InvalidWalletIdLength {{ actual: 10 }}; \
         got {result:?}"
    );
}

/// `load()` query cost is bounded per wallet.
///
/// `load()` now drives the platform-address reader off
/// `wallet_meta::list_ids` and issues a fixed, small number of
/// statements per listed wallet (the dedup collapse traded the old
/// constant-query bulk scans for the fail-hard per-wallet readers).
/// This pins the per-wallet statement count so a future regression
/// that fans out into an unbounded per-row round trip is caught.
///
/// Verified by enabling `sqlite3_trace_v2` on the persister's
/// connection, counting `Stmt` events for the duration of one
/// `load()`. `serial_test::serial` because the trace counter is a
/// process-wide `AtomicUsize` (`Connection::trace_v2`'s callback must
/// be a `fn`, not a `Fn`).
#[test]
#[serial_test::serial]
fn tc_p4_012_load_query_count_bounded() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    fn cb(ev: rusqlite::trace::TraceEvent<'_>) {
        if let rusqlite::trace::TraceEvent::Stmt(_, _) = ev {
            COUNTER.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn count_load_queries(persister: &common::SqlitePersister) -> usize {
        // Hold the conn briefly to install the trace, then drop the
        // guard before calling load() (load takes its own lock).
        {
            let conn = persister.lock_conn_for_test();
            conn.trace_v2(
                rusqlite::trace::TraceEventCodes::SQLITE_TRACE_STMT,
                Some(cb),
            );
        }
        COUNTER.store(0, Ordering::Relaxed);
        persister.load().expect("load");
        let n = COUNTER.load(Ordering::Relaxed);
        // Disable trace so other tests don't accidentally inherit it.
        {
            let conn = persister.lock_conn_for_test();
            conn.trace_v2(rusqlite::trace::TraceEventCodes::SQLITE_TRACE_STMT, None);
        }
        n
    }

    fn seed_wallets(persister: &common::SqlitePersister, n: usize) {
        for i in 0..n {
            let id = wid(0xC0 + i as u8);
            ensure_wallet_meta(persister, &id);
            let mut cs = PlatformWalletChangeSet::default();
            cs.platform_addresses = Some(PlatformAddressChangeSet {
                addresses: vec![entry(id, 0, 0, 0xA0 + i as u8)],
                sync_height: Some(1),
                ..Default::default()
            });
            persister.store(id, cs).unwrap();
        }
    }

    let (p1, _tmp1, _path1) = fresh_persister();
    seed_wallets(&p1, 1);
    let count_one = count_load_queries(&p1);

    let (p10, _tmp10, _path10) = fresh_persister();
    seed_wallets(&p10, 10);
    let count_ten = count_load_queries(&p10);

    // `load()` issues a fixed number of grouped scans regardless of
    // wallet count: `wallet_meta::list_ids` plus one scan each over
    // `platform_address_sync`, `platform_addresses`, and the
    // `platform_payment` `account_registrations`. The count must NOT
    // grow with the number of wallets — that's the constant-query
    // contract.
    assert_eq!(
        count_one, count_ten,
        "load() query count must not grow with wallet count \
         (N=1 → {count_one}, N=10 → {count_ten})"
    );
    assert_eq!(
        count_one, 4,
        "load() must issue exactly 4 grouped statements \
         (list_ids + sync + addresses + registrations), got {count_one}"
    );
}

/// empty database → defaults, ZERO warnings.
#[tracing_test::traced_test]
#[test]
fn tc_p4_010_empty_db_default_state() {
    let (persister, _tmp, path) = fresh_persister();
    drop(persister);
    let p2 = reopen(&path);
    let state = p2.load().unwrap();
    assert!(state.is_empty());
    assert!(logs_contain("wallets_seen=0"));
    assert!(logs_contain("wallets_pending_rehydration=0"));
}

/// A pending tombstone must never destroy an ESTABLISHED pair-row. The
/// contacts table is one row per `(owner, contact)` pair, so an unfiltered
/// `removed_incoming` / `removed_sent` DELETE aimed at a pending entry
/// would take the established row — both request blobs plus the user's
/// alias/note/hidden/accepted-accounts — with it (the ignore-an-
/// established-contact shape). The writer's state filter must scope the
/// DELETE to the pending state the tombstone describes. Was red against
/// the unfiltered DELETE.
#[test]
fn pending_tombstones_do_not_destroy_established_rows() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xA7);
    ensure_wallet_meta(&persister, &w);

    let owner = Identifier::from([0x51; 32]);
    let contact = Identifier::from([0x52; 32]);
    let est_key = SentContactRequestKey {
        owner_id: owner,
        recipient_id: contact,
    };

    // Flush 1: the pair is established, with full user metadata.
    let mut established = std::collections::BTreeMap::new();
    established.insert(est_key, established_contact(0x51, 0x52));
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                contacts: Some(ContactChangeSet {
                    established,
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();

    // Flush 2: pending tombstones for the SAME pair (both directions —
    // e.g. an `ignore_sender` emitted against an established contact by a
    // pre-guard caller). Neither may touch the established row.
    let mut cs = ContactChangeSet::default();
    cs.removed_incoming.insert(ReceivedContactRequestKey {
        owner_id: owner,
        sender_id: contact,
    });
    cs.removed_sent.insert(SentContactRequestKey {
        owner_id: owner,
        recipient_id: contact,
    });
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                contacts: Some(cs),
                ..Default::default()
            },
        )
        .unwrap();
    drop(persister);

    let p2 = reopen(&path);
    let conn = p2.lock_conn_for_test();
    let state = platform_wallet_storage::sqlite::schema::contacts::load_state_for_test(&conn, &w)
        .expect("contacts load_state");
    let survived = state
        .established
        .get(&est_key)
        .expect("established row must survive pending tombstones");
    assert_eq!(survived.alias, Some("best friend".to_string()));
    assert_eq!(survived.note, Some("met at conf".to_string()));
    assert_eq!(survived.accepted_accounts, vec![1, 7, 42]);

    // Control: the same tombstones DO delete genuinely-pending rows.
    let (persister, _tmp2, path2) = fresh_persister();
    let w2 = wid(0xA8);
    ensure_wallet_meta(&persister, &w2);
    let mut incoming = std::collections::BTreeMap::new();
    incoming.insert(
        ReceivedContactRequestKey {
            owner_id: owner,
            sender_id: contact,
        },
        contact_request_entry(0x52, 0x51),
    );
    persister
        .store(
            w2,
            PlatformWalletChangeSet {
                contacts: Some(ContactChangeSet {
                    incoming_requests: incoming,
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    let mut cs = ContactChangeSet::default();
    cs.removed_incoming.insert(ReceivedContactRequestKey {
        owner_id: owner,
        sender_id: contact,
    });
    persister
        .store(
            w2,
            PlatformWalletChangeSet {
                contacts: Some(cs),
                ..Default::default()
            },
        )
        .unwrap();
    drop(persister);
    let p3 = reopen(&path2);
    let conn = p3.lock_conn_for_test();
    let state = platform_wallet_storage::sqlite::schema::contacts::load_state_for_test(&conn, &w2)
        .expect("contacts load_state");
    assert!(
        state.incoming_requests.is_empty(),
        "a received-state row is still deleted by its tombstone"
    );
}

/// Un-ignore must be durable across a restart: the `ignored_senders`
/// TABLE (maintained transactionally by both ignore and un-ignore) is the
/// authoritative restore source — NOT the `entry_blob`'s snapshot copy,
/// which goes stale on un-ignore (nothing re-flushes the identity entry)
/// and is UNION-merged across buffered snapshots. Blob-based restore
/// resurrected the ignore, stickily. Was red against the blob-based
/// loader.
#[test]
fn tc_p4_020_unignore_survives_restart_table_is_authoritative() {
    use platform_wallet_storage::sqlite::schema::identities;
    use std::collections::BTreeMap;
    use std::collections::BTreeSet;

    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xC1);
    ensure_wallet_meta(&persister, &w);

    let owner = Identifier::from([0x61; 32]);
    let sender = Identifier::from([0x62; 32]);

    // Flush 1: the identity entry snapshot carries the sender as ignored
    // (as a live sweep would have persisted it), and the contacts
    // changeset records the ignore in the table.
    let mut entry = identity_entry(0x61, Some(0));
    entry.ignored_senders = BTreeSet::from([sender]);
    let mut identities = BTreeMap::new();
    identities.insert(entry.id, entry);
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                identities: Some(IdentityChangeSet {
                    identities,
                    removed: Default::default(),
                }),
                contacts: Some(ContactChangeSet {
                    ignored: BTreeSet::from([(owner, sender)]),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();

    // Flush 2: the user un-ignores. Production persists ONLY the contact
    // changeset (no fresh identity snapshot) — the blob still lists the
    // sender as ignored.
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                contacts: Some(ContactChangeSet {
                    unignored: BTreeSet::from([(owner, sender)]),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    drop(persister);

    // Restart: the restored identity must NOT have the sender ignored —
    // the table row was deleted; the stale blob copy must not resurrect it.
    let p2 = reopen(&path);
    let conn = p2.lock_conn_for_test();
    let state = identities::load_state(&conn, &w).expect("load_state");
    drop(conn);
    let managed = state
        .wallet_identities
        .get(&w)
        .and_then(|bucket| bucket.get(&0))
        .expect("restored identity");
    assert!(
        !managed.is_sender_ignored(&sender),
        "un-ignore must survive a restart — the stale entry_blob snapshot \
         must not resurrect the ignore"
    );

    // Control (same loader, opposite direction): with the table row still
    // present, the restore DOES ignore the sender — even when the blob
    // snapshot never recorded it (blob older than the ignore).
    let (persister, _tmp2, path2) = fresh_persister();
    let w2 = wid(0xC2);
    ensure_wallet_meta(&persister, &w2);
    let entry = identity_entry(0x61, Some(0)); // blob: no ignored senders
    let mut identities = BTreeMap::new();
    identities.insert(entry.id, entry);
    persister
        .store(
            w2,
            PlatformWalletChangeSet {
                identities: Some(IdentityChangeSet {
                    identities,
                    removed: Default::default(),
                }),
                contacts: Some(ContactChangeSet {
                    ignored: BTreeSet::from([(owner, sender)]),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    drop(persister);
    let p3 = reopen(&path2);
    let conn = p3.lock_conn_for_test();
    let state = identities::load_state(&conn, &w2).expect("load_state");
    drop(conn);
    let managed = state
        .wallet_identities
        .get(&w2)
        .and_then(|bucket| bucket.get(&0))
        .expect("restored identity");
    assert!(
        managed.is_sender_ignored(&sender),
        "an ignore recorded in the table restores even when the blob snapshot predates it"
    );
}
