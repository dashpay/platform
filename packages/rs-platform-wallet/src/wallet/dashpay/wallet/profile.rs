//! DashPay profile sync + create/update.

use std::sync::Arc;

use dpp::document::DocumentV0Getters;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::identity_public_key::Purpose;
use dpp::platform_value::Value;
use dpp::prelude::Identifier;

use super::*;
use crate::broadcaster::TransactionBroadcaster;
use crate::error::PlatformWalletError;

// ---------------------------------------------------------------------------
// Sync profiles
// ---------------------------------------------------------------------------

impl<B: TransactionBroadcaster + ?Sized> DashPayWallet<B> {
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
            info.identity_manager.identities().keys().copied().collect()
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
    ) -> Result<Option<crate::wallet::dashpay::DashPayProfile>, PlatformWalletError> {
        use dash_sdk::drive::query::WhereClause;
        use dash_sdk::drive::query::WhereOperator;
        use dash_sdk::platform::FetchMany;
        use dpp::document::Document;
        use dpp::platform_value::platform_value;

        // Build query: profile documents WHERE $ownerId = identity_id.
        let query = dash_sdk::platform::DocumentQuery {
            data_contract: Arc::clone(dashpay_contract),
            document_type_name: "profile".to_string(),
            where_clauses: vec![WhereClause {
                field: "$ownerId".to_string(),
                operator: WhereOperator::Equal,
                value: platform_value!(identity_id),
            }],
            order_by_clauses: vec![],
            limit: 1,
            start: None,
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

        Ok(Some(crate::wallet::dashpay::DashPayProfile {
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
// Profile create / update
// ---------------------------------------------------------------------------

impl<B: TransactionBroadcaster + ?Sized> DashPayWallet<B> {
    /// Create a new DashPay profile document on Platform for `identity_id`.
    ///
    /// Steps:
    /// 1. Load the DashPay contract.
    /// 2. Compute `avatarHash` (SHA-256) and `avatarFingerprint` (dHash)
    ///    from `input.avatar_bytes` when present.
    /// 3. Build a `profile` document with the supplied fields.
    /// 4. Retrieve the identity and signing key from the wallet manager.
    /// 5. Broadcast the document creation via the SDK.
    /// 6. Cache the resulting [`DashPayProfile`] on [`ManagedIdentity`].
    /// 7. Return the cached profile.
    pub async fn create_profile(
        &self,
        identity_id: &Identifier,
        input: crate::wallet::dashpay::ProfileUpdate,
    ) -> Result<crate::wallet::dashpay::DashPayProfile, PlatformWalletError> {
        use dash_sdk::platform::transition::put_document::PutDocument;
        use dpp::data_contract::accessors::v0::DataContractV0Getters;
        use dpp::document::Document;
        use dpp::document::DocumentV0;
        use dpp::platform_value::Value;

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
            let hash = crate::wallet::dashpay::calculate_avatar_hash(bytes);
            let fingerprint = crate::wallet::dashpay::calculate_dhash_fingerprint(bytes)
                .map_err(|e| PlatformWalletError::InvalidIdentityData(e))?;
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

        // 4. Retrieve identity, identity_index, and signing key.
        let (_identity, identity_index, signing_key) = {
            let wm = self.wallet_manager.read().await;
            let info = wm
                .get_wallet_info(&self.wallet_id)
                .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
            let managed = info
                .identity_manager
                .managed_identity(identity_id)
                .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?;
            let idx = managed.identity_index;
            let key = managed
                .identity
                .public_keys()
                .values()
                .find(|k| k.purpose() == Purpose::AUTHENTICATION)
                .cloned()
                .ok_or_else(|| {
                    PlatformWalletError::InvalidIdentityData(
                        "Identity has no authentication key for signing".to_string(),
                    )
                })?;
            (managed.identity.clone(), idx, key)
        };

        // Build a stub document — the SDK will assign the real ID during
        // `put_to_platform_and_wait_for_response` (entropy-based generation).
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

        // 5. Broadcast via PutDocument trait (handles ID + entropy generation).
        let signer = IdentitySigner::new(
            self.wallet_manager.clone(),
            self.wallet_id,
            self.sdk.network,
            identity_index,
        );

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
                None, // entropy auto-generated
                signing_key,
                None, // no token payment
                &signer,
                None, // default settings
            )
            .await
            .map_err(PlatformWalletError::Sdk)?;

        // 6. Build and cache the profile locally.
        let profile = crate::wallet::dashpay::DashPayProfile {
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

    /// Update an existing DashPay profile on Platform for `identity_id`.
    ///
    /// Fetches the current profile document to obtain its ID and revision,
    /// applies the fields from `input`, then broadcasts a document replace
    /// transition. The local cache is updated on success.
    pub async fn update_profile(
        &self,
        identity_id: &Identifier,
        input: crate::wallet::dashpay::ProfileUpdate,
    ) -> Result<crate::wallet::dashpay::DashPayProfile, PlatformWalletError> {
        use dash_sdk::platform::transition::put_document::PutDocument;
        use dpp::data_contract::accessors::v0::DataContractV0Getters;
        use dpp::document::Document;
        use dpp::document::DocumentV0;
        use dpp::document::INITIAL_REVISION;
        use dpp::platform_value::Value;

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

        // 2. Fetch the existing profile document to get its Platform ID and
        //    current revision. We must query the raw Document rather than the
        //    parsed DashPayProfile because we need the document ID field.
        let (existing_doc_id, current_revision) = {
            use dash_sdk::drive::query::WhereClause;
            use dash_sdk::drive::query::WhereOperator;
            use dash_sdk::platform::FetchMany;
            use dpp::platform_value::platform_value;

            let query = dash_sdk::platform::DocumentQuery {
                data_contract: Arc::clone(&dashpay_contract),
                document_type_name: "profile".to_string(),
                where_clauses: vec![WhereClause {
                    field: "$ownerId".to_string(),
                    operator: WhereOperator::Equal,
                    value: platform_value!(identity_id),
                }],
                order_by_clauses: vec![],
                limit: 1,
                start: None,
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

        // 3. Compute avatar hashes when raw bytes are provided.
        let (avatar_hash, avatar_fingerprint) = if let Some(ref bytes) = input.avatar_bytes {
            let hash = crate::wallet::dashpay::calculate_avatar_hash(bytes);
            let fingerprint = crate::wallet::dashpay::calculate_dhash_fingerprint(bytes)
                .map_err(|e| PlatformWalletError::InvalidIdentityData(e))?;
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

        // 4. Build the updated property map.
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

        // 5. Retrieve identity_index and signing key.
        let (identity_index, signing_key) = {
            let wm = self.wallet_manager.read().await;
            let info = wm
                .get_wallet_info(&self.wallet_id)
                .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
            let managed = info
                .identity_manager
                .managed_identity(identity_id)
                .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?;
            let key = managed
                .identity
                .public_keys()
                .values()
                .find(|k| k.purpose() == Purpose::AUTHENTICATION)
                .cloned()
                .ok_or_else(|| {
                    PlatformWalletError::InvalidIdentityData(
                        "Identity has no authentication key for signing".to_string(),
                    )
                })?;
            (managed.identity_index, key)
        };

        // 6. Build the document with the existing ID and bumped revision.
        let updated_document = Document::V0(DocumentV0 {
            id: existing_doc_id,
            owner_id: *identity_id,
            properties,
            // Bumping revision signals to `put_to_platform` that this is a
            // replace transition (revision > INITIAL_REVISION).
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

        // 7. Broadcast the replace transition.
        let signer = IdentitySigner::new(
            self.wallet_manager.clone(),
            self.wallet_id,
            self.sdk.network,
            identity_index,
        );

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
                None, // entropy not used for replace
                signing_key,
                None, // no token payment
                &signer,
                None, // default settings
            )
            .await
            .map_err(PlatformWalletError::Sdk)?;

        // 8. Build and cache the updated profile.
        let profile = crate::wallet::dashpay::DashPayProfile {
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
