//! Platform-node public-key pool reconstruction for SQLite load.

use key_wallet::account::AccountType;

/// Errors while inserting a pre-derived platform-node key into its managed pool.
#[derive(Debug, thiserror::Error)]
pub(super) enum PlatformNodePoolError {
    /// The wallet has no managed provider-platform account.
    #[error("wallet has no managed provider platform account")]
    NoManagedAccount,
    /// The provider-platform account path cannot be constructed for the network.
    #[error("provider platform account derivation path is invalid")]
    InvalidAccountPath {
        #[source]
        source: key_wallet::error::Error,
    },
    /// The managed provider-platform account lacks its hardened-only pool.
    #[error("provider platform account has no AbsentHardened pool")]
    MissingHardenedPool,
    /// The persisted or derived index cannot form a hardened child number.
    #[error("platform-node index {index} cannot form a hardened child number")]
    InvalidChildIndex {
        index: u32,
        #[source]
        source: key_wallet::error::Error,
    },
    /// The hardened-only pool refused the restored entry.
    #[error("platform-node pool refused the entry at index {index}")]
    PoolRefused {
        index: u32,
        #[source]
        source: key_wallet::error::Error,
    },
}

/// Insert one pre-derived platform-node key and restore its usage state.
pub(super) fn insert_platform_node_pool_entry(
    wallet_info: &mut key_wallet::wallet::managed_wallet_info::ManagedWalletInfo,
    network: key_wallet::Network,
    index: u32,
    address: dashcore::Address,
    public_key: [u8; 32],
    used: bool,
) -> Result<(), PlatformNodePoolError> {
    use key_wallet::managed_account::address_pool::{AddressPoolType, AddressState, PublicKeyType};
    use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
    use key_wallet::AddressInfo;

    let Some(account) = wallet_info.accounts.provider_platform_keys.as_mut() else {
        return Err(PlatformNodePoolError::NoManagedAccount);
    };
    let account_path = AccountType::ProviderPlatformKeys
        .derivation_path(network)
        .map_err(|source| PlatformNodePoolError::InvalidAccountPath { source })?;
    let pool = account
        .managed_account_type_mut()
        .address_pools_mut()
        .into_iter()
        .find(|pool| pool.pool_type() == AddressPoolType::AbsentHardened)
        .ok_or(PlatformNodePoolError::MissingHardenedPool)?;
    let child = key_wallet::bip32::ChildNumber::from_hardened_idx(index)
        .map_err(key_wallet::error::Error::Bip32)
        .map_err(|source| PlatformNodePoolError::InvalidChildIndex { index, source })?;
    let mut children: Vec<key_wallet::bip32::ChildNumber> = account_path.as_ref().to_vec();
    children.push(child);
    let info = AddressInfo {
        address,
        public_key: Some(PublicKeyType::EdDSA(public_key.to_vec())),
        index,
        path: key_wallet::bip32::DerivationPath::from(children),
        state: if used {
            AddressState::Used
        } else {
            AddressState::Available
        },
    };
    pool.insert(info)
        .map_err(|source| PlatformNodePoolError::PoolRefused { index, source })
}

#[cfg(test)]
mod tests {
    use super::*;
    use key_wallet::Network;
    // Shared with `platform-wallet`'s own `rebuild_provider_key_account` tests
    // via its `test-utils` feature (see this crate's `[dev-dependencies]`) —
    // one fixture instead of two drifting copies.
    use platform_wallet::changeset::provider_key_account::provider_key_test_wallet;
    use platform_wallet::wallet::provider_key_at_index::derive_platform_node_public_keys;

    #[test]
    fn insert_used_platform_node_pool_entry_restores_used_bookkeeping() {
        use key_wallet::managed_account::address_pool::AddressPoolType;
        use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
        use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;

        let wallet = provider_key_test_wallet();
        let key = derive_platform_node_public_keys(&wallet, Network::Testnet, 8)
            .expect("platform-node derivation")
            .pop()
            .expect("derived key");
        let payload = dashcore::address::Payload::PubkeyHash(
            dashcore::PubkeyHash::from_byte_array(key.node_id),
        );
        let address = dashcore::Address::new(Network::Testnet, payload);
        let mut wallet_info = ManagedWalletInfo::from_wallet(&wallet, 0);

        insert_platform_node_pool_entry(
            &mut wallet_info,
            Network::Testnet,
            key.index,
            address,
            key.public_key,
            true,
        )
        .expect("restore used platform-node row");

        let account = wallet_info
            .accounts
            .provider_platform_keys
            .as_ref()
            .expect("managed platform-node account");
        let pool = account
            .managed_account_type()
            .address_pools()
            .into_iter()
            .find(|pool| pool.pool_type() == AddressPoolType::AbsentHardened)
            .cloned()
            .expect("AbsentHardened pool");
        let restored = pool.addresses().get(&key.index).expect("restored entry");
        assert!(restored.is_used());
        assert_eq!(pool.highest_used(), Some(key.index));
    }

    #[test]
    fn insert_platform_node_pool_entry_rejects_unmanaged_account() {
        use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;

        let wallet = provider_key_test_wallet();
        let mut wallet_info = ManagedWalletInfo::from_wallet(&wallet, 0);
        wallet_info.accounts.provider_platform_keys = None;
        let address = dashcore::Address::new(
            Network::Testnet,
            dashcore::address::Payload::PubkeyHash(dashcore::PubkeyHash::from_byte_array(
                [0x42; 20],
            )),
        );

        let err = insert_platform_node_pool_entry(
            &mut wallet_info,
            Network::Testnet,
            0,
            address.clone(),
            [0x24; 32],
            false,
        )
        .expect_err("an unmanaged account must not report a successful insert");

        assert!(matches!(err, PlatformNodePoolError::NoManagedAccount));
    }

    #[test]
    fn insert_platform_node_pool_entry_clears_stale_used_bookkeeping_on_downgrade() {
        use key_wallet::managed_account::address_pool::AddressPoolType;
        use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
        use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;

        let wallet = provider_key_test_wallet();
        let keys = derive_platform_node_public_keys(&wallet, Network::Testnet, 8)
            .expect("platform-node derivation");
        let mut wallet_info = ManagedWalletInfo::from_wallet(&wallet, 0);

        for key in [&keys[2], &keys[7]] {
            let payload = dashcore::address::Payload::PubkeyHash(
                dashcore::PubkeyHash::from_byte_array(key.node_id),
            );
            let address = dashcore::Address::new(Network::Testnet, payload);
            insert_platform_node_pool_entry(
                &mut wallet_info,
                Network::Testnet,
                key.index,
                address.clone(),
                key.public_key,
                true,
            )
            .expect("insert used platform-node row");
        }

        for (key, expected_highest) in [(&keys[7], Some(keys[2].index)), (&keys[2], None)] {
            let payload = dashcore::address::Payload::PubkeyHash(
                dashcore::PubkeyHash::from_byte_array(key.node_id),
            );
            let address = dashcore::Address::new(Network::Testnet, payload);
            insert_platform_node_pool_entry(
                &mut wallet_info,
                Network::Testnet,
                key.index,
                address.clone(),
                key.public_key,
                false,
            )
            .expect("downgrade platform-node row to available");

            let account = wallet_info
                .accounts
                .provider_platform_keys
                .as_ref()
                .expect("managed platform-node account");
            let pool = account
                .managed_account_type()
                .address_pools()
                .into_iter()
                .find(|pool| pool.pool_type() == AddressPoolType::AbsentHardened)
                .expect("AbsentHardened pool");
            assert!(!pool.addresses()[&key.index].is_used());
            assert_eq!(pool.highest_used(), expected_highest);
        }
    }

    #[test]
    fn insert_platform_node_pool_entry_rejects_missing_hardened_pool() {
        use key_wallet::managed_account::address_pool::AddressPoolType;
        use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
        use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;

        let wallet = provider_key_test_wallet();
        let keys = derive_platform_node_public_keys(&wallet, Network::Testnet, 1)
            .expect("platform-node derivation");
        let mut info = ManagedWalletInfo::from_wallet(&wallet, 0);
        let account = info
            .accounts
            .provider_platform_keys
            .as_mut()
            .expect("managed platform-node account must exist");
        for pool in account.managed_account_type_mut().address_pools_mut() {
            *pool = key_wallet::AddressPool::new_without_generation(
                pool.base_path().clone(),
                AddressPoolType::Absent,
                pool.gap_limit(),
                pool.network(),
            );
        }

        let key = &keys[0];
        let address = dashcore::Address::new(
            Network::Testnet,
            dashcore::address::Payload::PubkeyHash(dashcore::PubkeyHash::from_byte_array(
                key.node_id,
            )),
        );
        let result = insert_platform_node_pool_entry(
            &mut info,
            Network::Testnet,
            key.index,
            address.clone(),
            key.public_key,
            false,
        );
        assert!(matches!(
            result,
            Err(PlatformNodePoolError::MissingHardenedPool)
        ));
    }
}
