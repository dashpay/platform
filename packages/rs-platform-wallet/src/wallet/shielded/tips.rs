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
use crate::wallet::identity::types::dashpay::profile::parse_shielded_address;
use crate::wallet::platform_wallet::{PlatformWallet, PlatformWalletInfo};
use crate::PlatformWalletError;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::prelude::Identifier;
use grovedb_commitment_tree::Scope;
use key_wallet::wallet::root_extended_keys::RootExtendedPrivKey;
use key_wallet::Wallet;
use key_wallet_manager::WalletManager;
use std::sync::{Arc, PoisonError, Weak};

pub(crate) struct PreparedTipAccount {
    account: u32,
    coordinator: Weak<NetworkShieldedCoordinator>,
    clear_generation: u64,
    /// The hash of the MASTER authentication key the seed derives at the
    /// account's slot, which the identity carried when it was prepared.
    master_key_hash: [u8; 20],
}

impl PreparedTipAccount {
    /// The coordinator the account was prepared on, while it is alive.
    pub(crate) fn coordinator(&self) -> Option<Arc<NetworkShieldedCoordinator>> {
        self.coordinator.upgrade()
    }

    /// The prepared account, while the preparation stands: its coordinator is alive
    /// and has not been cleared since.
    pub(crate) fn standing_account(&self) -> Option<u32> {
        self.coordinator
            .upgrade()
            .is_some_and(|coordinator| coordinator.clear_generation() == self.clear_generation)
            .then_some(self.account)
    }

    /// [`Self::standing_account`], while `identity` still carries the key
    /// the seed proved. The preparation is recorded by identity id, and an
    /// identity removed and added back is another record under the same id:
    /// it is prepared only if it carries that key too, which is what the
    /// seed proof would find for it.
    pub(crate) fn standing_account_of(&self, identity: &dpp::identity::Identity) -> Option<u32> {
        self.standing_account()
            .filter(|_| carries_master_key(identity, &self.master_key_hash))
    }
}

/// Whether `identity` carries `key_hash` on an ECDSA MASTER authentication
/// key. The hash alone proves nothing: a hash-only key (`ECDSA_HASH160`) or
/// a key of another purpose or level can carry it on an identity whose
/// master key is another wallet's or another index's. A disabled key still
/// counts: rotating the master key does not change the index the identity
/// was created at.
pub(crate) fn carries_master_key(identity: &dpp::identity::Identity, key_hash: &[u8; 20]) -> bool {
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::identity::identity_public_key::methods::hash::IdentityPublicKeyHashMethodsV0;
    use dpp::identity::{KeyType, Purpose, SecurityLevel};

    identity.public_keys().values().any(|key| {
        key.key_type() == KeyType::ECDSA_SECP256K1
            && key.purpose() == Purpose::AUTHENTICATION
            && key.security_level() == SecurityLevel::MASTER
            && key.public_key_hash().ok() == Some(*key_hash)
    })
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

/// Prove `identity_index` with the seed: the identity must carry the MASTER
/// authentication key this wallet derives at that index (the key discovery and
/// registration both use), as [`carries_master_key`] defines. Fails closed
/// when no key matches, including when no public keys were restored. Returns
/// the hash of the key that proved it.
fn verify_identity_index(
    seed: &[u8],
    network: key_wallet::Network,
    identity_index: u32,
    identity: &dpp::identity::Identity,
) -> Result<[u8; 20], PlatformWalletError> {
    use crate::wallet::identity::network::{
        derive_identity_auth_key_hash_from_master, MASTER_KEY_INDEX,
    };

    let master = key_wallet::bip32::ExtendedPrivKey::new_master(network, seed)
        .map_err(|e| PlatformWalletError::ShieldedKeyDerivation(e.to_string()))?;
    let expected = derive_identity_auth_key_hash_from_master(
        &master,
        network,
        identity_index,
        MASTER_KEY_INDEX,
    )?;
    if carries_master_key(identity, &expected) {
        Ok(expected)
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
            .filter_map(|identity| {
                let index = identity.identity_index?;
                let account = shielded_identity_account_index(index);
                if account.is_err() {
                    tracing::warn!(
                        identity = %identity.identity.id(),
                        identity_index = index,
                        "identity index is outside the identity shielded account range; \
                         its shielded account is not bound"
                    );
                }
                account.ok()
            })
            .collect())
    }

    /// Fails unless `seed` is the seed this wallet was created from: the wallet id is
    /// computed from the seed's root public key, with or without the network. A wallet
    /// restored watch-only or externally signed keeps the id it was created with, so
    /// the same seed passes for it.
    ///
    /// A shielded bind derives the wallet's viewing keys from the seed it is given, and
    /// a first bind has no persisted key against which to detect a wrong one. Keys of
    /// another seed would give out addresses, a contact address among them, that a
    /// restore from this wallet's own seed never finds.
    pub(crate) fn ensure_seed_of_this_wallet(
        &self,
        seed: &[u8],
    ) -> Result<(), PlatformWalletError> {
        let root = RootExtendedPrivKey::new_master(seed)
            .map_err(|e| PlatformWalletError::ShieldedKeyDerivation(e.to_string()))?;
        let public = root.to_root_extended_pub_key();
        drop(root);
        let scoped_id =
            Wallet::compute_wallet_id_from_root_extended_pub_key(&public, Some(self.sdk.network));
        let legacy_id = Wallet::compute_wallet_id_from_root_extended_pub_key(&public, None);
        if self.wallet_id() != scoped_id && self.wallet_id() != legacy_id {
            return Err(PlatformWalletError::ShieldedKeyDerivation(
                "The supplied seed does not belong to this wallet".to_string(),
            ));
        }
        Ok(())
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
        // The account's keys are derived for this wallet's network. Bound to
        // a coordinator of another network they would scan the wrong chain,
        // while the address was handed out and recorded as prepared.
        coordinator.ensure_network(self.sdk.network)?;
        // Sampled before the first wait. A Clear that runs while this waits
        // for admission below has bumped it by the time this is admitted, and
        // the host wipes its own rows only after that Clear returns: the bind
        // here would load them and restore the notes the Clear removed.
        let clear_generation = coordinator.clear_generation();
        // Refused before any wait, as the bind below refuses it too.
        self.ensure_seed_of_this_wallet(seed)?;
        let _lifecycle = self.core().generation_payment_guard().await;
        if !self.core().is_current_generation().await {
            return Err(PlatformWalletError::WalletNotFound(hex::encode(
                self.wallet_id(),
            )));
        }
        let _operation = coordinator.operation_guard().await;
        if coordinator.clear_generation() != clear_generation {
            return Err(PlatformWalletError::ShieldedNotBound);
        }
        let (account, master_key_hash) = self.verified_tip_account(seed, identity_id).await?;
        // Read the bound set and bind the augmented copy as one step: a bind
        // replaces the registration, so another bind landing in between
        // would have its ordinary accounts dropped by this stale snapshot.
        let _config = self.shielded_config_lock.lock().await;
        // The keys this handle holds outlive a Clear, which drops the
        // registration without touching them. Preparing from them would bind
        // the wallet again on its own, and restore whatever rows the host has
        // not wiped yet. The wallet must have been bound on this coordinator
        // with no Clear since.
        self.ensure_shielded_bound_on(coordinator)?;
        let mut accounts = self.shielded_account_indices().await;
        if accounts.is_empty() {
            return Err(PlatformWalletError::ShieldedNotBound);
        }
        #[cfg(test)]
        test_hooks::after_snapshot(self).await;
        accounts.push(account);
        self.bind_shielded_locked_unfenced(seed, &accounts, coordinator, clear_generation)
            .await?;
        self.persister()
            .flush()
            .map_err(PlatformWalletError::from_flush_failure)?;
        let address = self
            .shielded_default_address(account)
            .await
            .ok_or(PlatformWalletError::ShieldedNotBound)?;
        if coordinator.clear_generation() != clear_generation {
            return Err(PlatformWalletError::ShieldedNotBound);
        }
        // The wallet-manager lock was released after the proof above, so the
        // identity can have been removed and added back at another slot while
        // the account was bound. Prove its slot again and record under that
        // lock, which every change of a slot takes for writing: what is
        // recorded is the account of the slot the identity holds as it is
        // recorded.
        let wm = self.wallet_manager.read().await;
        if self.verified_tip_account_in(&wm, seed, identity_id)?.0 != account {
            return Err(PlatformWalletError::InvalidIdentityData(
                "The identity moved to another slot while its tip account was prepared; \
                 prepare its tip address again"
                    .to_string(),
            ));
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
                    master_key_hash,
                },
            );
        drop(wm);
        Ok(address)
    }

    /// The shielded account prepared for `identity_id` in this session
    /// ([`Self::prepare_shielded_identity_address`]), while the preparation
    /// stands: its coordinator is alive and has not been cleared since, the
    /// wallet has not been removed, and the identity still holds the slot
    /// that selects the account and still carries the key the seed proved
    /// there. `None` otherwise. A recorded identity index alone is not proof
    /// of the account, so this is what tells a caller that the seed proved
    /// it.
    ///
    /// An identity removed and added back at another slot after its
    /// preparation has another account: the preparation of the old one no
    /// longer stands, and the identity is prepared again. Added back at the
    /// same slot, it is prepared only if it carries the proved key.
    pub async fn prepared_shielded_identity_account(
        &self,
        identity_id: &Identifier,
    ) -> Option<u32> {
        let wm = self.wallet_manager.read().await;
        let managed = wm
            .get_wallet_info(&self.wallet_id())?
            .identity_manager
            .managed_identity(identity_id)
            .filter(|identity| identity.wallet_id == Some(self.wallet_id()))?;
        let current = managed
            .identity_index
            .and_then(|index| shielded_identity_account_index(index).ok())?;
        self.prepared_tip_accounts
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(identity_id)
            .and_then(|prepared| prepared.standing_account_of(&managed.identity))
            .filter(|prepared| *prepared == current)
    }

    /// Refuse a profile `shieldedAddress` this wallet must not publish for
    /// `identity_id`. An address of another wallet passes: only its owner
    /// can tell what it is. One of this wallet's own passes only when it is
    /// the default address of the identity's dedicated account (the one
    /// [`Self::prepare_shielded_identity_address`] returns), and that account
    /// stands prepared for the identity in this session
    /// ([`Self::prepared_shielded_identity_account`]). An ordinary account's
    /// address would tie the wallet's own funds to the identity, another
    /// identity's account would tie the two identities together, and another
    /// address of the identity's own account is one it gives to a contact.
    ///
    /// The wallet tells its own addresses by the viewing keys of its bound
    /// shielded accounts. Before the first bind it holds none and could not
    /// tell, so it refuses with [`PlatformWalletError::ShieldedNotBound`]
    /// instead of letting one of its own through. An account of the same
    /// seed that is not bound (never bound, or left out of the last bind)
    /// is not recognised: the wallet holds no key of it, and its address
    /// passes as another wallet's would.
    ///
    /// The caller holds the admission of the coordinator the identity's
    /// account was prepared on ([`Self::prepared_tip_coordinator`]) from
    /// before this check until the profile is written, so a Clear cannot
    /// end the preparation in between.
    pub(crate) async fn check_shielded_address_publication(
        &self,
        identity_id: &Identifier,
        address: &[u8; 43],
    ) -> Result<(), PlatformWalletError> {
        let address = parse_shielded_address(address).ok_or_else(|| {
            PlatformWalletError::InvalidIdentityData(
                "Invalid or unsupported shieldedAddress".to_string(),
            )
        })?;
        let owning_account = {
            let keys = self.shielded_keys.read().await;
            let keys = keys.as_ref().ok_or(PlatformWalletError::ShieldedNotBound)?;
            keys.iter().find_map(|(account, views)| {
                views
                    .full_viewing_key
                    .scope_for_address(&address)
                    .map(|scope| (*account, scope, views.default_address == address))
            })
        };
        let Some((owning_account, scope, is_default_address)) = owning_account else {
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
        // The account's other external addresses are the ones it hands to
        // contacts, one each. Published in the profile, such an address
        // would no longer tell its contact apart.
        if !is_default_address {
            return Err(PlatformWalletError::InvalidIdentityData(
                "Only the default address of the identity's shielded account can be \
                 published in its profile; its other addresses are reserved for contacts"
                    .to_string(),
            ));
        }
        if self.prepared_shielded_identity_account(identity_id).await == Some(owning_account) {
            Ok(())
        } else {
            Err(PlatformWalletError::InvalidIdentityData(
                "This shielded address was not prepared for the identity's account in this \
                 session; prepare the identity's shielded address again"
                    .to_string(),
            ))
        }
    }

    /// The coordinator `identity_id`'s shielded account was prepared on in
    /// this session, while it is alive. `None` when the identity was never
    /// prepared.
    pub(crate) fn prepared_tip_coordinator(
        &self,
        identity_id: &Identifier,
    ) -> Option<Arc<NetworkShieldedCoordinator>> {
        self.prepared_tip_accounts
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(identity_id)
            .and_then(PreparedTipAccount::coordinator)
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
    ) -> Result<(u32, [u8; 20]), PlatformWalletError> {
        let wm = self.wallet_manager.read().await;
        self.verified_tip_account_in(&wm, seed, identity_id)
    }

    /// The shielded account of the slot the seed proves `identity_id` holds
    /// now ([`Self::verified_tip_account`]), for a spend from it.
    pub(crate) async fn verified_identity_account(
        &self,
        seed: &[u8],
        identity_id: &Identifier,
    ) -> Result<u32, PlatformWalletError> {
        self.verified_tip_account(seed, identity_id)
            .await
            .map(|(account, _)| account)
    }

    /// [`Self::verified_tip_account`] under a wallet-manager lock the caller
    /// holds.
    fn verified_tip_account_in(
        &self,
        wm: &WalletManager<PlatformWalletInfo>,
        seed: &[u8],
        identity_id: &Identifier,
    ) -> Result<(u32, [u8; 20]), PlatformWalletError> {
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
        let index = identity
            .identity_index
            .ok_or(PlatformWalletError::IdentityIndexNotSet(*identity_id))?;
        let master_key_hash =
            verify_identity_index(seed, self.sdk.network, index, &identity.identity)?;
        Ok((shielded_identity_account_index(index)?, master_key_hash))
    }
}

/// Test-only pause points (between tip preparation's account snapshot and
/// its bind, and before and after a profile write's publication check),
/// keyed by wallet instance (shared by its clones) so parallel
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
        PublicationAdmitted,
        PublicationChecked,
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

    /// Pause a profile write right after it chose the coordinator admission
    /// to hold, before its shielded publication check.
    pub(crate) fn pause_before_publication_check(
        wallet: &PlatformWallet,
    ) -> (oneshot::Receiver<()>, oneshot::Sender<()>) {
        pause(wallet, Stage::PublicationAdmitted)
    }

    pub(crate) async fn before_publication_check(wallet: &PlatformWallet) {
        reach(wallet, Stage::PublicationAdmitted).await;
    }

    /// Pause a profile write right after its shielded publication check,
    /// before the profile is submitted.
    pub(crate) fn pause_after_publication_check(
        wallet: &PlatformWallet,
    ) -> (oneshot::Receiver<()>, oneshot::Sender<()>) {
        pause(wallet, Stage::PublicationChecked)
    }

    pub(crate) async fn after_publication_check(wallet: &PlatformWallet) {
        reach(wallet, Stage::PublicationChecked).await;
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
