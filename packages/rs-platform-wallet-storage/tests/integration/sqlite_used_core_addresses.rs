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

/// A funding pool whose every address is marked used, by locks that never
/// reached the chain (rejected at broadcast, or abandoned before it), comes
/// back with fresh unused addresses beyond them. The stored used marks
/// survive the in-memory hand-back of such an index, and no transaction
/// refills the pool, so without the load-time refill each restart would
/// leave the account with nothing to fund the next lock from.
#[test]
fn should_refill_an_asset_lock_funding_pool_whose_every_address_is_used() {
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
    assert!(
        pool.addresses()
            .values()
            .any(|info| info.index > highest && !info.is_used()),
        "the pool is refilled beyond the used addresses"
    );
}
