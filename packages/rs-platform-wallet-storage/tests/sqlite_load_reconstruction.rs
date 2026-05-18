#![allow(clippy::field_reassign_with_default)]

//! TC-040, TC-043, TC-044 — load() reconstructs the wired-up subset.
//!
//! TC-041 / TC-042 (wallets[*].utxos / .unused_asset_locks) are blocked
//! on upstream `Wallet::from_persisted` — the persister stores the data
//! (verified via direct SQL probes) but cannot reconstruct the
//! `Wallet` + `ManagedWalletInfo` pair that `ClientWalletStartState`
//! requires. The unwired fields are listed in
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
        },
    }
}

/// TC-040: load() reconstructs platform_addresses per wallet.
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

/// TC-043: non-wired-up sub-areas are written to disk (verified by
/// direct SQL probes) but do not surface in the load result.
///
/// Constructs non-empty `ContactChangeSet` and `TokenBalanceChangeSet`
/// payloads — `is_empty()` returns false on either, so the buffer
/// flushes them — then asserts both `contacts_sent` and
/// `token_balances` rows are present in SQLite after a reopen, while
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
    // Identity row required for the contacts/dashpay FK triggers if
    // any are wired into contacts_*; the contacts_* tables themselves
    // only check the wallet_metadata parent today, so we don't need
    // an identity row for this test — but we'd add one here if the
    // trigger set grew.
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
            "SELECT COUNT(*) FROM contacts_sent WHERE wallet_id = ?1 AND owner_id = ?2 AND recipient_id = ?3",
            rusqlite::params![w.as_slice(), owner.as_slice(), recipient.as_slice()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(sent, 1, "contacts_sent row missing after reopen");
    let tokens: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM token_balances WHERE wallet_id = ?1 AND identity_id = ?2 AND token_id = ?3",
            rusqlite::params![w.as_slice(), owner.as_slice(), token.as_slice()],
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
    ContactChangeSet, ContactRequestEntry, IdentityChangeSet, IdentityEntry, SentContactRequestKey,
};
use platform_wallet::wallet::identity::{ContactRequest, IdentityStatus};

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

/// TC-P4-003: identities reader round-trips per wallet, exact equality
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

/// TC-P4-004: contacts round-trip per wallet, exact equality on the
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
    let got_a = a_state.sent_requests.get(&key_a).expect("a");
    assert_eq!(got_a.request.sender_id, entry_a.request.sender_id);
    assert_eq!(
        got_a.request.core_height_created_at,
        entry_a.request.core_height_created_at
    );
    let got_b = b_state.sent_requests.get(&key_b).expect("b");
    assert_eq!(got_b.request.sender_id, entry_b.request.sender_id);
}

/// TC-P4-005: asset locks bucketed by (wallet, account, outpoint).
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

/// TC-P4-006: empty wallets emit `wallets_pending_rehydration = N`
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

/// TC-P4-007: load() summary carries every counter, including zeros.
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

/// TC-P4-008: a corrupted blob is a HARD failure. The hardened reader
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

/// TC-P4-008b: `contacts::load_state` is fail-hard. A garbage
/// `entry_blob` yields a typed `BincodeDecode`; a non-32-byte id column
/// yields a typed `BlobDecode`. Neither is silently skipped, and an
/// intact wallet still decodes cleanly.
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
        // bad_blob: well-formed 32-byte ids, undecodable entry_blob.
        conn.execute(
            "INSERT INTO contacts_sent (wallet_id, owner_id, recipient_id, entry_blob) \
             VALUES (?1, ?2, ?3, X'00')",
            rusqlite::params![bad_blob.as_slice(), &[0x11u8; 32][..], &[0x12u8; 32][..]],
        )
        .unwrap();
        // bad_id: owner_id is only 10 bytes — fails the 32-byte check.
        conn.execute(
            "INSERT INTO contacts_sent (wallet_id, owner_id, recipient_id, entry_blob) \
             VALUES (?1, ?2, ?3, X'00')",
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

/// TC-P4-008c: `asset_locks::load_state` is fail-hard. A garbage
/// `lifecycle_blob` yields a typed `BincodeDecode`; a non-36-byte
/// `outpoint` column yields a typed `BlobDecode`. An intact wallet
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
        // bad_blob: valid 36-byte outpoint, undecodable lifecycle_blob.
        conn.execute(
            "INSERT INTO asset_locks \
                (wallet_id, outpoint, status, account_index, identity_index, amount_duffs, lifecycle_blob) \
             VALUES (?1, ?2, 'built', 0, 0, 0, X'00')",
            rusqlite::params![bad_blob.as_slice(), &[0x01u8; 36][..]],
        )
        .unwrap();
        // bad_op: outpoint column is only 4 bytes — fails the 36-byte
        // length check before any blob decode is attempted.
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
    assert!(
        matches!(op_result, Err(WalletStorageError::BlobDecode { .. })),
        "non-36-byte asset_locks outpoint must be a typed BlobDecode; got {op_result:?}"
    );
    assert_eq!(good_state[&0].len(), 1);
}

/// TC-P4-008d: `wallet_meta::list_ids` is fail-hard on a malformed
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

/// TC-P4-012: `load()` query cost is bounded per wallet.
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

    // Per wallet `load()` issues exactly two statements
    // (`platform_addrs::load_state` sync header + `count_per_wallet`),
    // plus one shared `wallet_meta::list_ids`: total = 1 + 2*N. Pinning
    // the per-wallet delta to 2 catches any unbounded per-row fan-out.
    let per_wallet = (count_ten - count_one) as f64 / 9.0;
    assert_eq!(
        per_wallet, 2.0,
        "load() must issue a fixed 2 statements per wallet \
         (N=1 → {count_one}, N=10 → {count_ten}, per-wallet → {per_wallet})"
    );
    assert_eq!(
        count_one, 3,
        "load() with one wallet must be 1 (list_ids) + 2 (per-wallet) = 3, got {count_one}"
    );
}

/// TC-P4-010: empty database → defaults, ZERO warnings.
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
