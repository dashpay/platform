#![allow(clippy::field_reassign_with_default)]

//! Genesis-rescan regression for UTXO attribution when a freshly-derived
//! gap-limit-edge address has no persisted pool row.

mod common;

use common::{ensure_wallet_meta, fresh_persister, wid};

use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
use key_wallet::wallet::Wallet;
use key_wallet::AddressInfo;
use platform_wallet::changeset::AccountRegistrationEntry;
use platform_wallet::changeset::{
    CoreChangeSet, PlatformWalletChangeSet, PlatformWalletPersistence,
};
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet_storage::sqlite::schema::core_state;
use platform_wallet_storage::LoadCtx;

fn manifest_for(wallet: &Wallet) -> Vec<AccountRegistrationEntry> {
    wallet
        .accounts
        .all_accounts()
        .into_iter()
        .map(|account| AccountRegistrationEntry {
            account_type: account.account_type,
            account_xpub: account.account_xpub,
        })
        .collect()
}

/// The LAST address in the wallet's Standard BIP44 external pool — the
/// gap-limit-edge address, the one most likely to be a fresh extension and
/// thus the worst case for the retired attribution race.
fn wallet_and_gap_limit_edge_address(seed_byte: u8) -> (Wallet, AddressInfo) {
    use key_wallet::account::AccountType;
    use key_wallet::managed_account::address_pool::AddressPoolType;

    let wallet = Wallet::from_seed_bytes(
        [seed_byte; 64],
        key_wallet::Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .unwrap();
    let info = ManagedWalletInfo::from_wallet(&wallet, 0);

    for managed in info.all_managed_accounts() {
        let account_type = managed.managed_account_type().to_account_type();
        if !matches!(account_type, AccountType::Standard { index: 0, .. }) {
            continue;
        }
        for pool in managed.managed_account_type().address_pools() {
            if pool.pool_type != AddressPoolType::External || pool.addresses.is_empty() {
                continue;
            }
            let mut infos: Vec<AddressInfo> = pool.addresses.values().cloned().collect();
            infos.sort_by_key(|address| address.index);
            return (wallet, infos.pop().unwrap());
        }
    }
    panic!("wallet must expose a non-empty Standard BIP44 external pool");
}

fn utxo_at(addr: &dashcore::Address, vout: u32, value: u64) -> key_wallet::Utxo {
    use dashcore::hashes::Hash;
    key_wallet::Utxo {
        outpoint: dashcore::OutPoint {
            txid: dashcore::Txid::from_byte_array([0x7E; 32]),
            vout,
        },
        txout: dashcore::TxOut {
            value,
            script_pubkey: addr.script_pubkey(),
        },
        address: addr.clone(),
        height: 7,
        is_coinbase: false,
        is_confirmed: true,
        is_instantlocked: false,
        is_locked: false,
        is_trusted: false,
    }
}

/// A UTXO without a pool row follows the real restart path into the first
/// funds account with its exact balance.
#[test]
fn utxo_on_fresh_gap_limit_address_rehydrates_under_first_funds_account() {
    let (persister, _tmp, path) = fresh_persister();
    let w: WalletId = wid(0xD1);
    ensure_wallet_meta(&persister, &w);

    let (wallet, edge) = wallet_and_gap_limit_edge_address(0x55);
    let addr = edge.address.clone();
    let utxo = utxo_at(&addr, 0, 777_000);
    let outpoint = utxo.outpoint;

    persister
        .store(
            w,
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    new_utxos: vec![utxo],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .expect("a UTXO on a fresh gap-limit address must persist, not abort");
    drop(persister);

    let reopened = platform_wallet_storage::SqlitePersister::open(
        platform_wallet_storage::SqlitePersisterConfig::new(&path),
    )
    .expect("reopen persister");
    let conn = reopened.lock_conn_for_test();
    let (core, utxo_accounts) =
        core_state::load_state(&conn, &w, key_wallet::Network::Testnet, &LoadCtx::strict())
            .expect("load state");
    drop(conn);

    assert!(
        utxo_accounts.is_empty(),
        "the missing pool row must exercise the unattributed fallback"
    );

    let mut managed = ManagedWalletInfo::from_wallet(&wallet, 1);
    platform_wallet_storage::sqlite::util::apply_persisted_core_state(
        &mut managed,
        &manifest_for(&wallet),
        &core,
        &utxo_accounts,
        &Default::default(),
        &LoadCtx::strict(),
    )
    .expect("rehydration must apply the unattributed UTXO");

    let first_funds = managed.accounts.all_funding_accounts().remove(0);
    assert!(
        first_funds.utxos.contains_key(&outpoint),
        "the UTXO must land in the first funds account"
    );
    assert_eq!(first_funds.balance.total(), 777_000);
    assert_eq!(WalletInfoInterface::balance(&managed).total(), 777_000);
}
