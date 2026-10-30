//! Mutate an identity's public-key set.

use async_trait::async_trait;
use dpp::address_funds::AddressWitness;
use dpp::identity::accessors::IdentityGettersV0;

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

use dpp::identity::signer::Signer;

use dash_sdk::platform::transition::put_settings::PutSettings;

use crate::error::PlatformWalletError;
use crate::wallet::identity::state::{check_keys_can_be_disabled, KeyDisableRefusal};

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
    /// Whether this wallet lets an identity update that only disables
    /// `disable_public_keys` of the managed identity `identity_id` through:
    /// the inner `Err` is the [`KeyDisableRefusal`] — the last valid
    /// authentication key of a security level (DIP-11), or the last enabled
    /// TRANSFER key able to sign.
    ///
    /// Judges the identity this wallet holds, which is the one
    /// [`Self::update_identity_with_external_signer`] builds the update from,
    /// with the same [`check_keys_can_be_disabled`] the update entry points
    /// refuse with. No added key counts here, so an update can go through
    /// where this refuses when it also adds a replacement key. Like the entry
    /// points it reads no clock: an authentication key with any expiry does
    /// not count as remaining.
    ///
    /// The outer `Err` is a wallet failure: the wallet or the identity is not
    /// loaded.
    pub async fn check_keys_can_be_disabled(
        &self,
        identity_id: &Identifier,
        disable_public_keys: &[KeyID],
    ) -> Result<Result<(), KeyDisableRefusal>, PlatformWalletError> {
        let wm = self.wallet_manager.read().await;
        let info = wm.get_wallet_info(&self.wallet_id).ok_or_else(|| {
            PlatformWalletError::WalletNotFound(
                "Wallet info not found in wallet manager".to_string(),
            )
        })?;
        let managed = info
            .identity_manager
            .identity(identity_id)
            .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?;
        Ok(check_keys_can_be_disabled(
            &managed.identity,
            &[],
            disable_public_keys,
        ))
    }

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
    /// An update [`check_keys_can_be_disabled`] refuses is refused before a
    /// nonce is fetched or anything is signed: disabling the last valid
    /// authentication key of a security level with
    /// `PlatformWalletError::Sdk(dash_sdk::Error::Protocol(ProtocolError::LastAuthenticationKey(_)))`
    /// (DIP-11), the last enabled TRANSFER key able to sign with
    /// `PlatformWalletError::LastEnabledKey(_)`. Keys added in the same update
    /// count as remaining.
    ///
    /// NOTE: callers that ADD keys via `add_public_keys` are
    /// responsible for pre-persisting the new keys' private material
    /// to whatever store the supplied signer reads from (iOS Keychain
    /// in the typical case). The signer here only signs the update
    /// transition itself; it does not derive the new keys.
    ///
    /// CACHE INVARIANT: this function does NOT refresh the in-process
    /// `IdentityManager` after a successful broadcast. The local
    /// cached `Identity` keeps the pre-update revision and key set
    /// until the caller invokes [`Self::refresh_identity`] (or the
    /// next sync round). A subsequent call to this function for the
    /// same identity without an intervening refresh will reuse the
    /// stale revision and Platform will reject the duplicate. It is
    /// documented here rather than fixed because the refresh requires
    /// a wallet-manager write lock that may already be held higher in
    /// the call stack.
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
                crate::error::PlatformWalletError::WalletNotFound(
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

        // Refuse a disable this wallet does not allow before fetching a nonce
        // or signing anything.
        check_keys_can_be_disabled(&identity, &add_public_keys, &disable_public_keys)?;

        // Increment revision for the update transition.
        let original_revision = identity.revision();
        identity.set_revision(original_revision + 1);

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

        state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(&self.sdk, settings)
            .await
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to broadcast identity update: {}",
                    e
                ))
            })?;

        // Post-broadcast local apply.
        //
        // Drive accepted the transition, so the on-chain identity
        // now carries the new keys / disabled flags. If we leave
        // the local `ManagedIdentity` cache untouched here:
        //   - The next state transition for this identity reuses
        //     the stale revision and Platform rejects it as a
        //     duplicate.
        //   - The Swift `PersistentPublicKey` rows never receive
        //     the new keys (rows arrive via the
        //     `IdentityKeysChangeSet` persister callback that
        //     `add_key` fires).
        //   - The Identity Keys list in the iOS app keeps showing
        //     only the pre-update key set.
        //
        // `ManagedIdentity::add_key` handles both the in-memory
        // mutation AND the `IdentityKeysChangeSet` upsert. Run it
        // on each new key under a write lock; the breadcrumb lets
        // the iOS Keychain re-derive the matching private bytes
        // from the wallet seed + DIP-9 path.
        if !added_keys_for_local_apply.is_empty() || !disabled_ids_for_local_apply.is_empty() {
            let mut wm = self.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&self.wallet_id).ok_or_else(|| {
                crate::error::PlatformWalletError::WalletNotFound(
                    "Wallet info not found in wallet manager".to_string(),
                )
            })?;
            if let Some(managed) = info.identity_manager.managed_identity_mut(identity_id) {
                // Bump the cached revision so a subsequent update
                // doesn't reuse the pre-broadcast value.
                let cached_revision = managed.identity.revision();
                managed.identity.set_revision(cached_revision + 1);

                for key in added_keys_for_local_apply {
                    let breadcrumb = identity_index.map(|idx| (self.wallet_id, idx, key.id()));
                    managed
                        .add_key(key, breadcrumb, &self.persister)
                        .map_err(|e| {
                            PlatformWalletError::Persistence(format!(
                                "identity key not persisted after update: {e}"
                            ))
                        })?;
                }

                if !disabled_ids_for_local_apply.is_empty() {
                    // Disable-side counterpart to the `add_key` loop
                    // above: stamp `disabled_at` on the matching cached
                    // keys and fire the persister so the Swift
                    // `PersistentPublicKey.disabledAt` rows flip without
                    // a network re-fetch. The local wall-clock timestamp
                    // is a placeholder — the next Platform refresh
                    // reconciles it to the authoritative on-chain block
                    // time. `disable_keys` reuses the same derivation
                    // breadcrumb `add_key` carries, so the disabled key
                    // keeps its private-key linkage.
                    let disabled_at = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_millis() as u64)
                        .unwrap_or_default();
                    managed.disable_keys(
                        &disabled_ids_for_local_apply,
                        disabled_at,
                        &self.persister,
                    );
                }
            }
        }

        Ok(())
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
    /// An update [`check_keys_can_be_disabled`] refuses is refused before a
    /// nonce is fetched or anything is signed, with the same errors as
    /// [`Self::update_identity_with_external_signer`].
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
    ) -> Result<dpp::state_transition::proof_result::StateTransitionProofResult, PlatformWalletError>
    {
        use dash_sdk::platform::transition::broadcast::BroadcastStateTransition;
        use dpp::state_transition::identity_update_transition::methods::IdentityUpdateTransitionMethodsV0;
        use dpp::state_transition::identity_update_transition::IdentityUpdateTransition;

        check_keys_can_be_disabled(identity, &add_public_keys, &disable_public_keys)?;

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
        .map_err(|e| PlatformWalletError::Sdk(dash_sdk::Error::Protocol(e)))?;

        // Broadcast and wait for confirmation.
        let result = state_transition
            .broadcast_and_wait(&self.sdk, settings)
            .await?;

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{test_platform_wallet_manager_with_sdk, NoopTestPersister};
    use crate::wallet::identity::LastEnabledKeyError;
    use crate::wallet::persister::{NoPlatformPersistence, WalletPersister};
    use crate::wallet::platform_wallet::WalletId;
    use dash_sdk::query_types::IdentityNonceFetcher;
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dpp::identity::key_disable_rule::LastAuthenticationKeyError;
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

    async fn manager_holding(
        identity: Identity,
    ) -> (
        Arc<crate::PlatformWalletManager<NoopTestPersister>>,
        Arc<crate::wallet::PlatformWallet>,
    ) {
        let (manager, wallet_id) = test_platform_wallet_manager_with_sdk(
            dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"),
        )
        .await;
        let wallet = manager.get_wallet(&wallet_id).await.expect("wallet");
        {
            let persister = WalletPersister::new(wallet_id, Arc::new(NoPlatformPersistence));
            let mut wm = manager.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("wallet info");
            info.identity_manager
                .add_identity(identity, 0, wallet_id, &persister)
                .expect("add identity");
        }
        (manager, wallet)
    }

    #[tokio::test]
    async fn should_judge_a_disable_against_the_managed_identity() {
        let (_manager, wallet) = manager_holding(identity()).await;
        let identity_id = Identifier::from([1; 32]);

        let refused = wallet
            .identity()
            .check_keys_can_be_disabled(&identity_id, &[1])
            .await
            .expect("the identity is loaded");
        assert_eq!(
            refused,
            Err(KeyDisableRefusal::LastAuthenticationKey(
                LastAuthenticationKeyError {
                    key_id: 1,
                    security_level: SecurityLevel::CRITICAL,
                }
            ))
        );
        // CRITICAL 1 stands in for HIGH 2.
        assert_eq!(
            wallet
                .identity()
                .check_keys_can_be_disabled(&identity_id, &[2])
                .await
                .expect("the identity is loaded"),
            Ok(())
        );
    }

    #[tokio::test]
    async fn should_not_count_an_expiring_key_like_the_builder() {
        // The builder reads no clock and counts no key with an expiry, and
        // neither does the verdict: for a disable-only update the two agree.
        let mut identity = identity();
        identity.add_public_key(
            IdentityPublicKey::V0(IdentityPublicKeyV0 {
                id: 3,
                purpose: Purpose::AUTHENTICATION,
                security_level: SecurityLevel::CRITICAL,
                contract_bounds: None,
                key_type: KeyType::ECDSA_SECP256K1,
                read_only: false,
                data: BinaryData::new(vec![5; 33]),
                disabled_at: None,
            })
            .with_limits(None, Some(u64::MAX)),
        );
        let (_manager, wallet) = manager_holding(identity.clone()).await;
        let identity_id = Identifier::from([1; 32]);
        assert_eq!(
            wallet
                .identity()
                .check_keys_can_be_disabled(&identity_id, &[1])
                .await
                .expect("the identity is loaded"),
            Err(KeyDisableRefusal::LastAuthenticationKey(
                LastAuthenticationKeyError {
                    key_id: 1,
                    security_level: SecurityLevel::CRITICAL,
                }
            ))
        );
        // The update path refuses the same disable.
        let result = wallet
            .identity()
            .update_identity_with_signer(&identity, &0, vec![], vec![1], &HoldsEveryKey, None)
            .await;
        assert!(
            matches!(
                result,
                Err(PlatformWalletError::Sdk(dash_sdk::Error::Protocol(
                    ProtocolError::LastAuthenticationKey(_)
                )))
            ),
            "{result:?}"
        );
    }

    #[tokio::test]
    async fn should_report_an_identity_the_wallet_does_not_hold() {
        let (_manager, wallet) = manager_holding(identity()).await;
        let unknown = Identifier::from([9; 32]);
        let result = wallet
            .identity()
            .check_keys_can_be_disabled(&unknown, &[1])
            .await;
        assert!(
            matches!(result, Err(PlatformWalletError::IdentityNotFound(id)) if id == unknown),
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
                Err(PlatformWalletError::Sdk(dash_sdk::Error::Protocol(
                    ProtocolError::LastAuthenticationKey(ref refusal)
                )))
                    if refusal.key_id == 1 && refusal.security_level == SecurityLevel::CRITICAL
            ),
            "{result:?}"
        );
    }

    fn transfer_key(id: u32) -> IdentityPublicKey {
        IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id,
            purpose: Purpose::TRANSFER,
            security_level: SecurityLevel::CRITICAL,
            contract_bounds: None,
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: BinaryData::new(vec![id as u8 + 2; 33]),
            disabled_at: None,
        })
    }

    #[tokio::test]
    async fn should_judge_the_last_transfer_key_as_wallet_policy() {
        let mut identity = identity();
        identity.add_public_key(transfer_key(3));
        let (_manager, wallet) = manager_holding(identity).await;
        assert_eq!(
            wallet
                .identity()
                .check_keys_can_be_disabled(&Identifier::from([1; 32]), &[3])
                .await
                .expect("the identity is loaded"),
            Err(KeyDisableRefusal::LastEnabledKey(LastEnabledKeyError {
                key_id: 3,
                purpose: Purpose::TRANSFER,
            }))
        );
    }

    /// Both update entry points refuse the last transfer key before any
    /// network call: the mock SDK has no expectations, so a nonce fetch would
    /// fail with a different error.
    #[tokio::test]
    async fn should_refuse_disabling_the_last_transfer_key_before_fetching_a_nonce() {
        let mut identity = identity();
        identity.add_public_key(transfer_key(3));
        let (_manager, wallet) = manager_holding(identity.clone()).await;

        let result = wallet
            .identity()
            .update_identity_with_external_signer(
                &Identifier::from([1; 32]),
                vec![],
                vec![3],
                &HoldsEveryKey,
                None,
            )
            .await;
        assert!(
            matches!(
                result,
                Err(PlatformWalletError::LastEnabledKey(ref refusal))
                    if refusal.key_id == 3 && refusal.purpose == Purpose::TRANSFER
            ),
            "{result:?}"
        );

        let result = wallet
            .identity()
            .update_identity_with_signer(&identity, &0, vec![], vec![3], &HoldsEveryKey, None)
            .await;
        assert!(
            matches!(result, Err(PlatformWalletError::LastEnabledKey(_))),
            "{result:?}"
        );
    }
}
