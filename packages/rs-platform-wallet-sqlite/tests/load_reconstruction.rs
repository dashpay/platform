#![allow(clippy::field_reassign_with_default)]

//! TC-040, TC-043, TC-044 — load() reconstructs the wired-up subset.
//!
//! TC-041 / TC-042 (wallets[*].utxos / .unused_asset_locks) are blocked
//! on upstream `Wallet::from_persisted` — the persister stores the data
//! (verified via direct SQL probes) but cannot reconstruct the
//! `Wallet` + `ManagedWalletInfo` pair that `ClientWalletStartState`
//! requires. They're tracked in a TODO in `persister.rs::load`.

mod common;

use common::{ensure_wallet_meta, fresh_persister, wid};
use dash_sdk::platform::address_sync::AddressFunds;
use key_wallet::PlatformP2PKHAddress;
use platform_wallet::changeset::{
    PlatformAddressBalanceEntry, PlatformAddressChangeSet, PlatformWalletChangeSet,
    PlatformWalletPersistence,
};

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
    let p2 = platform_wallet_sqlite::SqlitePersister::open(
        platform_wallet_sqlite::SqlitePersisterConfig::new(&path),
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
    let p2 = platform_wallet_sqlite::SqlitePersister::open(
        platform_wallet_sqlite::SqlitePersisterConfig::new(&path),
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
