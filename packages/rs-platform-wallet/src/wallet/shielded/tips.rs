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

/// Prove `identity_index` with the seed: the identity must carry the MASTER
/// authentication key this wallet derives at that index (the key discovery and
/// registration both use). Fails closed when no key matches, including when
/// no public keys were restored.
fn verify_identity_index(
    seed: &[u8],
    network: key_wallet::Network,
    identity_index: u32,
    identity: &dpp::identity::Identity,
) -> Result<(), PlatformWalletError> {
    use crate::wallet::identity::network::{
        derive_identity_auth_key_hash_from_master, MASTER_KEY_INDEX,
    };
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::identity_public_key::methods::hash::IdentityPublicKeyHashMethodsV0;

    let master = key_wallet::bip32::ExtendedPrivKey::new_master(network, seed)
        .map_err(|e| PlatformWalletError::ShieldedKeyDerivation(e.to_string()))?;
    let expected = derive_identity_auth_key_hash_from_master(
        &master,
        network,
        identity_index,
        MASTER_KEY_INDEX,
    )?;
    let verified = identity
        .public_keys()
        .values()
        .any(|key| key.public_key_hash().ok() == Some(expected));
    if verified {
        Ok(())
    } else {
        Err(PlatformWalletError::InvalidIdentityData(format!(
            "Identity index {identity_index} is not verified for this wallet; \
             rediscover the identity before using its tip account"
        )))
    }
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
            let index = identity.identity_index.ok_or_else(|| {
                PlatformWalletError::InvalidIdentityData(
                    "Tip account requires a recoverable identity index".to_string(),
                )
            })?;
            // A restored index is metadata, and hosts that cannot represent
            // "unknown" have restored a placeholder 0 for wallet-attached
            // identities. Publishing then would hand out identity 0's tip
            // address, so require the seed to reproduce this identity's
            // MASTER key at the claimed index before allocating its account.
            verify_identity_index(seed, self.sdk.network, index, &identity.identity)?;
            shielded_tip_account_index(index)?
        };
        // Read the bound set and bind the augmented copy as one step: a bind
        // replaces the registration, so another bind landing in between
        // would have its ordinary accounts dropped by this stale snapshot.
        let _config = self.shielded_config_lock.lock().await;
        let mut accounts = self.shielded_account_indices().await;
        if accounts.is_empty() {
            return Err(PlatformWalletError::ShieldedNotBound);
        }
        #[cfg(test)]
        test_hooks::after_snapshot(self.wallet_id()).await;
        accounts.push(account);
        self.bind_shielded_locked(seed, &accounts, coordinator)
            .await?;
        self.persister()
            .flush()
            .map_err(|e| PlatformWalletError::Persistence(e.to_string()))?;
        self.shielded_default_address(account)
            .await
            .ok_or(PlatformWalletError::ShieldedNotBound)
    }

    /// Refuse to spend from a dedicated tip account whose slot holds an identity
    /// the seed does not prove to be at that index. Hosts select the source
    /// account from the identity's recorded index, and a host-restored
    /// placeholder index 0 would otherwise spend identity 0's tip pool on
    /// another identity's behalf. A tip account whose slot is empty (a retired
    /// identity's) has no such ambiguity and stays spendable.
    async fn verify_tip_account_owner(
        &self,
        seed: &[u8],
        account: u32,
    ) -> Result<(), PlatformWalletError> {
        let index = account - SHIELDED_TIP_ACCOUNT_BASE;
        let wm = self.wallet_manager.read().await;
        let info = wm
            .get_wallet_info(&self.wallet_id())
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id())))?;
        let Some(owner) = info
            .identity_manager
            .wallet_identities
            .get(&self.wallet_id())
            .and_then(|bucket| bucket.get(&index))
        else {
            return Ok(());
        };
        verify_identity_index(seed, self.sdk.network, index, &owner.identity)
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
        if is_shielded_tip_account(account) {
            self.verify_tip_account_owner(seed, account).await?;
        }
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

/// Test-only pause point between tip preparation's account snapshot and
/// its bind, keyed by wallet so parallel tests do not interfere.
#[cfg(test)]
pub(crate) mod test_hooks {
    use std::collections::HashMap;
    use std::sync::Mutex;
    use tokio::sync::oneshot;

    type Hook = (oneshot::Sender<()>, oneshot::Receiver<()>);
    static AFTER_SNAPSHOT: Mutex<Option<HashMap<[u8; 32], Hook>>> = Mutex::new(None);

    /// Arm the pause for `wallet`. Returns a receiver that fires when
    /// preparation reaches the pause, and the sender that resumes it.
    pub(crate) fn pause_after_snapshot(
        wallet: [u8; 32],
    ) -> (oneshot::Receiver<()>, oneshot::Sender<()>) {
        let (reached_tx, reached_rx) = oneshot::channel();
        let (resume_tx, resume_rx) = oneshot::channel();
        AFTER_SNAPSHOT
            .lock()
            .unwrap()
            .get_or_insert_with(HashMap::new)
            .insert(wallet, (reached_tx, resume_rx));
        (reached_rx, resume_tx)
    }

    pub(crate) async fn after_snapshot(wallet: [u8; 32]) {
        let hook = AFTER_SNAPSHOT
            .lock()
            .unwrap()
            .as_mut()
            .and_then(|hooks| hooks.remove(&wallet));
        if let Some((reached, resume)) = hook {
            let _ = reached.send(());
            let _ = resume.await;
        }
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
