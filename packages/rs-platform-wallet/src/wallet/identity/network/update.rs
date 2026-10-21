//! Mutate an identity's public-key set.

use async_trait::async_trait;
use dpp::address_funds::AddressWitness;
use dpp::identity::accessors::IdentityGettersV0;
use std::collections::BTreeSet;

use dpp::identity::accessors::IdentitySettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::Identity;
use dpp::identity::IdentityPublicKey;
use dpp::identity::KeyID;
use dpp::identity::KeyType;
use dpp::identity::Purpose;
use dpp::identity::SecurityLevel;
use dpp::platform_value::BinaryData;
use dpp::prelude::Identifier;
use dpp::ProtocolError;
use std::time::{SystemTime, UNIX_EPOCH};

use dpp::identity::signer::Signer;

use dash_sdk::platform::transition::put_settings::PutSettings;

use crate::changeset::{IdentityKeyDerivationIndices, KeyWithBreadcrumb, PersistenceError};
use crate::error::PlatformWalletError;

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
    /// NOTE: callers that ADD keys via `add_public_keys` are
    /// responsible for pre-persisting the new keys' private material
    /// to whatever store the supplied signer reads from (iOS Keychain
    /// in the typical case). The signer here only signs the update
    /// transition itself; it does not derive the new keys.
    ///
    /// Every added key is stored as derived from the wallet seed at the
    /// identity's index and the key's id. A caller adding a key from
    /// outside the wallet (a browser session key, say) declares it through
    /// [`Self::update_identity_with_external_signer_at_revision`]'s
    /// `external_key_ids`, so it is stored without derivation coordinates.
    ///
    /// CACHE: after a successful broadcast the in-process
    /// `IdentityManager` takes the update locally (see
    /// `apply_landed_update`): the cached revision moves to the
    /// transition's and the added and disabled keys are applied, so a
    /// follow-up update needs no refresh in between. A cache already
    /// refreshed to the transition or past it keeps its keys and stores
    /// them, taking from the update only an added key it lacks. A refresh that
    /// fetched the identity before the update landed and finishes after
    /// this step does not take the cache back: a fetched identity at a
    /// lower revision than the cache's keeps the cache's revision, key set
    /// and every disable (see `ManagedIdentity::take_fetched_identity`).
    /// When the call fails after the broadcast (the proof wait timed out,
    /// say), the update may have landed without this step: refresh before
    /// the next update. A `Persistence` error means it landed and is in the
    /// cache, but was not stored: do not resubmit it.
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
        self.update_identity_with_external_signer_at_revision(
            identity_id,
            add_public_keys,
            &[],
            disable_public_keys,
            None,
            signer,
            settings,
        )
        .await
    }

    /// [`Self::update_identity_with_external_signer`], with the revision
    /// the transition must carry.
    ///
    /// The transition takes the cached identity's revision plus one, read
    /// when the update starts. A caller that records the revision before
    /// submitting (to tell later whether the update can still land: Drive
    /// takes an identity update only at one above the identity's revision)
    /// cannot see that read, and a refresh in between moves it. With
    /// `revision` set, an update that would carry another one fails with
    /// [`PlatformWalletError::IdentityRevisionMismatch`] before anything is
    /// signed or broadcast, so the revision the caller recorded is the only
    /// one this update can land at. `None` takes whatever the cache gives,
    /// as [`Self::update_identity_with_external_signer`] does.
    ///
    /// `external_key_ids` names the added keys that do not come from the
    /// wallet's derivation (a key the caller generated or received, whose
    /// private key the wallet cannot derive from its seed). They are stored
    /// without derivation coordinates, so no client takes them for keys it
    /// can sign with by deriving; every other added key is stored as derived
    /// at the identity's index and the key's id. An id that names no added
    /// key is refused before anything is signed.
    #[allow(clippy::too_many_arguments)]
    pub async fn update_identity_with_external_signer_at_revision<S>(
        &self,
        identity_id: &Identifier,
        add_public_keys: Vec<IdentityPublicKey>,
        external_key_ids: &[KeyID],
        disable_public_keys: Vec<u32>,
        revision: Option<u64>,
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

        if let Some(id) = external_key_ids
            .iter()
            .find(|id| !add_public_keys.iter().any(|key| key.id() == **id))
        {
            return Err(PlatformWalletError::InvalidIdentityData(format!(
                "external key id {id} names no key this update adds"
            )));
        }

        // Increment revision for the update transition.
        let transition_revision = update_revision(identity_id, identity.revision(), revision)?;
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
        .map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!(
                "Failed to create identity update transition: {}",
                e
            ))
        })?;

        state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(&self.sdk, settings)
            .await
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to broadcast identity update: {}",
                    e
                ))
            })?;

        self.apply_landed_update(
            identity_id,
            identity_index,
            transition_revision,
            added_keys_for_local_apply,
            external_key_ids,
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
    ///     `IdentityKeysChangeSet` persister callback).
    ///   - The Identity Keys list in the iOS app keeps showing
    ///     only the pre-update key set.
    ///
    /// `ManagedIdentity::apply_landed_keys` applies the added and
    /// disabled keys in memory and stores them with the landed
    /// revision as one changeset, under a write lock; the breadcrumb
    /// lets the iOS Keychain re-derive the matching private bytes
    /// from the wallet seed + DIP-9 path.
    ///
    /// A cache already refreshed to `transition_revision` or past it
    /// holds the update as Platform has it: the payload is not applied,
    /// and the touched keys are stored from the cache's copies (an added
    /// key the cache lacks from the payload, taken into the cache too).
    /// Either way a store failure is returned as an update that landed
    /// but was not stored.
    ///
    /// An added key named in `external_key_ids` is stored without a
    /// breadcrumb and refutes its slot's coordinates on both paths, which
    /// clears a breadcrumb a persister holds there (one an older update
    /// inferred); every other one with the breadcrumb of its slot,
    /// `(wallet_id, identity_index, key_id)`.
    async fn apply_landed_update(
        &self,
        identity_id: &Identifier,
        identity_index: Option<u32>,
        transition_revision: u64,
        added_keys: Vec<IdentityPublicKey>,
        external_key_ids: &[KeyID],
        disabled_ids: Vec<u32>,
    ) -> Result<(), PlatformWalletError> {
        let added_breadcrumb = |id: KeyID| {
            identity_index
                .filter(|_| !external_key_ids.contains(&id))
                .map(|idx| (self.wallet_id, idx, id))
        };
        // A declared external key refutes the coordinates an update would
        // have inferred for it (older code stored them), so a persister clears
        // exactly that breadcrumb.
        let external_slot = |id: KeyID| -> BTreeSet<IdentityKeyDerivationIndices> {
            identity_index
                .filter(|_| external_key_ids.contains(&id))
                .map(|identity_index| IdentityKeyDerivationIndices {
                    identity_index,
                    key_index: id,
                })
                .into_iter()
                .collect()
        };
        let mut wm = self.wallet_manager.write().await;
        let info = wm.get_wallet_info_mut(&self.wallet_id).ok_or_else(|| {
            PlatformWalletError::WalletNotFound(
                "Wallet info not found in wallet manager".to_string(),
            )
        })?;
        let Some(managed) = info.identity_manager.managed_identity_mut(identity_id) else {
            return Ok(());
        };

        // A cache refreshed to the transition or past it already holds
        // this update's keys as Platform has them (on-chain `disabled_at`
        // included) and whatever later updates did to them: applying the
        // payload would take those back. A refresh stores the keys it adds
        // or changes without breadcrumbs, though, so the touched keys are
        // stored here from the cache's own copies, with the breadcrumbs the
        // path below would give them: an added key its slot's unless the
        // caller declared it external, a disabled key none (the persisters
        // keep the one they hold).
        // A key the update both adds and disables is taken as added, on
        // both paths.
        let added_ids: Vec<u32> = added_keys.iter().map(|key| key.id()).collect();
        let disabled: Vec<u32> = disabled_ids
            .into_iter()
            .filter(|id| !added_ids.contains(id))
            .collect();

        if managed.identity.revision() >= transition_revision {
            let mut keys = Vec::new();
            // An added key the cache lacks is stored, and taken into the
            // cache, as the update registered it.
            for key in &added_keys {
                let id = key.id();
                keys.push(KeyWithBreadcrumb {
                    key: managed
                        .identity
                        .public_keys()
                        .get(&id)
                        .unwrap_or(key)
                        .clone(),
                    breadcrumb: added_breadcrumb(id),
                    derivation_rejected: external_slot(id),
                });
            }
            for &id in &disabled {
                let Some(key) = managed.identity.public_keys().get(&id) else {
                    tracing::warn!(
                        identity = %identity_id,
                        key_id = id,
                        "landed update: a disabled key id is not on the cached identity; skipping",
                    );
                    continue;
                };
                keys.push(KeyWithBreadcrumb {
                    key: key.clone(),
                    breadcrumb: None,
                    derivation_rejected: BTreeSet::new(),
                });
            }
            // Stored with the identity's snapshot even when no touched key
            // is in the cache, so the revision reaches the store too.
            return managed
                .store_keys_with_snapshot(keys, &self.persister)
                .map_err(landed_but_not_stored);
        }

        // The identity is at the transition's revision now, so a
        // subsequent update must not reuse the pre-broadcast value.
        managed.identity.set_revision(transition_revision);

        // The added and disabled keys and the landed revision go to the
        // store as one changeset, which holds the revision even when no key
        // row matched. Disabled keys get no breadcrumb (the persisters keep
        // the one they hold) and a local wall-clock `disabled_at` placeholder: a later fetch brings
        // the on-chain block time into memory and the store. Memory holds
        // the whole update even when the store fails, and the rows stay
        // queued for the identity's next key store.
        let added = added_keys
            .into_iter()
            .map(|key| {
                let breadcrumb = added_breadcrumb(key.id());
                let derivation_rejected = external_slot(key.id());
                KeyWithBreadcrumb {
                    key,
                    breadcrumb,
                    derivation_rejected,
                }
            })
            .collect();
        let disabled_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or_default();
        managed
            .apply_landed_keys(added, &disabled, disabled_at, &self.persister)
            .map_err(landed_but_not_stored)
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
/// above it. With `expected` set, any other value is refused.
fn update_revision(
    identity_id: &Identifier,
    cached: u64,
    expected: Option<u64>,
) -> Result<u64, PlatformWalletError> {
    let next = cached.checked_add(1).ok_or_else(|| {
        PlatformWalletError::InvalidIdentityData(format!(
            "identity {identity_id} is at the last revision"
        ))
    })?;
    match expected {
        Some(expected) if expected != next => Err(PlatformWalletError::IdentityRevisionMismatch {
            identity_id: *identity_id,
            expected,
            actual: next,
        }),
        _ => Ok(next),
    }
}

/// The error for a landed update the wallet could not store. The update is
/// on Platform and in the cache either way, so the caller must not resubmit
/// it. Its key rows stay queued, and the identity's next key store (a
/// refresh, load, discovery or another landed update) carries them.
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
        assert_eq!(update_revision(&id, 4, None).unwrap(), 5);
        assert_eq!(update_revision(&id, 4, Some(5)).unwrap(), 5);
    }

    #[test]
    fn should_refuse_an_update_asked_for_another_revision() {
        let id = Identifier::from([7u8; 32]);
        // The cache moved past the revision the caller read.
        match update_revision(&id, 5, Some(5)) {
            Err(PlatformWalletError::IdentityRevisionMismatch {
                expected, actual, ..
            }) => assert_eq!((expected, actual), (5, 6)),
            other => panic!("unexpected: {other:?}"),
        }
        assert!(matches!(
            update_revision(&id, 3, Some(5)),
            Err(PlatformWalletError::IdentityRevisionMismatch { .. })
        ));
    }

    #[test]
    fn should_refuse_an_identity_at_the_last_revision() {
        let id = Identifier::from([7u8; 32]);
        assert!(matches!(
            update_revision(&id, u64::MAX, None),
            Err(PlatformWalletError::InvalidIdentityData(_))
        ));
    }
}

#[cfg(test)]
mod update_identity_at_revision_tests {
    use super::super::signing_key::tests::{
        wallet_with_signing_keys_on, CapturingStore, FailingStore,
    };
    use super::*;
    use crate::changeset::PlatformWalletPersistence;
    use crate::wallet::persister::NoPlatformPersistence;
    use crate::PlatformWallet;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeySettersV0;
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    /// Counts every call the update makes into it.
    #[derive(Debug, Default)]
    struct RecordingSigner(AtomicUsize);

    #[async_trait]
    impl Signer<IdentityPublicKey> for RecordingSigner {
        async fn sign(&self, _: &IdentityPublicKey, _: &[u8]) -> Result<BinaryData, ProtocolError> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Err(ProtocolError::Generic("recording signer".to_string()))
        }
        async fn sign_create_witness(
            &self,
            _: &IdentityPublicKey,
            _: &[u8],
        ) -> Result<AddressWitness, ProtocolError> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Err(ProtocolError::Generic("recording signer".to_string()))
        }
        fn can_sign_with(&self, _: &IdentityPublicKey) -> bool {
            self.0.fetch_add(1, Ordering::SeqCst);
            true
        }
    }

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
    /// signable MASTER key (id 0), so an update that got past the revision
    /// check would consult the signer and reach the mock SDK.
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

    /// The mock SDK has no expectations, so the update fails if it gets as
    /// far as the nonce fetch; the refusal must come before it and before
    /// the signer is consulted.
    #[tokio::test]
    async fn should_refuse_another_revision_before_signing_or_reaching_platform() {
        let identity_id = Identifier::default();
        // Cache moved past the recorded revision (5 → 6), or is behind it (3 → 4).
        for (cached, expected, actual) in [(5, 5, 6), (3, 5, 4)] {
            let wallet = wallet_at_revision(cached).await;
            let wallet = wallet.identity();
            let before = cached_identity(wallet, &identity_id).await;
            let signer = RecordingSigner::default();
            let error = wallet
                .update_identity_with_external_signer_at_revision(
                    &identity_id,
                    vec![],
                    &[],
                    vec![1],
                    Some(expected),
                    &signer,
                    None,
                )
                .await
                .unwrap_err();
            match error {
                PlatformWalletError::IdentityRevisionMismatch {
                    identity_id: id,
                    expected: e,
                    actual: a,
                } => assert_eq!((id, e, a), (identity_id, expected, actual)),
                other => panic!("unexpected: {other:?}"),
            }
            assert_eq!(signer.0.load(Ordering::SeqCst), 0);
            assert_eq!(cached_identity(wallet, &identity_id).await, before);
        }

        // Control: at the matching revision the same update gets past the
        // check, consults the signer and then fails at the mock SDK, so the
        // zero count above comes from the refusal, not from the fixture.
        let wallet = wallet_at_revision(4).await;
        let signer = RecordingSigner::default();
        let error = wallet
            .identity()
            .update_identity_with_external_signer_at_revision(
                &identity_id,
                vec![],
                &[],
                vec![1],
                Some(5),
                &signer,
                Some(PutSettings {
                    request_settings: dash_sdk::RequestSettings {
                        retries: Some(0),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
            )
            .await
            .unwrap_err();
        assert!(matches!(error, PlatformWalletError::Sdk(_)), "{error:?}");
        assert!(signer.0.load(Ordering::SeqCst) > 0);
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
            .apply_landed_update(&identity_id, Some(0), 5, vec![added.clone()], &[], vec![])
            .await
            .unwrap();
        let identity = cached_identity(wallet, &identity_id).await;
        assert_eq!(identity.revision(), 5);
        assert_eq!(identity.public_keys().get(&3), Some(&added));

        let wallet = wallet_at_revision(4).await;
        let wallet = wallet.identity();
        wallet
            .apply_landed_update(&identity_id, Some(0), 5, vec![], &[], vec![1])
            .await
            .unwrap();
        let identity = cached_identity(wallet, &identity_id).await;
        assert_eq!(identity.revision(), 5);
        assert!(identity.public_keys()[&1].disabled_at().is_some());

        // A cache refreshed to 5 or past it already holds the update and
        // what came after, and is left as it is.
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
                .apply_landed_update(&identity_id, Some(0), 5, vec![added.clone()], &[], vec![1])
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
    /// cache was refreshed to 6 before the confirmation of 5 came back:
    /// key 3 stays disabled, and is stored that way with its breadcrumb (a
    /// refresh stores its rows without one). A key the update disabled is
    /// stored as the refresh has it, with the on-chain `disabled_at` and no
    /// breadcrumb, which the persisters keep from the row they hold.
    #[tokio::test]
    async fn should_store_the_newer_cache_keys_instead_of_an_older_landed_payload() {
        let identity_id = Identifier::default();
        let added = key_3();
        let store = Arc::new(CapturingStore::default());
        let wallet = wallet_at_revision_on(6, store.clone()).await;
        let wallet = wallet.identity();
        {
            let mut wm = wallet.wallet_manager.write().await;
            let managed = wm
                .get_wallet_info_mut(&wallet.wallet_id)
                .unwrap()
                .identity_manager
                .managed_identity_mut(&identity_id)
                .unwrap();
            let mut disabled = added.clone();
            disabled.set_disabled_at(1);
            managed.identity.add_public_key(disabled);
            let mut key_1 = managed.identity.public_keys()[&1].clone();
            key_1.set_disabled_at(2);
            managed.identity.add_public_key(key_1);
        }
        wallet
            .apply_landed_update(&identity_id, Some(0), 5, vec![added], &[], vec![1])
            .await
            .unwrap();
        let identity = cached_identity(wallet, &identity_id).await;
        assert_eq!(identity.revision(), 6);
        assert_eq!(identity.public_keys()[&3].disabled_at(), Some(1));
        assert_eq!(identity.public_keys()[&1].disabled_at(), Some(2));

        let stored = store.stored_key(identity_id, 3).expect("key 3 stored");
        assert_eq!(stored.public_key.disabled_at(), Some(1));
        assert_eq!(stored.wallet_id, Some(wallet.wallet_id));
        assert_eq!(
            stored
                .derivation_indices
                .map(|indices| (indices.identity_index, indices.key_index)),
            Some((0, 3))
        );
        let stored = store.stored_key(identity_id, 1).expect("key 1 stored");
        assert_eq!(stored.public_key.disabled_at(), Some(2));
        assert_eq!(
            stored.derivation_indices, None,
            "a disable says nothing about where the key came from"
        );
    }

    /// A store failure after a landed update is returned on both paths,
    /// as an update that landed but was not stored, and the cache keeps the
    /// whole update in memory.
    #[tokio::test]
    async fn should_report_a_store_failure_after_a_landed_update() {
        let identity_id = Identifier::default();
        // The cache behind the transition takes the payload.
        let store = Arc::new(FailingStore::default());
        let wallet = wallet_at_revision_on(4, store.clone()).await;
        let wallet = wallet.identity();
        store.arm();
        let error = wallet
            .apply_landed_update(&identity_id, Some(0), 5, vec![key_3()], &[], vec![1])
            .await
            .unwrap_err();
        assert!(
            matches!(error, PlatformWalletError::Persistence(_)),
            "{error:?}"
        );
        let identity = cached_identity(wallet, &identity_id).await;
        assert_eq!(identity.revision(), 5);
        assert_eq!(identity.public_keys().get(&3), Some(&key_3()));
        assert!(identity.public_keys()[&1].disabled_at().is_some());

        // A cache at or past it only stores, and a disabled-only failure is
        // reported too.
        let store = Arc::new(FailingStore::default());
        let wallet = wallet_at_revision_on(6, store.clone()).await;
        let wallet = wallet.identity();
        store.arm();
        let error = wallet
            .apply_landed_update(&identity_id, Some(0), 5, vec![], &[], vec![1])
            .await
            .unwrap_err();
        assert!(
            matches!(error, PlatformWalletError::Persistence(_)),
            "{error:?}"
        );
    }

    /// The landed revision reaches the store even when the update writes no
    /// key row: here it disables a key the cache does not have.
    #[tokio::test]
    async fn should_store_the_landed_revision_on_its_own() {
        let identity_id = Identifier::default();
        let store = Arc::new(CapturingStore::default());
        let wallet = wallet_at_revision_on(4, store.clone()).await;
        let wallet = wallet.identity();
        store.clear();
        wallet
            .apply_landed_update(&identity_id, Some(0), 5, vec![], &[], vec![7])
            .await
            .unwrap();
        assert!(store.key_rows().is_empty());
        assert_eq!(store.stored_revision(identity_id), Some(5));

        // An update with keys stores them and the revision in one changeset.
        let store = Arc::new(CapturingStore::default());
        let wallet = wallet_at_revision_on(4, store.clone()).await;
        let wallet = wallet.identity();
        store.clear();
        wallet
            .apply_landed_update(&identity_id, Some(0), 5, vec![key_3()], &[], vec![1])
            .await
            .unwrap();
        assert_eq!(store.changeset_count(), 1);
        assert_eq!(store.stored_revision(identity_id), Some(5));
        assert_eq!(store.key_rows().len(), 2);

        // A cache already at the transition stores its snapshot too, even
        // when no touched key is in it.
        let store = Arc::new(CapturingStore::default());
        let wallet = wallet_at_revision_on(5, store.clone()).await;
        let wallet = wallet.identity();
        store.clear();
        wallet
            .apply_landed_update(&identity_id, Some(0), 5, vec![], &[], vec![7])
            .await
            .unwrap();
        assert_eq!(store.changeset_count(), 1);
        assert_eq!(store.stored_revision(identity_id), Some(5));
    }

    /// A cache at the transition's revision that lacks a key the update
    /// added takes the update's copy, in memory and in the store, with its
    /// breadcrumb: Platform has it either way.
    #[tokio::test]
    async fn should_store_an_added_key_a_cache_at_its_revision_lacks() {
        let identity_id = Identifier::default();
        let store = Arc::new(CapturingStore::default());
        let wallet = wallet_at_revision_on(5, store.clone()).await;
        let wallet = wallet.identity();
        wallet
            .apply_landed_update(&identity_id, Some(0), 5, vec![key_3()], &[], vec![])
            .await
            .unwrap();
        let identity = cached_identity(wallet, &identity_id).await;
        assert_eq!(identity.revision(), 5);
        assert_eq!(identity.public_keys().get(&3), Some(&key_3()));
        let stored = store.stored_key(identity_id, 3).expect("key 3 stored");
        assert_eq!(stored.public_key, key_3());
        assert_eq!(
            stored
                .derivation_indices
                .map(|indices| (indices.identity_index, indices.key_index)),
            Some((0, 3))
        );
    }

    /// An added key the caller declares external is stored without a
    /// derivation breadcrumb whether the cache is behind the update or
    /// already at it, so no client takes it for a key it can derive.
    #[tokio::test]
    async fn should_store_an_external_added_key_without_a_breadcrumb() {
        let identity_id = Identifier::default();
        for cached in [4, 5] {
            let store = Arc::new(CapturingStore::default());
            let wallet = wallet_at_revision_on(cached, store.clone()).await;
            let wallet = wallet.identity();
            wallet
                .apply_landed_update(&identity_id, Some(0), 5, vec![key_3()], &[3], vec![])
                .await
                .unwrap();
            let stored = store.stored_key(identity_id, 3).expect("key 3 stored");
            assert_eq!(stored.public_key, key_3(), "cached at {cached}");
            assert_eq!(stored.derivation_indices, None, "cached at {cached}");
            assert_eq!(stored.wallet_id, None, "cached at {cached}");
            assert_eq!(
                stored.derivation_rejected,
                BTreeSet::from([IdentityKeyDerivationIndices {
                    identity_index: 0,
                    key_index: 3,
                }]),
                "a declared external key refutes its slot's breadcrumb, cached at {cached}"
            );
        }
    }

    /// A key the update both adds and disables is taken as added on both
    /// paths, and an out-of-wallet identity's declared external key leaves
    /// its origin unknown (there is no slot of this wallet to refute).
    #[tokio::test]
    async fn should_take_an_added_and_disabled_key_as_added_and_leave_an_out_of_wallet_external_key_unknown(
    ) {
        let identity_id = Identifier::default();
        for cached in [4, 5] {
            let store = Arc::new(CapturingStore::default());
            let wallet = wallet_at_revision_on(cached, store.clone()).await;
            let wallet = wallet.identity();
            wallet
                .apply_landed_update(&identity_id, Some(0), 5, vec![key_3()], &[], vec![3])
                .await
                .unwrap();
            let stored = store.stored_key(identity_id, 3).expect("key 3 stored");
            assert_eq!(stored.public_key.disabled_at(), None, "cached at {cached}");
            assert!(stored.derivation_indices.is_some(), "cached at {cached}");

            let store = Arc::new(CapturingStore::default());
            let wallet = wallet_at_revision_on(cached, store.clone()).await;
            let wallet = wallet.identity();
            wallet
                .apply_landed_update(&identity_id, None, 5, vec![key_3()], &[3], vec![])
                .await
                .unwrap();
            let stored = store.stored_key(identity_id, 3).expect("key 3 stored");
            assert_eq!(stored.derivation_indices, None, "cached at {cached}");
            assert!(stored.derivation_rejected.is_empty(), "cached at {cached}");
        }
    }

    /// An external key id that names no key the update adds is refused
    /// before anything is signed.
    #[tokio::test]
    async fn should_refuse_an_external_id_the_update_does_not_add() {
        let identity_id = Identifier::default();
        let wallet = wallet_at_revision(4).await;
        let signer = RecordingSigner::default();
        let error = wallet
            .identity()
            .update_identity_with_external_signer_at_revision(
                &identity_id,
                vec![key_3()],
                &[9],
                vec![],
                None,
                &signer,
                None,
            )
            .await
            .unwrap_err();
        assert!(
            matches!(error, PlatformWalletError::InvalidIdentityData(_)),
            "{error:?}"
        );
        assert_eq!(signer.0.load(Ordering::SeqCst), 0);
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
}
