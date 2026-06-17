//! DashPay profile sync + create/update.

use std::sync::Arc;

use dpp::document::DocumentV0Getters;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::Purpose;
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::identity::KeyType;
use dpp::identity::SecurityLevel;
use dpp::platform_value::Value;
use dpp::prelude::Identifier;

use super::*;
use crate::broadcaster::TransactionBroadcaster;
use crate::error::PlatformWalletError;

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

        // 2. The DashPay contract (G9: process-wide cache — no
        //    per-call re-parse, no network round-trip).
        let dashpay_contract = super::dashpay_contract()?;

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
        };

        let docs = Document::fetch_many(&self.sdk, query)
            .await
            .map_err(PlatformWalletError::Sdk)?;

        // Take the first result (profile is unique per $ownerId).
        let doc = match docs.into_values().next() {
            Some(Some(d)) => d,
            _ => return Ok(None),
        };

        Ok(Some(profile_from_properties(doc.properties())))
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
        use dpp::data_contract::accessors::v0::DataContractV0Getters;
        use dpp::document::Document;
        use dpp::document::DocumentV0;

        // 1. The DashPay data contract (G9: process-wide cache).
        let dashpay_contract = super::dashpay_contract()?;

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

        let _result_doc = self
            .sdk_writer
            .put_document(super::sdk_writer::PutDocumentParams {
                document: stub_document,
                document_type: profile_document_type,
                signing_public_key: signing_key,
                signer: signer as &(dyn Signer<IdentityPublicKey> + Send + Sync),
            })
            .await?;

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
        use dpp::data_contract::accessors::v0::DataContractV0Getters;
        use dpp::document::Document;
        use dpp::document::DocumentV0;
        use dpp::document::INITIAL_REVISION;

        // 1. The DashPay contract (G9: process-wide cache).
        let dashpay_contract = super::dashpay_contract()?;

        // 2. Fetch existing profile document for ID + revision + its
        //    current property map (seed for the read-modify-write merge).
        let (existing_doc_id, current_revision, existing_properties) = {
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
            };

            let docs = Document::fetch_many(&self.sdk, query)
                .await
                .map_err(PlatformWalletError::Sdk)?;

            match docs.into_values().next() {
                Some(Some(doc)) => {
                    let id = doc.id();
                    let rev = doc.revision().unwrap_or(INITIAL_REVISION);
                    (id, rev, doc.properties().clone())
                }
                _ => {
                    return Err(PlatformWalletError::InvalidIdentityData(
                        "No existing profile document found to update".to_string(),
                    ));
                }
            }
        };

        // 3. Compute avatar hashes only when new bytes are provided.
        //    Without new bytes the existing avatar fields are retained by
        //    the read-modify-write merge below (seeded from the on-platform
        //    document), so no local-cache fallback is needed.
        let (avatar_hash, avatar_fingerprint) = if let Some(ref bytes) = input.avatar_bytes {
            let hash = crate::wallet::identity::calculate_avatar_hash(bytes);
            let fingerprint = crate::wallet::identity::calculate_dhash_fingerprint(bytes)
                .map_err(PlatformWalletError::InvalidIdentityData)?;
            (Some(hash), Some(fingerprint))
        } else {
            (None, None)
        };

        // 4. Read-modify-write: seed from the existing document's
        //    properties so a partial update preserves sibling fields,
        //    then overlay only the caller-provided fields.
        let properties =
            merge_profile_properties(existing_properties, &input, avatar_hash, avatar_fingerprint);

        // The returned profile overwrites the local cache, so it must
        // reflect the merged on-platform state, not the partial input.
        let returned_profile = profile_from_properties(&properties);

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

        let _result_doc = self
            .sdk_writer
            .put_document(super::sdk_writer::PutDocumentParams {
                document: updated_document,
                document_type: profile_document_type,
                signing_public_key: signing_key,
                signer: signer as &(dyn Signer<IdentityPublicKey> + Send + Sync),
            })
            .await?;

        let profile = returned_profile;

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

// ---------------------------------------------------------------------------
// Property-map helpers (read-modify-write merge + parse)
// ---------------------------------------------------------------------------

/// Read-modify-write merge of a [`ProfileUpdate`] onto the property map of
/// the existing on-platform profile document.
///
/// A profile update is **partial**: only the fields the caller set change.
/// Seeding from `existing` and overlaying just the provided fields
/// preserves sibling fields (publicMessage, avatarUrl, …) that a
/// fresh-map build would silently drop — the on-platform data loss this
/// guards against. Avatar hash/fingerprint are overlaid only when the
/// caller supplied new avatar bytes (`avatar_hash`/`avatar_fingerprint`
/// are `Some`); otherwise the existing avatar fields are retained.
fn merge_profile_properties(
    mut existing: std::collections::BTreeMap<String, Value>,
    input: &crate::wallet::identity::ProfileUpdate,
    avatar_hash: Option<[u8; 32]>,
    avatar_fingerprint: Option<[u8; 8]>,
) -> std::collections::BTreeMap<String, Value> {
    if let Some(name) = &input.display_name {
        existing.insert("displayName".to_string(), Value::Text(name.clone()));
    }
    if let Some(msg) = &input.public_message {
        existing.insert("publicMessage".to_string(), Value::Text(msg.clone()));
    }
    if let Some(url) = &input.avatar_url {
        existing.insert("avatarUrl".to_string(), Value::Text(url.clone()));
    }
    if let Some(hash) = avatar_hash {
        existing.insert("avatarHash".to_string(), Value::Bytes32(hash));
    }
    if let Some(fp) = avatar_fingerprint {
        existing.insert("avatarFingerprint".to_string(), Value::Bytes(fp.to_vec()));
    }
    existing
}

/// Parse a profile document's property map into a [`DashPayProfile`].
/// Empty strings are normalized to `None`. `avatarHash`/`avatarFingerprint`
/// are read via `as_bytes_slice` so both `Bytes` and the sized `Bytes32`
/// representation round-trip.
fn profile_from_properties(
    props: &std::collections::BTreeMap<String, Value>,
) -> crate::wallet::identity::DashPayProfile {
    let text = |key: &str| {
        props
            .get(key)
            .and_then(|v: &Value| v.as_str().map(ToString::to_string))
            .filter(|s| !s.is_empty())
    };
    // `publicMessage` from the contract is the bio/about-me field.
    let public_message = text("publicMessage");
    let avatar_hash = props
        .get("avatarHash")
        .and_then(|v: &Value| v.as_bytes_slice().ok())
        .and_then(|bytes| <[u8; 32]>::try_from(bytes).ok());
    let avatar_fingerprint = props
        .get("avatarFingerprint")
        .and_then(|v: &Value| v.as_bytes_slice().ok())
        .and_then(|bytes| <[u8; 8]>::try_from(bytes).ok());

    crate::wallet::identity::DashPayProfile {
        display_name: text("displayName"),
        bio: public_message.clone(),
        avatar_url: text("avatarUrl"),
        avatar_hash,
        avatar_fingerprint,
        public_message,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wallet::identity::ProfileUpdate;
    use std::collections::BTreeMap;

    fn existing_full() -> BTreeMap<String, Value> {
        let mut m = BTreeMap::new();
        m.insert("displayName".to_string(), Value::Text("Alice".into()));
        m.insert("publicMessage".to_string(), Value::Text("hello world".into()));
        m.insert("avatarUrl".to_string(), Value::Text("https://x/a.png".into()));
        m.insert("avatarHash".to_string(), Value::Bytes32([7u8; 32]));
        m.insert(
            "avatarFingerprint".to_string(),
            Value::Bytes(vec![1, 2, 3, 4, 5, 6, 7, 8]),
        );
        m
    }

    /// A partial update (only `displayName`) must NOT wipe sibling fields.
    /// Contrasts the fixed read-modify-write (seed from existing) against
    /// the old fresh-map build (empty seed), which dropped them — so the
    /// test would fail against the pre-fix behavior.
    #[test]
    fn partial_update_preserves_sibling_fields() {
        let input = ProfileUpdate {
            display_name: Some("Alice 2".to_string()),
            ..Default::default()
        };

        // Fixed: seed from the existing on-platform properties.
        let merged = merge_profile_properties(existing_full(), &input, None, None);
        assert_eq!(
            merged.get("displayName").and_then(|v| v.as_str()),
            Some("Alice 2")
        );
        assert_eq!(
            merged.get("publicMessage").and_then(|v| v.as_str()),
            Some("hello world"),
        );
        assert_eq!(
            merged.get("avatarUrl").and_then(|v| v.as_str()),
            Some("https://x/a.png"),
        );
        assert!(merged.contains_key("avatarHash"));
        assert!(merged.contains_key("avatarFingerprint"));

        // The old behavior — building a fresh/empty map — is exactly what
        // caused the data loss: the same overlay drops every field the
        // caller didn't set.
        let buggy = merge_profile_properties(BTreeMap::new(), &input, None, None);
        assert!(
            buggy.get("publicMessage").is_none(),
            "regression guard: a fresh/empty seed wipes sibling fields"
        );
        assert!(buggy.get("avatarUrl").is_none());
    }

    /// Avatar fields are overlaid only when new bytes are supplied;
    /// otherwise the existing avatar is retained through the merge.
    #[test]
    fn avatar_overlaid_only_when_new_bytes_present() {
        let input = ProfileUpdate {
            display_name: Some("x".into()),
            ..Default::default()
        };

        // No new avatar bytes => existing avatar retained.
        let merged = merge_profile_properties(existing_full(), &input, None, None);
        let prof = profile_from_properties(&merged);
        assert_eq!(prof.avatar_hash, Some([7u8; 32]));
        assert_eq!(prof.avatar_fingerprint, Some([1, 2, 3, 4, 5, 6, 7, 8]));

        // New avatar bytes => overlaid.
        let merged2 =
            merge_profile_properties(existing_full(), &input, Some([9u8; 32]), Some([9u8; 8]));
        let prof2 = profile_from_properties(&merged2);
        assert_eq!(prof2.avatar_hash, Some([9u8; 32]));
        assert_eq!(prof2.avatar_fingerprint, Some([9u8; 8]));
    }

    /// The returned profile (which overwrites the local cache) reflects
    /// the merged state, not the partial input — so a partial update does
    /// not wipe the local mirror either.
    #[test]
    fn returned_profile_reflects_merge_not_input() {
        let input = ProfileUpdate {
            display_name: Some("Alice 2".into()),
            ..Default::default()
        };
        let merged = merge_profile_properties(existing_full(), &input, None, None);
        let prof = profile_from_properties(&merged);
        assert_eq!(prof.display_name.as_deref(), Some("Alice 2"));
        assert_eq!(prof.public_message.as_deref(), Some("hello world"));
        assert_eq!(prof.bio.as_deref(), Some("hello world"));
        assert_eq!(prof.avatar_url.as_deref(), Some("https://x/a.png"));
    }
}
