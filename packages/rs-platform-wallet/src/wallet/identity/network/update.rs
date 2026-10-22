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
    /// `external_key_ids` names the added keys that do not come from the
    /// wallet's derivation (a key the caller generated or received, such
    /// as a browser session key, whose private key the wallet cannot
    /// derive from its seed). They are stored without derivation
    /// coordinates, so no client takes them for keys it can sign with by
    /// deriving. Every other added key is stored as derived at the
    /// identity's index and the key's id. An id that names no added key
    /// is refused before anything is signed.
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
        external_key_ids: &[KeyID],
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

        if let Some(id) = external_key_ids
            .iter()
            .find(|id| !add_public_keys.iter().any(|key| key.id() == **id))
        {
            return Err(PlatformWalletError::InvalidIdentityData(format!(
                "external key id {id} names no key this update adds"
            )));
        }

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
            added_keys_for_local_apply,
            external_key_ids,
            disabled_ids_for_local_apply,
        )
        .await
    }

    /// Post-broadcast local apply of an update Drive accepted.
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
    /// An added key named in `external_key_ids` is stored without a
    /// breadcrumb; every other one with the breadcrumb of its slot,
    /// `(wallet_id, identity_index, key_id)`.
    async fn apply_landed_update(
        &self,
        identity_id: &Identifier,
        identity_index: Option<u32>,
        added_keys: Vec<IdentityPublicKey>,
        external_key_ids: &[KeyID],
        disabled_ids: Vec<u32>,
    ) -> Result<(), PlatformWalletError> {
        if !added_keys.is_empty() || !disabled_ids.is_empty() {
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

                for key in added_keys {
                    let breadcrumb = identity_index
                        .filter(|_| !external_key_ids.contains(&key.id()))
                        .map(|idx| (self.wallet_id, idx, key.id()));
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
                    // is a placeholder — the next Platform refresh
                    // reconciles it to the authoritative on-chain block
                    // time. `disable_keys` reuses the same derivation
                    // breadcrumb `add_key` carries, so the disabled key
                    // keeps its private-key linkage.
                    let disabled_at = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_millis() as u64)
                        .unwrap_or_default();
                    managed.disable_keys(&disabled_ids, disabled_at, &self.persister);
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

#[cfg(test)]
mod external_key_tests {
    use super::*;
    use crate::changeset::{
        ClientStartState, IdentityKeyEntry, PersistenceError, PlatformWalletChangeSet,
        PlatformWalletPersistence,
    };
    use crate::pubkey_binds_expected_key_data;
    use crate::wallet::identity::network::{
        derive_connect_keypair_from_master, derive_ecdsa_identity_auth_keypair_from_master,
        ConnectKey,
    };
    use crate::wallet::platform_wallet::WalletId;
    use crate::PlatformWallet;
    use dpp::identity::identity_public_key::contract_bounds::ContractBounds;
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dpp::identity::identity_public_key::v1::IdentityPublicKeyV1;
    use dpp::util::hash::ripemd160_sha256;
    use dpp::version::PlatformVersion;
    use key_wallet::bip32::ExtendedPrivKey;
    use key_wallet::mnemonic::Mnemonic;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    /// The phrase `test_platform_wallet_manager_with` builds its wallet from.
    const TEST_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon \
         abandon abandon abandon abandon abandon about";
    const NETWORK: key_wallet::Network = key_wallet::Network::Testnet;

    /// Keeps every changeset the wallet stores.
    #[derive(Default)]
    struct CapturingStore(Mutex<Vec<PlatformWalletChangeSet>>);

    impl PlatformWalletPersistence for CapturingStore {
        fn store(
            &self,
            _: WalletId,
            changeset: PlatformWalletChangeSet,
        ) -> Result<(), PersistenceError> {
            self.0.lock().unwrap().push(changeset);
            Ok(())
        }
        fn flush(&self, _: WalletId) -> Result<(), PersistenceError> {
            Ok(())
        }
        fn load(&self) -> Result<ClientStartState, PersistenceError> {
            Ok(ClientStartState::default())
        }
    }

    impl CapturingStore {
        /// The last row stored for key `key_id` of `identity_id`.
        fn stored_key(&self, identity_id: Identifier, key_id: KeyID) -> Option<IdentityKeyEntry> {
            self.0
                .lock()
                .unwrap()
                .iter()
                .rev()
                .filter_map(|cs| cs.identity_keys.as_ref())
                .find_map(|keys| keys.upserts.get(&(identity_id, key_id)).cloned())
        }
    }

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

    fn master() -> ExtendedPrivKey {
        let seed = Mnemonic::from_phrase(TEST_MNEMONIC)
            .expect("test mnemonic")
            .to_seed("");
        ExtendedPrivKey::new_master(NETWORK, &seed).expect("master")
    }

    /// The test wallet's own key at DIP-9 slot `(identity_index, key_id)`.
    fn slot_key(identity_index: u32, key_id: KeyID) -> [u8; 33] {
        derive_ecdsa_identity_auth_keypair_from_master(&master(), NETWORK, identity_index, key_id)
            .expect("slot derive")
            .public_key
    }

    fn wallet_key(id: KeyID, security_level: SecurityLevel) -> IdentityPublicKey {
        IdentityPublicKeyV0 {
            id,
            purpose: Purpose::AUTHENTICATION,
            security_level,
            key_type: KeyType::ECDSA_SECP256K1,
            data: slot_key(0, id).to_vec().into(),
            ..Default::default()
        }
        .into()
    }

    /// A wallet over `store` holding, at identity index 0, an identity with
    /// the wallet's own MASTER (0) and HIGH (1) keys. The manager owns the
    /// wallet and is returned to keep it alive.
    async fn wallet_on(
        store: Arc<CapturingStore>,
    ) -> (impl Sized, Arc<PlatformWallet>, Identifier) {
        let (manager, wallet_id) =
            crate::test_support::test_platform_wallet_manager_with(store).await;
        let wallet = manager.get_wallet(&wallet_id).await.expect("test wallet");
        let mut identity = Identity::default_versioned(PlatformVersion::latest()).unwrap();
        let identity_id = Identifier::from([0x5a; 32]);
        identity.set_id(identity_id);
        identity.add_public_key(wallet_key(0, SecurityLevel::MASTER));
        identity.add_public_key(wallet_key(1, SecurityLevel::HIGH));
        identity.set_revision(1);
        wallet
            .wallet_manager()
            .write()
            .await
            .get_wallet_info_mut(&wallet_id)
            .unwrap()
            .identity_manager
            .add_identity(identity, 0, wallet_id, wallet.persister())
            .unwrap();
        (manager, wallet, identity_id)
    }

    /// A key the update adds; its bytes are not the wallet's.
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

    /// The session key a DashConnect one-QR login registers (AppConnectLogin
    /// `sessionIdentityPubkey`): derived on the connect path, not at an
    /// identity slot.
    fn dashconnect_session_key(identity_id: Identifier, key_id: KeyID) -> IdentityPublicKey {
        let derived = derive_connect_keypair_from_master(
            &master(),
            NETWORK,
            ConnectKey::SessionAuthentication {
                identity_id,
                request_id: [0x77; 32],
            },
        )
        .expect("connect derive");
        IdentityPublicKeyV1 {
            id: key_id,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            contract_bounds: Some(ContractBounds::SingleContract {
                id: Identifier::from([0xc0; 32]),
            }),
            key_type: KeyType::ECDSA_HASH160,
            read_only: false,
            data: ripemd160_sha256(&derived.public_key).to_vec().into(),
            disabled_at: None,
            total_budget: Some(100_000_000),
            expires_at: Some(u64::MAX / 2),
        }
        .into()
    }

    fn breadcrumb_of(row: &IdentityKeyEntry) -> Option<(u32, KeyID)> {
        row.derivation_indices
            .map(|indices| (indices.identity_index, indices.key_index))
    }

    /// An added key the caller declares external is stored without a
    /// derivation breadcrumb and without a wallet, so no client takes it for
    /// a key it can derive; it still reaches the cache.
    #[tokio::test]
    async fn should_store_an_external_added_key_without_a_breadcrumb() {
        let store = Arc::new(CapturingStore::default());
        let (_manager, wallet, identity_id) = wallet_on(store.clone()).await;
        wallet
            .identity()
            .apply_landed_update(&identity_id, Some(0), vec![key_3()], &[3], vec![])
            .await
            .unwrap();

        let row = store.stored_key(identity_id, 3).expect("key 3 stored");
        assert_eq!(row.public_key, key_3());
        assert_eq!(row.wallet_id, None);
        assert_eq!(breadcrumb_of(&row), None);
        let wm = wallet.wallet_manager().read().await;
        let cached = wm
            .get_wallet_info(&wallet.wallet_id())
            .unwrap()
            .identity_manager
            .identity(&identity_id)
            .unwrap()
            .identity
            .clone();
        assert_eq!(cached.public_keys().get(&3), Some(&key_3()));
        assert_eq!(cached.revision(), 2);
    }

    /// An added key the caller does not declare external keeps the
    /// breadcrumb of its slot, `(wallet_id, identity_index, key_id)`, as
    /// before.
    #[tokio::test]
    async fn should_keep_the_slot_breadcrumb_for_an_added_key_not_declared_external() {
        let store = Arc::new(CapturingStore::default());
        let (_manager, wallet, identity_id) = wallet_on(store.clone()).await;
        wallet
            .identity()
            .apply_landed_update(&identity_id, Some(0), vec![key_3()], &[], vec![])
            .await
            .unwrap();

        let row = store.stored_key(identity_id, 3).expect("key 3 stored");
        assert_eq!(row.wallet_id, Some(wallet.wallet_id()));
        assert_eq!(breadcrumb_of(&row), Some((0, 3)));
    }

    /// C1 (#5501): a DashConnect session key and a wallet-derived key added
    /// in one update. The session key is stored without a breadcrumb; before
    /// the fix it got the breadcrumb of the slot its key id occupies, where
    /// the wallet derives another key. The wallet's own key keeps its slot
    /// breadcrumb, and binds to what that slot derives.
    #[tokio::test]
    async fn should_c1_store_a_dashconnect_session_key_without_a_breadcrumb() {
        let store = Arc::new(CapturingStore::default());
        let (_manager, wallet, identity_id) = wallet_on(store.clone()).await;
        // `AppConnect.nextKeyId`: one past the highest id on the identity.
        let session = dashconnect_session_key(identity_id, 2);
        let own = wallet_key(3, SecurityLevel::HIGH);
        wallet
            .identity()
            .apply_landed_update(
                &identity_id,
                Some(0),
                vec![session.clone(), own.clone()],
                &[2],
                vec![],
            )
            .await
            .unwrap();

        let row = store
            .stored_key(identity_id, 2)
            .expect("session key stored");
        assert_eq!(row.public_key, session);
        assert_eq!(row.wallet_id, None);
        assert_eq!(breadcrumb_of(&row), None);
        // What the slot the old breadcrumb named derives is not the session key.
        assert!(!pubkey_binds_expected_key_data(
            &slot_key(0, 2),
            session.data().as_slice()
        ));

        let row = store.stored_key(identity_id, 3).expect("own key stored");
        assert_eq!(row.wallet_id, Some(wallet.wallet_id()));
        assert_eq!(breadcrumb_of(&row), Some((0, 3)));
        assert!(pubkey_binds_expected_key_data(
            &slot_key(0, 3),
            own.data().as_slice()
        ));
    }

    /// An external key id that names no key the update adds is refused
    /// before the signer is consulted or Platform reached. The control
    /// update, with the id naming its added key, gets to the signer and then
    /// fails at the mock SDK, so the zero count comes from the refusal.
    #[tokio::test]
    async fn should_refuse_an_external_id_the_update_does_not_add() {
        let store = Arc::new(CapturingStore::default());
        let (_manager, wallet, identity_id) = wallet_on(store).await;
        // Key 1 is on the identity but not added by this update.
        for external in [vec![9], vec![1], vec![3, 9]] {
            let signer = RecordingSigner::default();
            let error = wallet
                .identity()
                .update_identity_with_external_signer(
                    &identity_id,
                    vec![key_3()],
                    &external,
                    vec![],
                    &signer,
                    None,
                )
                .await
                .unwrap_err();
            match error {
                PlatformWalletError::InvalidIdentityData(message) => assert!(
                    message.contains("names no key this update adds"),
                    "{external:?}: {message}"
                ),
                other => panic!("{external:?}: unexpected {other:?}"),
            }
            assert_eq!(signer.0.load(Ordering::SeqCst), 0, "{external:?}");
        }

        let signer = RecordingSigner::default();
        let error = wallet
            .identity()
            .update_identity_with_external_signer(
                &identity_id,
                vec![key_3()],
                &[3],
                vec![],
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
        assert!(
            !matches!(error, PlatformWalletError::InvalidIdentityData(_)),
            "{error:?}"
        );
        assert!(signer.0.load(Ordering::SeqCst) > 0);
    }
}
