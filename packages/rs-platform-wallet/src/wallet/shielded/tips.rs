//! Dedicated DashPay tip accounts. Account selection is wallet policy, not
//! consensus: external Orchard addresses remain valid profile values.
//!
//! Reserve the upper half of the ZIP-32 account space for tips. A wallet-owned
//! identity at index i uses account 0x40000000 + i. Ordinary accounts use the
//! lower half. This mapping never depends on a username, the current profile,
//! or local allocation history: identity discovery followed by shielded bind
//! reconstructs it even after publication was removed. Address rotation uses
//! another diversifier in the same account, so all old addresses remain covered.

use super::store::ShieldedStore;
use super::NetworkShieldedCoordinator;
use crate::wallet::platform_wallet::{PlatformWallet, WalletId};
use crate::{PlatformWalletError, ShieldedTipRecipient};
use dpp::identity::accessors::IdentityGettersV0;
use dpp::prelude::Identifier;
use dpp::serialization::PlatformDeserializableTrusted;
use dpp::state_transition::StateTransition;
use grovedb_commitment_tree::PaymentAddress;
use std::sync::{Arc, PoisonError};

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

/// A wallet identity's dedicated tip account; see
/// [`PlatformWallet::identity_tip_account`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IdentityTipAccount {
    pub account: u32,
    /// Whether this session proved the identity holds the slot selecting
    /// `account`. Unverified, the account may belong to another identity.
    pub verified: bool,
}

/// How a tip recipient compares with the one last confirmed for the same
/// name from this wallet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShieldedTipRecipientChange {
    /// No payment to this name was confirmed from this wallet before.
    New,
    /// The same identity and address as last confirmed.
    Unchanged,
    /// The name now resolves to a different identity or address.
    Changed { previous: ShieldedTipRecipient },
}

/// What a user confirms before tipping a name; see
/// [`PlatformWallet::review_shielded_tip`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShieldedTipReview {
    pub recipient: ShieldedTipRecipient,
    pub change: ShieldedTipRecipientChange,
    /// An earlier payment to this address is unresolved, and a send is
    /// refused until shielded sync settles it.
    pub unresolved: bool,
}

/// Whether a failed tip may be submitted again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShieldedTipDisposition {
    /// Nothing was broadcast, or it was definitively rejected: the user may
    /// review and send again.
    NotSent,
    /// The payment may have executed or may still execute: do not send it
    /// again; shielded sync settles it.
    Unresolved,
}

impl ShieldedTipDisposition {
    /// The disposition of a failed [`PlatformWallet::send_shielded_tip`].
    /// Only [`PlatformWalletError::ShieldedSpendUnconfirmed`] can follow a
    /// broadcast whose outcome is open: every other error of the guarded
    /// transfer is raised before its broadcast or after a definitive
    /// rejection that released its notes.
    pub fn of(error: &PlatformWalletError) -> Self {
        match error {
            PlatformWalletError::ShieldedSpendUnconfirmed { .. } => Self::Unresolved,
            _ => Self::NotSent,
        }
    }
}

/// Whether `wallet_id` holds an unresolved shielded transfer to `address`:
/// one recorded before broadcast whose outcome is not yet known. A transfer
/// whose activity row is missing names no recipient and counts as unresolved
/// for every address.
fn has_unresolved_transfer_to<S: ShieldedStore>(
    store: &S,
    wallet_id: WalletId,
    address: &[u8; 43],
) -> Result<bool, PlatformWalletError> {
    let redrives = store
        .pending_redrives_for_wallet(wallet_id)
        .map_err(|e| PlatformWalletError::ShieldedStoreError(e.to_string()))?;
    for (id, redrive) in redrives {
        if !matches!(
            StateTransition::deserialize_from_bytes_trusted(&redrive.st_bytes),
            Ok(StateTransition::ShieldedTransfer(_))
        ) {
            continue;
        }
        let entry = store
            .get_activity_by_entry_id(id, &redrive.activity_id)
            .map_err(|e| PlatformWalletError::ShieldedStoreError(e.to_string()))?;
        match entry.and_then(|entry| entry.counterparty) {
            Some(counterparty) if counterparty.as_slice() != address.as_slice() => {}
            _ => return Ok(true),
        }
    }
    Ok(false)
}

/// The recipient last confirmed for `label` from `wallet_id`, if any.
fn confirmed_tip_recipient<S: ShieldedStore>(
    store: &S,
    wallet_id: WalletId,
    label: &str,
) -> Result<Option<ShieldedTipRecipient>, PlatformWalletError> {
    Ok(store
        .confirmed_tip_recipient(wallet_id, label)
        .map_err(|e| PlatformWalletError::ShieldedStoreError(e.to_string()))?
        .map(|(identity_id, address)| ShieldedTipRecipient {
            identity_id: identity_id.into(),
            address,
        }))
}

/// Whether a tip to a name may still be paid by an unsettled earlier one:
/// an unresolved transfer to its current address, or to the address last
/// confirmed for the name (the recipient may have rotated its address since
/// the earlier tip).
fn has_unresolved_tip_to<S: ShieldedStore>(
    store: &S,
    wallet_id: WalletId,
    address: &[u8; 43],
    previous: Option<&ShieldedTipRecipient>,
) -> Result<bool, PlatformWalletError> {
    if has_unresolved_transfer_to(store, wallet_id, address)? {
        return Ok(true);
    }
    match previous {
        Some(previous) if previous.address != *address => {
            has_unresolved_transfer_to(store, wallet_id, &previous.address)
        }
        _ => Ok(false),
    }
}

/// The key a confirmed recipient is stored under: the DPNS label with an
/// optional `.dash` suffix removed, in homograph-safe form, as name
/// resolution matches it.
fn tip_recipient_label(username: &str) -> String {
    let name = username.trim();
    let label = match name.rfind('.') {
        Some(dot) if name[dot..].eq_ignore_ascii_case(".dash") => &name[..dot],
        _ => name,
    };
    dash_sdk::platform::dpns_usernames::convert_to_homograph_safe_chars(label)
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
        let mut accounts = self.shielded_account_indices().await;
        if accounts.is_empty() {
            return Err(PlatformWalletError::ShieldedNotBound);
        }
        #[cfg(test)]
        test_hooks::after_snapshot(self).await;
        accounts.push(account);
        self.bind_shielded_locked(seed, &accounts, coordinator)
            .await?;
        self.persister()
            .flush()
            .map_err(|e| PlatformWalletError::Persistence(e.to_string()))?;
        let address = self
            .shielded_default_address(account)
            .await
            .ok_or(PlatformWalletError::ShieldedNotBound)?;
        self.prepared_tip_accounts
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(*identity_id, account);
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
        let mut wm = self.wallet_manager.write().await;
        let info = wm
            .get_wallet_info_mut(&self.wallet_id())
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id())))?;
        let identity = info
            .identity_manager
            .managed_identity_mut(identity_id)
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
        identity.slot_verified = true;
        shielded_tip_account_index(index)
    }

    /// The dedicated tip account of a wallet identity's slot, and whether
    /// this session has proved the identity holds that slot. Hosts show an
    /// identity's tip balance only when it is verified: an unverified index
    /// may be a restored placeholder whose account belongs to another
    /// identity. Discovery, loading, or preparing the identity's tip address
    /// verifies it.
    pub async fn identity_tip_account(
        &self,
        identity_id: &Identifier,
    ) -> Result<IdentityTipAccount, PlatformWalletError> {
        let wm = self.wallet_manager.read().await;
        let identity = wm
            .get_wallet_info(&self.wallet_id())
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id())))?
            .identity_manager
            .managed_identity(identity_id)
            .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?;
        let index = identity
            .identity_index
            .filter(|_| identity.wallet_id == Some(self.wallet_id()))
            .ok_or_else(|| {
                PlatformWalletError::InvalidIdentityData(
                    "Tip account requires a wallet-owned identity".to_string(),
                )
            })?;
        Ok(IdentityTipAccount {
            account: shielded_tip_account_index(index)?,
            verified: identity.slot_verified,
        })
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
            ShieldedTipSource::Account(account) if is_shielded_tip_account(account) => {
                let wm = self.wallet_manager.read().await;
                let info = wm.get_wallet_info(&self.wallet_id()).ok_or_else(|| {
                    PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id()))
                })?;
                let occupant = info
                    .identity_manager
                    .wallet_identities
                    .get(&self.wallet_id())
                    .and_then(|bucket| bucket.get(&(account - SHIELDED_TIP_ACCOUNT_BASE)));
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

    /// Refuse to publish one of this wallet's dedicated tip addresses unless
    /// preparation, in this session, proved with the seed that `identity_id`
    /// holds the slot selecting that account, and the identity still holds
    /// it. Discovery can move an identity whose keys derive at several
    /// indices, and a restored placeholder index is not proof of a slot.
    /// Addresses of ordinary accounts and of other wallets are the owner's
    /// choice. Only bound accounts are recognised; preparation binds the
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
                keys.iter()
                    .filter(|(account, _)| is_shielded_tip_account(**account))
                    .find(|(_, views)| views.full_viewing_key.scope_for_address(&address).is_some())
                    .map(|(account, _)| *account)
            })
        };
        let Some(owning_account) = owning_account else {
            return Ok(());
        };
        let prepared = self
            .prepared_tip_accounts
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(identity_id)
            .copied();
        let wm = self.wallet_manager.read().await;
        let current = wm
            .get_wallet_info(&self.wallet_id())
            .and_then(|info| info.identity_manager.managed_identity(identity_id))
            .filter(|identity| identity.wallet_id == Some(self.wallet_id()))
            .and_then(|identity| identity.identity_index)
            .and_then(|index| shielded_tip_account_index(index).ok());
        if prepared == Some(owning_account) && current == Some(owning_account) {
            Ok(())
        } else {
            Err(PlatformWalletError::InvalidIdentityData(
                "This tip address was not prepared for the identity's current tip account; \
                 prepare the identity's tip address again"
                    .to_string(),
            ))
        }
    }

    /// Resolve `username` for the user to confirm a tip: its current
    /// recipient, how that compares with the recipient last confirmed for the
    /// name from this wallet, and whether an earlier payment to that address
    /// is still unresolved (a send to it would be refused).
    pub async fn review_shielded_tip(
        &self,
        coordinator: &Arc<NetworkShieldedCoordinator>,
        username: &str,
    ) -> Result<ShieldedTipReview, PlatformWalletError> {
        let recipient = self
            .identity()
            .dashpay()
            .resolve_shielded_tip(username)
            .await?;
        let store = coordinator.store().read().await;
        let previous =
            confirmed_tip_recipient(&*store, self.wallet_id(), &tip_recipient_label(username))?;
        let unresolved = has_unresolved_tip_to(
            &*store,
            self.wallet_id(),
            &recipient.address,
            previous.as_ref(),
        )?;
        let change = match previous {
            None => ShieldedTipRecipientChange::New,
            Some(previous) if previous == recipient => ShieldedTipRecipientChange::Unchanged,
            Some(previous) => ShieldedTipRecipientChange::Changed { previous },
        };
        Ok(ShieldedTipReview {
            recipient,
            change,
            unresolved,
        })
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
        let _sending = self
            .tip_send_in_progress
            .try_lock()
            .map_err(|_| PlatformWalletError::ShieldedTipPending)?;
        let _placement = self.identity.identity_placement.read().await;
        let account = self.tip_source_account(seed, source).await?;
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
        // An earlier payment to this address may still execute, so a second
        // one is refused until sync settles it; then the user's confirmation
        // becomes the recipient on file for the name.
        {
            let label = tip_recipient_label(username);
            let mut store = coordinator.store().write().await;
            let previous = confirmed_tip_recipient(&*store, self.wallet_id(), &label)?;
            if has_unresolved_tip_to(
                &*store,
                self.wallet_id(),
                &current.address,
                previous.as_ref(),
            )? {
                return Err(PlatformWalletError::ShieldedTipPending);
            }
            store
                .confirm_tip_recipient(
                    self.wallet_id(),
                    &label,
                    (current.identity_id.to_buffer(), current.address),
                )
                .map_err(|e| PlatformWalletError::ShieldedStoreError(e.to_string()))?;
        }
        // Recorded as unresolved before broadcast, so a relaunch still
        // refuses a second payment until its outcome is known.
        self.shielded_transfer_with(
            coordinator,
            seed,
            account,
            &current.address,
            amount,
            memo,
            prover,
            true,
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
    static AFTER_SNAPSHOT: Mutex<Option<HashMap<usize, Hook>>> = Mutex::new(None);

    fn key(wallet: &PlatformWallet) -> usize {
        Arc::as_ptr(&wallet.shielded_config_lock) as usize
    }

    /// Arm the pause for `wallet`. Returns a receiver that fires when
    /// preparation reaches the pause, and the sender that resumes it.
    pub(crate) fn pause_after_snapshot(
        wallet: &PlatformWallet,
    ) -> (oneshot::Receiver<()>, oneshot::Sender<()>) {
        let (reached_tx, reached_rx) = oneshot::channel();
        let (resume_tx, resume_rx) = oneshot::channel();
        AFTER_SNAPSHOT
            .lock()
            .unwrap()
            .get_or_insert_with(HashMap::new)
            .insert(key(wallet), (reached_tx, resume_rx));
        (reached_rx, resume_tx)
    }

    pub(crate) async fn after_snapshot(wallet: &PlatformWallet) {
        let hook = AFTER_SNAPSHOT
            .lock()
            .unwrap()
            .as_mut()
            .and_then(|hooks| hooks.remove(&key(wallet)));
        if let Some((reached, resume)) = hook {
            let _ = reached.send(());
            let _ = resume.await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::activity::{
        ShieldedActivityEntry, ShieldedActivityKind, ShieldedActivityStatus, ShieldedDirection,
    };
    use super::super::store::{InMemoryShieldedStore, PendingRedrive, SubwalletId};
    use super::super::OrchardKeySet;
    use super::*;
    use dpp::serialization::PlatformSerializable;
    use dpp::state_transition::shielded_transfer_transition::v0::ShieldedTransferTransitionV0;
    use dpp::state_transition::shielded_transfer_transition::ShieldedTransferTransition;
    use key_wallet::Network;

    #[test]
    fn should_key_confirmed_recipients_by_the_resolved_dpns_label() {
        assert_eq!(
            tip_recipient_label("Alice"),
            tip_recipient_label("a11ce.DASH")
        );
        assert_eq!(
            tip_recipient_label(" alice.dash "),
            tip_recipient_label("alice")
        );
        assert_ne!(
            tip_recipient_label("alice"),
            tip_recipient_label("alice.dash.dash")
        );
    }

    #[test]
    fn should_classify_only_an_unconfirmed_spend_as_unresolved() {
        let unconfirmed = PlatformWalletError::ShieldedSpendUnconfirmed {
            operation: "transfer",
            reason: "result wait timed out".to_string(),
        };
        assert_eq!(
            ShieldedTipDisposition::of(&unconfirmed),
            ShieldedTipDisposition::Unresolved
        );
        for not_sent in [
            PlatformWalletError::ShieldedTipPending,
            PlatformWalletError::ShieldedBroadcastFailed("rejected".to_string()),
            PlatformWalletError::InvalidIdentityData("recipient changed".to_string()),
        ] {
            assert_eq!(
                ShieldedTipDisposition::of(&not_sent),
                ShieldedTipDisposition::NotSent,
                "{not_sent}"
            );
        }
    }

    /// A transfer recorded before broadcast blocks another payment to its
    /// recipient until it settles. Without its activity row the recipient is
    /// unknown, so it blocks every recipient; records of other spend kinds
    /// and other wallets do not count.
    #[test]
    fn should_treat_a_recorded_transfer_as_unresolved_for_its_recipient() {
        let wallet_id = [1; 32];
        let id = SubwalletId::new(wallet_id, 0);
        let transfer = StateTransition::ShieldedTransfer(ShieldedTransferTransition::V0(
            ShieldedTransferTransitionV0 {
                actions: vec![],
                value_balance: 0,
                anchor: [0; 32],
                proof: vec![],
                binding_signature: [0; 64],
            },
        ))
        .serialize_to_bytes()
        .unwrap();
        let record = |st_bytes: Vec<u8>| PendingRedrive {
            activity_id: [2; 32],
            anchor: [0; 32],
            nullifiers: vec![[3; 32]],
            st_bytes,
            attempts: 0,
            identity_nonce_finalized: false,
            identity_user_abandoned: false,
        };
        let (recipient, other) = ([4; 43], [5; 43]);

        let mut store = InMemoryShieldedStore::new();
        store.arm_redrive(id, record(transfer)).unwrap();
        assert!(has_unresolved_transfer_to(&store, wallet_id, &other).unwrap());
        store
            .save_activity(
                id,
                &ShieldedActivityEntry {
                    id: [2; 32],
                    kind: ShieldedActivityKind::Sent,
                    direction: ShieldedDirection::Out,
                    amount: 1_000,
                    fee: None,
                    counterparty: Some(recipient.to_vec()),
                    memo: None,
                    block_height: None,
                    status: ShieldedActivityStatus::Pending,
                    created_at_ms: 0,
                    min_note_position: None,
                    note_cmxs: vec![],
                    spent_nullifiers: vec![],
                },
            )
            .unwrap();
        assert!(has_unresolved_transfer_to(&store, wallet_id, &recipient).unwrap());
        assert!(!has_unresolved_transfer_to(&store, wallet_id, &other).unwrap());
        assert!(!has_unresolved_transfer_to(&store, [9; 32], &recipient).unwrap());

        let mut unshield_like = InMemoryShieldedStore::new();
        unshield_like.arm_redrive(id, record(vec![7; 16])).unwrap();
        assert!(!has_unresolved_transfer_to(&unshield_like, wallet_id, &recipient).unwrap());

        // The name's recipient rotated its address after the earlier tip: the
        // tip to the new address is still blocked through the confirmed one.
        let confirmed = ShieldedTipRecipient {
            identity_id: [6; 32].into(),
            address: recipient,
        };
        assert!(has_unresolved_tip_to(&store, wallet_id, &other, Some(&confirmed)).unwrap());
        assert!(!has_unresolved_tip_to(&store, wallet_id, &other, None).unwrap());
    }

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
