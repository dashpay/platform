//! DPNS name registration, resolution, search, and contest queries.

use dpp::identity::accessors::IdentityGettersV0;

use dpp::identity::Identity;
use dpp::identity::IdentityPublicKey;
use dpp::identity::KeyType;
use dpp::identity::Purpose;
use dpp::identity::SecurityLevel;
use dpp::prelude::Identifier;

use dpp::identity::signer::Signer;
use dpp::ProtocolError;

use async_trait::async_trait;
use dpp::address_funds::AddressWitness;
use dpp::platform_value::BinaryData;

use crate::error::PlatformWalletError;
use crate::wallet::identity::types::key_storage::DpnsNameInfo;

use super::*;

// ---------------------------------------------------------------------------
// Signer reference adapter
// ---------------------------------------------------------------------------

/// Thin wrapper that lets us pass a borrowed `&S` (where `S: Signer<K>`)
/// into APIs whose generic bound requires an owned `S2: Signer<K>`.
///
/// `dpp::identity::signer::Signer` is not implemented for `&T`, so a
/// caller who only has `&KeychainSigner` can't directly satisfy
/// `RegisterDpnsNameInput<S: Signer<IdentityPublicKey>>`. This wrapper
/// owns a `&'a S`, copies the trait's `Send + Sync + Debug` bounds, and
/// delegates each method back to the inner reference. Used internally
/// by [`IdentityWallet::register_name_with_external_signer`] so the
/// public FFI surface can pass the iOS `KeychainSigner` by reference
/// without forcing a clone or `Arc` round-trip per call.
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
// Contest vote-state public types
// ---------------------------------------------------------------------------

/// Ephemeral snapshot of a DPNS contest's vote state.
///
/// NOT cached on [`ManagedIdentity`](crate::wallet::identity::ManagedIdentity)
/// — contest dynamics (votes accruing, winner resolution) change
/// throughout the voting period, so a cached snapshot would go
/// stale fast. Produced on-demand by
/// [`IdentityWallet::contest_vote_state`] and consumed by the UI
/// layer for an instantaneous read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContestVoteState {
    /// The contested label (e.g., "alice").
    pub label: String,
    /// Voting end time in milliseconds since epoch.
    pub end_time_ms: u64,
    /// Current contenders + their vote tallies.
    pub contenders: Vec<ContestContender>,
    /// Count of abstain votes cast by masternodes.
    pub abstain_votes: u32,
    /// Count of lock votes (contest → no winner, no registration).
    pub lock_votes: u32,
    /// Resolution state. `None` while voting is ongoing.
    pub winner: ContestWinner,
}

/// One contender's summary within a [`ContestVoteState`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContestContender {
    /// The contending identity.
    pub identity_id: Identifier,
    /// Masternode vote count supporting this contender. 0 before
    /// any votes are cast.
    pub vote_tally: u32,
}

/// Contest resolution status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ContestWinner {
    /// Contest is still in the voting period (no winner yet).
    #[default]
    None,
    /// Contest resolved — this identity won the name.
    WonByIdentity(Identifier),
    /// Contest resolved as locked — nobody wins, the label becomes
    /// unavailable.
    Locked,
}

// ---------------------------------------------------------------------------
// DPNS name operations
// ---------------------------------------------------------------------------

impl IdentityWallet {
    /// Register a DPNS name using an externally-provided identity and signer.
    ///
    /// Unlike
    /// [`register_name_with_external_signer`](Self::register_name_with_external_signer),
    /// this method does **not** look up the identity in the internal
    /// `IdentityManager`. The caller supplies the `Identity`, the
    /// signing key, and a `Signer` directly.
    ///
    /// Returns the full domain name (e.g. "alice.dash").
    pub async fn register_name_with_signer<S: Signer<IdentityPublicKey>>(
        &self,
        identity: Identity,
        name: &str,
        identity_public_key: IdentityPublicKey,
        signer: S,
    ) -> Result<String, dash_sdk::Error> {
        use dash_sdk::platform::dpns_usernames::RegisterDpnsNameInput;

        let input = RegisterDpnsNameInput {
            label: name.to_string(),
            identity,
            identity_public_key,
            signer,
            preorder_callback: None,
        };

        let result = self.sdk.register_dpns_name(input).await?;
        Ok(result.full_domain_name)
    }

    /// Register a DPNS name for an identity using an
    /// externally-supplied signer.
    ///
    /// # Arguments
    ///
    /// * `identity_id` - The identity to register the name for.
    /// * `name` - The desired username label (e.g., "alice").
    /// * `signer` - External `Signer<IdentityPublicKey>` for the
    ///   document state-transition signature — the architecturally
    ///   correct path per `swift-sdk/CLAUDE.md`.
    ///
    /// The identity is looked up from the in-process `IdentityManager`
    /// so we can locate the HIGH/CRITICAL authentication key the
    /// document state transition requires; `signer.sign(...)` is
    /// invoked through the caller-supplied trait object. Works for
    /// watch-only wallets where the seed lives in iOS Keychain
    /// rather than the in-process `WalletManager`.
    ///
    /// On success the just-registered label is appended to
    /// `ManagedIdentity.dpns_names` and an identity changeset is queued.
    pub async fn register_name_with_external_signer<S>(
        &self,
        identity_id: &Identifier,
        name: &str,
        signer: &S,
    ) -> Result<String, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
    {
        use dash_sdk::platform::dpns_usernames::RegisterDpnsNameInput;

        // Ensure the on-chain DPNS contract is fetched and registered
        // with the SDK's context provider BEFORE broadcasting: the
        // post-broadcast proof of the preorder/domain documents needs it,
        // `Sdk::register_dpns_name` never registers it back, and a host
        // that doesn't pre-seed known contracts (e.g. a headless
        // consumer) would otherwise fail proof verification with
        // "unknown contract ... in document verification" even though
        // the registration landed on-chain.
        self.dpns_contract().await?;

        let (identity, auth_key) = {
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
            // DPNS name registration writes a document state transition,
            // which DPP requires to be signed by a HIGH-or-stricter
            // authentication key. MASTER is intentionally excluded —
            // it's reserved for identity-self-modification operations
            // (identity update, key rotation, withdrawal) and is
            // rejected by the protocol on document-side state
            // transitions.
            let key = identity
                .get_first_public_key_matching(
                    Purpose::AUTHENTICATION,
                    [SecurityLevel::HIGH, SecurityLevel::CRITICAL].into(),
                    [KeyType::ECDSA_SECP256K1].into(),
                    false,
                )
                .ok_or_else(|| {
                    PlatformWalletError::InvalidIdentityData(
                        "No HIGH or CRITICAL authentication key found on identity \
                         (required for document state transitions)"
                            .to_string(),
                    )
                })?
                .clone();
            (identity, key)
        };

        let input = RegisterDpnsNameInput {
            label: name.to_string(),
            identity,
            identity_public_key: auth_key,
            // Wrap the borrowed signer in `SignerRef` so the
            // `RegisterDpnsNameInput<S: Signer<IdentityPublicKey>>`
            // bound is satisfied without forcing the caller to hand
            // over ownership / wrap in an Arc per call.
            signer: SignerRef(signer),
            preorder_callback: None,
        };

        let result = self.sdk.register_dpns_name(input).await.map_err(|e| {
            // Preserve a structured key-unavailable signer failure so the FFI
            // boundary can still restore code 31; only genuine operation
            // failures get stringified into `InvalidIdentityData`.
            crate::error::preserve_signer_key_unavailable_or(e, |e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to register DPNS name '{}': {}",
                    name, e
                ))
            })
        })?;

        // Same book-keeping as `register_name`: append the just-
        // registered label to `ManagedIdentity.dpns_names` and emit
        // an `IdentityChangeSet` so the Swift persister callback
        // refreshes `PersistentIdentity.dpnsName` automatically.
        // Skip on duplicate to avoid emitting a redundant entry on
        // re-runs.
        let acquired_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .ok();
        let label_to_store = name.to_string();
        {
            let mut wm = self.wallet_manager.write().await;
            if let Some(info) = wm.get_wallet_info_mut(&self.wallet_id) {
                if let Some(managed) = info.identity_manager.managed_identity_mut(identity_id) {
                    if !managed
                        .dpns_names
                        .iter()
                        .any(|existing| existing.label == label_to_store)
                    {
                        managed.add_dpns_name(
                            DpnsNameInfo {
                                label: label_to_store,
                                acquired_at,
                            },
                            &self.persister,
                        );
                    }
                }
            }
        }

        Ok(result.full_domain_name)
    }

    /// Resolve a DPNS name to an identity identifier.
    ///
    /// Accepts both "alice" and "alice.dash" formats.
    pub async fn resolve_name(
        &self,
        name: &str,
    ) -> Result<Option<Identifier>, PlatformWalletError> {
        self.sdk.resolve_dpns_name(name).await.map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!(
                "Failed to resolve DPNS name '{}': {}",
                name, e
            ))
        })
    }

    /// Fetch all DPNS usernames owned by `identity_id` from Platform
    /// and merge them into the local
    /// [`ManagedIdentity.dpns_names`](crate::wallet::identity::ManagedIdentity)
    /// cache.
    ///
    /// Skips labels that are already in the cache, so repeated syncs
    /// don't emit duplicate entries. New labels get an
    /// `acquired_at` timestamp of best-effort wall-clock millis —
    /// DPNS documents carry their own `$createdAt` but
    /// `DpnsUsername` doesn't surface it on the query result today.
    ///
    /// Returns the number of newly-added labels.
    ///
    /// Use this from the iOS load path instead of
    /// [`Sdk::get_dpns_usernames_by_identity`] directly — the wallet
    /// path additionally updates the persister changeset, so
    /// `PersistentIdentity` + in-app views see the refresh via
    /// `on_persist_identities_fn`.
    pub async fn sync_dpns_names(
        &self,
        identity_id: &Identifier,
    ) -> Result<u32, PlatformWalletError> {
        let usernames = self
            .sdk
            .get_dpns_usernames_by_identity(*identity_id, None)
            .await
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to fetch DPNS usernames for identity {identity_id}: {e}",
                ))
            })?;

        if usernames.is_empty() {
            return Ok(0);
        }

        let acquired_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .ok();

        let mut added = 0u32;
        {
            let mut wm = self.wallet_manager.write().await;
            let Some(info) = wm.get_wallet_info_mut(&self.wallet_id) else {
                return Ok(0);
            };
            let Some(managed) = info.identity_manager.managed_identity_mut(identity_id) else {
                return Ok(0);
            };
            for username in usernames {
                if managed
                    .dpns_names
                    .iter()
                    .any(|existing| existing.label == username.label)
                {
                    continue;
                }
                managed.add_dpns_name(
                    DpnsNameInfo {
                        label: username.label,
                        acquired_at,
                    },
                    &self.persister,
                );
                added += 1;
            }
        }
        Ok(added)
    }

    /// Fetch the list of DPNS contests `identity_id` is currently
    /// contending for (still in the voting period, not resolved)
    /// and replace the local
    /// [`ManagedIdentity.contested_dpns_names`](crate::wallet::identity::ManagedIdentity)
    /// list wholesale with the canonical set.
    ///
    /// Uses `set_contested_dpns_names` rather than
    /// `add_contested_dpns_name` so resolved contests
    /// (won / locked) automatically drop out of the local cache —
    /// the sync result IS the authoritative set. Individual add/
    /// remove flows (e.g. "I just contended a name") can still use
    /// `add_contested_dpns_name` for immediate UI reflection; the
    /// next sync round reconciles.
    ///
    /// Returns the canonical contested-name labels.
    pub async fn sync_contested_dpns_names(
        &self,
        identity_id: &Identifier,
    ) -> Result<Vec<String>, PlatformWalletError> {
        let contests = self
            .sdk
            .get_non_resolved_dpns_contests_for_identity(*identity_id, None)
            .await
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to fetch contested DPNS names for identity {identity_id}: {e}",
                ))
            })?;

        let labels: Vec<String> = contests.keys().cloned().collect();

        // Drop the write lock before returning the labels. Using
        // `set_contested_dpns_names` means the snapshot-based
        // changeset emits a fresh list — resolved contests that the
        // local cache still carried get dropped on merge/apply.
        {
            let mut wm = self.wallet_manager.write().await;
            if let Some(info) = wm.get_wallet_info_mut(&self.wallet_id) {
                if let Some(managed) = info.identity_manager.managed_identity_mut(identity_id) {
                    managed.set_contested_dpns_names(labels.clone(), &self.persister);
                }
            }
        }
        Ok(labels)
    }

    /// Fetch the current vote state for a contested DPNS label the
    /// identity is contending for.
    ///
    /// Goes via
    /// `Sdk::get_non_resolved_dpns_contests_for_identity` under the
    /// hood so the returned snapshot includes the contest's
    /// `end_time` alongside the contender list + vote tallies +
    /// winner (if any). Returns `None` when `identity_id` isn't a
    /// contender for `label` — either because the contest doesn't
    /// exist, the identity isn't contending, or the contest already
    /// resolved and the identity filter dropped it.
    ///
    /// NOT cached on `ManagedIdentity` — vote state changes
    /// throughout the voting period, so the UI queries this fresh
    /// every time it needs it.
    pub async fn contest_vote_state(
        &self,
        identity_id: &Identifier,
        label: &str,
    ) -> Result<Option<ContestVoteState>, PlatformWalletError> {
        use dpp::voting::vote_info_storage::contested_document_vote_poll_winner_info::ContestedDocumentVotePollWinnerInfo;

        let contests = self
            .sdk
            .get_non_resolved_dpns_contests_for_identity(*identity_id, None)
            .await
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to fetch contest vote state for identity {identity_id}, \
                     label {label:?}: {e}",
                ))
            })?;

        let Some(contest_info) = contests.get(label) else {
            return Ok(None);
        };

        let contenders: Vec<ContestContender> = contest_info
            .contenders
            .contenders
            .iter()
            .map(|(id, c)| ContestContender {
                identity_id: *id,
                vote_tally: c.vote_tally().unwrap_or(0),
            })
            .collect();

        let winner = match &contest_info.contenders.winner {
            Some((ContestedDocumentVotePollWinnerInfo::WonByIdentity(id), _)) => {
                ContestWinner::WonByIdentity(*id)
            }
            Some((ContestedDocumentVotePollWinnerInfo::Locked, _)) => ContestWinner::Locked,
            Some((ContestedDocumentVotePollWinnerInfo::NoWinner, _)) | None => ContestWinner::None,
        };

        Ok(Some(ContestVoteState {
            label: label.to_string(),
            end_time_ms: contest_info.end_time,
            contenders,
            abstain_votes: contest_info.contenders.abstain_vote_tally.unwrap_or(0),
            lock_votes: contest_info.contenders.lock_vote_tally.unwrap_or(0),
            winner,
        }))
    }

    /// Search for DPNS names by prefix.
    pub async fn search_names(
        &self,
        prefix: &str,
        limit: Option<u32>,
    ) -> Result<Vec<dash_sdk::platform::dpns_usernames::DpnsUsername>, PlatformWalletError> {
        self.sdk
            .search_dpns_names(prefix, limit)
            .await
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to search DPNS names with prefix '{}': {}",
                    prefix, e
                ))
            })
    }
}
