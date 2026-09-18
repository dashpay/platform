//! Legacy databases retain their address state while Core starts a full rescan.

mod common;

use common::fresh_persister;
use dashcore::hashes::Hash;
use key_wallet::account::ManagedAccountTrait;
use key_wallet::managed_account::address_pool::{AddressPoolType, AddressState, KeySource};
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
use key_wallet::wallet::Wallet;
use platform_wallet::changeset::{
    AccountAddressPoolEntry, AccountRegistrationEntry, CoreChangeSet, PlatformWalletChangeSet,
    PlatformWalletPersistence, WalletMetadataEntry,
};
use platform_wallet_storage::{SqlitePersister, SqlitePersisterConfig};

#[test]
fn legacy_wallet_restarts_from_birth_and_preserves_deep_unused_and_used_addresses() {
    let (persister, _tmp, path) = fresh_persister();
    let wallet = Wallet::from_seed_bytes(
        [0xC5; 64],
        key_wallet::Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .unwrap();
    let wallet_id = wallet.wallet_id;
    let mut info = ManagedWalletInfo::from_wallet(&wallet, 50);
    let account = info.first_bip44_managed_account_mut().unwrap();
    let account_type = account.managed_account_type().to_account_type();
    let manifest: Vec<_> = wallet
        .accounts
        .all_accounts()
        .into_iter()
        .map(|account| AccountRegistrationEntry {
            account_type: account.account_type,
            account_xpub: account.account_xpub,
        })
        .collect();
    let xpub = manifest
        .iter()
        .find(|entry| entry.account_type == account_type)
        .unwrap()
        .account_xpub;
    let pool = account
        .managed_account_type_mut()
        .address_pools_mut()
        .into_iter()
        .find(|pool| pool.pool_type == AddressPoolType::External)
        .unwrap();
    pool.generate_addresses(90, &KeySource::Public(xpub), true)
        .unwrap();
    let used = pool.address_at_index(65).unwrap();
    pool.mark_used(&used);
    let unused = pool.address_at_index(80).unwrap();
    pool.addresses.get_mut(&81).unwrap().state = AddressState::Reserved { at: 123_456 };
    let reserved = pool.address_at_index(81).unwrap();
    pool.highest_generated = Some(49_999);
    pool.generate_addresses(1, &KeySource::Public(xpub), true)
        .unwrap();
    let sparse = pool.address_at_index(50_000).unwrap();
    let entries = pool.addresses.values().cloned().collect();
    let coin = key_wallet::Utxo::new(
        dashcore::OutPoint::new(dashcore::Txid::from_byte_array([0xC6; 32]), 0),
        dashcore::TxOut {
            value: 50_000,
            script_pubkey: used.script_pubkey(),
        },
        used.clone(),
        60,
        false,
    );
    persister
        .store(
            wallet_id,
            PlatformWalletChangeSet {
                wallet_metadata: Some(WalletMetadataEntry {
                    network: wallet.network,
                    wallet_group_id: [0; 32],
                    birth_height: 50,
                }),
                account_registrations: manifest,
                account_address_pools: vec![AccountAddressPoolEntry {
                    account_type,
                    pool_type: AddressPoolType::External,
                    addresses: entries,
                }],
                core: Some(CoreChangeSet {
                    new_utxos: vec![coin],
                    synced_height: Some(100),
                    last_processed_height: Some(100),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    drop(persister);

    for _ in 0..2 {
        let reopened = SqlitePersister::open(SqlitePersisterConfig::new(&path)).unwrap();
        let state = reopened.load().unwrap();
        let restored = &state.wallets.get(&wallet_id).unwrap().wallet_info;
        assert_eq!(restored.metadata.synced_height, 49);
        assert_eq!(restored.metadata.last_processed_height, 49);
        assert_eq!(restored.balance.total(), 0);
        assert!(restored.utxos().is_empty());
        assert!(restored.transaction_history().is_empty());
        let account = restored.first_bip44_managed_account().unwrap();
        assert!(account.get_address_info(&used).unwrap().is_used());
        assert!(account.get_address_info(&unused).unwrap().is_available());
        assert!(account.get_address_info(&sparse).unwrap().is_available());
        assert_eq!(
            account.get_address_info(&reserved).unwrap().reserved_at(),
            Some(123_456)
        );
        let conn = reopened.lock_conn_for_test();
        let saved_height: u32 = conn
            .query_row(
                "SELECT synced_height FROM core_sync_state WHERE wallet_id = ?1",
                [wallet_id.as_slice()],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            saved_height, 100,
            "legacy rows remain available for history"
        );
    }
}
