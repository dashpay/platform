#![allow(clippy::field_reassign_with_default)]

//! `load()` marks a used-then-emptied address as used in the assembled
//! `core_wallet_info` pools (derived from the full `core_utxos` set, spent +
//! unspent) so the rehydrated wallet never hands it out again as a fresh
//! receive address (address reuse).

mod common;

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

/// A spend whose previous output was never persisted arrives carrying a real
/// address but an empty `script_pubkey` — the upstream `derive_spent_utxos`
/// shape, where the script belongs to the *previous* transaction and isn't
/// available. That row must never reach `core_utxos`: the reuse-guard read
/// scans every stored script, so one undecodable row would reject the load of
/// the whole wallet file — every wallet in it, funded ones included.
#[test]
fn spent_only_utxo_with_empty_script_does_not_block_load() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xEE);

    let wallet = Wallet::from_seed_bytes(
        [0x43; 64],
        key_wallet::Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .unwrap();
    let info = ManagedWalletInfo::from_wallet(&wallet, 7);
    let address = WalletInfoInterface::monitored_addresses(&info)
        .into_iter()
        .next()
        .unwrap();

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

    // No prior `new_utxos` write for this outpoint, so the spend finds no row
    // to mark and would otherwise be materialized as a placeholder.
    let spent = key_wallet::Utxo {
        outpoint: dashcore::OutPoint {
            txid: {
                use dashcore::hashes::Hash;
                dashcore::Txid::from_byte_array([0xA1; 32])
            },
            vout: 0,
        },
        txout: dashcore::TxOut {
            value: 500_000,
            script_pubkey: dashcore::ScriptBuf::default(),
        },
        address,
        height: 0,
        is_coinbase: false,
        is_confirmed: false,
        is_instantlocked: false,
        is_locked: false,
        is_trusted: false,
    };
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    spent_utxos: vec![spent],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    drop(persister);

    let p2 = reopen(&path);
    let state = p2
        .load()
        .expect("an unrecorded spend must not make the wallet file unloadable");
    let slice = state.wallets.get(&w).expect("wallet slice");
    assert_eq!(
        slice.wallet_info.balance.total(),
        0,
        "the spend contributes no balance"
    );
}
