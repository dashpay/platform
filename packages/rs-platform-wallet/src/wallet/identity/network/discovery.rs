//! Identity discovery via gap-limit HD scan.

use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::Identity;

use crate::error::PlatformWalletError;

use super::*;

// ---------------------------------------------------------------------------
// Identity discovery (gap-limit scan)
// ---------------------------------------------------------------------------

/// Configuration knobs for [`IdentityWallet::discover`].
///
/// The defaults match `sync()`'s historical behavior: resume one past
/// the wallet's highest already-registered identity index and stop
/// after [`IDENTITY_GAP_LIMIT`] consecutive empty slots. Callers who
/// want a full rescan pass `start_index: Some(0)`; callers who want a
/// deeper gap tolerance bump `gap_limit`.
///
/// The scan only looks at key index 0 within each identity index. That
/// covers every identity this crate registers (see `registration.rs`:
/// `key_index = 0` is always the MASTER authentication key and is
/// always registered with its pubkey hash indexed on Platform, so it's
/// always discoverable via the unique-key query). Identities
/// registered by foreign clients using non-zero key indices are not
/// covered; that's a deliberate simplification since we haven't
/// encountered wallets that need it.
#[derive(Debug, Clone, Copy)]
pub struct IdentityDiscoveryOptions {
    /// Identity index to start scanning from. `None` means "resume one
    /// past the wallet's highest already-registered identity index"
    /// (default).
    pub start_index: Option<u32>,
    /// How many consecutive empty identity indices to tolerate before
    /// stopping. Defaults to [`IDENTITY_GAP_LIMIT`].
    pub gap_limit: u32,
}

impl Default for IdentityDiscoveryOptions {
    fn default() -> Self {
        Self {
            start_index: None,
            gap_limit: IDENTITY_GAP_LIMIT,
        }
    }
}

impl IdentityWallet {
    /// Thin wrapper around [`Self::discover`] using default options —
    /// resume from the cached scan index, stop after `IDENTITY_GAP_LIMIT`
    /// consecutive misses. Kept for back-compat with existing callers.
    pub async fn sync(&self) -> Result<Vec<Identity>, PlatformWalletError> {
        self.discover(IdentityDiscoveryOptions::default()).await
    }

    /// Discover identities owned by this wallet via gap-limit scanning.
    ///
    /// For each identity index starting at `opts.start_index` (or one
    /// past the wallet's highest already-registered identity index
    /// when `None`), derives the ECDSA authentication public key at
    /// key index 0 from the BIP-32 tree and queries Platform for a
    /// registered identity bound to that key hash (the unique-key
    /// lookup). Stops after `opts.gap_limit` consecutive misses.
    ///
    /// For every discovered identity this method also:
    /// - queries DPNS for associated usernames,
    /// - emits an `IdentityKeysChangeSet` upsert with the
    ///   `(wallet_id, identity_index, key_index)` derivation breadcrumb
    ///   so the client (iOS Keychain) can re-derive the private key,
    /// - records the wallet id on the managed identity, and
    /// - sets the identity status to `Active`.
    ///
    /// Newly-discovered identities are added to the local identity
    /// manager and returned. Resume position is derived from
    /// [`IdentityManager::highest_registration_index`] — no separate
    /// watermark to maintain.
    pub async fn discover(
        &self,
        opts: IdentityDiscoveryOptions,
    ) -> Result<Vec<Identity>, PlatformWalletError> {
        use super::identity_handle::{identity_auth_derivation_path, MASTER_KEY_INDEX};
        use crate::wallet::identity::state::managed_identity::key_storage::DpnsNameInfo;
        use crate::wallet::identity::state::managed_identity::key_storage::IdentityStatus;
        use dash_sdk::platform::types::identity::PublicKeyHash;
        use dash_sdk::platform::Fetch;
        use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
        use dpp::util::hash::ripemd160_sha256;

        // `MASTER_KEY_INDEX = 0` pulled in from `identity_handle` —
        // only key_index 0 is ever registered as the MASTER auth
        // key; scanning higher indices is redundant since the same
        // identity would be returned by any of its authentication
        // pubkey hashes.

        let (network, cached_start_index, wallet_id) = {
            let wm = self.wallet_manager.read().await;
            let wallet = wm.get_wallet(&self.wallet_id).ok_or_else(|| {
                crate::error::PlatformWalletError::WalletNotFound(
                    "Wallet not found in wallet manager".to_string(),
                )
            })?;
            let info = wm.get_wallet_info(&self.wallet_id).ok_or_else(|| {
                crate::error::PlatformWalletError::WalletNotFound(
                    "Wallet info not found in wallet manager".to_string(),
                )
            })?;
            // Resume one past the highest already-registered slot.
            let resume_from = info
                .identity_manager
                .highest_registration_index(&self.wallet_id)
                .map_or(0, |i| i + 1);
            (wallet.network, resume_from, self.wallet_id)
        };

        let start_index = opts.start_index.unwrap_or(cached_start_index);
        let gap_limit = opts.gap_limit.max(1);

        let mut consecutive_misses = 0u32;
        let mut identity_index = start_index;
        let mut discovered: Vec<Identity> = Vec::new();

        while consecutive_misses < gap_limit {
            let key_hash_array = {
                let wm = self.wallet_manager.read().await;
                let wallet = wm.get_wallet(&self.wallet_id).ok_or_else(|| {
                    crate::error::PlatformWalletError::WalletNotFound(
                        "Wallet not found in wallet manager".to_string(),
                    )
                })?;
                derive_identity_auth_key_hash(wallet, network, identity_index, MASTER_KEY_INDEX)?
            };

            // Query Platform for an identity registered with this key
            // hash. No locks are held during this network call.
            let fetch_result = Identity::fetch(&self.sdk, PublicKeyHash(key_hash_array)).await;

            match fetch_result {
                Ok(Some(identity)) => {
                    let identity_id = identity.id();

                    // Same helper the FFI-side preview uses —
                    // keeps the scan and the preview on a single
                    // path-building code path.
                    let full_path =
                        identity_auth_derivation_path(network, identity_index, MASTER_KEY_INDEX)?;

                    // Find the KeyID in the on-chain identity whose
                    // hash matches our derived key so the derivation
                    // path gets stored against the right KeyID.
                    let matched_key_id_and_pub = identity
                        .public_keys()
                        .iter()
                        .find(|(_, pk)| {
                            let pk_hash = ripemd160_sha256(pk.data().as_slice());
                            pk_hash.as_slice() == key_hash_array
                        })
                        .map(|(kid, pk)| (*kid, pk.clone()));

                    // Acquire write lock to add/enrich the identity.
                    let mut wm_guard = self.wallet_manager.write().await;
                    let info_guard =
                        wm_guard
                            .get_wallet_info_mut(&self.wallet_id)
                            .ok_or_else(|| {
                                crate::error::PlatformWalletError::WalletNotFound(
                                    "Wallet info not found in wallet manager".to_string(),
                                )
                            })?;
                    let is_new = info_guard.identity_manager.identity(&identity_id).is_none();
                    if is_new {
                        info_guard.identity_manager.add_identity(
                            identity.clone(),
                            identity_index,
                            wallet_id,
                            &self.persister,
                        )?;
                    }

                    if let Some(managed) = info_guard
                        .identity_manager
                        .managed_identity_mut(&identity_id)
                    {
                        managed.set_status(IdentityStatus::Active, &self.persister);
                        managed.wallet_id = Some(wallet_id);

                        if let Some((_kid, pub_key)) = matched_key_id_and_pub {
                            // Pass the DIP-9 breadcrumb so the client can
                            // re-derive the private key from its wallet
                            // mnemonic via the iOS Keychain path.
                            managed.add_key(
                                pub_key,
                                Some((wallet_id, identity_index, MASTER_KEY_INDEX)),
                                &self.persister,
                            );
                        }
                    }
                    // `full_path` is no longer needed once `add_key`
                    // takes the breadcrumb instead of the materialized
                    // DerivationPath.
                    let _ = full_path;
                    drop(wm_guard);

                    if is_new {
                        discovered.push(identity.clone());
                    }
                    consecutive_misses = 0;
                }
                Ok(None) => {
                    consecutive_misses += 1;
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to query identity at index {}: {}",
                        identity_index,
                        e
                    );
                    // Treat a transient fetch error as a miss so the
                    // scan eventually terminates; the user can rerun
                    // to pick up anything we skipped over.
                    consecutive_misses += 1;
                }
            }

            identity_index += 1;
        }

        // --- DPNS lookup for all discovered identities ---
        for identity in &discovered {
            let identity_id = identity.id();
            match self
                .sdk
                .get_dpns_usernames_by_identity(identity_id, None)
                .await
            {
                Ok(usernames) => {
                    let mut wm_guard = self.wallet_manager.write().await;
                    let info_guard =
                        wm_guard
                            .get_wallet_info_mut(&self.wallet_id)
                            .ok_or_else(|| {
                                crate::error::PlatformWalletError::WalletNotFound(
                                    "Wallet info not found in wallet manager".to_string(),
                                )
                            })?;
                    if let Some(managed) = info_guard
                        .identity_manager
                        .managed_identity_mut(&identity_id)
                    {
                        for username in usernames {
                            managed.add_dpns_name(
                                DpnsNameInfo {
                                    label: username.label,
                                    acquired_at: None,
                                },
                                &self.persister,
                            );
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to fetch DPNS names for identity {}: {}",
                        identity_id,
                        e
                    );
                }
            }
        }

        // No standalone scan watermark to advance — resume position is
        // derived next call from `IdentityManager::highest_registration_index`,
        // which the inserted identities above already bumped.
        let _ = cached_start_index;
        let _ = identity_index;

        Ok(discovered)
    }
}
