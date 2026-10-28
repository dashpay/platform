//! DashPay profile sync + create/update.

use std::sync::Arc;

use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::document::{Document, DocumentV0Getters};
use dpp::fee::Credits;
use dpp::identity::accessors::IdentityGettersV0;
#[cfg(test)]
use dpp::identity::identity_public_key::Purpose;
use dpp::identity::signer::Signer;
use dpp::identity::SecurityLevel;
use dpp::identity::{Identity, IdentityPublicKey};
use dpp::platform_value::Value;
use dpp::prelude::Identifier;

use dash_sdk::dapi_grpc::platform::v0::ResponseMetadata;
use dash_sdk::platform::dashpay::ProfileFields;

use super::*;
use crate::broadcaster::TransactionBroadcaster;
use crate::error::PlatformWalletError;
use crate::wallet::identity::{
    calculate_avatar_hash, calculate_dhash_fingerprint, ContactProfileEntry, DashPayProfile,
    ProfileUpdate,
};

// Profile documents require HIGH or CRITICAL authentication; MASTER is reserved
// for identity operations and cannot authorize an ordinary document write. A key
// bound to another contract or document type is skipped, a key without limits is
// preferred, and an expired one or one the signer cannot sign with is skipped.
fn profile_signing_key<'a>(
    identity: &'a Identity,
    dashpay_contract_id: Identifier,
    signer: &impl Signer<IdentityPublicKey>,
) -> Result<Option<&'a IdentityPublicKey>, dash_sdk::Error> {
    super::usable_authentication_key(
        identity,
        signer,
        dashpay_contract_id,
        "profile",
        &[SecurityLevel::HIGH, SecurityLevel::CRITICAL],
        &super::WALLET_SIGNING_KEY_TYPES,
    )
}

// ---------------------------------------------------------------------------
// Sync profiles
// ---------------------------------------------------------------------------

impl<B: TransactionBroadcaster + ?Sized> DashPayView<'_, B> {
    /// Fetch DashPay profile documents from Platform for all managed
    /// identities and cache them on [`ManagedIdentity`].
    ///
    /// Fetches through [`Sdk::get_profiles`](dash_sdk::Sdk::get_profiles) (`In`
    /// queries, not N+1; a failed fetch leaves the cache as is for the next
    /// sweep), and persists only when the
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

        // 3. Fetch (no guard held). A failure logs and leaves the cache as is;
        //    an id absent from the result is confirmed-absent (`None`).
        let found = match self.fetch_profiles(&dashpay_contract, &identity_ids).await {
            Ok(found) => found,
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "Failed to fetch the own profiles; will retry next sweep"
                );
                return Ok(0);
            }
        };
        let fetched: std::collections::BTreeMap<Identifier, Option<DashPayProfile>> = identity_ids
            .iter()
            .map(|id| (*id, found.get(id).cloned().flatten()))
            .collect();

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
    /// is resolved from the identity's active HIGH or CRITICAL ECDSA
    /// authentication keys (full public key or HASH160) — the signer
    /// is responsible for producing a signature for whatever key is
    /// picked. Keys `signer.can_sign_with` rejects are skipped.
    ///
    /// All other behavior — avatar hashing, document construction,
    /// local cache update via the persister — is identical to the
    /// legacy variant.
    pub async fn create_profile_with_external_signer<S>(
        &self,
        identity_id: &Identifier,
        input: ProfileUpdate,
        signer: &S,
    ) -> Result<DashPayProfile, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
    {
        // 1. The DashPay data contract (process-wide cache).
        let dashpay_contract = super::dashpay_contract()?;

        // 2. The profile fields, with avatar hashes computed from any raw bytes.
        let fields = profile_fields(input)?;

        // 3. Look up identity + signing key. The identity_index is not
        // needed here — the signer is supplied externally.
        let signing_key = {
            let wm = self.wallet_manager.read().await;
            let info = wm
                .get_wallet_info(&self.wallet_id)
                .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
            let managed = info
                .identity_manager
                .managed_identity(identity_id)
                .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?;
            let identity = managed.identity.clone();
            drop(wm);
            profile_signing_key(&identity, dashpay_contract.id(), signer)?
                .cloned()
                .ok_or_else(|| {
                    PlatformWalletError::InvalidIdentityData(
                        "No HIGH or CRITICAL authentication key found on identity \
                         (required for document state transitions)"
                            .to_string(),
                    )
                })?
        };

        let (created, owner_balance, metadata) = self
            .sdk
            .create_profile_with_owner_balance_and_metadata(
                &dashpay_contract,
                *identity_id,
                fields,
                &signing_key,
                signer,
            )
            .await
            .map_err(PlatformWalletError::Sdk)?;

        Ok(self
            .record_own_profile_write(identity_id, &created, owner_balance, metadata)
            .await)
    }

    /// Update an existing DashPay profile document using an
    /// externally-supplied signer.
    ///
    /// Mirrors [`Self::update_profile`] but signing is routed through
    /// the supplied `&S: Signer<IdentityPublicKey>`.
    pub async fn update_profile_with_external_signer<S>(
        &self,
        identity_id: &Identifier,
        input: ProfileUpdate,
        signer: &S,
    ) -> Result<DashPayProfile, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
    {
        // 1. The DashPay contract (process-wide cache).
        let dashpay_contract = super::dashpay_contract()?;

        // 2. The profile fields; avatar hashes are computed only when new
        //    bytes are provided. The SDK merges them onto the on-platform
        //    document, so unset fields (avatar included) are retained.
        let fields = profile_fields(input)?;

        // 3. Look up signing key.
        let signing_key = {
            let wm = self.wallet_manager.read().await;
            let info = wm
                .get_wallet_info(&self.wallet_id)
                .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
            let managed = info
                .identity_manager
                .managed_identity(identity_id)
                .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?;
            let identity = managed.identity.clone();
            drop(wm);
            profile_signing_key(&identity, dashpay_contract.id(), signer)?
                .cloned()
                .ok_or_else(|| {
                    PlatformWalletError::InvalidIdentityData(
                        "No HIGH or CRITICAL authentication key found on identity \
                         (required for document state transitions)"
                            .to_string(),
                    )
                })?
        };

        // The returned profile overwrites the local cache, so it must
        // reflect the merged on-platform state, not the partial input.
        let (merged, owner_balance, metadata) = self
            .sdk
            .update_profile_with_owner_balance_and_metadata(
                &dashpay_contract,
                *identity_id,
                fields,
                &signing_key,
                signer,
            )
            .await
            .map_err(PlatformWalletError::Sdk)?;

        Ok(self
            .record_own_profile_write(identity_id, &merged, owner_balance, metadata)
            .await)
    }

    /// Keep what a confirmed profile write of an own identity proved: the
    /// owner's balance its proof carries, as every proven document write
    /// keeps it, and the profile document, cached in the same sanitized form
    /// the sync path caches and returned.
    pub(super) async fn record_own_profile_write(
        &self,
        identity_id: &Identifier,
        document: &Document,
        owner_balance: Option<Credits>,
        metadata: ResponseMetadata,
    ) -> DashPayProfile {
        self.persist_proven_owner_balance(identity_id, owner_balance, metadata)
            .await;
        let profile = cacheable_profile(document.properties());
        let mut wm = self.wallet_manager.write().await;
        if let Some(info) = wm.get_wallet_info_mut(&self.wallet_id) {
            if let Some(managed) = info.identity_manager.managed_identity_mut(identity_id) {
                managed.set_dashpay_profile(profile.clone(), &self.persister);
            }
        }
        profile.unwrap_or_default()
    }
}

// ---------------------------------------------------------------------------
// Profile-field helpers
// ---------------------------------------------------------------------------

/// The SDK profile fields for a [`ProfileUpdate`]. The avatar hash and dHash
/// fingerprint are computed only when new avatar bytes are provided.
fn profile_fields(input: ProfileUpdate) -> Result<ProfileFields, PlatformWalletError> {
    let (avatar_hash, avatar_fingerprint) = if let Some(bytes) = &input.avatar_bytes {
        let hash = calculate_avatar_hash(bytes);
        let fingerprint =
            calculate_dhash_fingerprint(bytes).map_err(PlatformWalletError::InvalidIdentityData)?;
        (Some(hash), Some(fingerprint))
    } else {
        (None, None)
    };
    Ok(ProfileFields {
        display_name: input.display_name,
        public_message: input.public_message,
        avatar_url: input.avatar_url,
        avatar_hash,
        avatar_fingerprint,
    })
}

/// Parse a profile document's property map into a [`DashPayProfile`].
/// Empty strings are normalized to `None`. `avatarHash`/`avatarFingerprint`
/// are read via `as_bytes_slice` so both `Bytes` and the sized `Bytes32`
/// representation round-trip.
fn profile_from_properties(props: &std::collections::BTreeMap<String, Value>) -> DashPayProfile {
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

    DashPayProfile {
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
    /// fetch per owner with failure isolation, and write the
    /// per-owner cache with full-replace + persist-on-change. Contacts that
    /// are themselves managed identities are skipped (their own
    /// `dashpay_profile` is authoritative). Display-only: a failure never
    /// aborts the sweep. Returns the number of cache entries changed.
    ///
    /// Fetching pending senders' profiles is an accepted privacy cost: an
    /// observer could link the `$ownerId In [...]` query to our inbound set,
    /// but the DAPI node that serves our `toUserId == me` query already sees
    /// that whole set, so the profile fetch adds little.
    ///
    /// Profiles are refetched after `CONTACT_PROFILE_REFRESH_MS` rather than by
    /// `$updatedAt`: `$ownerId In [...] AND $updatedAt > marker` cannot be
    /// proven in one query (an `In` on the first index field plus a range on
    /// the second is not a contiguous index range). A per-owner `$updatedAt`
    /// query would lose the batching.
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

        // 2. Fetch (no guard held). Per owner: one `get_profiles` call; a
        //    failure logs and continues so the other owners still land. An id
        //    requested but absent from the result is confirmed-absent (cached
        //    as `None` — the negative cache).
        // One owner's fetched contacts: each contact id paired with its profile, or
        // `None` when confirmed-absent (the negative cache).
        type OwnerContactProfiles = Vec<(Identifier, Option<DashPayProfile>)>;
        let mut results: Vec<(Identifier, OwnerContactProfiles)> = Vec::new();
        for (owner_id, to_fetch) in plan {
            match self.fetch_profiles(&dashpay_contract, &to_fetch).await {
                Ok(found) => {
                    let owner_results: OwnerContactProfiles = to_fetch
                        .iter()
                        .map(|id| (*id, found.get(id).cloned().flatten()))
                        .collect();
                    results.push((owner_id, owner_results));
                }
                Err(e) => {
                    tracing::warn!(
                        owner = %owner_id,
                        error = %e,
                        "Failed to fetch the contact profiles; will retry next sweep"
                    );
                }
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

    /// Fetch profiles through [`Sdk::get_profiles`](dash_sdk::Sdk::get_profiles),
    /// returning the present profiles keyed by owner id (absent ids are simply
    /// missing). The `profile` `ownerId` index is unique, so the set lookup
    /// needs no pagination (≤1 profile per owner).
    async fn fetch_profiles(
        &self,
        dashpay_contract: &Arc<dpp::data_contract::DataContract>,
        owner_ids: &[Identifier],
    ) -> Result<std::collections::BTreeMap<Identifier, Option<DashPayProfile>>, PlatformWalletError>
    {
        let docs = self
            .sdk
            .get_profiles(dashpay_contract, owner_ids)
            .await
            .map_err(PlatformWalletError::Sdk)?;

        Ok(docs
            .into_iter()
            .map(|(owner, doc)| (owner, cacheable_profile(doc.properties())))
            .collect())
    }
}

/// Parse a profile document into the form the cache stores, for every path
/// that caches one. An untrusted avatar URL is dropped rather than cached. A
/// doc that parses to no populated field is `None` (confirmed-absent, the
/// negative cache), not a cached-present empty profile — so self-heal keeps
/// it honest.
fn cacheable_profile(props: &std::collections::BTreeMap<String, Value>) -> Option<DashPayProfile> {
    let mut profile = profile_from_properties(props);
    if profile
        .avatar_url
        .as_deref()
        .is_some_and(|u| !is_valid_avatar_url(u))
    {
        profile.avatar_url = None;
    }
    (profile != DashPayProfile::default()).then_some(profile)
}

#[cfg(test)]
mod tests {
    use super::super::signing_key::tests::KeyFilter;
    use super::*;
    use std::collections::BTreeMap;

    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dpp::identity::KeyType;
    use dpp::version::PlatformVersion;

    fn profile_key(key_type: KeyType, security_level: SecurityLevel) -> IdentityPublicKeyV0 {
        IdentityPublicKeyV0 {
            id: 1,
            purpose: Purpose::AUTHENTICATION,
            security_level,
            key_type,
            data: vec![1; key_type.default_size()].into(),
            ..Default::default()
        }
    }

    fn identity_with_key(key: IdentityPublicKeyV0) -> Identity {
        let mut identity = Identity::default_versioned(PlatformVersion::latest()).unwrap();
        identity.add_public_key(key.into());
        identity
    }

    const DASHPAY: [u8; 32] = [0xDA; 32];

    fn pick(identity: &Identity) -> Option<&IdentityPublicKey> {
        profile_signing_key(identity, Identifier::from(DASHPAY), &KeyFilter(|_| true)).unwrap()
    }

    #[test]
    fn should_reject_a_key_bound_to_another_contract() {
        use dpp::identity::identity_public_key::contract_bounds::ContractBounds;

        let mut elsewhere = profile_key(KeyType::ECDSA_SECP256K1, SecurityLevel::HIGH);
        elsewhere.contract_bounds = Some(ContractBounds::SingleContract {
            id: Identifier::from([0x0E; 32]),
        });
        assert!(pick(&identity_with_key(elsewhere)).is_none());

        let mut dashpay = profile_key(KeyType::ECDSA_SECP256K1, SecurityLevel::HIGH);
        dashpay.contract_bounds = Some(ContractBounds::SingleContractDocumentType {
            id: Identifier::from(DASHPAY),
            document_type_name: "profile".to_string(),
        });
        assert!(pick(&identity_with_key(dashpay)).is_some());
    }

    #[test]
    fn should_select_high_and_critical_ecdsa_profile_keys() {
        for key_type in [KeyType::ECDSA_SECP256K1, KeyType::ECDSA_HASH160] {
            for level in [SecurityLevel::HIGH, SecurityLevel::CRITICAL] {
                let identity = identity_with_key(profile_key(key_type, level));
                assert!(pick(&identity).is_some(), "{key_type:?}/{level:?}");
            }
        }
    }

    #[test]
    fn should_reject_ineligible_profile_keys() {
        let empty = Identity::default_versioned(PlatformVersion::latest()).unwrap();
        assert!(pick(&empty).is_none());
        for key_type in [
            KeyType::BLS12_381,
            KeyType::BIP13_SCRIPT_HASH,
            KeyType::EDDSA_25519_HASH160,
        ] {
            assert!(pick(&identity_with_key(profile_key(
                key_type,
                SecurityLevel::HIGH
            )))
            .is_none());
        }
        for key_type in [KeyType::ECDSA_SECP256K1, KeyType::ECDSA_HASH160] {
            for level in [SecurityLevel::MASTER, SecurityLevel::MEDIUM] {
                assert!(pick(&identity_with_key(profile_key(key_type, level))).is_none());
            }
            let mut key = profile_key(key_type, SecurityLevel::HIGH);
            key.disabled_at = Some(1);
            assert!(pick(&identity_with_key(key)).is_none());
            let mut key = profile_key(key_type, SecurityLevel::CRITICAL);
            key.purpose = Purpose::TRANSFER;
            assert!(pick(&identity_with_key(key)).is_none());
        }
    }

    #[test]
    fn should_select_high_hash160_from_reported_identity_layout() {
        let mut identity =
            identity_with_key(profile_key(KeyType::ECDSA_HASH160, SecurityLevel::HIGH));
        let mut master = profile_key(KeyType::ECDSA_HASH160, SecurityLevel::MASTER);
        master.id = 0;
        identity.add_public_key(master.into());
        let mut transfer = profile_key(KeyType::ECDSA_HASH160, SecurityLevel::CRITICAL);
        transfer.id = 2;
        transfer.purpose = Purpose::TRANSFER;
        identity.add_public_key(transfer.into());
        assert_eq!(pick(&identity), identity.public_keys().get(&1));
    }

    /// The parsed profile reflects every field of a full property map; the
    /// `publicMessage` is both the bio and the public message, and an
    /// `avatarHash` stored as the sized `Bytes32` round-trips.
    #[test]
    fn should_read_every_profile_field_from_properties() {
        let mut props = BTreeMap::new();
        props.insert("displayName".to_string(), Value::Text("Alice".into()));
        props.insert("publicMessage".to_string(), Value::Text("hello".into()));
        props.insert(
            "avatarUrl".to_string(),
            Value::Text("https://x/a.png".into()),
        );
        props.insert("avatarHash".to_string(), Value::Bytes32([7u8; 32]));
        props.insert(
            "avatarFingerprint".to_string(),
            Value::Bytes(vec![1, 2, 3, 4, 5, 6, 7, 8]),
        );

        let prof = profile_from_properties(&props);
        assert_eq!(prof.display_name.as_deref(), Some("Alice"));
        assert_eq!(prof.public_message.as_deref(), Some("hello"));
        assert_eq!(prof.bio.as_deref(), Some("hello"));
        assert_eq!(prof.avatar_url.as_deref(), Some("https://x/a.png"));
        assert_eq!(prof.avatar_hash, Some([7u8; 32]));
        assert_eq!(prof.avatar_fingerprint, Some([1, 2, 3, 4, 5, 6, 7, 8]));
    }

    /// Every cached profile, own or contact, is sanitized the same way: a
    /// non-https avatar URL is dropped while the other fields stay, and a
    /// document with no populated field is not cached as a profile.
    #[test]
    fn should_cache_a_profile_without_an_untrusted_avatar_url() {
        let mut props = BTreeMap::new();
        props.insert("displayName".to_string(), Value::Text("Alice".into()));
        props.insert(
            "avatarUrl".to_string(),
            Value::Text("http://example.com/a.png".into()),
        );

        let cached = cacheable_profile(&props).expect("display name keeps it present");
        assert_eq!(cached.display_name.as_deref(), Some("Alice"));
        assert_eq!(cached.avatar_url, None);

        props.insert(
            "avatarUrl".to_string(),
            Value::Text("https://example.com/a.png".into()),
        );
        let cached = cacheable_profile(&props).unwrap();
        assert_eq!(
            cached.avatar_url.as_deref(),
            Some("https://example.com/a.png")
        );

        assert_eq!(cacheable_profile(&BTreeMap::new()), None);
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
}
