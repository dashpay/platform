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
            txid: {
                use dashcore::hashes::Hash;
                dashcore::Txid::from_byte_array([0x99; 32])
            },
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

/// A contact's external account is no funds account, yet its used
/// addresses must come back on it: a relaunch that handed a paid contact
/// address out again would link two payments on-chain. A contact payment
/// stores its address reserved before the broadcast, so an unsettled
/// reservation — the process died mid-send — reloads used as well, while a
/// released one reloads available.
#[test]
fn contact_pool_reloads_used_and_unsettled_reserved_addresses() {
    use key_wallet::account::account_collection::DashpayAccountKey;
    use key_wallet::account::{AccountType, StandardAccountType};
    use key_wallet::managed_account::address_pool::AddressState;
    use platform_wallet::changeset::AccountAddressPoolEntry;

    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xDE);
    let wallet = Wallet::from_seed_bytes(
        [0x43; 64],
        key_wallet::Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .unwrap();
    // Any xpub serves as the contact's: the account is watch-only.
    let contact_xpub = wallet
        .accounts
        .all_accounts()
        .into_iter()
        .find(|a| {
            a.account_type
                == AccountType::Standard {
                    index: 0,
                    standard_account_type: StandardAccountType::BIP44Account,
                }
        })
        .expect("BIP44 account 0")
        .account_xpub;
    let key = DashpayAccountKey {
        index: 0,
        user_identity_id: [0xC1; 32],
        friend_identity_id: [0xC2; 32],
    };
    let contact = AccountType::DashpayExternalAccount {
        index: key.index,
        user_identity_id: key.user_identity_id,
        friend_identity_id: key.friend_identity_id,
    };
    let mut manifest: Vec<AccountRegistrationEntry> = wallet
        .accounts
        .all_accounts()
        .into_iter()
        .map(|a| AccountRegistrationEntry {
            account_type: a.account_type,
            account_xpub: a.account_xpub,
        })
        .collect();
    manifest.push(AccountRegistrationEntry {
        account_type: contact,
        account_xpub: contact_xpub,
    });
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
    drop(persister);

    let contact_pool = |persister: &SqlitePersister| {
        let state = persister.load().expect("load");
        let info = &state.wallets.get(&w).expect("wallet slice").wallet_info;
        let account = info
            .accounts
            .dashpay_external_accounts
            .get(&key)
            .expect("the contact's external account is rebuilt");
        let pool = account
            .managed_account_type()
            .address_pools()
            .into_iter()
            .next()
            .expect("contact pool");
        let mut infos: Vec<key_wallet::AddressInfo> = pool.addresses.values().cloned().collect();
        infos.sort_by_key(|info| info.index);
        (pool.pool_type, infos)
    };

    // Index 0: paid. Index 1: reserved, then the process dies mid-send.
    // Index 2: reserved, then released after a rejection.
    let persister = reopen(&path);
    let (pool_type, mut infos) = contact_pool(&persister);
    infos.truncate(3);
    infos[0].state = AddressState::Used;
    infos[1].state = AddressState::Reserved { at: 1_752_528_626 };
    infos[2].state = AddressState::Reserved { at: 1_752_528_626 };
    let store_pool = |infos: Vec<key_wallet::AddressInfo>| {
        persister
            .store(
                w,
                PlatformWalletChangeSet {
                    account_address_pools: vec![AccountAddressPoolEntry {
                        account_type: contact,
                        pool_type,
                        addresses: infos,
                    }],
                    ..Default::default()
                },
            )
            .unwrap();
    };
    store_pool(infos.clone());
    infos[2].state = AddressState::Available;
    store_pool(infos.clone());
    drop(persister);

    let (_, reloaded) = contact_pool(&reopen(&path));
    let state_of = |index: u32| {
        reloaded
            .iter()
            .find(|info| info.index == index)
            .map(|info| info.state)
            .expect("index in the restored pool")
    };
    assert!(
        matches!(state_of(0), AddressState::Used),
        "a paid contact address reloads used"
    );
    assert!(
        matches!(state_of(1), AddressState::Used),
        "an unsettled reservation must not reload as a fresh address"
    );
    assert!(
        matches!(state_of(2), AddressState::Available),
        "a released reservation reloads available"
    );
}
