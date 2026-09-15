//! Dedicated DashPay tip accounts. Account selection is wallet policy, not
//! consensus: external Orchard addresses remain valid profile values.
//!
//! Reserve the upper half of the ZIP-32 account space for tips. A wallet-owned
//! identity at index i uses account 0x40000000 + i. Ordinary accounts use the
//! lower half. This mapping never depends on a username, the current profile,
//! or local allocation history: identity discovery followed by shielded bind
//! reconstructs it even after publication was removed. Address rotation uses
//! another diversifier in the same account, so all old addresses remain covered.

use super::NetworkShieldedCoordinator;
use crate::wallet::platform_wallet::PlatformWallet;
use crate::{PlatformWalletError, ShieldedTipRecipient};
use dpp::prelude::Identifier;
use std::sync::Arc;

pub const SHIELDED_TIP_ACCOUNT_BASE: u32 = 0x4000_0000;

pub fn shielded_tip_account_index(identity_index: u32) -> Result<u32, PlatformWalletError> {
    if identity_index >= SHIELDED_TIP_ACCOUNT_BASE {
        return Err(PlatformWalletError::ShieldedKeyDerivation(
            "Identity index exceeds the dedicated tip account range".to_string(),
        ));
    }
    Ok(SHIELDED_TIP_ACCOUNT_BASE + identity_index)
}

pub fn is_shielded_tip_account(account: u32) -> bool {
    (SHIELDED_TIP_ACCOUNT_BASE..0x8000_0000).contains(&account)
}

impl PlatformWallet {
    pub(crate) async fn discovered_tip_accounts(&self) -> Result<Vec<u32>, PlatformWalletError> {
        let wm = self.wallet_manager.read().await;
        let info = wm
            .get_wallet_info(&self.wallet_id())
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id())))?;
        // An identity outside this convention can still use ordinary accounts
        // or publish an external address; it must not block wallet-wide sync.
        Ok(info
            .identity_manager
            .managed_identities()
            .filter(|identity| identity.wallet_id == Some(self.wallet_id()))
            .filter_map(|identity| identity.identity_index)
            .filter_map(|index| shielded_tip_account_index(index).ok())
            .collect())
    }

    /// Prepare a dedicated receiving account without publishing anything. Full
    /// bind persists its viewing key and registers it with the coordinator before
    /// callers can publish the returned address. Repeated calls are idempotent.
    /// Requires an existing shielded bind so preparing a tip account cannot
    /// silently replace the host's ordinary account configuration.
    pub async fn prepare_shielded_tip_address(
        &self,
        seed: &[u8],
        identity_id: &Identifier,
        coordinator: &Arc<NetworkShieldedCoordinator>,
    ) -> Result<[u8; 43], PlatformWalletError> {
        // Do not publish a key from a mis-associated mnemonic. A first bind
        // has no persisted FVK against which to detect a wrong seed.
        let root = key_wallet::wallet::root_extended_keys::RootExtendedPrivKey::new_master(seed)
            .map_err(|e| PlatformWalletError::ShieldedKeyDerivation(e.to_string()))?;
        let public = root.to_root_extended_pub_key();
        drop(root);
        let scoped_id = key_wallet::Wallet::compute_wallet_id_from_root_extended_pub_key(
            &public,
            Some(self.sdk.network),
        );
        let legacy_id =
            key_wallet::Wallet::compute_wallet_id_from_root_extended_pub_key(&public, None);
        if self.wallet_id() != scoped_id && self.wallet_id() != legacy_id {
            return Err(PlatformWalletError::ShieldedKeyDerivation(
                "The supplied seed does not belong to this wallet".to_string(),
            ));
        }
        let account = {
            let wm = self.wallet_manager.read().await;
            let info = wm.get_wallet_info(&self.wallet_id()).ok_or_else(|| {
                PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id()))
            })?;
            let identity = info
                .identity_manager
                .managed_identity(identity_id)
                .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?;
            if identity.wallet_id != Some(self.wallet_id()) {
                return Err(PlatformWalletError::InvalidIdentityData(
                    "Tip account requires a wallet-owned identity".to_string(),
                ));
            }
            shielded_tip_account_index(identity.identity_index.ok_or_else(|| {
                PlatformWalletError::InvalidIdentityData(
                    "Tip account requires a recoverable identity index".to_string(),
                )
            })?)?
        };
        let mut accounts = self.shielded_account_indices().await;
        if accounts.is_empty() {
            return Err(PlatformWalletError::ShieldedNotBound);
        }
        accounts.push(account);
        self.bind_shielded(seed, &accounts, coordinator).await?;
        self.persister()
            .flush()
            .map_err(|e| PlatformWalletError::Persistence(e.to_string()))?;
        self.shielded_default_address(account)
            .await
            .ok_or(PlatformWalletError::ShieldedNotBound)
    }

    /// Send only to the identity/address pair that the user confirmed. A changed
    /// name owner or profile requires a new confirmation; there is no fallback to
    /// a transparent rail. Network changes after this check cannot change the
    /// destination, which remains the explicitly confirmed raw address.
    #[allow(clippy::too_many_arguments)]
    pub async fn send_shielded_tip<P: dpp::shielded::builder::OrchardProver>(
        &self,
        coordinator: &Arc<NetworkShieldedCoordinator>,
        seed: &[u8],
        account: u32,
        username: &str,
        expected_recipient: &ShieldedTipRecipient,
        amount: u64,
        memo: [u8; 36],
        prover: P,
    ) -> Result<(), PlatformWalletError> {
        let current = self
            .identity()
            .dashpay()
            .resolve_shielded_tip(username)
            .await?;
        if current != *expected_recipient {
            return Err(PlatformWalletError::InvalidIdentityData(
                "The tip recipient changed; review and confirm the payment again".to_string(),
            ));
        }
        self.shielded_transfer_to(
            coordinator,
            seed,
            account,
            &current.address,
            amount,
            memo,
            prover,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::super::OrchardKeySet;
    use super::*;
    use key_wallet::Network;

    #[test]
    fn should_derive_distinct_recoverable_tip_accounts() {
        let seed = [42u8; 64];
        let ordinary = OrchardKeySet::from_seed(&seed, Network::Testnet, 0).unwrap();
        let index = shielded_tip_account_index(0).unwrap();
        let tip = OrchardKeySet::from_seed(&seed, Network::Testnet, index).unwrap();
        let restored = OrchardKeySet::from_seed(
            &seed,
            Network::Testnet,
            shielded_tip_account_index(0).unwrap(),
        )
        .unwrap();
        let other = OrchardKeySet::from_seed(
            &seed,
            Network::Testnet,
            shielded_tip_account_index(1).unwrap(),
        )
        .unwrap();
        assert_ne!(
            ordinary.full_viewing_key.to_bytes(),
            tip.full_viewing_key.to_bytes()
        );
        assert_ne!(
            tip.default_address.to_raw_address_bytes(),
            other.default_address.to_raw_address_bytes()
        );
        assert_eq!(
            tip.full_viewing_key.to_bytes(),
            restored.full_viewing_key.to_bytes()
        );
        assert!(!is_shielded_tip_account(0));
        assert!(is_shielded_tip_account(index));
        assert!(shielded_tip_account_index(SHIELDED_TIP_ACCOUNT_BASE).is_err());
    }
}
