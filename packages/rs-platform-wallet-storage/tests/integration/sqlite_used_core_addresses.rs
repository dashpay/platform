#![allow(clippy::field_reassign_with_default)]

//! `load()` marks a used-then-emptied address as used in the assembled
//! `core_wallet_info` pools (derived from the full `core_utxos` set, spent +
//! unspent) so the rehydrated wallet never hands it out again as a fresh
//! receive address (address reuse).

use crate::common;

use common::{fresh_persister, wid};
use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
use key_wallet::wallet::Wallet;
use platform_wallet::changeset::{
    AccountRegistrationEntry, CoreChangeSet, PlatformWalletChangeSet, PlatformWalletPersistence,
    WalletMetadataEntry,
};
use platform_wallet_storage::{SqlitePersister, SqlitePersisterConfig};

fn reopen(path: &std::path::Path) -> SqlitePersister {
    SqlitePersister::open(SqlitePersisterConfig::new(path)).expect("reopen")
}

/// A spent (zero-balance) UTXO's address is still reported in
/// `used_core_addresses`, even though it no longer contributes to balance.
#[test]
fn spent_utxo_address_is_marked_used() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xDD);

    let wallet = Wallet::from_seed_bytes(
        [0x42; 64],
        key_wallet::Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .unwrap();
    let info = ManagedWalletInfo::from_wallet(&wallet, 7);
    let address = WalletInfoInterface::monitored_addresses(&info)
        .into_iter()
        .next()
        .unwrap();

    // Register the wallet so it appears in load()'s payload.
    let manifest: Vec<AccountRegistrationEntry> = wallet
        .accounts
        .all_accounts()
        .into_iter()
        .map(|a| AccountRegistrationEntry {
            account_type: a.account_type,
            account_xpub: a.account_xpub,
        })
        .collect();
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                wallet_metadata: Some(WalletMetadataEntry {
                    network: key_wallet::Network::Testnet,
                    wallet_group_id: [0u8; 32],
                    birth_height: 7,
                }),
                account_registrations: manifest,
                ..Default::default()
            },
        )
        .unwrap();

    // A UTXO on `address`, then spend it: the row stays on disk with
    // spent = 1 and contributes no balance.
    let utxo = key_wallet::Utxo {
        outpoint: dashcore::OutPoint {
            txid: { dashcore::Txid::from_byte_array([0x99; 32]) },
            vout: 0,
        },
        txout: dashcore::TxOut {
            value: 500_000,
            script_pubkey: address.script_pubkey(),
        },
        address: address.clone(),
        height: 5,
        is_coinbase: false,
        is_confirmed: true,
        is_instantlocked: false,
        is_locked: false,
        is_trusted: false,
    };
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    new_utxos: vec![utxo.clone()],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    spent_utxos: vec![utxo],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    drop(persister);

    let p2 = reopen(&path);
    let state = p2.load().expect("load");
    let slice = state.wallets.get(&w).expect("wallet slice");

    // Spent UTXO: contributes no balance, but its address is still marked used
    // in the assembled wallet's pool so it is never handed out as fresh again.
    assert_eq!(
        slice.wallet_info.balance.total(),
        0,
        "the spent UTXO must not contribute balance"
    );
    let funds = slice
        .wallet_info
        .accounts
        .all_funding_accounts()
        .into_iter()
        .next()
        .expect("funds account present");
    let marked_used = funds
        .managed_account_type()
        .address_pools()
        .iter()
        .any(|p| {
            p.address_info(&address)
                .map(|i| i.is_used())
                .unwrap_or(false)
        });
    assert!(
        marked_used,
        "a spent UTXO's address must still be marked used in the pool"
    );
}

/// The asset-lock funding key accounts restore their stored pools on load.
/// Their credit outputs never appear as on-chain UTXOs, so no scan finds
/// their used indices again: without the stored pool a restart rebuilds the
/// unbound identity top-up account with its initial window only, all unused,
/// and a recorded top-up built on a later index could not find its signing
/// key on resume. Every stored index comes back at its path, with its used
/// mark, beyond the initial window included.
#[test]
fn should_restore_an_asset_lock_funding_pool_beyond_its_initial_window() {
    use key_wallet::account::AccountType;
    use key_wallet::managed_account::address_pool::KeySource;
    use platform_wallet::changeset::AccountAddressPoolEntry;

    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xDE);
    let wallet = Wallet::from_seed_bytes(
        [0x43; 64],
        key_wallet::Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .unwrap();
    let mut info = ManagedWalletInfo::from_wallet(&wallet, 7);
    let account_xpub = wallet
        .accounts
        .all_accounts()
        .into_iter()
        .find(|a| a.account_type == AccountType::IdentityTopUpNotBoundToIdentity)
        .expect("the default wallet has an unbound top-up account")
        .account_xpub;
    let manifest: Vec<AccountRegistrationEntry> = wallet
        .accounts
        .all_accounts()
        .into_iter()
        .map(|a| AccountRegistrationEntry {
            account_type: a.account_type,
            account_xpub: a.account_xpub,
        })
        .collect();
    let pool = info
        .accounts
        .identity_topup_not_bound
        .as_mut()
        .expect("managed unbound top-up account")
        .managed_account_type_mut()
        .address_pools_mut()
        .into_iter()
        .next()
        .expect("one pool");
    let window = pool.addresses().len() as u32;
    pool.generate_addresses(3, &KeySource::Public(account_xpub))
        .unwrap();
    let deep = window + 1;
    let first = pool.address_at_index(1).expect("index 1");
    let deep_address = pool.address_at_index(deep).expect("the deep index");
    assert!(pool.mark_used(&first));
    assert!(pool.mark_used(&deep_address));
    let pool_type = pool.pool_type();
    let deep_info = pool.addresses()[&deep].clone();
    let snapshot = AccountAddressPoolEntry {
        account_type: AccountType::IdentityTopUpNotBoundToIdentity,
        pool_type,
        addresses: pool.addresses().values().cloned().collect(),
    };
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                wallet_metadata: Some(WalletMetadataEntry {
                    network: key_wallet::Network::Testnet,
                    wallet_group_id: [0u8; 32],
                    birth_height: 7,
                }),
                account_registrations: manifest,
                account_address_pools: vec![snapshot],
                ..Default::default()
            },
        )
        .unwrap();
    drop(persister);

    let state = reopen(&path).load().expect("load");
    let mut restored = state.wallets[&w].wallet_info.clone();
    let pool = restored
        .accounts
        .identity_topup_not_bound
        .as_mut()
        .expect("restored unbound top-up account")
        .managed_account_type_mut()
        .address_pools_mut()
        .into_iter()
        .next()
        .expect("one pool");
    let entry = pool
        .addresses()
        .get(&deep)
        .expect("the index beyond the initial window is restored");
    assert_eq!(entry.address, deep_info.address);
    assert_eq!(entry.path, deep_info.path);
    assert!(entry.is_used());
    assert!(pool.addresses()[&1].is_used());
    assert!(!pool.addresses()[&0].is_used());
    assert_eq!(pool.address_index(&deep_info.address), Some(deep));
    assert_eq!(pool.highest_used(), Some(deep));
}

/// A funding pool whose every address is marked used comes back exactly as
/// stored, not extended beyond the used addresses. A lock built on an index
/// past them would sit beyond the addresses a recovery from scratch scans,
/// and nothing on chain at the earlier indices would lead that recovery to
/// it: refusing the lock is the safe outcome. Marks left by locks that never
/// went out are cleared when the wallet hands their index back instead.
#[test]
fn should_not_extend_a_restored_funding_pool_whose_every_address_is_used() {
    use key_wallet::account::AccountType;
    use platform_wallet::changeset::AccountAddressPoolEntry;

    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xDF);
    let wallet = Wallet::from_seed_bytes(
        [0x44; 64],
        key_wallet::Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .unwrap();
    let mut info = ManagedWalletInfo::from_wallet(&wallet, 7);
    let manifest: Vec<AccountRegistrationEntry> = wallet
        .accounts
        .all_accounts()
        .into_iter()
        .map(|a| AccountRegistrationEntry {
            account_type: a.account_type,
            account_xpub: a.account_xpub,
        })
        .collect();
    let pool = info
        .accounts
        .identity_topup_not_bound
        .as_mut()
        .expect("managed unbound top-up account")
        .managed_account_type_mut()
        .address_pools_mut()
        .into_iter()
        .next()
        .expect("one pool");
    for address in pool.all_addresses() {
        assert!(pool.mark_used(&address));
    }
    assert!(pool.unused_addresses().is_empty());
    let highest = pool.highest_generated().expect("an initial window");
    let snapshot = AccountAddressPoolEntry {
        account_type: AccountType::IdentityTopUpNotBoundToIdentity,
        pool_type: pool.pool_type(),
        addresses: pool.addresses().values().cloned().collect(),
    };
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                wallet_metadata: Some(WalletMetadataEntry {
                    network: key_wallet::Network::Testnet,
                    wallet_group_id: [0u8; 32],
                    birth_height: 7,
                }),
                account_registrations: manifest,
                account_address_pools: vec![snapshot],
                ..Default::default()
            },
        )
        .unwrap();
    drop(persister);

    let state = reopen(&path).load().expect("load");
    let mut restored = state.wallets[&w].wallet_info.clone();
    let pool = restored
        .accounts
        .identity_topup_not_bound
        .as_mut()
        .expect("restored unbound top-up account")
        .managed_account_type_mut()
        .address_pools_mut()
        .into_iter()
        .next()
        .expect("one pool");
    assert!(
        (0..=highest).all(|index| pool.addresses()[&index].is_used()),
        "every stored used mark is kept"
    );
    assert_eq!(
        pool.highest_generated(),
        Some(highest),
        "the pool is not extended beyond the stored addresses"
    );
    assert!(
        pool.unused_addresses().is_empty(),
        "no unused address is offered past the used ones"
    );
}

/// Releasing an unsent funding address clears its stored used mark, so after a
/// restart the account's next lock takes that index again rather than one
/// further along, past the addresses a restore from the seed scans. Under
/// `FlushMode::Manual` the used mark still buffered is written first, so it
/// does not land after the release. A release naming another address at the
/// same index, and the other used addresses, are left as they are.
#[test]
fn should_clear_the_stored_used_mark_of_a_released_unsent_funding_address() {
    use key_wallet::account::AccountType;
    use platform_wallet::changeset::AccountAddressPoolEntry;

    let (persister, _tmp, path) = common::fresh_persister_with_mode(common::FlushMode::Manual);
    let w = wid(0xE0);
    let wallet = Wallet::from_seed_bytes(
        [0x45; 64],
        key_wallet::Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .unwrap();
    let mut info = ManagedWalletInfo::from_wallet(&wallet, 7);
    let manifest: Vec<AccountRegistrationEntry> = wallet
        .accounts
        .all_accounts()
        .into_iter()
        .map(|a| AccountRegistrationEntry {
            account_type: a.account_type,
            account_xpub: a.account_xpub,
        })
        .collect();
    let pool = info
        .accounts
        .identity_topup_not_bound
        .as_mut()
        .expect("managed unbound top-up account")
        .managed_account_type_mut()
        .address_pools_mut()
        .into_iter()
        .next()
        .expect("one pool");
    let released = pool.address_at_index(0).expect("index 0");
    let kept = pool.address_at_index(1).expect("index 1");
    assert!(pool.mark_used(&released));
    assert!(pool.mark_used(&kept));
    let pool_type = pool.pool_type();
    let snapshot = AccountAddressPoolEntry {
        account_type: AccountType::IdentityTopUpNotBoundToIdentity,
        pool_type,
        addresses: pool.addresses().values().cloned().collect(),
    };
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                wallet_metadata: Some(WalletMetadataEntry {
                    network: key_wallet::Network::Testnet,
                    wallet_group_id: [0u8; 32],
                    birth_height: 7,
                }),
                account_registrations: manifest,
                account_address_pools: vec![snapshot],
                ..Default::default()
            },
        )
        .unwrap();

    // Another address at index 1: refused by its script, nothing changes.
    persister
        .release_unsent_pool_address(
            w,
            &AccountType::IdentityTopUpNotBoundToIdentity,
            pool_type,
            1,
            &released,
        )
        .expect("a release that matches no row");
    persister
        .release_unsent_pool_address(
            w,
            &AccountType::IdentityTopUpNotBoundToIdentity,
            pool_type,
            0,
            &released,
        )
        .expect("release");
    drop(persister);

    let state = reopen(&path).load().expect("load");
    let mut restored = state.wallets[&w].wallet_info.clone();
    let pool = restored
        .accounts
        .identity_topup_not_bound
        .as_mut()
        .expect("restored unbound top-up account")
        .managed_account_type_mut()
        .address_pools_mut()
        .into_iter()
        .next()
        .expect("one pool");
    assert!(
        !pool.addresses()[&0].is_used(),
        "the released address is unused after a restart"
    );
    assert!(
        pool.addresses()[&1].is_used(),
        "the other used mark is kept"
    );
    assert_eq!(pool.highest_used(), Some(1));
    assert_eq!(
        pool.unused_addresses().first(),
        Some(&released),
        "the next lock takes the released index again"
    );
}

/// The addresses a funding account derives on seeing its locks on chain are
/// stored, so a restored pool keeps the window those locks moved. Builds
/// store only the indices they used, all marked used once their locks went
/// out; without the derived ones a restored pool would offer no address to
/// the next lock although every earlier lock confirmed. A derivation of an
/// account that is not a funding account is not stored here.
#[test]
fn should_restore_the_window_a_funding_account_derived_on_chain() {
    use key_wallet::account::account_type::StandardAccountType;
    use key_wallet::account::AccountType;
    use key_wallet::managed_account::address_pool::{KeySource, PublicKeyType};
    use platform_wallet::changeset::AccountAddressPoolEntry;
    use platform_wallet::DerivedAddress;

    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xE1);
    let wallet = Wallet::from_seed_bytes(
        [0x46; 64],
        key_wallet::Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .unwrap();
    let mut info = ManagedWalletInfo::from_wallet(&wallet, 7);
    let manifest: Vec<AccountRegistrationEntry> = wallet
        .accounts
        .all_accounts()
        .into_iter()
        .map(|a| AccountRegistrationEntry {
            account_type: a.account_type,
            account_xpub: a.account_xpub,
        })
        .collect();
    let xpub = wallet
        .accounts
        .identity_topup_not_bound
        .as_ref()
        .expect("unbound top-up account")
        .account_xpub;
    let pool = info
        .accounts
        .identity_topup_not_bound
        .as_mut()
        .expect("managed unbound top-up account")
        .managed_account_type_mut()
        .address_pools_mut()
        .into_iter()
        .next()
        .expect("one pool");
    for address in pool.all_addresses() {
        assert!(pool.mark_used(&address));
    }
    let highest = pool.highest_generated().expect("an initial window");
    let pool_type = pool.pool_type();
    let snapshot = AccountAddressPoolEntry {
        account_type: AccountType::IdentityTopUpNotBoundToIdentity,
        pool_type,
        addresses: pool.addresses().values().cloned().collect(),
    };
    // What seeing those locks on chain derives.
    let derived = pool
        .maintain_gap_limit(&KeySource::Public(xpub))
        .expect("derive past the used addresses");
    assert!(!derived.is_empty());
    let to_event =
        |account_type: AccountType,
         info: &key_wallet::managed_account::address_pool::AddressInfo| {
            let Some(PublicKeyType::ECDSA(bytes)) = info.public_key.as_ref() else {
                panic!("an ECDSA funding key");
            };
            DerivedAddress {
                account_type,
                pool_type,
                derivation_index: info.index,
                address: info.address.clone(),
                public_key: dashcore::PublicKey::from_slice(bytes).unwrap(),
            }
        };
    let mut addresses_derived: Vec<DerivedAddress> = derived
        .iter()
        .map(|info| to_event(AccountType::IdentityTopUpNotBoundToIdentity, info))
        .collect();
    // The same index under a BIP44 account: not a funding account.
    addresses_derived.push(to_event(
        AccountType::Standard {
            index: 0,
            standard_account_type: StandardAccountType::BIP44Account,
        },
        &derived[0],
    ));
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                wallet_metadata: Some(WalletMetadataEntry {
                    network: key_wallet::Network::Testnet,
                    wallet_group_id: [0u8; 32],
                    birth_height: 7,
                }),
                account_registrations: manifest,
                account_address_pools: vec![snapshot],
                ..Default::default()
            },
        )
        .unwrap();
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    addresses_derived,
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    drop(persister);

    let state = reopen(&path).load().expect("load");
    let mut restored = state.wallets[&w].wallet_info.clone();
    let pool = restored
        .accounts
        .identity_topup_not_bound
        .as_mut()
        .expect("restored unbound top-up account")
        .managed_account_type_mut()
        .address_pools_mut()
        .into_iter()
        .next()
        .expect("one pool");
    assert!(
        (0..=highest).all(|index| pool.addresses()[&index].is_used()),
        "every stored used mark is kept"
    );
    let next = &derived[0];
    assert_eq!(
        pool.unused_addresses().first(),
        Some(&next.address),
        "the next lock takes the first address derived on chain"
    );
    assert_eq!(pool.addresses()[&next.index].path, next.path);
    let conn = rusqlite::Connection::open(&path).unwrap();
    let rows: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM core_address_pool WHERE wallet_id = ?1 AND script = ?2",
            rusqlite::params![w.as_slice(), next.address.script_pubkey().as_bytes()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        rows, 1,
        "only the funding account's derivation is stored, not the BIP44 one"
    );
}

/// A test wallet, its managed info, and the manifest that registers it.
fn funding_test_wallet(seed: u8) -> (Wallet, ManagedWalletInfo, Vec<AccountRegistrationEntry>) {
    let wallet = Wallet::from_seed_bytes(
        [seed; 64],
        key_wallet::Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .unwrap();
    let info = ManagedWalletInfo::from_wallet(&wallet, 7);
    let manifest = wallet
        .accounts
        .all_accounts()
        .into_iter()
        .map(|a| AccountRegistrationEntry {
            account_type: a.account_type,
            account_xpub: a.account_xpub,
        })
        .collect();
    (wallet, info, manifest)
}

/// The unbound top-up account's only pool.
fn unbound_top_up_pool(
    info: &mut ManagedWalletInfo,
) -> &mut key_wallet::managed_account::address_pool::AddressPool {
    info.accounts
        .identity_topup_not_bound
        .as_mut()
        .expect("managed unbound top-up account")
        .managed_account_type_mut()
        .address_pools_mut()
        .into_iter()
        .next()
        .expect("one pool")
}

fn register_wallet(
    persister: &SqlitePersister,
    w: platform_wallet::wallet::platform_wallet::WalletId,
    manifest: Vec<AccountRegistrationEntry>,
    account_address_pools: Vec<platform_wallet::changeset::AccountAddressPoolEntry>,
) {
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                wallet_metadata: Some(WalletMetadataEntry {
                    network: key_wallet::Network::Testnet,
                    wallet_group_id: [0u8; 32],
                    birth_height: 7,
                }),
                account_registrations: manifest,
                account_address_pools,
                ..Default::default()
            },
        )
        .unwrap();
}

/// Sync marks a funding address used on finding a lock that pays it, and the
/// wallet can close before any build stores a pool snapshot. The mark is
/// stored with the sync changeset, so after a restart the key is not handed
/// out again for the next lock.
#[test]
fn should_keep_a_funding_address_sync_marked_used_after_reopen() {
    use key_wallet::account::AccountType;
    use key_wallet::transaction_checking::DerivedAddressInfo;

    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xE2);
    let (_wallet, mut info, manifest) = funding_test_wallet(0x47);
    register_wallet(&persister, w, manifest, Vec::new());
    let pool = unbound_top_up_pool(&mut info);
    let pool_type = pool.pool_type();
    let mut seen = pool.addresses()[&0].clone();
    assert!(pool.mark_used(&seen.address));
    seen.state = key_wallet::managed_account::address_pool::AddressState::Used;
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    addresses_marked_used: vec![DerivedAddressInfo {
                        account_type: AccountType::IdentityTopUpNotBoundToIdentity,
                        pool_type,
                        info: seen.clone(),
                    }],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    drop(persister);

    let state = reopen(&path).load().expect("load");
    let mut restored = state.wallets[&w].wallet_info.clone();
    let pool = unbound_top_up_pool(&mut restored);
    assert!(
        pool.addresses()[&0].is_used(),
        "the address sync saw used stays used"
    );
    assert_ne!(
        pool.unused_addresses().first(),
        Some(&seen.address),
        "the next lock does not reuse its key"
    );
}

/// Older writers stored the addresses sync derived with no public key. Such
/// rows, within the initial window and past it, load: an in-window used mark
/// is applied, and a row past the window is restored at its derivation path,
/// so a lock built on it can still be signed, and the stored row after it is
/// not refused as leaving a gap.
#[test]
fn should_restore_funding_rows_stored_without_a_public_key() {
    use key_wallet::managed_account::address_pool::KeySource;

    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xE3);
    let (wallet, mut info, manifest) = funding_test_wallet(0x48);
    let xpub = wallet
        .accounts
        .identity_topup_not_bound
        .as_ref()
        .expect("unbound top-up account")
        .account_xpub;
    register_wallet(&persister, w, manifest, Vec::new());
    let pool = unbound_top_up_pool(&mut info);
    let window = pool.addresses().len() as u32;
    pool.generate_addresses(2, &KeySource::Public(xpub))
        .unwrap();
    let expected: Vec<_> = pool.addresses().values().cloned().collect();
    let conn = rusqlite::Connection::open(&path).unwrap();
    // Legacy rows: no key material. Index 1 and the first index past the
    // window are used, the next one is not.
    for (index, used) in [(1, true), (window, true), (window + 1, false)] {
        conn.execute(
            "INSERT INTO core_address_pool \
                (wallet_id, account_type, account_index, key_class, user_identity_id, \
                 friend_identity_id, pool_type, address_index, script, used) \
             VALUES (?1, 'identity_topup_unbound', 0, 0, zeroblob(32), zeroblob(32), \
                     ?2, ?3, ?4, ?5)",
            rusqlite::params![
                w.as_slice(),
                2i64,
                i64::from(index),
                expected[index as usize].address.script_pubkey().as_bytes(),
                used,
            ],
        )
        .unwrap();
    }
    drop(conn);
    drop(persister);

    let state = reopen(&path).load().expect("legacy rows load");
    let mut restored = state.wallets[&w].wallet_info.clone();
    let pool = unbound_top_up_pool(&mut restored);
    assert!(pool.addresses()[&1].is_used());
    assert!(!pool.addresses()[&0].is_used());
    let past = &pool.addresses()[&window];
    assert_eq!(past.address, expected[window as usize].address);
    assert_eq!(past.path, expected[window as usize].path);
    assert!(past.is_used());
    let next = &pool.addresses()[&(window + 1)];
    assert_eq!(next.address, expected[window as usize + 1].address);
    assert!(!next.is_used());
}

/// A store written before the funding accounts' derived addresses were kept
/// holds a pool whose every stored address is used, the last by a lock that
/// confirmed. Sync moved the window past that lock, and the load moves it the
/// same way from the stored transaction, so the next lock has an address.
#[test]
fn should_extend_an_upgraded_funding_pool_past_a_lock_stored_on_chain() {
    use dashcore::blockdata::transaction::special_transaction::asset_lock::AssetLockPayload;
    use dashcore::blockdata::transaction::special_transaction::TransactionPayload;
    use dashcore::{BlockHash, ScriptBuf, Transaction, TxOut};
    use key_wallet::account::AccountType;
    use key_wallet::managed_account::transaction_record::{
        TransactionDirection, TransactionRecord,
    };
    use key_wallet::transaction_checking::{BlockInfo, TransactionContext, TransactionType};
    use platform_wallet::changeset::AccountAddressPoolEntry;

    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xE4);
    let (wallet, mut info, manifest) = funding_test_wallet(0x49);
    let pool = unbound_top_up_pool(&mut info);
    for address in pool.all_addresses() {
        assert!(pool.mark_used(&address));
    }
    let highest = pool.highest_generated().expect("an initial window");
    let gap_limit = pool.gap_limit();
    let paid = pool.addresses()[&highest].address.clone();
    let snapshot = AccountAddressPoolEntry {
        account_type: AccountType::IdentityTopUpNotBoundToIdentity,
        pool_type: pool.pool_type(),
        addresses: pool.addresses().values().cloned().collect(),
    };
    register_wallet(&persister, w, manifest, vec![snapshot]);
    let lock = |credit: &dashcore::Address, context: TransactionContext| {
        TransactionRecord::new(
            Transaction {
                version: 3,
                lock_time: 0,
                input: Vec::new(),
                output: vec![TxOut {
                    value: 100_000,
                    script_pubkey: ScriptBuf::new_op_return(&[]),
                }],
                special_transaction_payload: Some(TransactionPayload::AssetLockPayloadType(
                    AssetLockPayload {
                        version: 1,
                        credit_outputs: vec![TxOut {
                            value: 100_000,
                            script_pubkey: credit.script_pubkey(),
                        }],
                    },
                )),
            },
            AccountType::IdentityTopUpNotBoundToIdentity,
            context,
            TransactionType::AssetLock,
            TransactionDirection::Outgoing,
            Vec::new(),
            Vec::new(),
            0,
        )
    };
    let xpub = wallet
        .accounts
        .identity_topup_not_bound
        .as_ref()
        .expect("unbound top-up account")
        .account_xpub;
    // A mempool-only lock paying an address past the window moves nothing: it
    // may never have been accepted.
    let mut probe = unbound_top_up_pool(&mut info).clone();
    probe
        .generate_addresses(
            gap_limit * 3,
            &key_wallet::managed_account::address_pool::KeySource::Public(xpub),
        )
        .unwrap();
    let far = probe.addresses()[&(highest + gap_limit * 2)]
        .address
        .clone();
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    records: vec![
                        lock(
                            &paid,
                            TransactionContext::InBlock(BlockInfo::new(
                                100,
                                BlockHash::all_zeros(),
                                1_700_000_000,
                            )),
                        ),
                        lock(&far, TransactionContext::Mempool),
                    ],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    drop(persister);

    let state = reopen(&path).load().expect("load");
    let mut restored = state.wallets[&w].wallet_info.clone();
    let pool = unbound_top_up_pool(&mut restored);
    assert!(
        (0..=highest).all(|index| pool.addresses()[&index].is_used()),
        "every stored used mark is kept"
    );
    assert_eq!(
        pool.highest_generated(),
        Some(highest + gap_limit),
        "the window moves past the confirmed lock as sync moved it, not past the mempool one"
    );
    assert_eq!(
        pool.unused_addresses().first(),
        Some(&probe.addresses()[&(highest + 1)].address),
        "the next lock takes the address after the confirmed one"
    );
}

/// A stored funding row whose address is not the one its index derives (a
/// damaged but well-formed script) is not restored as stored: a lock paid to
/// it could not be signed for on Platform. The derived address takes its
/// place, used, since the key the row stood for may have been handed out.
#[test]
fn should_restore_the_derived_address_over_a_damaged_funding_row() {
    use key_wallet::managed_account::address_pool::KeySource;

    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xE5);
    let (wallet, mut info, manifest) = funding_test_wallet(0x4A);
    let xpub = wallet
        .accounts
        .identity_topup_not_bound
        .as_ref()
        .expect("unbound top-up account")
        .account_xpub;
    register_wallet(&persister, w, manifest, Vec::new());
    let pool = unbound_top_up_pool(&mut info);
    let window = pool.addresses().len() as u32;
    pool.generate_addresses(2, &KeySource::Public(xpub))
        .unwrap();
    let expected: Vec<_> = pool.addresses().values().cloned().collect();
    let conn = rusqlite::Connection::open(&path).unwrap();
    // Past the window, unused, but holding the address of index 0.
    conn.execute(
        "INSERT INTO core_address_pool \
            (wallet_id, account_type, account_index, key_class, user_identity_id, \
             friend_identity_id, pool_type, address_index, script, used) \
         VALUES (?1, 'identity_topup_unbound', 0, 0, zeroblob(32), zeroblob(32), \
                 2, ?2, ?3, 0)",
        rusqlite::params![
            w.as_slice(),
            i64::from(window),
            expected[0].address.script_pubkey().as_bytes(),
        ],
    )
    .unwrap();
    drop(conn);
    drop(persister);

    let state = reopen(&path).load().expect("load");
    let mut restored = state.wallets[&w].wallet_info.clone();
    let pool = unbound_top_up_pool(&mut restored);
    let row = &pool.addresses()[&window];
    assert_eq!(row.address, expected[window as usize].address);
    assert_eq!(row.path, expected[window as usize].path);
    assert!(
        row.is_used(),
        "the damaged row's key may have been handed out"
    );
    assert_eq!(pool.address_index(&expected[0].address), Some(0));
}
