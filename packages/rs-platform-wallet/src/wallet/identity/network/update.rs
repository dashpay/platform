//! Mutate an identity's public-key set.

use async_trait::async_trait;
use dpp::address_funds::AddressWitness;
use dpp::identity::accessors::IdentityGettersV0;

use dpp::identity::accessors::IdentitySettersV0;
use dpp::identity::identity_public_key::accessors::v0::{
    IdentityPublicKeyGettersV0, IdentityPublicKeySettersV0,
};
use dpp::identity::Identity;
use dpp::identity::IdentityPublicKey;
use dpp::identity::KeyType;
use dpp::identity::Purpose;
use dpp::identity::SecurityLevel;
use dpp::platform_value::BinaryData;
use dpp::prelude::Identifier;
use dpp::ProtocolError;
use std::time::{SystemTime, UNIX_EPOCH};

use dpp::identity::signer::Signer;

use dash_sdk::platform::transition::put_settings::PutSettings;

use crate::changeset::{KeyWithBreadcrumb, PersistenceError};
use crate::error::{as_identity_revision_invalid, PlatformWalletError};

use super::signing_key::AvailableSigningKey;
use super::*;

// Borrowed-signer adapter — see `dpns.rs` for the same pattern.
pub(super) struct SignerRef<'a, S: ?Sized>(pub(super) &'a S);

impl<'a, S: ?Sized> std::fmt::Debug for SignerRef<'a, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SignerRef")
    }
}

#[async_trait]
impl<'a, K, S> Signer<K> for SignerRef<'a, S>
where
    K: Send + Sync,
    S: Signer<K> + ?Sized + Send + Sync,
{
    async fn sign(&self, key: &K, data: &[u8]) -> Result<BinaryData, ProtocolError> {
        self.0.sign(key, data).await
    }

    async fn sign_create_witness(
        &self,
        key: &K,
        data: &[u8],
    ) -> Result<AddressWitness, ProtocolError> {
        self.0.sign_create_witness(key, data).await
    }

    fn can_sign_with(&self, key: &K) -> bool {
        self.0.can_sign_with(key)
    }
}

// ---------------------------------------------------------------------------
// Identity update (add/disable keys)
// ---------------------------------------------------------------------------

impl IdentityWallet {
    /// Update an identity using an externally-supplied signer.
    ///
    /// Signing is routed through the supplied `&S: Signer<IdentityPublicKey>`.
    /// Required for external-signable wallets.
    ///
    /// The identity is still looked up from the in-process
    /// `IdentityManager` so we can pick the MASTER auth key the
    /// identity-update state transition requires (DPP gates this on
    /// MASTER specifically — HIGH/CRITICAL aren't accepted).
    ///
    /// An update that would disable the last valid authentication key of a
    /// security level is refused by the transition builder before anything is
    /// signed or broadcast, with
    /// `PlatformWalletError::Sdk(dash_sdk::Error::Protocol(ProtocolError::LastAuthenticationKey(_)))`
    /// (DIP-11); keys added in the same update count as valid.
    ///
    /// NOTE: callers that ADD keys via `add_public_keys` are
    /// responsible for pre-persisting the new keys' private material
    /// to whatever store the supplied signer reads from (iOS Keychain
    /// in the typical case). The signer here only signs the update
    /// transition itself; it does not derive the new keys.
    ///
    /// CACHE: after a successful broadcast the in-process
    /// `IdentityManager` takes the update locally (see
    /// `apply_landed_update`): the cached revision moves to the
    /// transition's and the added and disabled keys are applied, so a
    /// follow-up update needs no refresh in between. A cache already
    /// refreshed to the transition or past it holds the update as Platform
    /// has it and keeps it. A refresh that fetched the identity before the
    /// update landed and finishes after this step does not take the cache
    /// back: a fetched identity at a lower revision than the cache's keeps
    /// the cache's revision, key set and every disable (see
    /// `ManagedIdentity::take_fetched_identity`). When Drive refuses the
    /// update for its revision and holds a lower one than the cache, the
    /// cache takes Drive's (see `take_rejected_revision`), so the next
    /// update carries the right one. When the call fails after the
    /// broadcast (the proof wait timed out, say), the update may have
    /// landed without this step: refresh before the next update.
    pub async fn update_identity_with_external_signer<S>(
        &self,
        identity_id: &Identifier,
        add_public_keys: Vec<IdentityPublicKey>,
        disable_public_keys: Vec<u32>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<(), PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
    {
        use dash_sdk::platform::transition::broadcast::BroadcastStateTransition;
        use dpp::state_transition::identity_update_transition::methods::IdentityUpdateTransitionMethodsV0;
        use dpp::state_transition::identity_update_transition::IdentityUpdateTransition;
        use dpp::state_transition::proof_result::StateTransitionProofResult;

        // Snapshot the local identity + its `identity_index` (needed
        // for the derivation breadcrumb on the post-broadcast apply
        // pass below). Read lock only — the broadcast itself doesn't
        // touch local state.
        let (mut identity, identity_index) = {
            let wm = self.wallet_manager.read().await;
            let info = wm.get_wallet_info(&self.wallet_id).ok_or_else(|| {
                PlatformWalletError::WalletNotFound(
                    "Wallet info not found in wallet manager".to_string(),
                )
            })?;
            let manager = &info.identity_manager;
            let identity = manager
                .identity(identity_id)
                .map(|m| m.identity.clone())
                .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?;
            // `identity_index` may be `None` for out-of-wallet
            // identities (third-party identities the user added by
            // id). Tolerated here — we'll skip the breadcrumb on
            // the local-apply pass and the persister callback still
            // gets the new key, just without `(wallet_id,
            // identity_index, key_index)` derivation metadata for
            // the iOS Keychain re-derivation path.
            let index = manager.identity_index(identity_id);
            (identity, index)
        };

        // Increment revision for the update transition.
        let transition_revision = update_revision(identity_id, identity.revision())?;
        identity.set_revision(transition_revision);

        // Pick the MASTER signing key — DPP requires identity update
        // transitions to be authorized by MASTER specifically.
        let master_key_id = identity
            .available_signing_key(
                signer,
                Purpose::AUTHENTICATION,
                &[SecurityLevel::MASTER],
                &[KeyType::ECDSA_SECP256K1],
                true,
            )?
            .map(|key| key.id())
            .ok_or_else(|| {
                PlatformWalletError::InvalidIdentityData(
                    "No signable master key found on identity".to_string(),
                )
            })?;

        let identity_nonce = self
            .sdk
            .get_identity_nonce(identity.id(), true, settings)
            .await?;

        let user_fee_increase = settings
            .and_then(|s| s.user_fee_increase)
            .unwrap_or_default();

        // `try_from_identity_with_signer` consumes the keys vec, so
        // clone before handing it off — we need the originals to
        // apply locally after the broadcast succeeds.
        let added_keys_for_local_apply = add_public_keys.clone();
        let disabled_ids_for_local_apply = disable_public_keys.clone();

        let state_transition = IdentityUpdateTransition::try_from_identity_with_signer(
            &identity,
            &master_key_id,
            add_public_keys,
            disable_public_keys,
            identity_nonce,
            user_fee_increase,
            &SignerRef(signer),
            self.sdk.version(),
            None,
        )
        .await
        .map_err(|e| match e {
            // The builder's DIP-11 refusal stays typed, as on the
            // caller-supplied-identity entry point.
            refusal @ ProtocolError::LastAuthenticationKey(_) => {
                PlatformWalletError::Sdk(dash_sdk::Error::Protocol(refusal))
            }
            e => PlatformWalletError::InvalidIdentityData(format!(
                "Failed to create identity update transition: {}",
                e
            )),
        })?;

        if let Err(e) = state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(&self.sdk, settings)
            .await
        {
            self.take_rejected_revision(identity_id, transition_revision, &e)
                .await;
            return Err(PlatformWalletError::InvalidIdentityData(format!(
                "Failed to broadcast identity update: {}",
                e
            )));
        }

        self.apply_landed_update(
            identity_id,
            identity_index,
            transition_revision,
            added_keys_for_local_apply,
            disabled_ids_for_local_apply,
        )
        .await
    }

    /// Post-broadcast local apply of an update Drive accepted at
    /// `transition_revision`.
    ///
    /// Drive accepted the transition, so the on-chain identity
    /// now carries the new keys / disabled flags. If we leave
    /// the local `ManagedIdentity` cache untouched here:
    ///   - The next state transition for this identity reuses
    ///     the stale revision and Platform rejects it as a
    ///     duplicate.
    ///   - The Swift `PersistentPublicKey` rows never receive
    ///     the new keys (rows arrive via the
    ///     `IdentityKeysChangeSet` persister callback that
    ///     `add_key` fires).
    ///   - The Identity Keys list in the iOS app keeps showing
    ///     only the pre-update key set.
    ///
    /// A cache behind `transition_revision` moves to it, and the update's
    /// keys are layered in memory and stored with the scalar snapshot,
    /// revision included, as one fallible write under the write lock, so
    /// the landed revision is stored even when no key matched. An added
    /// key carries the breadcrumb that lets the iOS Keychain re-derive the
    /// matching private bytes from the wallet seed + DIP-9 path.
    ///
    /// A cache already refreshed to `transition_revision` or past it (a
    /// fetch answered between the update's snapshot and this step) holds the
    /// update as Platform has it, on-chain `disabled_at` included, and
    /// whatever later updates did: the payload is not applied over it. The
    /// fetch that put it there may not have been stored (a failed mobile
    /// round leaves the fold in memory), so the update's keys are stored
    /// here from the cache's own copies, with the scalar snapshot, as one
    /// fallible write: an added key with the breadcrumb the update gives it
    /// (a refresh stores its rows without one), a disabled key without one
    /// (the persisters keep what the row holds). The snapshot is stored even
    /// when no key matched.
    async fn apply_landed_update(
        &self,
        identity_id: &Identifier,
        identity_index: Option<u32>,
        transition_revision: u64,
        added_keys: Vec<IdentityPublicKey>,
        disabled_ids: Vec<u32>,
    ) -> Result<(), PlatformWalletError> {
        let mut wm = self.wallet_manager.write().await;
        let info = wm.get_wallet_info_mut(&self.wallet_id).ok_or_else(|| {
            PlatformWalletError::WalletNotFound(
                "Wallet info not found in wallet manager".to_string(),
            )
        })?;
        let Some(managed) = info.identity_manager.managed_identity_mut(identity_id) else {
            return Ok(());
        };

        if managed.identity.revision() >= transition_revision {
            let added_ids: Vec<u32> = added_keys.iter().map(|key| key.id()).collect();
            let mut keys: Vec<KeyWithBreadcrumb> = added_keys
                .into_iter()
                .map(|key| KeyWithBreadcrumb {
                    breadcrumb: identity_index.map(|idx| (self.wallet_id, idx, key.id())),
                    key: managed
                        .identity
                        .public_keys()
                        .get(&key.id())
                        .cloned()
                        .unwrap_or(key),
                })
                .collect();
            keys.extend(
                disabled_ids
                    .iter()
                    .filter(|id| !added_ids.contains(id))
                    .filter_map(|id| managed.identity.public_keys().get(id).cloned())
                    .map(|key| KeyWithBreadcrumb {
                        key,
                        breadcrumb: None,
                    }),
            );
            return managed
                .store_keys_with_snapshot(keys, &self.persister)
                .map_err(landed_but_not_stored);
        }

        // The identity is at the transition's revision now, so a
        // subsequent update must not reuse the pre-broadcast value.
        managed.identity.set_revision(transition_revision);

        // The added keys, then the disabled ones stamped with the local
        // wall-clock time, a placeholder the next fetch at this revision or
        // above reconciles to the on-chain block time. A disabled key keeps
        // the breadcrumb of the managed identity's own wallet slot, so it
        // keeps its private-key linkage. They are stored with the scalar
        // snapshot as one write, so the landed revision is stored even when
        // no key matched.
        let added_ids: Vec<u32> = added_keys.iter().map(|key| key.id()).collect();
        let mut keys: Vec<KeyWithBreadcrumb> = added_keys
            .into_iter()
            .map(|key| KeyWithBreadcrumb {
                breadcrumb: identity_index.map(|idx| (self.wallet_id, idx, key.id())),
                key,
            })
            .collect();
        let disabled_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or_default();
        let slot = managed.wallet_id.zip(managed.identity_index);
        keys.extend(
            disabled_ids
                .iter()
                .filter(|id| !added_ids.contains(id))
                .filter_map(|id| managed.identity.public_keys().get(id).cloned())
                .map(|mut key| {
                    key.set_disabled_at(disabled_at);
                    KeyWithBreadcrumb {
                        breadcrumb: slot.map(|(wallet_id, identity_index)| {
                            (wallet_id, identity_index, key.id())
                        }),
                        key,
                    }
                }),
        );
        managed
            .store_keys_with_snapshot(keys, &self.persister)
            .map_err(landed_but_not_stored)
    }

    /// Take Drive's revision from an update it refused for its revision.
    ///
    /// Drive takes an identity update only at one above the identity's
    /// revision and otherwise refuses it in the block with
    /// `InvalidIdentityRevisionError`, which carries the revision it holds.
    /// A cache ahead of that value cannot heal from a fetch, which comes back
    /// older and keeps the cache's revision, so every update would be
    /// refused again. A build before this one could leave a cache one past
    /// the chain (a fetch answered between an update's snapshot and its
    /// bump, which then counted twice). The cached revision moves down to
    /// Drive's and is stored with the scalar snapshot, so the next update
    /// built from the cache carries the right one; the key set is left as
    /// it is.
    ///
    /// The cache is lowered only while it is still at the revision the
    /// refused transition was built from (`transition_revision - 1`): a
    /// fetch or another update that moved it since knows more than this
    /// refusal. A cache at Drive's revision or below is left alone: raising
    /// it would take no keys with it. The error comes from one node without
    /// a proof, so a node that lies low gets later updates refused until a
    /// fetch, which is taken at or above the lowered revision, brings the
    /// cache back; it can never lock the cache ahead. A snapshot at a higher
    /// revision still buffered in the store wins the merge over this one,
    /// so a restart before it is written can bring the old revision back,
    /// to be lowered again by the next refusal. A failed store is logged;
    /// the caller gets the refusal either way.
    async fn take_rejected_revision(
        &self,
        identity_id: &Identifier,
        transition_revision: u64,
        error: &dash_sdk::Error,
    ) {
        let Some(rejection) = as_identity_revision_invalid(error) else {
            return;
        };
        if rejection.identity_id() != identity_id {
            return;
        }
        let current = *rejection.current_revision();
        let mut wm = self.wallet_manager.write().await;
        let Some(managed) = wm
            .get_wallet_info_mut(&self.wallet_id)
            .and_then(|info| info.identity_manager.managed_identity_mut(identity_id))
        else {
            return;
        };
        let built_from = transition_revision.saturating_sub(1);
        if managed.identity.revision() != built_from || built_from <= current {
            return;
        }
        managed.identity.set_revision(current);
        if let Err(e) = managed.store_keys_with_snapshot(vec![], &self.persister) {
            tracing::error!(
                identity = %identity_id,
                revision = current,
                "identity revision from Drive not persisted: {e}"
            );
        }
    }

    /// Update an identity using an externally-provided identity and signer.
    ///
    /// Unlike [`Self::update_identity_with_external_signer`], this method does
    /// **not** look up the identity in the internal `IdentityManager`. The
    /// caller supplies the `Identity`, master key ID, and a `Signer` directly.
    ///
    /// Keys in `add_public_keys` may be `IdentityPublicKey::V1` carrying a
    /// `total_budget` and/or `expires_at` (protocol version 14). The
    /// transition builder turns each key into the matching
    /// `IdentityPublicKeyInCreation` version, so a V1 key is registered
    /// with its limits inside the signed bytes and a V0 key is registered
    /// exactly as before. The consensus rules on limits (AUTHENTICATION
    /// purpose and a level below MASTER, non-zero budget, expiry in the
    /// future) are not re-checked here: Platform validates them and the
    /// caller receives the chain's error. See
    /// `docs/protocol/authentication-key-limits.md`.
    ///
    /// An update that would disable the last valid authentication key of a
    /// security level is refused by the transition builder before anything is
    /// signed or broadcast, with
    /// `dash_sdk::Error::Protocol(ProtocolError::LastAuthenticationKey(_))`
    /// (DIP-11).
    ///
    /// Returns the [`StateTransitionProofResult`] from the broadcast so callers
    /// can inspect proof-verified outcomes (e.g. updated keys, balance).
    pub async fn update_identity_with_signer<S: Signer<IdentityPublicKey>>(
        &self,
        identity: &Identity,
        master_key_id: &u32,
        add_public_keys: Vec<IdentityPublicKey>,
        disable_public_keys: Vec<u32>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<dpp::state_transition::proof_result::StateTransitionProofResult, dash_sdk::Error>
    {
        use dash_sdk::platform::transition::broadcast::BroadcastStateTransition;
        use dpp::state_transition::identity_update_transition::methods::IdentityUpdateTransitionMethodsV0;
        use dpp::state_transition::identity_update_transition::IdentityUpdateTransition;

        // Get identity nonce from Platform.
        let identity_nonce = self
            .sdk
            .get_identity_nonce(identity.id(), true, settings)
            .await?;

        let user_fee_increase = settings
            .and_then(|s| s.user_fee_increase)
            .unwrap_or_default();

        // Build the update transition.
        let state_transition = IdentityUpdateTransition::try_from_identity_with_signer(
            identity,
            master_key_id,
            add_public_keys,
            disable_public_keys,
            identity_nonce,
            user_fee_increase,
            signer,
            self.sdk.version(),
            None,
        )
        .await
        .map_err(dash_sdk::Error::Protocol)?;

        // Broadcast and wait for confirmation.
        let result = state_transition
            .broadcast_and_wait(&self.sdk, settings)
            .await?;

        Ok(result)
    }
}

/// The revision an update of an identity cached at `cached` carries: one
/// above it, refused at the last revision rather than wrapped.
fn update_revision(identity_id: &Identifier, cached: u64) -> Result<u64, PlatformWalletError> {
    cached.checked_add(1).ok_or_else(|| {
        PlatformWalletError::InvalidIdentityData(format!(
            "identity {identity_id} is at the last revision"
        ))
    })
}

/// The error for a landed update the wallet could not store. The update is
/// on Platform and in the cache either way, so the caller must not resubmit
/// it.
fn landed_but_not_stored(e: PersistenceError) -> PlatformWalletError {
    PlatformWalletError::Persistence(format!(
        "identity update landed on Platform and is in the wallet's cache, but the wallet could not \
         store it ({e}); do not resubmit it"
    ))
}

#[cfg(test)]
mod update_revision_tests {
    use super::*;

    #[test]
    fn should_carry_one_above_the_cached_revision() {
        let id = Identifier::from([7u8; 32]);
        assert_eq!(update_revision(&id, 4).unwrap(), 5);
    }

    #[test]
    fn should_refuse_an_identity_at_the_last_revision() {
        let id = Identifier::from([7u8; 32]);
        assert!(matches!(
            update_revision(&id, u64::MAX),
            Err(PlatformWalletError::InvalidIdentityData(_))
        ));
    }
}

#[cfg(test)]
mod update_identity_tests {
    use super::super::signing_key::tests::{wallet_with_signing_keys_on, CapturingStore};
    use super::*;
    use crate::changeset::PlatformWalletPersistence;
    use crate::wallet::persister::NoPlatformPersistence;
    use crate::PlatformWallet;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeySettersV0;
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use std::sync::atomic::Ordering;
    use std::sync::Arc;

    async fn cached_identity(wallet: &IdentityWallet, identity_id: &Identifier) -> Identity {
        let wm = wallet.wallet_manager.read().await;
        wm.get_wallet_info(&wallet.wallet_id)
            .unwrap()
            .identity_manager
            .identity(identity_id)
            .unwrap()
            .identity
            .clone()
    }

    /// A wallet whose cached identity is at `revision` and also holds a
    /// signable MASTER key (id 0).
    async fn wallet_at_revision(revision: u64) -> Arc<PlatformWallet> {
        wallet_at_revision_on(revision, Arc::new(NoPlatformPersistence)).await
    }

    /// [`wallet_at_revision`] over a given store.
    async fn wallet_at_revision_on<P: PlatformWalletPersistence + 'static>(
        revision: u64,
        persistence: Arc<P>,
    ) -> Arc<PlatformWallet> {
        let wallet = wallet_with_signing_keys_on(
            dash_sdk::SdkBuilder::new_mock().build().unwrap(),
            persistence,
        )
        .await;
        {
            let identity_wallet = wallet.identity();
            let mut wm = identity_wallet.wallet_manager.write().await;
            let managed = wm
                .get_wallet_info_mut(&identity_wallet.wallet_id)
                .unwrap()
                .identity_manager
                .managed_identity_mut(&Identifier::default())
                .unwrap();
            managed.identity.add_public_key(
                IdentityPublicKeyV0 {
                    id: 0,
                    purpose: Purpose::AUTHENTICATION,
                    security_level: SecurityLevel::MASTER,
                    key_type: KeyType::ECDSA_SECP256K1,
                    data: vec![0; KeyType::ECDSA_SECP256K1.default_size()].into(),
                    ..Default::default()
                }
                .into(),
            );
            managed.identity.set_revision(revision);
        }
        wallet
    }

    /// The key the landed-update tests add.
    fn key_3() -> IdentityPublicKey {
        IdentityPublicKeyV0 {
            id: 3,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            key_type: KeyType::ECDSA_SECP256K1,
            data: vec![3; KeyType::ECDSA_SECP256K1.default_size()].into(),
            ..Default::default()
        }
        .into()
    }

    #[tokio::test]
    async fn should_take_a_landed_update_only_into_a_cache_behind_it() {
        let identity_id = Identifier::default();
        let added = key_3();
        // A transition at 5 landed and the cache is still at 4: it takes
        // the revision and the payload.
        let wallet = wallet_at_revision(4).await;
        let wallet = wallet.identity();
        wallet
            .apply_landed_update(&identity_id, Some(0), 5, vec![added.clone()], vec![])
            .await
            .unwrap();
        let identity = cached_identity(wallet, &identity_id).await;
        assert_eq!(identity.revision(), 5);
        assert_eq!(identity.public_keys().get(&3), Some(&added));

        let wallet = wallet_at_revision(4).await;
        let wallet = wallet.identity();
        wallet
            .apply_landed_update(&identity_id, Some(0), 5, vec![], vec![1])
            .await
            .unwrap();
        let identity = cached_identity(wallet, &identity_id).await;
        assert_eq!(identity.revision(), 5);
        assert!(identity.public_keys()[&1].disabled_at().is_some());

        // A cache refreshed to 5 or past it already holds the update and
        // what came after, and is left as it is: one past the chain would
        // get the next update refused.
        for cached in [5, 6] {
            let wallet = wallet_at_revision(cached).await;
            let wallet = wallet.identity();
            {
                let mut wm = wallet.wallet_manager.write().await;
                wm.get_wallet_info_mut(&wallet.wallet_id)
                    .unwrap()
                    .identity_manager
                    .managed_identity_mut(&identity_id)
                    .unwrap()
                    .identity
                    .add_public_key(added.clone());
            }
            let before = cached_identity(wallet, &identity_id).await;
            wallet
                .apply_landed_update(&identity_id, Some(0), 5, vec![added.clone()], vec![1])
                .await
                .unwrap();
            assert_eq!(
                cached_identity(wallet, &identity_id).await,
                before,
                "cached at {cached}"
            );
        }
    }

    /// An update at 5 added key 3, a later one at 6 disabled it, and the
    /// cache was refreshed to 6 before the confirmation of 5 came back: key
    /// 3 stays disabled, and is stored that way with the breadcrumb the
    /// update gives it (a refresh stores its rows without one). A cache at
    /// the transition that lacks the added key takes the update's copy.
    #[tokio::test]
    async fn should_store_an_added_key_with_its_breadcrumb_when_the_cache_is_ahead() {
        let identity_id = Identifier::default();
        let store = Arc::new(CapturingStore::default());
        let wallet = wallet_at_revision_on(6, store.clone()).await;
        let wallet = wallet.identity();
        let mut disabled = key_3();
        disabled.set_disabled_at(1);
        {
            let mut wm = wallet.wallet_manager.write().await;
            wm.get_wallet_info_mut(&wallet.wallet_id)
                .unwrap()
                .identity_manager
                .managed_identity_mut(&identity_id)
                .unwrap()
                .identity
                .add_public_key(disabled.clone());
        }
        wallet
            .apply_landed_update(&identity_id, Some(0), 5, vec![key_3()], vec![])
            .await
            .unwrap();
        let identity = cached_identity(wallet, &identity_id).await;
        assert_eq!(identity.revision(), 6);
        assert_eq!(identity.public_keys()[&3], disabled);
        let stored = store.stored_key(identity_id, 3).expect("key 3 stored");
        assert_eq!(stored.public_key, disabled);
        assert_eq!(stored.wallet_id, Some(wallet.wallet_id));
        assert_eq!(
            stored
                .derivation_indices
                .map(|indices| (indices.identity_index, indices.key_index)),
            Some((0, 3))
        );

        let store = Arc::new(CapturingStore::default());
        let wallet = wallet_at_revision_on(5, store.clone()).await;
        let wallet = wallet.identity();
        wallet
            .apply_landed_update(&identity_id, Some(0), 5, vec![key_3()], vec![])
            .await
            .unwrap();
        let identity = cached_identity(wallet, &identity_id).await;
        assert_eq!(identity.revision(), 5);
        assert_eq!(identity.public_keys().get(&3), Some(&key_3()));
        let stored = store.stored_key(identity_id, 3).expect("key 3 stored");
        assert!(stored.derivation_indices.is_some());
    }

    /// A disable-only update at 5 whose confirmation finds the cache already
    /// at 5 with the disable (a concurrent fetch's fold whose store failed):
    /// the disabled key and the scalar snapshot are stored now, so a restart
    /// does not bring the key back active. With no key matched, the snapshot
    /// is still stored.
    #[tokio::test]
    async fn should_store_a_disable_the_cache_already_holds_when_the_confirmation_comes_back() {
        let identity_id = Identifier::default();
        let store = Arc::new(CapturingStore::default());
        let wallet = wallet_at_revision_on(5, store.clone()).await;
        let wallet = wallet.identity();
        {
            let mut wm = wallet.wallet_manager.write().await;
            let managed = wm
                .get_wallet_info_mut(&wallet.wallet_id)
                .unwrap()
                .identity_manager
                .managed_identity_mut(&identity_id)
                .unwrap();
            let mut key = managed.identity.public_keys()[&1].clone();
            key.set_disabled_at(7);
            managed.identity.add_public_key(key);
        }
        store.take();
        wallet
            .apply_landed_update(&identity_id, Some(0), 5, vec![], vec![1])
            .await
            .unwrap();
        let stored = store.stored_key(identity_id, 1).expect("key 1 stored");
        assert_eq!(stored.public_key.disabled_at(), Some(7), "the cache's copy");
        assert_eq!(stored.derivation_indices, None);
        let changesets = store.take();
        assert_eq!(changesets.len(), 1);
        assert!(
            changesets[0].identities.is_some(),
            "with the scalar snapshot"
        );

        wallet
            .apply_landed_update(&identity_id, Some(0), 5, vec![], vec![9])
            .await
            .unwrap();
        let changesets = store.take();
        assert_eq!(changesets.len(), 1, "the snapshot alone");
        assert!(changesets[0].identities.is_some());
    }

    /// A cache at 4 behind an update that landed at 5 stores the landed
    /// revision with the update's keys as one write: an added key with a
    /// disable, and a disable of a key the cache lacks, which stores the
    /// snapshot alone. A failed store is returned.
    #[tokio::test]
    async fn should_store_a_landed_revision_with_its_keys_in_one_write() {
        let identity_id = Identifier::default();
        let stored_revision = |changesets: &[crate::changeset::PlatformWalletChangeSet]| {
            changesets[0]
                .identities
                .as_ref()
                .and_then(|identities| identities.identities.get(&identity_id))
                .map(|entry| entry.revision)
        };
        for (added, disabled) in [(vec![key_3()], vec![1]), (vec![], vec![9])] {
            let store = Arc::new(CapturingStore::default());
            let wallet = wallet_at_revision_on(4, store.clone()).await;
            let wallet = wallet.identity();
            store.take();
            wallet
                .apply_landed_update(&identity_id, Some(0), 5, added.clone(), disabled.clone())
                .await
                .unwrap();
            let changesets = store.take();
            assert_eq!(
                changesets.len(),
                1,
                "added {added:?}, disabled {disabled:?}"
            );
            assert_eq!(stored_revision(&changesets), Some(5));
            let rows = changesets[0]
                .identity_keys
                .as_ref()
                .map_or(0, |keys| keys.upserts.len());
            assert_eq!(rows, if added.is_empty() { 0 } else { 2 });
            assert_eq!(cached_identity(wallet, &identity_id).await.revision(), 5);
        }

        let store = Arc::new(FailingStore::default());
        let wallet = wallet_at_revision_on(4, store.clone()).await;
        store.0.store(true, Ordering::SeqCst);
        let error = wallet
            .identity()
            .apply_landed_update(&identity_id, Some(0), 5, vec![], vec![1])
            .await
            .unwrap_err();
        assert!(
            matches!(&error, PlatformWalletError::Persistence(text) if text.contains("do not resubmit")),
            "{error:?}"
        );
    }

    /// Refuses every store once set.
    #[derive(Default)]
    struct FailingStore(std::sync::atomic::AtomicBool);

    impl PlatformWalletPersistence for FailingStore {
        fn store(
            &self,
            _: crate::wallet::platform_wallet::WalletId,
            _: crate::changeset::PlatformWalletChangeSet,
        ) -> Result<(), crate::changeset::PersistenceError> {
            if self.0.load(Ordering::SeqCst) {
                return Err(crate::changeset::PersistenceError::backend("store refused"));
            }
            Ok(())
        }
        fn flush(
            &self,
            _: crate::wallet::platform_wallet::WalletId,
        ) -> Result<(), crate::changeset::PersistenceError> {
            Ok(())
        }
        fn load(
            &self,
        ) -> Result<crate::changeset::ClientStartState, crate::changeset::PersistenceError>
        {
            Ok(Default::default())
        }
    }

    /// Drive's refusal of an update for an identity it holds at `current`,
    /// in the shape `shape` picks: the wait-stream envelope Drive answers
    /// with, or a bare consensus error, matched as well.
    fn revision_refusal(identity_id: Identifier, current: u64, shape: usize) -> dash_sdk::Error {
        use dpp::consensus::state::identity::invalid_identity_revision_error::InvalidIdentityRevisionError;
        use dpp::consensus::ConsensusError;

        let cause: ConsensusError = InvalidIdentityRevisionError::new(identity_id, current).into();
        match shape {
            0 => dash_sdk::Error::StateTransitionBroadcastError(
                dash_sdk::error::StateTransitionBroadcastError {
                    code: 40203,
                    message: cause.to_string(),
                    cause: Some(cause),
                },
            ),
            _ => dash_sdk::Error::Protocol(ProtocolError::ConsensusError(Box::new(cause))),
        }
    }

    /// A cache one past the chain (6 over Drive's 5, which a build before
    /// this one could leave) is refused on every update, and no fetch heals
    /// it: each comes back older. Drive's refusal carries 5, the cache takes
    /// it and stores it with the scalar snapshot, keys untouched, so the
    /// next update is built at 6.
    #[tokio::test]
    async fn should_take_drives_revision_from_an_update_refused_for_its_revision() {
        let identity_id = Identifier::default();
        for shape in [0, 1] {
            let store = Arc::new(CapturingStore::default());
            let wallet = wallet_at_revision_on(6, store.clone()).await;
            let wallet = wallet.identity();
            let before = cached_identity(wallet, &identity_id).await;
            assert_eq!(update_revision(&identity_id, before.revision()).unwrap(), 7);
            store.take();

            wallet
                .take_rejected_revision(&identity_id, 7, &revision_refusal(identity_id, 5, shape))
                .await;
            let identity = cached_identity(wallet, &identity_id).await;
            assert_eq!(identity.revision(), 5, "shape {shape}");
            assert_eq!(identity.public_keys(), before.public_keys());
            let changesets = store.take();
            assert_eq!(changesets.len(), 1);
            let stored = changesets[0]
                .identities
                .as_ref()
                .and_then(|cs| cs.identities.get(&identity_id))
                .expect("the scalar snapshot");
            assert_eq!(stored.revision, 5);
            assert_eq!(
                update_revision(&identity_id, identity.revision()).unwrap(),
                6
            );
        }
    }

    /// The cache is left alone when the refusal does not show it ahead of
    /// Drive: Drive at the cache's revision or above it (raising the cache
    /// would take no keys, and a lying node could lock it ahead), a cache
    /// that moved after the refused transition was built (a later update
    /// landed, or a fetch took a newer identity), a refusal for another
    /// identity, or another error.
    #[tokio::test]
    async fn should_keep_the_cached_revision_unless_drive_holds_a_lower_one() {
        let identity_id = Identifier::default();
        let other = Identifier::from([7; 32]);
        for (cached, transition, error) in [
            (5, 6, revision_refusal(identity_id, 5, 0)),
            (5, 6, revision_refusal(identity_id, 9, 1)),
            (7, 6, revision_refusal(identity_id, 5, 0)),
            (6, 6, revision_refusal(identity_id, 3, 1)),
            (5, 6, revision_refusal(other, 2, 0)),
            (5, 6, dash_sdk::Error::Generic("timeout".to_string())),
        ] {
            let store = Arc::new(CapturingStore::default());
            let wallet = wallet_at_revision_on(cached, store.clone()).await;
            let wallet = wallet.identity();
            let before = cached_identity(wallet, &identity_id).await;
            store.take();
            wallet
                .take_rejected_revision(&identity_id, transition, &error)
                .await;
            assert_eq!(
                cached_identity(wallet, &identity_id).await,
                before,
                "cached at {cached}: {error:?}"
            );
            assert!(store.take().is_empty(), "cached at {cached}: {error:?}");
        }
    }
}

#[cfg(test)]
#[path = "lagging_fetch_tests.rs"]
mod lagging_fetch_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{test_platform_wallet_manager_with_sdk, NoopTestPersister};
    use crate::wallet::persister::{NoPlatformPersistence, WalletPersister};
    use crate::wallet::platform_wallet::WalletId;
    use dash_sdk::query_types::IdentityNonceFetcher;
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dpp::identity::v0::IdentityV0;
    use std::collections::BTreeMap;
    use std::sync::Arc;

    /// Signs nothing; holds every key so the MASTER key is selectable.
    #[derive(Debug)]
    struct HoldsEveryKey;

    #[async_trait]
    impl Signer<IdentityPublicKey> for HoldsEveryKey {
        async fn sign(
            &self,
            _key: &IdentityPublicKey,
            _data: &[u8],
        ) -> Result<BinaryData, ProtocolError> {
            Err(ProtocolError::Generic(
                "not signing in this test".to_string(),
            ))
        }

        async fn sign_create_witness(
            &self,
            _key: &IdentityPublicKey,
            _data: &[u8],
        ) -> Result<AddressWitness, ProtocolError> {
            Err(ProtocolError::Generic(
                "not signing in this test".to_string(),
            ))
        }

        fn can_sign_with(&self, _key: &IdentityPublicKey) -> bool {
            true
        }
    }

    /// MASTER 0, CRITICAL 1, HIGH 2: one authentication key per level.
    fn identity() -> Identity {
        let keys = [
            (0, SecurityLevel::MASTER),
            (1, SecurityLevel::CRITICAL),
            (2, SecurityLevel::HIGH),
        ]
        .map(|(id, security_level)| {
            IdentityPublicKey::V0(IdentityPublicKeyV0 {
                id,
                purpose: Purpose::AUTHENTICATION,
                security_level,
                contract_bounds: None,
                key_type: KeyType::ECDSA_SECP256K1,
                read_only: false,
                data: BinaryData::new(vec![id as u8 + 2; 33]),
                disabled_at: None,
            })
        });
        IdentityV0 {
            id: Identifier::from([1; 32]),
            public_keys: keys
                .into_iter()
                .map(|key| (key.id(), key))
                .collect::<BTreeMap<_, _>>(),
            balance: 0,
            revision: 0,
        }
        .into()
    }

    /// A wallet over a mock SDK that answers only the identity-nonce query, so
    /// an update that got past the builder would fail its broadcast with a
    /// different error.
    async fn manager_answering_only_the_nonce() -> (
        Arc<crate::PlatformWalletManager<NoopTestPersister>>,
        WalletId,
    ) {
        let mut sdk = dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk");
        sdk.mock()
            .expect_fetch::<IdentityNonceFetcher, Identifier>(
                Identifier::from([1; 32]),
                Some(IdentityNonceFetcher(0)),
            )
            .await
            .expect("nonce expectation");
        test_platform_wallet_manager_with_sdk(sdk).await
    }

    #[tokio::test]
    async fn should_refuse_disabling_the_last_critical_key_before_sending_anything() {
        let (manager, wallet_id) = manager_answering_only_the_nonce().await;
        let wallet = manager.get_wallet(&wallet_id).await.expect("wallet");
        {
            let persister = WalletPersister::new(wallet_id, Arc::new(NoPlatformPersistence));
            let mut wm = manager.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("wallet info");
            info.identity_manager
                .add_identity(identity(), 0, wallet_id, &persister)
                .expect("add identity");
        }

        let result = wallet
            .identity()
            .update_identity_with_external_signer(
                &Identifier::from([1; 32]),
                vec![],
                vec![1],
                &HoldsEveryKey,
                None,
            )
            .await;
        assert!(
            matches!(
                result,
                Err(PlatformWalletError::Sdk(dash_sdk::Error::Protocol(
                    ProtocolError::LastAuthenticationKey(ref refusal)
                )))
                    if refusal.key_id == 1 && refusal.security_level == SecurityLevel::CRITICAL
            ),
            "{result:?}"
        );
    }

    /// The caller-supplied-identity entry point refuses the same way.
    #[tokio::test]
    async fn should_refuse_disabling_the_last_critical_key_with_a_supplied_identity() {
        let (manager, wallet_id) = manager_answering_only_the_nonce().await;
        let wallet = manager.get_wallet(&wallet_id).await.expect("wallet");

        let result = wallet
            .identity()
            .update_identity_with_signer(&identity(), &0, vec![], vec![1], &HoldsEveryKey, None)
            .await;
        assert!(
            matches!(
                result,
                Err(dash_sdk::Error::Protocol(ProtocolError::LastAuthenticationKey(ref refusal)))
                    if refusal.key_id == 1 && refusal.security_level == SecurityLevel::CRITICAL
            ),
            "{result:?}"
        );
    }
}
