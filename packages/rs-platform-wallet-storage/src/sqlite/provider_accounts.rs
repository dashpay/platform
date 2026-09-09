//! Provider account and public-key pool reconstruction for SQLite load.

use key_wallet::account::AccountType;
use platform_wallet::changeset::ProviderKeyExtendedPubKey;

/// Why a provider key-material account could not be rebuilt into an
/// [`AccountCollection`](key_wallet::account::account_collection::AccountCollection).
#[derive(Debug, thiserror::Error)]
pub(super) enum ProviderAccountRebuildError {
    /// The curve-specific account constructor rejected the key.
    #[error("provider key account is invalid")]
    Invalid(#[from] key_wallet::error::Error),
    /// The collection refused the account — its `account_type` does not match
    /// the curve (e.g. a BLS key offered as `ProviderPlatformKeys`).
    #[error("account collection rejected the provider key account: {0}")]
    Rejected(&'static str),
}

/// Rebuild a watch-only provider account in its curve-specific collection slot.
pub(super) fn rebuild_provider_key_account(
    accounts: &mut key_wallet::account::account_collection::AccountCollection,
    wallet_id: [u8; 32],
    network: key_wallet::Network,
    account_type: AccountType,
    extended_public_key: &ProviderKeyExtendedPubKey,
) -> Result<(), ProviderAccountRebuildError> {
    match extended_public_key {
        ProviderKeyExtendedPubKey::Bls(key) => {
            let account = key_wallet::account::BLSAccount::new(
                Some(wallet_id.to_vec()),
                account_type,
                key.clone(),
                network,
            )?;
            accounts
                .insert_bls_account(account)
                .map_err(ProviderAccountRebuildError::Rejected)
        }
        ProviderKeyExtendedPubKey::EdDSA(key) => {
            let account = key_wallet::account::EdDSAAccount::new(
                Some(wallet_id.to_vec()),
                account_type,
                key.clone(),
                network,
            )?;
            accounts
                .insert_eddsa_account(account)
                .map_err(ProviderAccountRebuildError::Rejected)
        }
    }
}

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
}

/// Insert one pre-derived platform-node key and restore its usage state.
pub(super) fn insert_platform_node_pool_entry(
    wallet_info: &mut key_wallet::wallet::managed_wallet_info::ManagedWalletInfo,
    network: key_wallet::Network,
    index: u32,
    address: dashcore::Address,
    script_pubkey: dashcore::ScriptBuf,
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
        .find(|pool| pool.pool_type == AddressPoolType::AbsentHardened)
        .ok_or(PlatformNodePoolError::MissingHardenedPool)?;
    let child = key_wallet::bip32::ChildNumber::from_hardened_idx(index)
        .map_err(key_wallet::error::Error::Bip32)
        .map_err(|source| PlatformNodePoolError::InvalidChildIndex { index, source })?;
    let mut children: Vec<key_wallet::bip32::ChildNumber> = account_path.as_ref().to_vec();
    children.push(child);
    let info = AddressInfo {
        address,
        script_pubkey,
        public_key: Some(PublicKeyType::EdDSA(public_key.to_vec())),
        index,
        path: key_wallet::bip32::DerivationPath::from(children),
        state: if used {
            AddressState::Used
        } else {
            AddressState::Available
        },
        tx_count: 0,
        total_received: 0,
        total_sent: 0,
        balance: 0,
        label: None,
        metadata: Default::default(),
    };

    pool.address_index.insert(info.address.clone(), index);
    pool.script_pubkey_index
        .insert(info.script_pubkey.clone(), index);
    pool.highest_generated = Some(
        pool.highest_generated
            .map_or(index, |highest| highest.max(index)),
    );
    if used {
        pool.used_indices.insert(index);
        pool.highest_used = Some(
            pool.highest_used
                .map_or(index, |highest| highest.max(index)),
        );
    } else {
        pool.used_indices.remove(&index);
        pool.highest_used = pool.used_indices.iter().max().copied();
    }
    pool.addresses.insert(index, info);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use key_wallet::Network;

    fn provider_key_test_wallet() -> key_wallet::wallet::Wallet {
        key_wallet::wallet::Wallet::from_seed_bytes(
            [0x42; 64],
            Network::Testnet,
            key_wallet::wallet::initialization::WalletAccountCreationOptions::Default,
        )
        .expect("provider key test wallet")
    }

    #[test]
    fn rebuild_provider_key_account_restores_bls_and_eddsa() {
        let wallet = provider_key_test_wallet();
        let bls_key = wallet
            .accounts
            .bls_account_of_type(AccountType::ProviderOperatorKeys)
            .expect("BLS provider account")
            .bls_public_key
            .clone();
        let eddsa_key = wallet
            .accounts
            .eddsa_account_of_type(AccountType::ProviderPlatformKeys)
            .expect("EdDSA provider account")
            .ed25519_public_key
            .clone();
        let mut accounts = key_wallet::account::account_collection::AccountCollection::new();
        let wallet_id = [0x24; 32];

        rebuild_provider_key_account(
            &mut accounts,
            wallet_id,
            Network::Testnet,
            AccountType::ProviderOperatorKeys,
            &ProviderKeyExtendedPubKey::Bls(bls_key),
        )
        .expect("rebuild BLS provider account");
        rebuild_provider_key_account(
            &mut accounts,
            wallet_id,
            Network::Testnet,
            AccountType::ProviderPlatformKeys,
            &ProviderKeyExtendedPubKey::EdDSA(eddsa_key),
        )
        .expect("rebuild EdDSA provider account");

        assert!(accounts
            .bls_account_of_type(AccountType::ProviderOperatorKeys)
            .is_some());
        assert!(accounts
            .eddsa_account_of_type(AccountType::ProviderPlatformKeys)
            .is_some());
    }

    #[test]
    fn rebuild_provider_key_account_rejects_curve_account_type_mismatch() {
        let wallet = provider_key_test_wallet();
        let bls_key = wallet
            .accounts
            .bls_account_of_type(AccountType::ProviderOperatorKeys)
            .expect("BLS provider account")
            .bls_public_key
            .clone();
        let mut accounts = key_wallet::account::account_collection::AccountCollection::new();

        let error = rebuild_provider_key_account(
            &mut accounts,
            [0x24; 32],
            Network::Testnet,
            AccountType::ProviderPlatformKeys,
            &ProviderKeyExtendedPubKey::Bls(bls_key),
        )
        .expect_err("BLS key must not rebuild as a platform-node account");

        assert!(matches!(error, ProviderAccountRebuildError::Rejected(_)));
        assert!(accounts
            .eddsa_account_of_type(AccountType::ProviderPlatformKeys)
            .is_none());
    }
}
