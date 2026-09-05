#![allow(clippy::field_reassign_with_default)]

//! `core_address_pool` writer and read-time UTXO attribution.
//! Covers TC-B-001 (pool rows with `used` flags), TC-B-002
//! (pool-resolved account index), TC-B-010 (idempotent per-changeset pool
//! state), TC-B-015 (`key_class` survives).

mod common;

use common::{ensure_wallet_meta, fresh_persister, wid};
use key_wallet::account::{AccountType, StandardAccountType};
use key_wallet::managed_account::address_pool::{AddressPoolType, AddressState, PublicKeyType};
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
use key_wallet::wallet::Wallet;
use key_wallet::{AddressInfo, Network, Utxo};
use platform_wallet::changeset::{
    AccountAddressPoolEntry, CoreChangeSet, PersistenceError, PlatformWalletChangeSet,
    PlatformWalletPersistence, ProviderKeyAccountEntry, ProviderKeyExtendedPubKey,
    WalletMetadataEntry,
};
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet_storage::sqlite::schema::{core_pool, core_state};
use platform_wallet_storage::{SqlitePersister, SqlitePersisterConfig, WalletStorageError};

/// Real external-pool `AddressInfo`s for a wallet's Standard BIP44 account 0,
/// sorted by derivation index — genuine scripts that round-trip.
fn external_infos(seed_byte: u8) -> Vec<AddressInfo> {
    let wallet = Wallet::from_seed_bytes(
        [seed_byte; 64],
        Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .unwrap();
    let info = ManagedWalletInfo::from_wallet(&wallet, 0);
    for managed in info.all_managed_accounts() {
        if !matches!(
            managed.managed_account_type().to_account_type(),
            AccountType::Standard { index: 0, .. }
        ) {
            continue;
        }
        for pool in managed.managed_account_type().address_pools() {
            if pool.pool_type != AddressPoolType::External || pool.addresses.is_empty() {
                continue;
            }
            let mut infos: Vec<AddressInfo> = pool.addresses.values().cloned().collect();
            infos.sort_by_key(|a| a.index);
            return infos;
        }
    }
    panic!("wallet must expose a non-empty Standard BIP44 external pool");
}

fn utxo_on(info: &AddressInfo, value: u64) -> Utxo {
    use dashcore::hashes::Hash;
    Utxo {
        outpoint: dashcore::OutPoint {
            txid: dashcore::Txid::from_byte_array([info.index as u8 ^ 0x5A; 32]),
            vout: 0,
        },
        txout: dashcore::TxOut {
            value,
            script_pubkey: info.script_pubkey.clone(),
        },
        address: info.address.clone(),
        height: 10,
        is_coinbase: false,
        is_confirmed: true,
        is_instantlocked: false,
        is_locked: false,
        is_trusted: false,
    }
}

fn pool_entry(
    account_type: AccountType,
    pool_type: AddressPoolType,
    addresses: Vec<AddressInfo>,
) -> AccountAddressPoolEntry {
    AccountAddressPoolEntry {
        account_type,
        pool_type,
        addresses,
    }
}

fn wallet_storage_error(err: PersistenceError) -> Box<WalletStorageError> {
    let source = match err {
        PersistenceError::Backend { source, .. } => source,
        other => panic!("expected Backend {{ .. }}, got {other:?}"),
    };
    source
        .downcast::<WalletStorageError>()
        .unwrap_or_else(|source| panic!("expected WalletStorageError, got {source}"))
}

fn provider_platform_registration(wallet: &Wallet) -> ProviderKeyAccountEntry {
    ProviderKeyAccountEntry {
        account_type: AccountType::ProviderPlatformKeys,
        extended_public_key: ProviderKeyExtendedPubKey::EdDSA(
            wallet
                .accounts
                .eddsa_account_of_type(AccountType::ProviderPlatformKeys)
                .expect("EdDSA platform account")
                .ed25519_public_key
                .clone(),
        ),
    }
}

fn typed_platform_node_info(seed_byte: u8, index: u32, key_byte: u8) -> AddressInfo {
    let mut info = external_infos(seed_byte)
        .into_iter()
        .nth(index as usize)
        .expect("derived address at requested index");
    info.public_key = Some(PublicKeyType::EdDSA(vec![key_byte; 32]));
    info
}

fn provider_platform_pool_entry(addresses: Vec<AddressInfo>) -> AccountAddressPoolEntry {
    pool_entry(
        AccountType::ProviderPlatformKeys,
        AddressPoolType::AbsentHardened,
        addresses,
    )
}

fn loaded_provider_platform_infos(
    persister: &SqlitePersister,
    wallet_id: &WalletId,
) -> Vec<AddressInfo> {
    let state = persister.load().expect("load wallet state");
    let wallet_info = &state
        .wallets
        .get(wallet_id)
        .expect("wallet rehydrated")
        .wallet_info;
    let account = wallet_info
        .all_managed_accounts()
        .into_iter()
        .find(|managed| {
            managed.managed_account_type().to_account_type() == AccountType::ProviderPlatformKeys
        })
        .expect("restored platform-node managed account");
    account
        .managed_account_type()
        .address_pools()
        .into_iter()
        .find(|pool| pool.pool_type == AddressPoolType::AbsentHardened)
        .expect("restored platform-node hardened pool")
        .addresses
        .values()
        .cloned()
        .collect()
}

/// TC-B-001 — six pool rows with `used` set on indices {0,2,4}; the pool
/// table is a first-class row store, not a `core_utxos` derivation.
#[test]
fn tc_b_001_pool_rows_with_used_flags() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xA0);
    ensure_wallet_meta(&persister, &w);

    let mut infos = external_infos(0x11);
    infos.truncate(6);
    assert_eq!(infos.len(), 6, "need at least six derived addresses");
    for info in infos.iter_mut() {
        info.state = if matches!(info.index, 0 | 2 | 4) {
            AddressState::Used
        } else {
            AddressState::Available
        };
    }
    let entry = pool_entry(
        AccountType::Standard {
            index: 0,
            standard_account_type: StandardAccountType::BIP44Account,
        },
        AddressPoolType::External,
        infos.clone(),
    );
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                account_address_pools: vec![entry],
                ..Default::default()
            },
        )
        .unwrap();

    let conn = persister.lock_conn_for_test();
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM core_address_pool \
             WHERE wallet_id = ?1 AND account_index = 0 AND key_class = 0 AND pool_type = 0",
            rusqlite::params![w.as_slice()],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 6, "exactly six scoped rows");

    for info in &infos {
        let used: i64 = conn
            .query_row(
                "SELECT used FROM core_address_pool \
                 WHERE wallet_id = ?1 AND account_index = 0 AND key_class = 0 \
                   AND pool_type = 0 AND address_index = ?2",
                rusqlite::params![w.as_slice(), i64::from(info.index)],
                |r| r.get(0),
            )
            .unwrap();
        let expect = i64::from(matches!(info.index, 0 | 2 | 4));
        assert_eq!(used, expect, "used flag for index {}", info.index);
    }
}

#[test]
fn reserved_address_persists_reservation_timestamp() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xB0);
    ensure_wallet_meta(&persister, &w);

    let mut info = external_infos(0xB0).remove(0);
    let reserved_at = 1_752_528_623;
    info.state = AddressState::Reserved { at: reserved_at };
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                account_address_pools: vec![pool_entry(
                    AccountType::Standard {
                        index: 0,
                        standard_account_type: StandardAccountType::BIP44Account,
                    },
                    AddressPoolType::External,
                    vec![info],
                )],
                ..Default::default()
            },
        )
        .unwrap();

    let conn = persister.lock_conn_for_test();
    let (used, stored_reserved_at): (i64, Option<i64>) = conn
        .query_row(
            "SELECT used, reserved_at FROM core_address_pool WHERE wallet_id = ?1",
            rusqlite::params![w.as_slice()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(used, 0);
    assert_eq!(stored_reserved_at, Some(reserved_at as i64));
}

#[test]
fn available_and_used_addresses_persist_without_reservation_timestamp() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xB1);
    ensure_wallet_meta(&persister, &w);

    let mut infos = external_infos(0xB1);
    infos.truncate(2);
    infos[0].state = AddressState::Available;
    infos[1].state = AddressState::Used;
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                account_address_pools: vec![pool_entry(
                    AccountType::Standard {
                        index: 0,
                        standard_account_type: StandardAccountType::BIP44Account,
                    },
                    AddressPoolType::External,
                    infos,
                )],
                ..Default::default()
            },
        )
        .unwrap();

    let conn = persister.lock_conn_for_test();
    let mut stmt = conn
        .prepare(
            "SELECT used, reserved_at FROM core_address_pool \
             WHERE wallet_id = ?1 ORDER BY address_index",
        )
        .unwrap();
    let rows = stmt
        .query_map(rusqlite::params![w.as_slice()], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, Option<i64>>(1)?))
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(rows, vec![(0, None), (1, None)]);
}

#[test]
fn used_address_cannot_regain_reservation_from_stale_snapshot() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xB2);
    ensure_wallet_meta(&persister, &w);

    let mut info = external_infos(0xB2).remove(0);
    let account_type = AccountType::Standard {
        index: 0,
        standard_account_type: StandardAccountType::BIP44Account,
    };
    info.state = AddressState::Used;
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                account_address_pools: vec![pool_entry(
                    account_type,
                    AddressPoolType::External,
                    vec![info.clone()],
                )],
                ..Default::default()
            },
        )
        .unwrap();

    info.state = AddressState::Reserved { at: 1_752_528_624 };
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                account_address_pools: vec![pool_entry(
                    account_type,
                    AddressPoolType::External,
                    vec![info],
                )],
                ..Default::default()
            },
        )
        .unwrap();

    let conn = persister.lock_conn_for_test();
    let state: (i64, Option<i64>) = conn
        .query_row(
            "SELECT used, reserved_at FROM core_address_pool WHERE wallet_id = ?1",
            rusqlite::params![w.as_slice()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(state, (1, None));
}

/// TC-B-002 — UTXOs resolve to their pool-declared account during reads.
#[test]
fn tc_b_002_account_index_is_resolved_from_pool() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xA2);
    ensure_wallet_meta(&persister, &w);

    let infos = external_infos(0x22);
    let addr0 = infos[0].clone();
    let addr1 = infos[1].clone();

    // Pools declaring the address' owning account: addr0 -> account 0,
    // addr1 -> account 1 (non-default).
    let pools = vec![
        pool_entry(
            AccountType::Standard {
                index: 0,
                standard_account_type: StandardAccountType::BIP44Account,
            },
            AddressPoolType::External,
            vec![addr0.clone()],
        ),
        pool_entry(
            AccountType::Standard {
                index: 1,
                standard_account_type: StandardAccountType::BIP44Account,
            },
            AddressPoolType::External,
            vec![addr1.clone()],
        ),
    ];
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                account_address_pools: pools,
                core: Some(CoreChangeSet {
                    new_utxos: vec![utxo_on(&addr0, 111), utxo_on(&addr1, 222)],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();

    let conn = persister.lock_conn_for_test();
    let by_account = core_state::list_unspent_utxos(&conn, &w).unwrap();
    assert_eq!(
        by_account.get(&1).map(|rows| rows[0].value),
        Some(222),
        "UTXO on account 1's address must resolve to account 1"
    );
    assert_eq!(
        by_account.get(&0).map(|rows| rows[0].value),
        Some(111),
        "UTXO on account 0's address must resolve to account 0"
    );
}

/// A UTXO whose script matches no pool row resolves to the fallback account.
#[test]
fn utxo_without_pool_row_resolves_to_account_zero() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xA3);
    ensure_wallet_meta(&persister, &w);

    let infos = external_infos(0x33);
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    new_utxos: vec![utxo_on(&infos[0], 500)],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();

    let conn = persister.lock_conn_for_test();
    let by_account = core_state::list_unspent_utxos(&conn, &w).unwrap();
    assert_eq!(by_account.get(&0).map(|rows| rows[0].value), Some(500));
}

/// TC-B-010 — a used-flag flip persists and a second no-op flush leaves the
/// pool rows unchanged; `used` is monotonic and never reverts.
#[test]
fn tc_b_010_pool_state_idempotent_and_monotonic() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xA4);
    ensure_wallet_meta(&persister, &w);

    let mut infos = external_infos(0x44);
    infos.truncate(3);
    let mk = |infos: &[AddressInfo]| {
        pool_entry(
            AccountType::Standard {
                index: 0,
                standard_account_type: StandardAccountType::BIP44Account,
            },
            AddressPoolType::External,
            infos.to_vec(),
        )
    };
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                account_address_pools: vec![mk(&infos)],
                ..Default::default()
            },
        )
        .unwrap();

    // Flip index 1 to used.
    let mut flipped = infos.clone();
    flipped[1].state = AddressState::Used;
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                account_address_pools: vec![mk(&flipped)],
                ..Default::default()
            },
        )
        .unwrap();

    let used_of = |conn: &rusqlite::Connection, idx: u32| -> i64 {
        conn.query_row(
            "SELECT used FROM core_address_pool \
             WHERE wallet_id = ?1 AND account_index = 0 AND pool_type = 0 AND address_index = ?2",
            rusqlite::params![w.as_slice(), i64::from(idx)],
            |r| r.get(0),
        )
        .unwrap()
    };
    {
        let conn = persister.lock_conn_for_test();
        assert_eq!(used_of(&conn, 1), 1, "flip must persist");
        assert_eq!(used_of(&conn, 0), 0, "unrelated row unchanged");
    }

    // A stale snapshot with used=false for index 1 must NOT un-use it.
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                account_address_pools: vec![mk(&infos)],
                ..Default::default()
            },
        )
        .unwrap();
    let conn = persister.lock_conn_for_test();
    assert_eq!(
        used_of(&conn, 1),
        1,
        "used is monotonic — a stale snapshot never reverts it"
    );
}

/// TC-B-015 — a non-default `key_class` round-trips into the pool row's PK.
#[test]
fn tc_b_015_key_class_survives() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xA5);
    ensure_wallet_meta(&persister, &w);

    let infos = external_infos(0x55);
    let entry = pool_entry(
        AccountType::PlatformPayment {
            account: 2,
            key_class: 1,
        },
        AddressPoolType::External,
        vec![infos[0].clone()],
    );
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                account_address_pools: vec![entry],
                ..Default::default()
            },
        )
        .unwrap();

    let conn = persister.lock_conn_for_test();
    let (account_index, key_class): (i64, i64) = conn
        .query_row(
            "SELECT account_index, key_class FROM core_address_pool WHERE wallet_id = ?1",
            rusqlite::params![w.as_slice()],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(account_index, 2, "PlatformPayment account index");
    assert_eq!(key_class, 1, "non-default key_class must survive");
}

/// A single external `AddressInfo` at derivation index 0 for a seed, with a
/// chosen `used` flag. Two seeds yield distinct scripts so a cross-account
/// overwrite is observable.
fn index_zero_info(seed_byte: u8, used: bool) -> Vec<AddressInfo> {
    let mut infos = external_infos(seed_byte);
    infos.truncate(1);
    infos[0].state = if used {
        AddressState::Used
    } else {
        AddressState::Available
    };
    infos
}

/// Assert the pool rows for `(wallet, account_type)` are exactly `(script,
/// used)`, and that `total` rows exist for the wallet overall.
fn assert_pool_row(
    persister: &platform_wallet_storage::SqlitePersister,
    w: &WalletId,
    label: &str,
    want_script: &[u8],
    want_used: i64,
) {
    let conn = persister.lock_conn_for_test();
    let (script, used): (Vec<u8>, i64) = conn
        .query_row(
            "SELECT script, used FROM core_address_pool \
             WHERE wallet_id = ?1 AND account_type = ?2",
            rusqlite::params![w.as_slice(), label],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap_or_else(|e| panic!("expected exactly one row for {label}: {e}"));
    assert_eq!(script, want_script, "{label} script must survive verbatim");
    assert_eq!(used, want_used, "{label} used flag must survive");
}

/// Two account types that both collapse to the `(account_index=0,
/// key_class=0)` sentinel — `IdentityRegistration` and `ProviderVotingKeys` —
/// must not overwrite each other's pool rows. Before the PK was widened with
/// `account_type` they upserted onto one PK tuple, silently losing one
/// account's `script` and merging `used`.
#[test]
fn distinct_account_types_sharing_index_zero_do_not_collide() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xA6);
    ensure_wallet_meta(&persister, &w);

    let id_reg = index_zero_info(0x61, true);
    let prov = index_zero_info(0x62, false);
    assert_ne!(
        id_reg[0].script_pubkey, prov[0].script_pubkey,
        "the two account types must carry distinct scripts to prove no overwrite"
    );

    persister
        .store(
            w,
            PlatformWalletChangeSet {
                account_address_pools: vec![
                    pool_entry(
                        AccountType::IdentityRegistration,
                        AddressPoolType::External,
                        id_reg.clone(),
                    ),
                    pool_entry(
                        AccountType::ProviderVotingKeys,
                        AddressPoolType::External,
                        prov.clone(),
                    ),
                ],
                ..Default::default()
            },
        )
        .unwrap();

    assert_pool_row(
        &persister,
        &w,
        "identity_registration",
        id_reg[0].script_pubkey.as_bytes(),
        1,
    );
    assert_pool_row(
        &persister,
        &w,
        "provider_voting",
        prov[0].script_pubkey.as_bytes(),
        0,
    );
    let conn = persister.lock_conn_for_test();
    let total: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM core_address_pool WHERE wallet_id = ?1",
            rusqlite::params![w.as_slice()],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(total, 2, "both account types must persist as separate rows");
}

/// `Standard { index: 0 }` and `CoinJoin { index: 0 }` also both map to
/// `(account_index=0, key_class=0)` yet are distinct accounts; the
/// `account_type` discriminator (`standard_bip44` vs `coinjoin`) must keep
/// their pool rows separate.
#[test]
fn standard_and_coinjoin_index_zero_do_not_collide() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xA7);
    ensure_wallet_meta(&persister, &w);

    let std0 = index_zero_info(0x71, true);
    let cj0 = index_zero_info(0x72, false);
    assert_ne!(
        std0[0].script_pubkey, cj0[0].script_pubkey,
        "the two account types must carry distinct scripts to prove no overwrite"
    );

    persister
        .store(
            w,
            PlatformWalletChangeSet {
                account_address_pools: vec![
                    pool_entry(
                        AccountType::Standard {
                            index: 0,
                            standard_account_type: StandardAccountType::BIP44Account,
                        },
                        AddressPoolType::External,
                        std0.clone(),
                    ),
                    pool_entry(
                        AccountType::CoinJoin { index: 0 },
                        AddressPoolType::External,
                        cj0.clone(),
                    ),
                ],
                ..Default::default()
            },
        )
        .unwrap();

    assert_pool_row(
        &persister,
        &w,
        "standard_bip44",
        std0[0].script_pubkey.as_bytes(),
        1,
    );
    assert_pool_row(
        &persister,
        &w,
        "coinjoin",
        cj0[0].script_pubkey.as_bytes(),
        0,
    );
}

/// Two DashPay contacts on one wallet both collapse to
/// `(account_type='dashpay_receiving', account_index=0, key_class=0)` — the
/// same `user_identity_id`, distinct `friend_identity_id`. Before the PK
/// carried the DashPay identity pair, the second contact's pool row would
/// silently overwrite the first's via `ON CONFLICT DO UPDATE`.
#[test]
fn distinct_dashpay_friends_do_not_collide_in_pool() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xA8);
    ensure_wallet_meta(&persister, &w);

    let user_identity_id = [0xABu8; 32];
    let friend_a = index_zero_info(0x81, true);
    let friend_b = index_zero_info(0x82, false);
    assert_ne!(
        friend_a[0].script_pubkey, friend_b[0].script_pubkey,
        "the two contacts must carry distinct scripts to prove no overwrite"
    );

    let dashpay_account = |friend_identity_id: [u8; 32]| AccountType::DashpayReceivingFunds {
        index: 0,
        user_identity_id,
        friend_identity_id,
    };

    persister
        .store(
            w,
            PlatformWalletChangeSet {
                account_address_pools: vec![
                    pool_entry(
                        dashpay_account([0x01; 32]),
                        AddressPoolType::External,
                        friend_a.clone(),
                    ),
                    pool_entry(
                        dashpay_account([0x02; 32]),
                        AddressPoolType::External,
                        friend_b.clone(),
                    ),
                ],
                ..Default::default()
            },
        )
        .unwrap();

    let conn = persister.lock_conn_for_test();
    let assert_friend_row = |friend_identity_id: [u8; 32], want_script: &[u8], want_used: i64| {
        let (script, used): (Vec<u8>, i64) = conn
            .query_row(
                "SELECT script, used FROM core_address_pool \
                 WHERE wallet_id = ?1 AND account_type = 'dashpay_receiving' \
                 AND user_identity_id = ?2 AND friend_identity_id = ?3",
                rusqlite::params![w.as_slice(), &user_identity_id[..], &friend_identity_id[..]],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap_or_else(|e| {
                panic!("expected exactly one row for friend {friend_identity_id:?}: {e}")
            });
        assert_eq!(
            script, want_script,
            "contact's script must survive verbatim"
        );
        assert_eq!(used, want_used, "contact's used flag must survive");
    };
    assert_friend_row([0x01; 32], friend_a[0].script_pubkey.as_bytes(), 1);
    assert_friend_row([0x02; 32], friend_b[0].script_pubkey.as_bytes(), 0);

    let total: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM core_address_pool WHERE wallet_id = ?1",
            rusqlite::params![w.as_slice()],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(total, 2, "both contacts must persist as separate rows");
}

/// Repro for a real gap found while merging PR #4117 with upstream PR #4127.
/// PR #4127 replaced the removed `derived_platform_node_keys` persistence with
/// generic `account_address_pools` snapshots, but SQLite's `core_pool.rs` and
/// `persister.rs` do not carry or restore the raw platform-node public key.
/// Reference: dashpay/platform#4113.
#[test]
fn platform_node_key_public_keys_survive_sqlite_store_and_load() {
    let wallet = Wallet::from_seed_bytes(
        [0x33u8; 64],
        Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .unwrap();
    let keys = platform_wallet::wallet::provider_key_at_index::derive_platform_node_public_keys(
        &wallet,
        Network::Testnet,
        3,
    )
    .expect("derive");
    let mut wallet_info = ManagedWalletInfo::from_wallet(&wallet, 0);
    platform_wallet::wallet::provider_key_at_index::populate_platform_node_pool(
        &mut wallet_info,
        &keys,
        Network::Testnet,
    )
    .expect("populate");

    let platform_node_account = wallet_info
        .all_managed_accounts()
        .into_iter()
        .find(|managed| {
            managed.managed_account_type().to_account_type() == AccountType::ProviderPlatformKeys
        })
        .expect("platform-node managed account");
    let platform_node_pool = platform_node_account
        .managed_account_type()
        .address_pools()
        .into_iter()
        .find(|pool| pool.pool_type == AddressPoolType::AbsentHardened)
        .expect("platform-node hardened pool");
    let addresses = platform_node_pool
        .addresses
        .values()
        .cloned()
        .collect::<Vec<AddressInfo>>();
    assert_eq!(
        addresses.len(),
        3,
        "the in-memory platform-node pool must contain all three derived keys"
    );
    assert!(
        addresses.iter().all(|info| info.public_key.is_some()),
        "the in-memory platform-node pool must carry every derived public key"
    );
    let pool_entry = AccountAddressPoolEntry {
        account_type: AccountType::ProviderPlatformKeys,
        pool_type: AddressPoolType::AbsentHardened,
        addresses,
    };
    let provider_registration = ProviderKeyAccountEntry {
        account_type: AccountType::ProviderPlatformKeys,
        extended_public_key: ProviderKeyExtendedPubKey::EdDSA(
            wallet
                .accounts
                .eddsa_account_of_type(AccountType::ProviderPlatformKeys)
                .expect("eddsa account")
                .ed25519_public_key
                .clone(),
        ),
    };

    let (persister, _tmp, path) = fresh_persister();
    let w: WalletId = wid(0x99);
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                wallet_metadata: Some(WalletMetadataEntry {
                    network: Network::Testnet,
                    wallet_group_id: [0; 32],
                    birth_height: 1,
                }),
                provider_key_account_registrations: vec![provider_registration],
                account_address_pools: vec![pool_entry],
                ..Default::default()
            },
        )
        .expect("store");
    drop(persister);

    let persister =
        SqlitePersister::open(SqlitePersisterConfig::new(&path)).expect("reopen persister");
    let state = persister.load().expect("load");
    let restored = &state
        .wallets
        .get(&w)
        .expect("wallet rehydrated")
        .wallet_info;
    let restored_platform_node_account = restored
        .all_managed_accounts()
        .into_iter()
        .find(|managed| {
            managed.managed_account_type().to_account_type() == AccountType::ProviderPlatformKeys
        })
        .expect("restored platform-node managed account");
    let restored_platform_node_pool = restored_platform_node_account
        .managed_account_type()
        .address_pools()
        .into_iter()
        .find(|pool| pool.pool_type == AddressPoolType::AbsentHardened)
        .expect("restored platform-node hardened pool");

    assert_eq!(
        restored_platform_node_pool.addresses.len(),
        3,
        "all three platform-node indices must survive SQLite store()->load()"
    );
    for key in &keys {
        let restored_info = restored_platform_node_pool
            .addresses
            .get(&key.index)
            .unwrap_or_else(|| panic!("platform-node index {} did not survive SQLite", key.index));
        let expected = Some(PublicKeyType::EdDSA(key.public_key.to_vec()));
        assert_eq!(
            restored_info.public_key, expected,
            "platform-node public key at index {} did not survive SQLite store()->load() \
             — see doc comment: core_pool.rs never persists AddressInfo.public_key",
            key.index
        );
    }
}

#[test]
fn conflicting_typed_pool_key_is_rejected_and_original_survives_load() {
    let wallet = Wallet::from_seed_bytes(
        [0xA1; 64],
        Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .expect("seed wallet");
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0x9A);
    let first = typed_platform_node_info(0xA2, 0, 0x11);
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                wallet_metadata: Some(WalletMetadataEntry {
                    network: Network::Testnet,
                    wallet_group_id: [0; 32],
                    birth_height: 1,
                }),
                provider_key_account_registrations: vec![provider_platform_registration(&wallet)],
                account_address_pools: vec![provider_platform_pool_entry(vec![first.clone()])],
                ..Default::default()
            },
        )
        .expect("store original typed pool key");

    let fresh_sibling = typed_platform_node_info(0xA2, 1, 0x22);
    let mut conflicting = first.clone();
    conflicting.public_key = Some(PublicKeyType::EdDSA(vec![0x33; 32]));
    let err = persister
        .store(
            w,
            PlatformWalletChangeSet {
                account_address_pools: vec![provider_platform_pool_entry(vec![
                    fresh_sibling,
                    conflicting,
                ])],
                ..Default::default()
            },
        )
        .expect_err("a different typed key at the same pool index must be rejected");
    let storage_error = wallet_storage_error(err);
    assert_eq!(storage_error.error_kind_str(), "typed_pool_key_conflict");

    let restored = loaded_provider_platform_infos(&persister, &w);
    assert_eq!(restored.len(), 1, "the rejected flush must be atomic");
    assert_eq!(
        restored[0].public_key, first.public_key,
        "the original typed key must remain intact"
    );
}

#[test]
fn untyped_pool_key_cannot_overwrite_persisted_typed_key() {
    let wallet = Wallet::from_seed_bytes(
        [0xA3; 64],
        Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .expect("seed wallet");
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0x9D);
    let original = typed_platform_node_info(0xA4, 0, 0x71);
    let mut untyped = original.clone();
    untyped.public_key = None;
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                wallet_metadata: Some(WalletMetadataEntry {
                    network: Network::Testnet,
                    wallet_group_id: [0; 32],
                    birth_height: 1,
                }),
                provider_key_account_registrations: vec![provider_platform_registration(&wallet)],
                account_address_pools: vec![provider_platform_pool_entry(vec![untyped.clone()])],
                ..Default::default()
            },
        )
        .expect("store initial untyped pool row");
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                account_address_pools: vec![provider_platform_pool_entry(vec![original.clone()])],
                ..Default::default()
            },
        )
        .expect("upgrade untyped pool row with typed key material");

    let err = persister
        .store(
            w,
            PlatformWalletChangeSet {
                account_address_pools: vec![provider_platform_pool_entry(vec![untyped])],
                ..Default::default()
            },
        )
        .expect_err("an untyped row must not erase a persisted typed key");
    assert_eq!(
        wallet_storage_error(err).error_kind_str(),
        "typed_pool_key_conflict"
    );

    let restored = loaded_provider_platform_infos(&persister, &w);
    assert_eq!(restored.len(), 1, "the rejected flush must be atomic");
    assert_eq!(restored[0].public_key, original.public_key);
}

#[test]
fn malformed_typed_pool_key_widths_are_rejected_before_insert() {
    let malformed_keys = [
        PublicKeyType::ECDSA(vec![0x11; 32]),
        PublicKeyType::EdDSA(vec![0x22; 31]),
        PublicKeyType::BLS(vec![0x33; 47]),
    ];

    for (case, malformed_key) in malformed_keys.into_iter().enumerate() {
        let (persister, _tmp, _path) = fresh_persister();
        let w: WalletId = wid(0xA0 + case as u8);
        ensure_wallet_meta(&persister, &w);
        let mut info = external_infos(0xA5 + case as u8)
            .into_iter()
            .next()
            .expect("derived address");
        info.public_key = Some(malformed_key);

        let err = persister
            .store(
                w,
                PlatformWalletChangeSet {
                    account_address_pools: vec![pool_entry(
                        AccountType::Standard {
                            index: 0,
                            standard_account_type: StandardAccountType::BIP44Account,
                        },
                        AddressPoolType::External,
                        vec![info],
                    )],
                    ..Default::default()
                },
            )
            .expect_err("a malformed typed key must be rejected before commit");
        assert_eq!(wallet_storage_error(err).error_kind_str(), "blob_decode");

        let conn = persister.lock_conn_for_test();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM core_address_pool WHERE wallet_id = ?1",
                rusqlite::params![w.as_slice()],
                |row| row.get(0),
            )
            .expect("count pool rows");
        assert_eq!(count, 0, "malformed key case {case} reached the database");
    }
}

#[test]
fn typed_pool_loader_rejects_mismatched_key_nullability() {
    for (case, clear_column) in ["key_type", "public_key"].into_iter().enumerate() {
        let (persister, _tmp, _path) = fresh_persister();
        let w: WalletId = wid(0xB0 + case as u8);
        ensure_wallet_meta(&persister, &w);
        let account_type = AccountType::Standard {
            index: 0,
            standard_account_type: StandardAccountType::BIP44Account,
        };
        let mut info = external_infos(0xB5 + case as u8)
            .into_iter()
            .next()
            .expect("derived address");
        info.public_key = Some(PublicKeyType::ECDSA(vec![0x44; 33]));
        persister
            .store(
                w,
                PlatformWalletChangeSet {
                    account_address_pools: vec![pool_entry(
                        account_type,
                        AddressPoolType::External,
                        vec![info],
                    )],
                    ..Default::default()
                },
            )
            .expect("store valid typed pool row");

        let conn = persister.lock_conn_for_test();
        conn.execute(
            &format!("UPDATE core_address_pool SET {clear_column} = NULL WHERE wallet_id = ?1"),
            rusqlite::params![w.as_slice()],
        )
        .expect("corrupt paired nullable columns");
        let err =
            core_pool::load_typed_pool_entries(&conn, &w, &account_type, AddressPoolType::External)
                .expect_err("mismatched typed-key nullability must fail hard");
        assert_eq!(err.error_kind_str(), "blob_decode", "case {clear_column}");
    }
}

#[test]
fn identical_typed_pool_key_is_idempotent() {
    let wallet = Wallet::from_seed_bytes(
        [0xB1; 64],
        Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .expect("seed wallet");
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0x9B);
    let info = typed_platform_node_info(0xB2, 0, 0x44);
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                wallet_metadata: Some(WalletMetadataEntry {
                    network: Network::Testnet,
                    wallet_group_id: [0; 32],
                    birth_height: 1,
                }),
                provider_key_account_registrations: vec![provider_platform_registration(&wallet)],
                account_address_pools: vec![provider_platform_pool_entry(vec![info.clone()])],
                ..Default::default()
            },
        )
        .expect("store original typed pool key");
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                account_address_pools: vec![provider_platform_pool_entry(vec![info.clone()])],
                ..Default::default()
            },
        )
        .expect("re-store identical typed pool key");

    let restored = loaded_provider_platform_infos(&persister, &w);
    assert_eq!(restored.len(), 1);
    assert_eq!(restored[0].public_key, info.public_key);
}

#[test]
fn typed_pool_key_at_fresh_index_succeeds() {
    let wallet = Wallet::from_seed_bytes(
        [0xC1; 64],
        Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .expect("seed wallet");
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0x9C);
    let first = typed_platform_node_info(0xC2, 0, 0x55);
    let fresh = typed_platform_node_info(0xC2, 1, 0x66);
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                wallet_metadata: Some(WalletMetadataEntry {
                    network: Network::Testnet,
                    wallet_group_id: [0; 32],
                    birth_height: 1,
                }),
                provider_key_account_registrations: vec![provider_platform_registration(&wallet)],
                account_address_pools: vec![provider_platform_pool_entry(vec![first])],
                ..Default::default()
            },
        )
        .expect("store initial typed pool key");
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                account_address_pools: vec![provider_platform_pool_entry(vec![fresh.clone()])],
                ..Default::default()
            },
        )
        .expect("store typed pool key at a fresh index");

    let restored = loaded_provider_platform_infos(&persister, &w);
    assert_eq!(restored.len(), 2);
    assert_eq!(
        restored
            .iter()
            .find(|info| info.index == fresh.index)
            .expect("fresh index restored")
            .public_key,
        fresh.public_key
    );
}
