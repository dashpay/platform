//! Dedicated DashPay identity shielded accounts. Account selection is wallet policy, not
//! consensus: external Orchard addresses remain valid profile values.
//!
//! Reserve the upper half of the ZIP-32 account space for identities. A wallet-owned
//! identity at index i uses account 0x40000000 + i. Ordinary accounts use the
//! lower half. This mapping never depends on a username, the current profile,
//! or local allocation history: identity discovery followed by shielded bind
//! reconstructs it even after publication was removed. Address rotation uses
//! another diversifier in the same account, so all old addresses remain covered.

use super::NetworkShieldedCoordinator;
use crate::wallet::platform_wallet::PlatformWallet;
use crate::{PlatformWalletError, ShieldedTipRecipient};
use dpp::identity::accessors::IdentityGettersV0;
use dpp::prelude::Identifier;
use grovedb_commitment_tree::PaymentAddress;
use orchard::keys::Scope;
use std::sync::{Arc, PoisonError, Weak};

pub(crate) struct PreparedTipAccount {
    account: u32,
    coordinator: Weak<NetworkShieldedCoordinator>,
    clear_generation: u64,
}

pub const SHIELDED_IDENTITY_ACCOUNT_BASE: u32 = 0x4000_0000;

pub fn shielded_identity_account_index(identity_index: u32) -> Result<u32, PlatformWalletError> {
    if identity_index >= SHIELDED_IDENTITY_ACCOUNT_BASE {
        return Err(PlatformWalletError::ShieldedKeyDerivation(
            "Identity index exceeds the identity shielded account range".to_string(),
        ));
    }
    Ok(SHIELDED_IDENTITY_ACCOUNT_BASE + identity_index)
}

pub fn is_shielded_identity_account(account: u32) -> bool {
    (SHIELDED_IDENTITY_ACCOUNT_BASE..0x8000_0000).contains(&account)
}

/// The account a tip is paid from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShieldedTipSource {
    /// The identity's dedicated tip account, at the slot the seed proves the
    /// identity holds when the notes are selected.
    Identity(Identifier),
    /// An explicit account: an ordinary account, or the tip account of a slot
    /// that holds no identity (a retired identity's funds). A slot that holds
    /// an identity is spent through [`Self::Identity`] instead.
    Account(u32),
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
            .filter_map(|index| shielded_identity_account_index(index).ok())
            .collect())
    }

    /// Prepare a dedicated receiving account without publishing anything. Full
    /// bind persists its viewing key and registers it with the coordinator before
    /// callers can publish the returned address. Repeated calls are idempotent.
    /// Requires an existing shielded bind so preparing a tip account cannot
    /// silently replace the host's ordinary account configuration.
    pub async fn prepare_shielded_identity_address(
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
        let _lifecycle = self.core().generation_payment_guard().await;
        if !self.core().is_current_generation().await {
            return Err(PlatformWalletError::WalletNotFound(hex::encode(
                self.wallet_id(),
            )));
        }
        let _operation = coordinator.operation_guard().await;
        // Held until the address is returned, so the identity cannot move to
        // another slot between the check below and the bind of the account
        // that slot selects. Publication rechecks it (see
        // `check_tip_address_publication`) because the slot can still change
        // before the host publishes.
        let _placement = self.identity.identity_placement.read().await;
        let account = self.verified_tip_account(seed, identity_id).await?;
        // Read the bound set and bind the augmented copy as one step: a bind
        // replaces the registration, so another bind landing in between
        // would have its ordinary accounts dropped by this stale snapshot.
        let _config = self.shielded_config_lock.lock().await;
        let clear_generation = coordinator.clear_generation();
        let mut accounts = self.shielded_account_indices().await;
        if accounts.is_empty() {
            return Err(PlatformWalletError::ShieldedNotBound);
        }
        #[cfg(test)]
        test_hooks::after_snapshot(self).await;
        accounts.push(account);
        self.bind_shielded_locked(seed, &accounts, coordinator)
            .await?;
        #[cfg(test)]
        test_hooks::after_bind(self).await;
        self.persister()
            .flush()
            .map_err(|e| PlatformWalletError::Persistence(e.to_string()))?;
        let address = self
            .shielded_default_address(account)
            .await
            .ok_or(PlatformWalletError::ShieldedNotBound)?;
        if coordinator.clear_generation() != clear_generation {
            return Err(PlatformWalletError::ShieldedNotBound);
        }
        self.prepared_tip_accounts
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(
                *identity_id,
                PreparedTipAccount {
                    account,
                    coordinator: Arc::downgrade(coordinator),
                    clear_generation,
                },
            );
        Ok(address)
    }

    /// The tip account of a wallet-owned identity's slot, proved with the
    /// seed. A restored index is metadata, and hosts that cannot represent
    /// "unknown" restore a placeholder 0 for wallet-attached identities, so
    /// the seed must reproduce the identity's MASTER key at the recorded index
    /// before the account that index selects is used for it.
    async fn verified_tip_account(
        &self,
        seed: &[u8],
        identity_id: &Identifier,
    ) -> Result<u32, PlatformWalletError> {
        let wm = self.wallet_manager.read().await;
        let info = wm
            .get_wallet_info(&self.wallet_id())
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id())))?;
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
        verify_identity_index(seed, self.sdk.network, index, &identity.identity)?;
        shielded_identity_account_index(index)
    }

    /// The account `source` spends. An identity spends the tip account of
    /// the slot the seed proves it holds. An explicit tip account is refused
    /// while its slot holds an identity: the user chose that identity's
    /// account by its number, and the identity in the slot may not be the one
    /// they meant.
    pub(crate) async fn tip_source_account(
        &self,
        seed: &[u8],
        source: ShieldedTipSource,
    ) -> Result<u32, PlatformWalletError> {
        match source {
            ShieldedTipSource::Identity(identity_id) => {
                self.verified_tip_account(seed, &identity_id).await
            }
            ShieldedTipSource::Account(account) if is_shielded_identity_account(account) => {
                let wm = self.wallet_manager.read().await;
                let info = wm.get_wallet_info(&self.wallet_id()).ok_or_else(|| {
                    PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id()))
                })?;
                let occupant = info
                    .identity_manager
                    .wallet_identities
                    .get(&self.wallet_id())
                    .and_then(|bucket| bucket.get(&(account - SHIELDED_IDENTITY_ACCOUNT_BASE)));
                match occupant {
                    Some(owner) => Err(PlatformWalletError::InvalidIdentityData(format!(
                        "Account {account} is the tip account of identity {}; \
                         send from that identity instead",
                        owner.identity.id()
                    ))),
                    None => Ok(account),
                }
            }
            ShieldedTipSource::Account(account) => Ok(account),
        }
    }

    /// Refuse to publish one of this wallet's shielded addresses unless
    /// preparation, in this session, proved with the seed that `identity_id`
    /// holds the slot selecting that account, and the identity still holds
    /// it. Discovery can move an identity whose keys derive at several
    /// indices, and a restored placeholder index is not proof of a slot.
    /// Ordinary accounts and internal-scope identity addresses cannot be published;
    /// this wallet scans external receiving addresses. Addresses of other wallets are
    /// the owner's choice. Only bound accounts are recognised; preparation binds the
    /// account it hands out. Callers hold the placement lock through the
    /// profile write.
    pub(crate) async fn check_tip_address_publication(
        &self,
        identity_id: &Identifier,
        address: &[u8; 43],
    ) -> Result<(), PlatformWalletError> {
        let Some(address) =
            Option::<PaymentAddress>::from(PaymentAddress::from_raw_address_bytes(address))
        else {
            return Ok(());
        };
        let owning_account = {
            let keys = self.shielded_keys.read().await;
            keys.as_ref().and_then(|keys| {
                keys.iter().find_map(|(account, views)| {
                    views
                        .full_viewing_key
                        .scope_for_address(&address)
                        .map(|scope| (*account, scope))
                })
            })
        };
        let Some((owning_account, scope)) = owning_account else {
            return Ok(());
        };
        if !is_shielded_identity_account(owning_account) {
            return Err(PlatformWalletError::InvalidIdentityData(
                "An address from this wallet's ordinary shielded accounts cannot be published \
                 in an identity profile"
                    .to_string(),
            ));
        }
        if matches!(scope, Scope::Internal) {
            return Err(PlatformWalletError::InvalidIdentityData(
                "Only external receiving addresses from this wallet can be published \
                 in an identity profile"
                    .to_string(),
            ));
        }
        let prepared = self
            .prepared_tip_accounts
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(identity_id)
            .is_some_and(|prepared| {
                prepared.account == owning_account
                    && prepared.coordinator.upgrade().is_some_and(|coordinator| {
                        coordinator.clear_generation() == prepared.clear_generation
                    })
            });
        let wm = self.wallet_manager.read().await;
        let current = wm
            .get_wallet_info(&self.wallet_id())
            .and_then(|info| info.identity_manager.managed_identity(identity_id))
            .filter(|identity| identity.wallet_id == Some(self.wallet_id()))
            .and_then(|identity| identity.identity_index)
            .and_then(|index| shielded_identity_account_index(index).ok());
        if prepared && current == Some(owning_account) {
            Ok(())
        } else {
            Err(PlatformWalletError::InvalidIdentityData(
                "This tip address was not prepared for the identity's current tip account; \
                 prepare the identity's tip address again"
                    .to_string(),
            ))
        }
    }

    /// Send only to the identity/address pair that the user confirmed. A changed
    /// name owner or profile requires a new confirmation; there is no fallback to
    /// a transparent rail. Network changes after this check cannot change the
    /// destination, which remains the explicitly confirmed raw address.
    ///
    /// The source is checked before anything is resolved or proved, and the
    /// placement lock is held through the spend, so an identity's tip is paid
    /// from the account of the slot it holds when its notes are selected.
    #[allow(clippy::too_many_arguments)]
    pub async fn send_shielded_tip<P: dpp::shielded::builder::OrchardProver>(
        &self,
        coordinator: &Arc<NetworkShieldedCoordinator>,
        seed: &[u8],
        source: ShieldedTipSource,
        username: &str,
        expected_recipient: &ShieldedTipRecipient,
        amount: u64,
        memo: [u8; 36],
        prover: P,
    ) -> Result<(), PlatformWalletError> {
        {
            let _placement = self.identity.identity_placement.read().await;
            self.tip_source_account(seed, source).await?;
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
        let _lifecycle = self.core().generation_payment_guard().await;
        if !self.core().is_current_generation().await {
            return Err(PlatformWalletError::WalletRemovedBeforeSend(hex::encode(
                self.wallet_id(),
            )));
        }
        let _operation = coordinator.operation_guard().await;
        let _placement = self.identity.identity_placement.read().await;
        let account = self.tip_source_account(seed, source).await?;
        #[cfg(test)]
        test_hooks::before_spend(self).await;
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
/// its bind, keyed by wallet instance (shared by its clones) so parallel
/// tests built from the same mnemonic do not interfere.
#[cfg(test)]
pub(crate) mod test_hooks {
    use super::PlatformWallet;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use tokio::sync::oneshot;

    type Hook = (oneshot::Sender<()>, oneshot::Receiver<()>);
    #[derive(Clone, Copy, PartialEq, Eq, Hash)]
    enum Stage {
        Snapshot,
        Bound,
        Spend,
    }
    static HOOKS: Mutex<Option<HashMap<(usize, Stage), Hook>>> = Mutex::new(None);

    fn key(wallet: &PlatformWallet) -> usize {
        Arc::as_ptr(&wallet.shielded_config_lock) as usize
    }

    fn pause(
        wallet: &PlatformWallet,
        stage: Stage,
    ) -> (oneshot::Receiver<()>, oneshot::Sender<()>) {
        let (reached_tx, reached_rx) = oneshot::channel();
        let (resume_tx, resume_rx) = oneshot::channel();
        HOOKS
            .lock()
            .unwrap()
            .get_or_insert_with(HashMap::new)
            .insert((key(wallet), stage), (reached_tx, resume_rx));
        (reached_rx, resume_tx)
    }

    async fn reach(wallet: &PlatformWallet, stage: Stage) {
        let hook = HOOKS
            .lock()
            .unwrap()
            .as_mut()
            .and_then(|hooks| hooks.remove(&(key(wallet), stage)));
        if let Some((reached, resume)) = hook {
            let _ = reached.send(());
            let _ = resume.await;
        }
    }

    pub(crate) fn pause_after_snapshot(
        wallet: &PlatformWallet,
    ) -> (oneshot::Receiver<()>, oneshot::Sender<()>) {
        pause(wallet, Stage::Snapshot)
    }

    pub(crate) async fn after_snapshot(wallet: &PlatformWallet) {
        reach(wallet, Stage::Snapshot).await;
    }

    pub(crate) fn pause_after_bind(
        wallet: &PlatformWallet,
    ) -> (oneshot::Receiver<()>, oneshot::Sender<()>) {
        pause(wallet, Stage::Bound)
    }

    pub(crate) async fn after_bind(wallet: &PlatformWallet) {
        reach(wallet, Stage::Bound).await;
    }

    pub(crate) fn pause_before_spend(
        wallet: &PlatformWallet,
    ) -> (oneshot::Receiver<()>, oneshot::Sender<()>) {
        pause(wallet, Stage::Spend)
    }

    pub(crate) async fn before_spend(wallet: &PlatformWallet) {
        reach(wallet, Stage::Spend).await;
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
        let index = shielded_identity_account_index(0).unwrap();
        let tip = OrchardKeySet::from_seed(&seed, Network::Testnet, index).unwrap();
        let restored = OrchardKeySet::from_seed(
            &seed,
            Network::Testnet,
            shielded_identity_account_index(0).unwrap(),
        )
        .unwrap();
        let other = OrchardKeySet::from_seed(
            &seed,
            Network::Testnet,
            shielded_identity_account_index(1).unwrap(),
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
        assert!(!is_shielded_identity_account(0));
        assert!(is_shielded_identity_account(index));
        assert!(shielded_identity_account_index(SHIELDED_IDENTITY_ACCOUNT_BASE).is_err());
    }
}
