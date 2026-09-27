//! Watch-only rebuild of provider key-material accounts, shared by every
//! persistence backend's load path.

use key_wallet::account::account_collection::AccountCollection;
use key_wallet::account::AccountType;
use key_wallet::Network;

use crate::changeset::ProviderKeyExtendedPubKey;

/// Why a provider key-material account could not be rebuilt into an
/// [`AccountCollection`].
#[derive(Debug, thiserror::Error)]
pub enum ProviderAccountRebuildError {
    /// The curve-specific account constructor rejected the key.
    #[error("provider key account is invalid")]
    Invalid(#[from] key_wallet::error::Error),
    /// The collection refused the account — its `account_type` does not match
    /// the curve (e.g. a BLS key offered as `ProviderPlatformKeys`).
    #[error("account collection rejected the provider key account: {0}")]
    Rejected(&'static str),
}

/// Rebuild a watch-only provider account in its curve-specific collection slot.
///
/// A BLS key becomes a `BLSAccount`, an EdDSA key an `EdDSAAccount`, both
/// parented to `wallet_id`; the account replaces whatever occupied that slot.
///
/// # Errors
///
/// [`ProviderAccountRebuildError::Rejected`] when `account_type` does not match
/// the key's curve (`ProviderOperatorKeys` ⇔ BLS, `ProviderPlatformKeys` ⇔
/// EdDSA); [`ProviderAccountRebuildError::Invalid`] when the account
/// constructor rejects the key.
pub fn rebuild_provider_key_account(
    accounts: &mut AccountCollection,
    wallet_id: [u8; 32],
    network: Network,
    account_type: AccountType,
    extended_public_key: &ProviderKeyExtendedPubKey,
) -> Result<(), ProviderAccountRebuildError> {
    match extended_public_key {
        #[cfg(feature = "bls")]
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
        #[cfg(feature = "eddsa")]
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

/// A wallet with both a BLS `ProviderOperatorKeys` account and an EdDSA
/// `ProviderPlatformKeys` account, for exercising [`rebuild_provider_key_account`].
///
/// Shared across crates (not just this module's own tests) so
/// `platform-wallet-storage`'s equivalent rebuild tests don't carry a second,
/// drifting copy — see `test-utils` in this crate's `Cargo.toml`.
#[cfg(any(test, feature = "test-utils"))]
pub fn provider_key_test_wallet() -> key_wallet::wallet::Wallet {
    key_wallet::wallet::Wallet::from_seed_bytes(
        [0x42; 64],
        Network::Testnet,
        key_wallet::wallet::initialization::WalletAccountCreationOptions::Default,
    )
    .expect("provider key test wallet")
}

#[cfg(all(test, feature = "bls", feature = "eddsa"))]
mod tests {
    use super::*;

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
        let mut accounts = AccountCollection::new();
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
        let mut accounts = AccountCollection::new();

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
