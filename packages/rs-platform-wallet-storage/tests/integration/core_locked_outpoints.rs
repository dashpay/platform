//! Masternode collateral locks (`CoreChangeSet::locked_outpoints`) survive a
//! restart: the store keeps them and `load()` hands them back to the wallet
//! through `ManagedWalletInfo::lock_outpoint`, so coin selection keeps
//! skipping the collateral.

use crate::common;

use common::{fresh_persister, wid};
use dashcore::{Address, OutPoint, TxOut, Txid};
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::wallet::managed_wallet_info::coin_selection::SelectionStrategy;
use key_wallet::wallet::managed_wallet_info::transaction_builder::TransactionBuilder;
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
use key_wallet::wallet::Wallet;
use key_wallet::Utxo;
use platform_wallet::changeset::{
    AccountRegistrationEntry, CoreChangeSet, PlatformWalletChangeSet, PlatformWalletPersistence,
    WalletMetadataEntry,
};
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet_storage::{SqlitePersister, SqlitePersisterConfig};

const COLLATERAL_DUFFS: u64 = 1_000 * 100_000_000;
const SPARE_DUFFS: u64 = 5 * 100_000_000;
const HEIGHT: u32 = 100;

fn reopen(path: &std::path::Path) -> SqlitePersister {
    SqlitePersister::open(SqlitePersisterConfig::new(path)).expect("reopen")
}

fn elsewhere() -> Address {
    use dashcore::address::Payload;
    use dashcore::PubkeyHash;
    Address::new(
        dashcore::Network::Testnet,
        Payload::PubkeyHash(PubkeyHash::from_byte_array([0xEE; 20])),
    )
}

fn confirmed_utxo(address: &Address, txid_byte: u8, value: u64) -> Utxo {
    Utxo {
        outpoint: OutPoint::new(Txid::from_byte_array([txid_byte; 32]), 0),
        txout: TxOut {
            value,
            script_pubkey: address.script_pubkey(),
        },
        address: address.clone(),
        height: HEIGHT,
        is_coinbase: false,
        is_confirmed: true,
        is_instantlocked: false,
        is_locked: false,
        is_trusted: false,
    }
}

/// Register a wallet and store a collateral coin, a spare coin and the lock
/// on the collateral. Returns `(collateral, spare)`.
fn store_wallet_with_locked_collateral(
    persister: &SqlitePersister,
    w: WalletId,
) -> (OutPoint, OutPoint) {
    let wallet = Wallet::from_seed_bytes(
        [0x5C; 64],
        key_wallet::Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .expect("wallet");
    let info = ManagedWalletInfo::from_wallet(&wallet, 1);
    let mut addresses = WalletInfoInterface::monitored_addresses(&info).into_iter();
    let collateral_address = addresses.next().expect("first address");
    let spare_address = addresses.next().expect("second address");

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
                    birth_height: 1,
                }),
                account_registrations: manifest,
                ..Default::default()
            },
        )
        .expect("register wallet");

    let collateral = confirmed_utxo(&collateral_address, 0xC0, COLLATERAL_DUFFS);
    let spare = confirmed_utxo(&spare_address, 0x5A, SPARE_DUFFS);
    let (collateral_op, spare_op) = (collateral.outpoint, spare.outpoint);
    common::store_and_flush(
        persister,
        w,
        PlatformWalletChangeSet {
            core: Some(CoreChangeSet {
                new_utxos: vec![collateral, spare],
                last_processed_height: Some(HEIGHT),
                synced_height: Some(HEIGHT),
                locked_outpoints: [collateral_op].into(),
                ..Default::default()
            }),
            ..Default::default()
        },
    );
    (collateral_op, spare_op)
}

#[test]
fn should_round_trip_locked_outpoints_through_the_store() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xC1);
    let (collateral, _spare) = store_wallet_with_locked_collateral(&persister, w);
    // A second store of the same lock is idempotent, not a constraint error.
    common::store_and_flush(
        &persister,
        w,
        PlatformWalletChangeSet {
            core: Some(CoreChangeSet {
                locked_outpoints: [collateral].into(),
                ..Default::default()
            }),
            ..Default::default()
        },
    );
    drop(persister);

    let state = reopen(&path).load().expect("load");
    let restored = &state.wallets.get(&w).expect("wallet slice").wallet_info;
    assert_eq!(
        restored
            .locked_outpoints()
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        vec![collateral],
        "the stored collateral lock must come back exactly once"
    );
}

#[test]
fn should_keep_the_collateral_out_of_coin_selection_after_reload() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xC2);
    let (collateral, spare) = store_wallet_with_locked_collateral(&persister, w);
    drop(persister);

    let mut state = reopen(&path).load().expect("load");
    let slice = state.wallets.get_mut(&w).expect("wallet slice");

    assert_eq!(
        slice.wallet_info.balance().locked(),
        COLLATERAL_DUFFS,
        "the collateral must count as locked, not spendable"
    );
    let spendable: Vec<OutPoint> = slice
        .wallet_info
        .get_spendable_utxos()
        .into_iter()
        .map(|u| u.outpoint)
        .collect();
    assert_eq!(spendable, vec![spare]);

    // `All` offers every candidate coin, as a max send does; the collateral
    // must not be among them.
    let account = slice
        .wallet
        .accounts
        .standard_bip44_accounts
        .get(&0)
        .expect("BIP44 account")
        .clone();
    let funds = slice
        .wallet_info
        .accounts
        .standard_bip44_accounts
        .get_mut(&0)
        .expect("BIP44 funds account");
    let (tx, _fee, _reservation) = TransactionBuilder::new()
        .set_current_height(HEIGHT)
        .set_selection_strategy(SelectionStrategy::All)
        .add_funding(funds, &account)
        .add_output(&elsewhere(), 100_000_000)
        .build_unsigned_reserved()
        .expect("the spare coin funds the sweep");
    let inputs: Vec<OutPoint> = tx.input.iter().map(|i| i.previous_output).collect();
    assert!(
        !inputs.contains(&collateral),
        "a restored wallet must never spend the masternode collateral"
    );
    assert_eq!(inputs, vec![spare]);
}
