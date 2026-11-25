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
