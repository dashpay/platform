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

use crate::changeset::KeyWithBreadcrumb;
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
    /// update for its revision and reports one below the cache's (a cache an
    /// older build left ahead of the chain, which no fetch heals since each
    /// comes back older), the update is sent once more at the revision Drive
    /// takes; the cache is not lowered on that unproven word, and the
    /// landed update brings the chain to the cache's revision (shipped builds
    /// leave a cache at most one past the chain, since every later update is
    /// refused). Drive charges for each refused update. When the call
    /// fails after the broadcast (the proof wait timed out, say), the update
    /// may have landed without this step: refresh before the next update.
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
    /// when the update starts (or, asked for it, one above the revision
    /// Drive reported when it refused the last update below the cache). A caller that records the revision before
    /// submitting (to tell later whether the update can still land: Drive
    /// takes an identity update only at one above the identity's revision)
    /// cannot see that read, and a refresh in between moves it. With
    /// `revision` set, an update that would carry another one fails with
    /// [`PlatformWalletError::IdentityRevisionMismatch`] before anything is
    /// signed or broadcast, so the revision the caller recorded is the only
    /// one this update can land at. Read the revision to record from
    /// [`Self::next_update_revision`]. When Drive refuses the update for its
    /// revision and reports one below the cache's, the update is not sent
    /// again at another revision: it fails with
    /// [`PlatformWalletError::IdentityRevisionRefused`], and
    /// [`Self::next_update_revision`] then gives the revision Drive takes,
    /// which the next update may be asked to carry. `None` takes whatever
    /// the cache gives, as [`Self::update_identity_with_external_signer`]
    /// does, and is sent once more on such a refusal.
    pub async fn update_identity_with_external_signer_at_revision<S>(
        &self,
        identity_id: &Identifier,
        add_public_keys: Vec<IdentityPublicKey>,
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
        let (mut identity, identity_index, refused_at) = {
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
            let refused_at = manager
                .identity(identity_id)
                .and_then(|m| m.revision_refused_at);
            (identity, index, refused_at)
        };

        let built_from = identity.revision();
        let mut transition_revision =
            update_revision(identity_id, built_from, revision, refused_at)?;

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

        let user_fee_increase = settings
            .and_then(|s| s.user_fee_increase)
            .unwrap_or_default();

        // Drive refuses an update whose revision is not one above the
        // identity's, and says which revision it holds. When that is below
        // the cache (a cache an older build left ahead of the chain), an
        // update asked to carry no revision is sent once more at the one
        // Drive takes; the cache itself is never lowered on that unproven
        // word, so a node that lies costs one more paid refusal. One that
        // hides a disable-only update still in the mempool behind a false
        // refusal can get both to land (Drive takes a disable of a key
        // already disabled and only re-stamps it): one more fee, and a cache
        // one revision behind until the next fetch. An update that adds a
        // key cannot land twice: Drive refuses its key id the second time.
        let mut retried = false;
        loop {
            identity.set_revision(transition_revision);
            let identity_nonce = self
                .sdk
                .get_identity_nonce(identity.id(), true, settings)
                .await?;

            let state_transition = IdentityUpdateTransition::try_from_identity_with_signer(
                &identity,
                &master_key_id,
                add_public_keys.clone(),
                disable_public_keys.clone(),
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

            let Err(e) = state_transition
                .broadcast_and_wait::<StateTransitionProofResult>(&self.sdk, settings)
                .await
            else {
                break;
            };
            let refused = revision_refusal(identity_id, &e);
            if let Some(drive) = refused {
                // Drive's word on the revision, or that the cache is not
                // ahead of it after all, replaces the last one.
                self.note_refused_revision(
                    identity_id,
                    built_from,
                    Some(drive).filter(|drive| *drive < built_from),
                )
                .await;
            }
            match after_refusal(
                refused,
                built_from,
                transition_revision,
                revision.is_some(),
                retried,
            ) {
                AfterRefusal::Retry(next) => {
                    retried = true;
                    transition_revision = next;
                }
                AfterRefusal::Refused(drive_revision) => {
                    return Err(PlatformWalletError::IdentityRevisionRefused {
                        identity_id: *identity_id,
                        refused: transition_revision,
                        drive_revision,
                    });
                }
                AfterRefusal::Fail => {
                    return Err(PlatformWalletError::InvalidIdentityData(format!(
                        "Failed to broadcast identity update: {}",
                        e
                    )));
                }
            }
        }

        self.apply_landed_update(
            identity_id,
            identity_index,
            transition_revision,
            add_public_keys,
            disable_public_keys,
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
    /// `ManagedIdentity::add_key` handles both the in-memory
    /// mutation AND the `IdentityKeysChangeSet` upsert. Run it
    /// on each new key under a write lock; the breadcrumb lets
    /// the iOS Keychain re-derive the matching private bytes
    /// from the wallet seed + DIP-9 path.
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
    /// when no key matched. A cache an older build left ahead of the chain
    /// is past the transition without holding it (the update was sent at
    /// the revision Drive takes): an added key it lacks takes the update's
    /// copy, and a disabled key it holds enabled is disabled with the local
    /// placeholder time. A cache that holds the update already shows both.
    /// Any note of a refused revision goes: Drive took this one.
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
        // Landed: Drive took this revision, so a refusal noted before it
        // says nothing any more.
        managed.revision_refused_at = None;

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
                    .map(|mut key| {
                        // A cache an older build left ahead of the chain
                        // does not hold the update it just landed.
                        if !key.is_disabled() {
                            key.set_disabled_at(local_disable_time());
                        }
                        KeyWithBreadcrumb {
                            key,
                            breadcrumb: None,
                        }
                    }),
            );
            return managed
                .store_keys_with_snapshot(keys, &self.persister)
                .map_err(|e| {
                    PlatformWalletError::Persistence(format!(
                        "identity key not persisted after update: {e}"
                    ))
                });
        }

        // The identity is at the transition's revision now, so a
        // subsequent update must not reuse the pre-broadcast value.
        managed.identity.set_revision(transition_revision);

        for key in added_keys {
            let breadcrumb = identity_index.map(|idx| (self.wallet_id, idx, key.id()));
            managed
                .add_key(key, breadcrumb, &self.persister)
                .map_err(|e| {
                    PlatformWalletError::Persistence(format!(
                        "identity key not persisted after update: {e}"
                    ))
                })?;
        }

        if !disabled_ids.is_empty() {
            // Disable-side counterpart to the `add_key` loop
            // above: stamp `disabled_at` on the matching cached
            // keys and fire the persister so the Swift
            // `PersistentPublicKey.disabledAt` rows flip without
            // a network re-fetch. The local wall-clock timestamp
            // is a placeholder — the next fetch at this revision or
            // above reconciles it to the authoritative on-chain block
            // time. `disable_keys` rebuilds the derivation breadcrumb
            // from the managed identity's own wallet slot, so the
            // disabled key keeps its private-key linkage.
            managed.disable_keys(&disabled_ids, local_disable_time(), &self.persister);
        }

        Ok(())
    }

    /// Record Drive's `drive_revision` from a refusal of an update built on
    /// the cached revision `built_from`, or clear it (`None`: Drive holds
    /// the cache's revision or above), while the cache is still there.
    async fn note_refused_revision(
        &self,
        identity_id: &Identifier,
        built_from: u64,
        drive_revision: Option<u64>,
    ) {
        let mut wm = self.wallet_manager.write().await;
        let Some(managed) = wm
            .get_wallet_info_mut(&self.wallet_id)
            .and_then(|info| info.identity_manager.managed_identity_mut(identity_id))
        else {
            return;
        };
        if managed.identity.revision() == built_from {
            managed.revision_refused_at = drive_revision.map(|drive| (built_from, drive));
        }
    }

    /// The revision the next update of `identity_id` carries: one above the
    /// cached identity's, or one above the revision Drive reported when it
    /// refused the last update for its revision while the cache is still
    /// where it was then ([`PlatformWalletError::IdentityRevisionRefused`]).
    /// A caller that records the revision before submitting reads it here
    /// and passes it to [`Self::update_identity_with_external_signer_at_revision`].
    /// After such a refusal the cached revision is at or above the one
    /// returned while the chain is below it, so settle a record of an update
    /// sent at it against a proved revision (a fetch whose answer is at or
    /// above the record), not against the cache.
    pub async fn next_update_revision(
        &self,
        identity_id: &Identifier,
    ) -> Result<u64, PlatformWalletError> {
        let wm = self.wallet_manager.read().await;
        let managed = wm
            .get_wallet_info(&self.wallet_id)
            .ok_or_else(|| {
                PlatformWalletError::WalletNotFound(
                    "Wallet info not found in wallet manager".to_string(),
                )
            })?
            .identity_manager
            .identity(identity_id)
            .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?;
        let cached = managed.identity.revision();
        match managed.revision_refused_at {
            Some((at, drive)) if at == cached => Ok(drive + 1),
            _ => update_revision(identity_id, cached, None, None),
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
/// above it. With `expected` set, any other value is refused, except one
/// above the revision Drive reported when it refused the last update below
/// the cache, while the cache is still at `cached` (`refused_at`).
fn update_revision(
    identity_id: &Identifier,
    cached: u64,
    expected: Option<u64>,
    refused_at: Option<(u64, u64)>,
) -> Result<u64, PlatformWalletError> {
    let next = cached.checked_add(1).ok_or_else(|| {
        PlatformWalletError::InvalidIdentityData(format!(
            "identity {identity_id} is at the last revision"
        ))
    })?;
    match expected {
        Some(expected) if expected == next => Ok(next),
        // Drive refused the last update and reported a revision below the
        // cache's, which is still where it was: the caller asks for the one
        // Drive takes (`next_update_revision`).
        Some(expected)
            if refused_at.is_some_and(|(at, drive)| at == cached && drive + 1 == expected) =>
        {
            Ok(expected)
        }
        Some(expected) => Err(PlatformWalletError::IdentityRevisionMismatch {
            identity_id: *identity_id,
            expected,
            actual: next,
        }),
        None => Ok(next),
    }
}

/// What an update does after a failed broadcast.
#[derive(Debug, PartialEq, Eq)]
enum AfterRefusal {
    /// Send the update once more, at this revision.
    Retry(u64),
    /// Fail with `IdentityRevisionRefused`, Drive at this revision.
    Refused(u64),
    /// Fail with the broadcast's error.
    Fail,
}

/// The step after a broadcast of an update built on the cached revision
/// `built_from` at `transition` failed, `refused` being Drive's revision
/// when it refused the update for its revision. Only a refusal below the
/// cache (the cache is ahead of the chain) and not already at the revision
/// Drive takes leads anywhere: a pinned update fails with it, an update
/// asked for no revision is sent once more at the one Drive takes.
fn after_refusal(
    refused: Option<u64>,
    built_from: u64,
    transition: u64,
    pinned: bool,
    retried: bool,
) -> AfterRefusal {
    let Some(drive) = refused
        .filter(|drive| *drive < built_from)
        .filter(|drive| drive + 1 != transition)
    else {
        return AfterRefusal::Fail;
    };
    if pinned {
        AfterRefusal::Refused(drive)
    } else if retried {
        AfterRefusal::Fail
    } else {
        AfterRefusal::Retry(drive + 1)
    }
}

/// Drive's revision from its refusal of an update of `identity_id` for its
/// revision (`InvalidIdentityRevisionError`, unproven).
fn revision_refusal(identity_id: &Identifier, error: &dash_sdk::Error) -> Option<u64> {
    as_identity_revision_invalid(error)
        .filter(|rejection| rejection.identity_id() == identity_id)
        .map(|rejection| *rejection.current_revision())
}

/// The local placeholder for a disable's `disabled_at`; the next fetch at
/// the update's revision or above takes the on-chain block time.
fn local_disable_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or_default()
}

#[cfg(test)]
mod update_revision_tests {
    use super::*;

    #[test]
    fn should_carry_one_above_the_cached_revision() {
        let id = Identifier::from([7u8; 32]);
        assert_eq!(update_revision(&id, 4, None, None).unwrap(), 5);
        assert_eq!(update_revision(&id, 4, Some(5), None).unwrap(), 5);
    }

    #[test]
    fn should_refuse_an_update_asked_for_another_revision() {
        let id = Identifier::from([7u8; 32]);
        // The cache moved past the revision the caller read.
        match update_revision(&id, 5, Some(5), None) {
            Err(PlatformWalletError::IdentityRevisionMismatch {
                expected, actual, ..
            }) => assert_eq!((expected, actual), (5, 6)),
            other => panic!("unexpected: {other:?}"),
        }
        assert!(matches!(
            update_revision(&id, 3, Some(5), None),
            Err(PlatformWalletError::IdentityRevisionMismatch { .. })
        ));
    }

    #[test]
    fn should_take_drives_revision_asked_for_only_while_the_cache_is_where_it_was_refused() {
        let id = Identifier::default();
        // Cache at 6, Drive refused and holds 4: the caller may ask for 5.
        assert_eq!(update_revision(&id, 6, Some(5), Some((6, 4))).unwrap(), 5);
        // One above the cache still goes.
        assert_eq!(update_revision(&id, 6, Some(7), Some((6, 4))).unwrap(), 7);
        // The cache moved since the refusal, or the caller asks for another.
        for (cached, expected, refused_at) in [(7, 5, (6, 4)), (6, 4, (6, 4)), (6, 5, (5, 4))] {
            assert!(
                matches!(
                    update_revision(&id, cached, Some(expected), Some(refused_at)),
                    Err(PlatformWalletError::IdentityRevisionMismatch { .. })
                ),
                "cached {cached}, asked {expected}, refused at {refused_at:?}"
            );
        }
        // An update asked for no revision carries one above the cache.
        assert_eq!(update_revision(&id, 6, None, Some((6, 4))).unwrap(), 7);
    }

    #[test]
    fn should_send_once_more_at_drives_revision_only_when_the_cache_is_ahead() {
        // Cache at 6, update at 7, Drive holds 5: sent once more at 6, or,
        // pinned, refused with 5.
        assert_eq!(
            after_refusal(Some(5), 6, 7, false, false),
            AfterRefusal::Retry(6)
        );
        assert_eq!(
            after_refusal(Some(5), 6, 7, true, false),
            AfterRefusal::Refused(5)
        );
        // The second send is refused too: no third.
        assert_eq!(
            after_refusal(Some(4), 6, 6, false, true),
            AfterRefusal::Fail
        );
        // Refused at the revision Drive reported: nothing more to try.
        assert_eq!(
            after_refusal(Some(5), 6, 6, false, true),
            AfterRefusal::Fail
        );
        assert_eq!(
            after_refusal(Some(5), 6, 6, true, false),
            AfterRefusal::Fail
        );
        // Drive at the cache's revision or above (the cache is behind; a
        // refresh heals it), or another failure.
        for refused in [Some(6), Some(9), None] {
            assert_eq!(
                after_refusal(refused, 6, 7, false, false),
                AfterRefusal::Fail
            );
            assert_eq!(
                after_refusal(refused, 6, 7, true, false),
                AfterRefusal::Fail
            );
        }
    }

    #[test]
    fn should_read_drives_revision_only_from_a_refusal_of_this_identity() {
        use dpp::consensus::state::identity::invalid_identity_revision_error::InvalidIdentityRevisionError;
        use dpp::consensus::ConsensusError;

        let id = Identifier::default();
        let refusal = |identity_id, current| -> ConsensusError {
            InvalidIdentityRevisionError::new(identity_id, current).into()
        };
        let wait_stream = dash_sdk::Error::StateTransitionBroadcastError(
            dash_sdk::error::StateTransitionBroadcastError {
                code: 40203,
                message: String::new(),
                cause: Some(refusal(id, 4)),
            },
        );
        let bare =
            dash_sdk::Error::Protocol(ProtocolError::ConsensusError(Box::new(refusal(id, 4))));
        assert_eq!(revision_refusal(&id, &wait_stream), Some(4));
        assert_eq!(revision_refusal(&id, &bare), Some(4));
        let other = dash_sdk::Error::Protocol(ProtocolError::ConsensusError(Box::new(refusal(
            Identifier::from([7; 32]),
            4,
        ))));
        assert_eq!(revision_refusal(&id, &other), None);
        assert_eq!(
            revision_refusal(&id, &dash_sdk::Error::Generic("timeout".to_string())),
            None
        );
    }

    #[test]
    fn should_refuse_an_identity_at_the_last_revision() {
        let id = Identifier::from([7u8; 32]);
        assert!(matches!(
            update_revision(&id, u64::MAX, None, None),
            Err(PlatformWalletError::InvalidIdentityData(_))
        ));
    }
}

#[cfg(test)]
mod update_identity_at_revision_tests {
    use super::super::signing_key::tests::{wallet_with_signing_keys_on, CapturingStore};
    use super::*;
    use crate::changeset::PlatformWalletPersistence;
    use crate::wallet::persister::NoPlatformPersistence;
    use crate::{BlockTime, PlatformWallet};
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

        // A cache refreshed to 5 or past it already holds the update (key 3
        // added, key 1 disabled for good) and what came after, and is left
        // as it is: one past the chain would get the next update refused.
        for cached in [5, 6] {
            let wallet = wallet_at_revision(cached).await;
            let wallet = wallet.identity();
            {
                let mut wm = wallet.wallet_manager.write().await;
                let managed = wm
                    .get_wallet_info_mut(&wallet.wallet_id)
                    .unwrap()
                    .identity_manager
                    .managed_identity_mut(&identity_id)
                    .unwrap();
                managed.identity.add_public_key(added.clone());
                let mut key_1 = managed.identity.public_keys()[&1].clone();
                key_1.set_disabled_at(42);
                managed.identity.add_public_key(key_1);
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

    async fn refused_at(wallet: &IdentityWallet) -> Option<(u64, u64)> {
        let wm = wallet.wallet_manager.read().await;
        wm.get_wallet_info(&wallet.wallet_id)
            .unwrap()
            .identity_manager
            .identity(&Identifier::default())
            .unwrap()
            .revision_refused_at
    }

    /// A cache at 6 that an older build left one past the chain at 5. The
    /// update asked for 7 is refused and Drive says 5; that is noted, the
    /// cache is not lowered, and the next update may be asked for 6, which
    /// gets past the revision check. Drive saying the cache is not ahead, or
    /// a fetch the cache takes whole (one at its revision or above), clears
    /// the note.
    #[tokio::test]
    async fn should_let_the_next_update_carry_drives_revision_without_lowering_the_cache() {
        let identity_id = Identifier::default();
        let wallet = wallet_at_revision(6).await;
        let wallet = wallet.identity();
        assert_eq!(wallet.next_update_revision(&identity_id).await.unwrap(), 7);

        // A refusal noted against another revision than the cache's is not.
        wallet.note_refused_revision(&identity_id, 5, Some(4)).await;
        assert_eq!(refused_at(wallet).await, None);

        wallet.note_refused_revision(&identity_id, 6, Some(5)).await;
        assert_eq!(cached_identity(wallet, &identity_id).await.revision(), 6);
        assert_eq!(wallet.next_update_revision(&identity_id).await.unwrap(), 6);

        let signer = RecordingSigner::default();
        let error = wallet
            .update_identity_with_external_signer_at_revision(
                &identity_id,
                vec![],
                vec![1],
                Some(6),
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

        // Drive says the cache is not ahead after all: the note goes.
        wallet.note_refused_revision(&identity_id, 6, None).await;
        assert_eq!(wallet.next_update_revision(&identity_id).await.unwrap(), 7);

        // A fetch taken at the cache's revision or above clears it.
        wallet.note_refused_revision(&identity_id, 6, Some(5)).await;
        {
            let mut wm = wallet.wallet_manager.write().await;
            let managed = wm
                .get_wallet_info_mut(&wallet.wallet_id)
                .unwrap()
                .identity_manager
                .managed_identity_mut(&identity_id)
                .unwrap();
            let fetched = managed.identity.clone();
            assert!(managed.take_fetched_identity(&fetched, BlockTime::new(1, 0, 0)));
        }
        assert_eq!(refused_at(wallet).await, None);
    }

    /// The update asked for Drive's revision lands at 6 on a cache an older
    /// build left at 6 without it: the cache is at the chain's revision now
    /// and takes the update it lacks, key 3 added and key 1 disabled, and
    /// the note of the refusal goes.
    #[tokio::test]
    async fn should_take_an_update_landed_at_drives_revision_into_a_cache_ahead_without_it() {
        let identity_id = Identifier::default();
        let store = Arc::new(CapturingStore::default());
        let wallet = wallet_at_revision_on(6, store.clone()).await;
        let wallet = wallet.identity();
        wallet.note_refused_revision(&identity_id, 6, Some(5)).await;
        store.take();
        wallet
            .apply_landed_update(&identity_id, Some(0), 6, vec![key_3()], vec![1])
            .await
            .unwrap();
        let identity = cached_identity(wallet, &identity_id).await;
        assert_eq!(identity.revision(), 6);
        assert_eq!(identity.public_keys().get(&3), Some(&key_3()));
        assert!(identity.public_keys()[&1].is_disabled());
        assert_eq!(refused_at(wallet).await, None);
        let stored = store.stored_key(identity_id, 1).expect("key 1 stored");
        assert!(stored.public_key.is_disabled());
        assert!(store.stored_key(identity_id, 3).is_some());
    }
}

#[cfg(test)]
#[path = "lagging_fetch_tests.rs"]
mod lagging_fetch_tests;
