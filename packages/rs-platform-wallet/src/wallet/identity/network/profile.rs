//! DashPay profile sync + create/update.

use std::sync::Arc;

use async_trait::async_trait;
use dpp::address_funds::AddressWitness;
use dpp::document::DocumentV0Getters;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::Purpose;
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::identity::KeyType;
use dpp::identity::SecurityLevel;
use dpp::platform_value::BinaryData;
use dpp::platform_value::Value;
use dpp::prelude::Identifier;
use dpp::ProtocolError;

use super::*;
use crate::broadcaster::TransactionBroadcaster;
use crate::error::PlatformWalletError;

// Borrowed-signer adapter — see `dpns.rs` for the pattern.
struct SignerRef<'a, S: ?Sized>(&'a S);

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
// Sync profiles
// ---------------------------------------------------------------------------

impl<B: TransactionBroadcaster + ?Sized> IdentityWallet<B> {
    /// Fetch DashPay profile documents from Platform for all managed
    /// identities and cache them on [`ManagedIdentity`].
    ///
    /// Returns the number of profiles that were successfully synced.
    pub async fn sync_profiles(&self) -> Result<u32, PlatformWalletError> {
        // 1. Collect all managed identity IDs under a short read lock.
        let identity_ids: Vec<Identifier> = {
            let wm = self.wallet_manager.read().await;
            let info = wm
                .get_wallet_info(&self.wallet_id)
                .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
            info.identity_manager
                .all_identities()
                .into_iter()
                .map(|i| i.id())
                .collect()
        };

        if identity_ids.is_empty() {
            return Ok(0);
        }

        // 2. Load the DashPay contract locally (no network round-trip needed).
        let dashpay_contract = Arc::new(
            dpp::system_data_contracts::load_system_data_contract(
                dpp::data_contracts::SystemDataContract::Dashpay,
                dpp::version::PlatformVersion::latest(),
            )
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to load DashPay contract: {e}"
                ))
            })?,
        );

        let mut profiles_synced = 0u32;

        // 3. For each identity fetch the profile document, then cache it.
        for identity_id in &identity_ids {
            match self
                .fetch_profile_document(&dashpay_contract, identity_id)
                .await
            {
                Ok(Some(profile)) => {
                    let mut wm = self.wallet_manager.write().await;
                    if let Some(info) = wm.get_wallet_info_mut(&self.wallet_id) {
                        if let Some(managed) =
                            info.identity_manager.managed_identity_mut(identity_id)
                        {
                            managed.set_dashpay_profile(Some(profile), &self.persister);
                            profiles_synced += 1;
                        }
                    }
                }
                Ok(None) => {
                    // No profile on Platform — clear local cache only when one
                    // is currently stored, to avoid needless writes.
                    let mut wm = self.wallet_manager.write().await;
                    if let Some(info) = wm.get_wallet_info_mut(&self.wallet_id) {
                        if let Some(managed) =
                            info.identity_manager.managed_identity_mut(identity_id)
                        {
                            if managed.dashpay_profile.is_some() {
                                managed.set_dashpay_profile(None, &self.persister);
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        identity = %identity_id,
                        error = %e,
                        "Failed to fetch DashPay profile"
                    );
                }
            }
        }

        Ok(profiles_synced)
    }

    /// Fetch a single `profile` document from the DashPay contract for
    /// `identity_id` and convert it into a [`DashPayProfile`].
    ///
    /// Returns `Ok(None)` when no profile document exists on Platform.
    async fn fetch_profile_document(
        &self,
        dashpay_contract: &Arc<dpp::data_contract::DataContract>,
        identity_id: &Identifier,
    ) -> Result<Option<crate::wallet::identity::DashPayProfile>, PlatformWalletError> {
        use dash_sdk::drive::query::WhereClause;
        use dash_sdk::drive::query::WhereOperator;
        use dash_sdk::platform::FetchMany;
        use dpp::document::Document;
        use dpp::platform_value::platform_value;

        // Build query: profile documents WHERE $ownerId = identity_id.
        let query = dash_sdk::platform::DocumentQuery {
            select: dash_sdk::drive::query::SelectProjection::documents(),
            data_contract: Arc::clone(dashpay_contract),
            document_type_name: "profile".to_string(),
            where_clauses: vec![WhereClause {
                field: "$ownerId".to_string(),
                operator: WhereOperator::Equal,
                value: platform_value!(identity_id),
            }],
            group_by: vec![],
            having: vec![],
            order_by_clauses: vec![],
            limit: 1,
            start: None,
            protocol_version_override: None,
        };

        let docs = Document::fetch_many(&self.sdk, query)
            .await
            .map_err(PlatformWalletError::Sdk)?;

        // Take the first result (profile is unique per $ownerId).
        let doc = match docs.into_values().next() {
            Some(Some(d)) => d,
            _ => return Ok(None),
        };

        let props = doc.properties();

        let display_name = props
            .get("displayName")
            .and_then(|v: &Value| v.as_str().map(ToString::to_string))
            .filter(|s| !s.is_empty());

        let public_message = props
            .get("publicMessage")
            .and_then(|v: &Value| v.as_str().map(ToString::to_string))
            .filter(|s| !s.is_empty());

        let avatar_url = props
            .get("avatarUrl")
            .and_then(|v: &Value| v.as_str().map(ToString::to_string))
            .filter(|s| !s.is_empty());

        let avatar_hash = props
            .get("avatarHash")
            .and_then(|v: &Value| v.as_bytes())
            .and_then(|bytes| <[u8; 32]>::try_from(bytes.as_slice()).ok());

        let avatar_fingerprint = props
            .get("avatarFingerprint")
            .and_then(|v: &Value| v.as_bytes())
            .and_then(|bytes| <[u8; 8]>::try_from(bytes.as_slice()).ok());

        Ok(Some(crate::wallet::identity::DashPayProfile {
            display_name,
            // `publicMessage` from the contract is the bio/about-me field.
            bio: public_message.clone(),
            avatar_url,
            avatar_hash,
            avatar_fingerprint,
            public_message,
        }))
    }
}

// ---------------------------------------------------------------------------
// Profile create / update — external-signer variants
// ---------------------------------------------------------------------------

impl<B: TransactionBroadcaster + ?Sized> IdentityWallet<B> {
    /// Create a DashPay profile document using an externally-supplied
    /// signer.
    ///
    /// Mirrors [`Self::create_profile`] but signing is routed through
    /// the supplied `&S: Signer<IdentityPublicKey>`. The signing key
    /// is still resolved from the identity's `public_keys` map (first
    /// AUTHENTICATION key, matching the legacy variant) — the signer
    /// is responsible for producing a signature for whatever key is
    /// picked.
    ///
    /// All other behavior — avatar hashing, document construction,
    /// local cache update via the persister — is identical to the
    /// legacy variant.
    pub async fn create_profile_with_external_signer<S>(
        &self,
        identity_id: &Identifier,
        input: crate::wallet::identity::ProfileUpdate,
        signer: &S,
    ) -> Result<crate::wallet::identity::DashPayProfile, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
    {
        use dash_sdk::platform::transition::put_document::PutDocument;
        use dpp::data_contract::accessors::v0::DataContractV0Getters;
        use dpp::document::Document;
        use dpp::document::DocumentV0;

        // 1. Load the DashPay data contract.
        let dashpay_contract = Arc::new(
            dpp::system_data_contracts::load_system_data_contract(
                dpp::data_contracts::SystemDataContract::Dashpay,
                dpp::version::PlatformVersion::latest(),
            )
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to load DashPay contract: {e}"
                ))
            })?,
        );

        // 2. Compute avatar hashes when raw bytes are provided.
        let (avatar_hash, avatar_fingerprint) = if let Some(ref bytes) = input.avatar_bytes {
            let hash = crate::wallet::identity::calculate_avatar_hash(bytes);
            let fingerprint = crate::wallet::identity::calculate_dhash_fingerprint(bytes)
                .map_err(PlatformWalletError::InvalidIdentityData)?;
            (Some(hash), Some(fingerprint))
        } else {
            (None, None)
        };

        // 3. Build the document property map.
        let mut properties = std::collections::BTreeMap::new();
        if let Some(ref name) = input.display_name {
            properties.insert("displayName".to_string(), Value::Text(name.clone()));
        }
        if let Some(ref msg) = input.public_message {
            properties.insert("publicMessage".to_string(), Value::Text(msg.clone()));
        }
        if let Some(ref url) = input.avatar_url {
            properties.insert("avatarUrl".to_string(), Value::Text(url.clone()));
        }
        if let Some(hash) = avatar_hash {
            properties.insert("avatarHash".to_string(), Value::Bytes32(hash));
        }
        if let Some(fp) = avatar_fingerprint {
            properties.insert("avatarFingerprint".to_string(), Value::Bytes(fp.to_vec()));
        }

        // 4. Look up identity + signing key. We no longer need the
        // identity_index — the signer is supplied externally.
        let signing_key = {
            let wm = self.wallet_manager.read().await;
            let info = wm
                .get_wallet_info(&self.wallet_id)
                .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
            let managed = info
                .identity_manager
                .managed_identity(identity_id)
                .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?;
            managed
                .identity
                // DashPay profile create/update writes a document state
                // transition, which DPP requires to be signed by a
                // HIGH-or-stricter authentication key. MASTER is
                // intentionally excluded — it's reserved for identity
                // self-modification (update / key rotation /
                // withdrawal) and rejected on document writes.
                .get_first_public_key_matching(
                    Purpose::AUTHENTICATION,
                    [SecurityLevel::HIGH, SecurityLevel::CRITICAL].into(),
                    [KeyType::ECDSA_SECP256K1].into(),
                    false,
                )
                .cloned()
                .ok_or_else(|| {
                    PlatformWalletError::InvalidIdentityData(
                        "No HIGH or CRITICAL authentication key found on identity \
                         (required for document state transitions)"
                            .to_string(),
                    )
                })?
        };

        let stub_document = Document::V0(DocumentV0 {
            id: Identifier::from([0u8; 32]),
            owner_id: *identity_id,
            properties,
            revision: None,
            created_at: None,
            updated_at: None,
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            creator_id: None,
        });

        let profile_document_type = dashpay_contract
            .document_type_for_name("profile")
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to get profile document type: {e}"
                ))
            })?
            .to_owned_document_type();

        let _result_doc = stub_document
            .put_to_platform_and_wait_for_response(
                &self.sdk,
                profile_document_type,
                None,
                signing_key,
                None,
                &SignerRef(signer),
                None,
            )
            .await
            .map_err(PlatformWalletError::Sdk)?;

        let profile = crate::wallet::identity::DashPayProfile {
            display_name: input.display_name,
            bio: input.public_message.clone(),
            avatar_url: input.avatar_url,
            avatar_hash,
            avatar_fingerprint,
            public_message: input.public_message,
        };

        {
            let mut wm = self.wallet_manager.write().await;
            if let Some(info) = wm.get_wallet_info_mut(&self.wallet_id) {
                if let Some(managed) = info.identity_manager.managed_identity_mut(identity_id) {
                    managed.set_dashpay_profile(Some(profile.clone()), &self.persister);
                }
            }
        }

        Ok(profile)
    }

    /// Update an existing DashPay profile document using an
    /// externally-supplied signer.
    ///
    /// Mirrors [`Self::update_profile`] but signing is routed through
    /// the supplied `&S: Signer<IdentityPublicKey>`.
    pub async fn update_profile_with_external_signer<S>(
        &self,
        identity_id: &Identifier,
        input: crate::wallet::identity::ProfileUpdate,
        signer: &S,
    ) -> Result<crate::wallet::identity::DashPayProfile, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
    {
        use dash_sdk::platform::transition::put_document::PutDocument;
        use dpp::data_contract::accessors::v0::DataContractV0Getters;
        use dpp::document::Document;
        use dpp::document::DocumentV0;
        use dpp::document::INITIAL_REVISION;

        // 1. Load the DashPay contract.
        let dashpay_contract = Arc::new(
            dpp::system_data_contracts::load_system_data_contract(
                dpp::data_contracts::SystemDataContract::Dashpay,
                dpp::version::PlatformVersion::latest(),
            )
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to load DashPay contract: {e}"
                ))
            })?,
        );

        // 2. Fetch existing profile document for ID + revision.
        let (existing_doc_id, current_revision) = {
            use dash_sdk::drive::query::WhereClause;
            use dash_sdk::drive::query::WhereOperator;
            use dash_sdk::platform::FetchMany;
            use dpp::platform_value::platform_value;

            let query = dash_sdk::platform::DocumentQuery {
                select: dash_sdk::drive::query::SelectProjection::documents(),
                data_contract: Arc::clone(&dashpay_contract),
                document_type_name: "profile".to_string(),
                where_clauses: vec![WhereClause {
                    field: "$ownerId".to_string(),
                    operator: WhereOperator::Equal,
                    value: platform_value!(identity_id),
                }],
                group_by: vec![],
                having: vec![],
                order_by_clauses: vec![],
                limit: 1,
                start: None,
                protocol_version_override: None,
            };

            let docs = Document::fetch_many(&self.sdk, query)
                .await
                .map_err(PlatformWalletError::Sdk)?;

            match docs.into_values().next() {
                Some(Some(doc)) => {
                    let id = doc.id();
                    let rev = doc.revision().unwrap_or(INITIAL_REVISION);
                    (id, rev)
                }
                _ => {
                    return Err(PlatformWalletError::InvalidIdentityData(
                        "No existing profile document found to update".to_string(),
                    ));
                }
            }
        };

        // 3. Compute avatar hashes when bytes are provided.
        let (avatar_hash, avatar_fingerprint) = if let Some(ref bytes) = input.avatar_bytes {
            let hash = crate::wallet::identity::calculate_avatar_hash(bytes);
            let fingerprint = crate::wallet::identity::calculate_dhash_fingerprint(bytes)
                .map_err(PlatformWalletError::InvalidIdentityData)?;
            (Some(hash), Some(fingerprint))
        } else {
            // Preserve existing avatar fields from the local cache.
            let wm = self.wallet_manager.read().await;
            let (h, f) = wm
                .get_wallet_info(&self.wallet_id)
                .and_then(|info| info.identity_manager.managed_identity(identity_id))
                .and_then(|m| m.dashpay_profile.as_ref())
                .map(|p| (p.avatar_hash, p.avatar_fingerprint))
                .unwrap_or((None, None));
            (h, f)
        };

        // 4. Build property map.
        let mut properties = std::collections::BTreeMap::new();
        if let Some(ref name) = input.display_name {
            properties.insert("displayName".to_string(), Value::Text(name.clone()));
        }
        if let Some(ref msg) = input.public_message {
            properties.insert("publicMessage".to_string(), Value::Text(msg.clone()));
        }
        if let Some(ref url) = input.avatar_url {
            properties.insert("avatarUrl".to_string(), Value::Text(url.clone()));
        }
        if let Some(hash) = avatar_hash {
            properties.insert("avatarHash".to_string(), Value::Bytes32(hash));
        }
        if let Some(fp) = avatar_fingerprint {
            properties.insert("avatarFingerprint".to_string(), Value::Bytes(fp.to_vec()));
        }

        // 5. Look up signing key.
        let signing_key = {
            let wm = self.wallet_manager.read().await;
            let info = wm
                .get_wallet_info(&self.wallet_id)
                .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
            let managed = info
                .identity_manager
                .managed_identity(identity_id)
                .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?;
            managed
                .identity
                // DashPay profile create/update writes a document state
                // transition, which DPP requires to be signed by a
                // HIGH-or-stricter authentication key. MASTER is
                // intentionally excluded — it's reserved for identity
                // self-modification (update / key rotation /
                // withdrawal) and rejected on document writes.
                .get_first_public_key_matching(
                    Purpose::AUTHENTICATION,
                    [SecurityLevel::HIGH, SecurityLevel::CRITICAL].into(),
                    [KeyType::ECDSA_SECP256K1].into(),
                    false,
                )
                .cloned()
                .ok_or_else(|| {
                    PlatformWalletError::InvalidIdentityData(
                        "No HIGH or CRITICAL authentication key found on identity \
                         (required for document state transitions)"
                            .to_string(),
                    )
                })?
        };

        let updated_document = Document::V0(DocumentV0 {
            id: existing_doc_id,
            owner_id: *identity_id,
            properties,
            revision: Some(current_revision + 1),
            created_at: None,
            updated_at: None,
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            creator_id: None,
        });

        let profile_document_type = dashpay_contract
            .document_type_for_name("profile")
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to get profile document type: {e}"
                ))
            })?
            .to_owned_document_type();

        let _result_doc = updated_document
            .put_to_platform_and_wait_for_response(
                &self.sdk,
                profile_document_type,
                None,
                signing_key,
                None,
                &SignerRef(signer),
                None,
            )
            .await
            .map_err(PlatformWalletError::Sdk)?;

        let profile = crate::wallet::identity::DashPayProfile {
            display_name: input.display_name,
            bio: input.public_message.clone(),
            avatar_url: input.avatar_url,
            avatar_hash,
            avatar_fingerprint,
            public_message: input.public_message,
        };

        {
            let mut wm = self.wallet_manager.write().await;
            if let Some(info) = wm.get_wallet_info_mut(&self.wallet_id) {
                if let Some(managed) = info.identity_manager.managed_identity_mut(identity_id) {
                    managed.set_dashpay_profile(Some(profile.clone()), &self.persister);
                }
            }
        }

        Ok(profile)
    }
}
