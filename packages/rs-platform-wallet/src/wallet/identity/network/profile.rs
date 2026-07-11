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
use crate::wallet::identity::{ContactProfileEntry, DashPayProfile};

// ---------------------------------------------------------------------------
// Sync profiles
// ---------------------------------------------------------------------------

impl<B: TransactionBroadcaster + ?Sized> DashPayView<'_, B> {
    /// Fetch DashPay profile documents from Platform for all managed
    /// identities and cache them on [`ManagedIdentity`].
    ///
    /// Fetches in `In`-chunks (one query per ≤`CONTACT_PROFILE_IN_CAP` ids, not
    /// N+1) with per-chunk failure isolation, and persists only when the
    /// fetched profile differs from the cached one — a `None` result clears the
    /// cache only when a profile is currently stored (both arms guarded so a
    /// no-change sweep writes nothing). Returns the number of identities whose
    /// cached profile changed.
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

        // 2. The DashPay contract (process-wide cache — no
        //    per-call re-parse, no network round-trip).
        let dashpay_contract = super::dashpay_contract()?;

        // 3. Fetch (no guard held) in `In`-chunks. A chunk failure logs and
        //    continues so the other chunks still land; an id present in the
        //    chunk but absent from the result is confirmed-absent (`None`).
        let mut fetched: std::collections::BTreeMap<Identifier, Option<DashPayProfile>> =
            std::collections::BTreeMap::new();
        for chunk in identity_ids.chunks(CONTACT_PROFILE_IN_CAP) {
            match self
                .fetch_contact_profiles_chunk(&dashpay_contract, chunk)
                .await
            {
                Ok(found) => {
                    for id in chunk {
                        fetched.insert(*id, found.get(id).cloned().flatten());
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "Failed to fetch an own-profile chunk; will retry next sweep"
                    );
                }
            }
        }

        // 4. Under the write guard: persist-on-change only (mirror the
        //    None-arm's is_some guard for both directions).
        let mut changed = 0u32;
        let mut wm = self.wallet_manager.write().await;
        let Some(info) = wm.get_wallet_info_mut(&self.wallet_id) else {
            return Ok(0);
        };
        for (identity_id, profile) in fetched {
            let Some(managed) = info.identity_manager.managed_identity_mut(&identity_id) else {
                continue;
            };
            if managed.dashpay().profile == profile {
                continue;
            }
            managed.set_dashpay_profile(profile, &self.persister);
            changed += 1;
        }

        Ok(changed)
    }
}

// ---------------------------------------------------------------------------
// Profile create / update — external-signer variants
// ---------------------------------------------------------------------------

impl<B: TransactionBroadcaster + ?Sized> DashPayView<'_, B> {
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

        // 1. The DashPay data contract (process-wide cache).
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

        // 1. The DashPay contract (process-wide cache).
        let dashpay_contract = super::dashpay_contract()?;

        // 2. Fetch existing profile document for ID + revision + its
        //    current property map (seed for the read-modify-write merge).
        let (existing_doc_id, current_revision, existing_properties) = {
            use dash_sdk::platform::FetchMany;

            let query = single_profile_query(&dashpay_contract, identity_id);

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

// ---------------------------------------------------------------------------
// Contact-profile sync
// ---------------------------------------------------------------------------

/// Max length of a DashPay `avatarUrl` (DIP-15). Longer is rejected.
const MAX_AVATAR_URL_LEN: usize = 2048;
/// Platform `In`-clause cardinality cap; also the profile-fetch chunk size.
const CONTACT_PROFILE_IN_CAP: usize = 100;
/// Re-fetch / re-check window for a cached contact profile. A present profile
/// is refreshed and a confirmed-absent one re-checked at most once per window,
/// bounding sync cost without the (unprovable-as-a-batch) `$updatedAt`
/// incremental query.
const CONTACT_PROFILE_REFRESH_MS: u64 = 60 * 60_000;

/// An `avatarUrl` is cached only if it is a bounded `https://` URL. An
/// attacker-controlled `http:` / `file:` / `javascript:` / oversized URL is
/// dropped before it can reach the persistent cache and the UI's image loader
/// (SSRF / tracking-pixel vector).
fn is_valid_avatar_url(url: &str) -> bool {
    !url.is_empty() && url.len() <= MAX_AVATAR_URL_LEN && url.starts_with("https://")
}

/// Whether a contact id should be (re)fetched this sweep: never-checked ids
/// always, otherwise only past the refresh window. The window applies equally
/// to present and confirmed-absent entries — for the latter it is the negative
/// cache that stops a profile-less contact being re-queried every sweep.
fn should_fetch_profile(entry: Option<&ContactProfileEntry>, now_ms: u64) -> bool {
    match entry {
        None => true,
        Some(e) => now_ms.saturating_sub(e.checked_at_ms) >= CONTACT_PROFILE_REFRESH_MS,
    }
}

/// Apply a freshly-fetched profile (`Some`) or confirmed-absent result
/// (`None`) to the cache with **full-replace** semantics (NOT a field merge —
/// a contact who removed a field must lose it), returning whether the stored
/// profile changed so the caller persists only on change. `checked_at_ms` is
/// always refreshed; a pure timestamp bump is not a change.
fn apply_fetched_profile(
    cache: &mut std::collections::BTreeMap<Identifier, ContactProfileEntry>,
    contact_id: Identifier,
    fetched: Option<DashPayProfile>,
    now_ms: u64,
) -> bool {
    let changed = cache.get(&contact_id).map(|e| &e.profile) != Some(&fetched);
    cache.insert(
        contact_id,
        ContactProfileEntry {
            profile: fetched,
            checked_at_ms: now_ms,
        },
    );
    changed
}

impl<B: TransactionBroadcaster + ?Sized> DashPayView<'_, B> {
    /// Fetch and cache **contact** profiles — established contacts + pending
    /// incoming-request senders — so the UI can show their name/avatar.
    ///
    /// Mirrors Android's
    /// `updateContactProfiles`: iterate the full contact set every sweep
    /// (so a contact established before this shipped is backfilled, and a
    /// dropped fetch self-heals next sweep), skip recently-checked ids,
    /// fetch in `In`-chunks with per-chunk failure isolation, and write the
    /// per-owner cache with full-replace + persist-on-change. Contacts that
    /// are themselves managed identities are skipped (their own
    /// `dashpay_profile` is authoritative). Display-only: a failure never
    /// aborts the sweep. Returns the number of cache entries changed.
    pub async fn sync_contact_profiles(&self) -> Result<u32, PlatformWalletError> {
        let now_ms = crate::util::now_ms();
        let dashpay_contract = super::dashpay_contract()?;

        // 1. Under a read guard: per owner, the contact ids worth fetching
        //    this sweep (established ∪ pending senders, minus own identities,
        //    minus recently-checked).
        let plan: Vec<(Identifier, Vec<Identifier>)> = {
            let wm = self.wallet_manager.read().await;
            let Some(info) = wm.get_wallet_info(&self.wallet_id) else {
                return Ok(0);
            };
            let own: std::collections::BTreeSet<Identifier> = info
                .identity_manager
                .all_identities()
                .into_iter()
                .map(|i| i.id())
                .collect();

            own.iter()
                .filter_map(|owner_id| {
                    let managed = info.identity_manager.managed_identity(owner_id)?;
                    let mut targets: std::collections::BTreeSet<Identifier> = managed
                        .dashpay()
                        .established_contacts()
                        .keys()
                        .copied()
                        .collect();
                    targets.extend(
                        managed
                            .dashpay()
                            .incoming_contact_requests()
                            .keys()
                            .copied(),
                    );
                    let to_fetch: Vec<Identifier> = targets
                        .into_iter()
                        .filter(|id| !own.contains(id))
                        .filter(|id| {
                            should_fetch_profile(managed.dashpay().contact_profiles.get(id), now_ms)
                        })
                        .collect();
                    (!to_fetch.is_empty()).then_some((*owner_id, to_fetch))
                })
                .collect()
        };

        if plan.is_empty() {
            return Ok(0);
        }

        // 2. Fetch (no guard held). Per chunk: one `In` query over ≤IN_CAP
        //    owner ids; a chunk failure logs and continues so the others
        //    still land. An id present in the chunk but absent from the
        //    result is confirmed-absent (cached as `None` — the negative
        //    cache).
        // One owner's fetched contacts: each contact id paired with its profile, or
        // `None` when confirmed-absent (the negative cache).
        type OwnerContactProfiles = Vec<(Identifier, Option<DashPayProfile>)>;
        let mut results: Vec<(Identifier, OwnerContactProfiles)> = Vec::new();
        for (owner_id, to_fetch) in plan {
            let mut owner_results: OwnerContactProfiles = Vec::new();
            for chunk in to_fetch.chunks(CONTACT_PROFILE_IN_CAP) {
                match self
                    .fetch_contact_profiles_chunk(&dashpay_contract, chunk)
                    .await
                {
                    Ok(found) => {
                        for id in chunk {
                            owner_results.push((*id, found.get(id).cloned().flatten()));
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            owner = %owner_id,
                            error = %e,
                            "Failed to fetch a contact-profile chunk; will retry next sweep"
                        );
                    }
                }
            }
            if !owner_results.is_empty() {
                results.push((owner_id, owner_results));
            }
        }

        // 3. Under the write guard: full-replace, persist-on-change.
        let mut written = 0u32;
        {
            let mut wm = self.wallet_manager.write().await;
            let Some(info) = wm.get_wallet_info_mut(&self.wallet_id) else {
                return Ok(0);
            };
            for (owner_id, owner_results) in results {
                let Some(managed) = info.identity_manager.managed_identity_mut(&owner_id) else {
                    continue;
                };
                for (contact_id, profile) in owner_results {
                    if apply_fetched_profile(
                        managed.dashpay_contact_profiles_mut(),
                        contact_id,
                        profile,
                        now_ms,
                    ) {
                        written += 1;
                    }
                }
                // Persist one changeset per owner. Every owner reaching here had
                // ≥1 profile (re)fetched this sweep, so `checked_at_ms` advanced
                // for at least one contact — persist unconditionally, not only on
                // content change, so the refresh-cache timestamps are durable. A
                // cold start otherwise reverts each timestamp to the last
                // content-changing sweep and re-fetches every still-fresh profile.
                // No meaningful write amplification: the store is paired with the
                // network fetch that just ran, and fetches are gated to once per
                // `CONTACT_PROFILE_REFRESH_MS` per contact. A failed store
                // self-heals on the next sweep.
                if let Err(e) = self.persister.store(managed.snapshot_changeset().into()) {
                    tracing::warn!(
                        owner = %owner_id,
                        error = %e,
                        "Failed to persist contact profiles; will retry next sweep"
                    );
                }
            }
        }

        Ok(written)
    }

    /// Run one `$ownerId In [chunk]` profile query, returning the present
    /// profiles keyed by owner id (absent ids are simply missing). The
    /// `profile` `ownerId` index is unique, so the set lookup needs no
    /// pagination (≤1 profile per owner). The query is built by
    /// [`contact_profiles_chunk_query`].
    async fn fetch_contact_profiles_chunk(
        &self,
        dashpay_contract: &Arc<dpp::data_contract::DataContract>,
        chunk: &[Identifier],
    ) -> Result<std::collections::BTreeMap<Identifier, Option<DashPayProfile>>, PlatformWalletError>
    {
        use dash_sdk::platform::FetchMany;
        use dpp::document::Document;

        if chunk.is_empty() {
            return Ok(Default::default());
        }
        let query = contact_profiles_chunk_query(dashpay_contract, chunk);

        let docs = Document::fetch_many(&self.sdk, query)
            .await
            .map_err(PlatformWalletError::Sdk)?;

        let mut out = std::collections::BTreeMap::new();
        for (_doc_id, maybe_doc) in docs {
            let Some(doc) = maybe_doc else { continue };
            let owner = doc.owner_id();
            let mut profile = profile_from_properties(doc.properties());
            // Drop an untrusted avatar URL rather than caching it.
            if profile
                .avatar_url
                .as_deref()
                .is_some_and(|u| !is_valid_avatar_url(u))
            {
                profile.avatar_url = None;
            }
            // A doc that parses to no populated field is treated as
            // confirmed-absent (negative cache), not a cached-present empty
            // profile — so self-heal keeps it honest.
            let entry = (profile != DashPayProfile::default()).then_some(profile);
            out.insert(owner, entry);
        }
        Ok(out)
    }
}

/// Build the single-owner `profile` fetch query WHERE `$ownerId = identity_id`
/// (`limit 1` — profile is unique per owner). Used by the external-signer
/// update's read-modify-write seed to fetch the current document's
/// id/revision/properties.
fn single_profile_query(
    dashpay_contract: &Arc<dpp::data_contract::DataContract>,
    identity_id: &Identifier,
) -> dash_sdk::platform::DocumentQuery {
    use dash_sdk::drive::query::{WhereClause, WhereOperator};
    use dpp::platform_value::platform_value;

    dash_sdk::platform::DocumentQuery {
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
    }
}

/// Build the `$ownerId In [chunk]` profile fetch query for one chunk.
///
/// `In` is a range operator, so DAPI requires a matching `orderBy` on the
/// range field or it rejects the query with "missing order by for range".
/// The `$ownerId` index is unique, so ordering does not change the result
/// set (≤1 profile per owner) — the `orderBy` is only there to satisfy that
/// range-orderBy rule.
fn contact_profiles_chunk_query(
    dashpay_contract: &Arc<dpp::data_contract::DataContract>,
    chunk: &[Identifier],
) -> dash_sdk::platform::DocumentQuery {
    use dash_sdk::drive::query::{OrderClause, WhereClause, WhereOperator};
    use dpp::platform_value::{platform_value, Value};

    let in_values = Value::Array(chunk.iter().map(|id| platform_value!(id)).collect());
    dash_sdk::platform::DocumentQuery {
        select: dash_sdk::drive::query::SelectProjection::documents(),
        data_contract: Arc::clone(dashpay_contract),
        document_type_name: "profile".to_string(),
        where_clauses: vec![WhereClause {
            field: "$ownerId".to_string(),
            operator: WhereOperator::In,
            value: in_values,
        }],
        group_by: vec![],
        having: vec![],
        order_by_clauses: vec![OrderClause {
            field: "$ownerId".to_string(),
            ascending: true,
        }],
        limit: CONTACT_PROFILE_IN_CAP as u32,
        start: None,
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
        m.insert(
            "publicMessage".to_string(),
            Value::Text("hello world".into()),
        );
        m.insert(
            "avatarUrl".to_string(),
            Value::Text("https://x/a.png".into()),
        );
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
            !buggy.contains_key("publicMessage"),
            "regression guard: a fresh/empty seed wipes sibling fields"
        );
        assert!(!buggy.contains_key("avatarUrl"));
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

    // --- contact-profile sync helpers ---

    /// Only bounded `https://` avatar URLs are cached — `http:`, scheme
    /// tricks, oversized, and empty are rejected (SSRF / tracking-pixel).
    #[test]
    fn avatar_url_validation_allows_only_bounded_https() {
        assert!(is_valid_avatar_url("https://example.com/a.png"));
        assert!(!is_valid_avatar_url("http://example.com/a.png"));
        assert!(!is_valid_avatar_url("javascript:alert(1)"));
        assert!(!is_valid_avatar_url("file:///etc/passwd"));
        assert!(!is_valid_avatar_url(""));
        let too_long = format!("https://x/{}", "a".repeat(MAX_AVATAR_URL_LEN));
        assert!(!is_valid_avatar_url(&too_long));
    }

    /// A never-checked id is fetched; a recently-checked one is skipped; a
    /// stale one (past the window) is re-fetched. Holds for both a present
    /// and a confirmed-absent (negative-cache) entry.
    #[test]
    fn should_fetch_respects_refresh_window_for_present_and_absent() {
        let now = 10 * CONTACT_PROFILE_REFRESH_MS;
        assert!(should_fetch_profile(None, now), "never-checked => fetch");

        for profile in [Some(DashPayProfile::default()), None] {
            let recent = ContactProfileEntry {
                profile: profile.clone(),
                checked_at_ms: now - 1, // just checked
            };
            assert!(
                !should_fetch_profile(Some(&recent), now),
                "recently-checked => skip (negative cache for absent)"
            );
            let stale = ContactProfileEntry {
                profile,
                checked_at_ms: now - CONTACT_PROFILE_REFRESH_MS,
            };
            assert!(
                should_fetch_profile(Some(&stale), now),
                "past the window => re-fetch / re-check"
            );
        }
    }

    /// Full-replace + persist-on-change: a new id changes; the same profile
    /// again does not (only the timestamp bumps); a different profile and a
    /// present→absent transition both change. Removed fields disappear.
    #[test]
    fn apply_fetched_profile_full_replace_and_change_detection() {
        let mut cache: BTreeMap<Identifier, ContactProfileEntry> = BTreeMap::new();
        let id = Identifier::from([0xC1; 32]);
        let with_avatar = DashPayProfile {
            display_name: Some("Bob".into()),
            avatar_url: Some("https://x/b.png".into()),
            ..Default::default()
        };

        // First write changes; checked_at recorded.
        assert!(apply_fetched_profile(
            &mut cache,
            id,
            Some(with_avatar.clone()),
            100
        ));
        assert_eq!(cache[&id].checked_at_ms, 100);

        // Identical profile again: no change, but the timestamp advances.
        assert!(!apply_fetched_profile(
            &mut cache,
            id,
            Some(with_avatar),
            200
        ));
        assert_eq!(cache[&id].checked_at_ms, 200);

        // Contact removed their avatar: full-replace drops it (a merge would
        // have kept it) — this is a change.
        let no_avatar = DashPayProfile {
            display_name: Some("Bob".into()),
            avatar_url: None,
            ..Default::default()
        };
        assert!(apply_fetched_profile(&mut cache, id, Some(no_avatar), 300));
        assert_eq!(cache[&id].profile.as_ref().unwrap().avatar_url, None);

        // Present -> confirmed-absent is a change and caches the negative.
        assert!(apply_fetched_profile(&mut cache, id, None, 400));
        assert!(cache[&id].profile.is_none());
    }

    /// DAPI rejects any query that uses a range where-operator without a
    /// matching `orderBy` on the range field ("missing order by for range").
    /// The profile chunk query filters by `$ownerId In [...]` — a range op —
    /// so every range clause it builds must carry an `orderBy` on its field.
    /// Guarded against vacuous truth: the query must actually contain a range
    /// clause, otherwise the invariant would pass trivially.
    #[test]
    fn contact_profiles_chunk_query_orders_by_every_range_field() {
        let contract = crate::wallet::identity::network::dashpay_contract()
            .expect("DashPay system contract loads");
        let chunk = vec![Identifier::new([1u8; 32]), Identifier::new([2u8; 32])];

        let query = contact_profiles_chunk_query(&contract, &chunk);

        // Non-vacuous: there is at least one range where-clause to satisfy.
        assert!(
            query.where_clauses.iter().any(|wc| wc.operator.is_range()),
            "expected a range where-clause (e.g. $ownerId In [...])"
        );

        for wc in &query.where_clauses {
            if wc.operator.is_range() {
                assert!(
                    query.order_by_clauses.iter().any(|oc| oc.field == wc.field),
                    "range clause on `{}` has no matching orderBy; DAPI rejects \
                     this with \"missing order by for range\"",
                    wc.field
                );
            }
        }
    }
}
