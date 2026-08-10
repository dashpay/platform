//! Identity discovery via gap-limit HD scan.

use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::Identity;
use key_wallet::bip32::ExtendedPrivKey;

use crate::error::PlatformWalletError;

use super::*;

/// Where the per-index MASTER auth pubkey hash for the discovery scan
/// comes from. The two discovery entry points differ *only* in this:
/// everything else (gap-limit bookkeeping, Platform lookup, identity
/// folding, DPNS enrichment) is shared in [`IdentityWallet::discover_inner`].
///
/// - [`KeyHashSource::ResidentWallet`] derives from the in-process
///   `Wallet` key material (the historical path). Only valid for wallets
///   that actually hold a private key in memory
///   (`WalletType::Mnemonic` / `Seed` / `ExtendedPrivKey`).
/// - [`KeyHashSource::Master`] derives from a master `ExtendedPrivKey`
///   the caller resolved from the wallet's mnemonic on demand. This is
///   the path the iOS Keychain-backed `WalletType::ExternalSignable`
///   shape uses: its seed lives outside the in-process wallet manager,
///   so the resident-wallet derive would fail with
///   `External signable wallet has no private key`.
enum KeyHashSource<'a> {
    /// Derive each probe hash from the in-memory wallet under a per-index
    /// read lock on the shared `WalletManager`.
    ResidentWallet,
    /// Derive each probe hash from this master xpriv (pure, no lock).
    Master(&'a ExtendedPrivKey),
}

/// For each on-chain key of `identity`, decide its derivation breadcrumb:
/// `Some((wallet_id, identity_index, key_id))` when `candidate_scalars`
/// holds a scalar that reproduces the on-chain key — so the client can
/// re-derive that key's private material from the wallet seed — else
/// `None`, a watch-only key the wallet cannot sign with.
///
/// Verification uses the canonical
/// [`IdentityPublicKey::validate_private_key_bytes`] — the same primitive
/// the protocol uses to validate key ownership — so the wallet's match is
/// identical to consensus and cannot drift. For ECDSA it recomputes the
/// compressed public key from the candidate scalar and compares; every key
/// the wallet could own is 33-byte compressed, so this matches all of them.
/// A key that is NOT reproducible from this wallet's seed stays watch-only:
/// a foreign key, a BLS/EdDSA key (an ECDSA-derived candidate never
/// reproduces a different-curve key), or an uncompressed externally-
/// registered ECDSA key (Platform's signature checks accept uncompressed
/// keys, but the wallet only ever derives the compressed form, so such a key
/// simply isn't wallet-derivable). So a non-reproducible key is never handed
/// a (wrong) breadcrumb — the load-bearing guard that stops the client from
/// materializing and signing with a key the identity does not authorize
/// on-chain. An ECDSA *authentication* key that fails to verify at its
/// `key_id` candidate is logged at `warn` so a still-unsignable import is
/// diagnosable in the field (no key material is logged).
fn breadcrumb_decisions(
    identity: &Identity,
    identity_index: u32,
    wallet_id: [u8; 32],
    network: key_wallet::Network,
    candidate_scalars: &std::collections::BTreeMap<
        dpp::identity::KeyID,
        zeroize::Zeroizing<[u8; 32]>,
    >,
) -> Vec<crate::changeset::KeyWithBreadcrumb> {
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::identity::identity_public_key::methods::hash::IdentityPublicKeyHashMethodsV0;
    use dpp::identity::KeyType;

    identity
        .public_keys()
        .iter()
        .map(|(key_id, on_chain_key)| {
            let reproduces = candidate_scalars
                .get(key_id)
                .map(|scalar| {
                    on_chain_key
                        .validate_private_key_bytes(scalar, network)
                        .unwrap_or(false)
                })
                .unwrap_or(false);
            let breadcrumb = if reproduces {
                // The candidate scalar reproduced the on-chain key, so this
                // wallet owns it: emit the DIP-9 coordinates. The scalar is
                // verified here and dropped — never carried out. The client
                // re-derives the key on demand from the Keychain seed at this
                // breadcrumb path when it needs to sign.
                Some((wallet_id, identity_index, *key_id))
            } else {
                if matches!(
                    on_chain_key.key_type(),
                    KeyType::ECDSA_SECP256K1 | KeyType::ECDSA_HASH160
                ) {
                    tracing::warn!(
                        identity = %identity.id(),
                        key_id = *key_id,
                        "discovered identity ECDSA key did not verify at its key_id \
                         derivation candidate; left watch-only (cannot sign with this key)"
                    );
                }
                None
            };
            crate::changeset::KeyWithBreadcrumb {
                key: on_chain_key.clone(),
                breadcrumb,
            }
        })
        .collect()
}

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
        self.discover_inner(opts, KeyHashSource::ResidentWallet)
            .await
    }

    /// Master-xpriv variant of [`Self::discover`]: run the identical
    /// gap-limit scan / identity-folding / DPNS-enrichment logic, but
    /// derive each probe's MASTER auth pubkey hash from the supplied
    /// master `ExtendedPrivKey` instead of from the in-memory wallet.
    ///
    /// This is the path the iOS Keychain-backed
    /// `WalletType::ExternalSignable` wallets must take: their seed lives
    /// outside the in-process wallet manager, so [`Self::discover`]'s
    /// resident-wallet derive fails with
    /// `External signable wallet has no private key`. The caller resolves
    /// the wallet's mnemonic into `master` on demand (see the FFI
    /// resolver path) and hands it in here; the derivation goes through
    /// the same [`derive_identity_auth_key_hash_from_master`] the
    /// registration path uses, so a rescan derives exactly the key
    /// material a key-resident wallet would.
    ///
    /// `master` must be the BIP-32 master node for this wallet on its
    /// network (`ExtendedPrivKey::new_master(network, mnemonic.to_seed(""))`).
    pub async fn discover_from_master(
        &self,
        opts: IdentityDiscoveryOptions,
        master: &ExtendedPrivKey,
    ) -> Result<Vec<Identity>, PlatformWalletError> {
        self.discover_inner(opts, KeyHashSource::Master(master))
            .await
    }

    /// Derive a candidate ECDSA auth scalar for every on-chain key of a
    /// just-found `identity`, verify each reproduces the published key, and
    /// return the per-key derivation-breadcrumb decisions (see
    /// [`breadcrumb_decisions`]). Shared by the discovery scan and the
    /// index-load path so both materialize the identity's *full* signable
    /// key set — not just the MASTER key — letting an imported identity
    /// sign with its HIGH / CRITICAL keys.
    ///
    /// `master == Some(..)` derives lock-free from the resolved master
    /// xpriv (external-signable wallets); `None` derives from the resident
    /// in-memory wallet under a brief read lock that is dropped before the
    /// caller takes the write lock to emit. `key_index == key_id` mirrors
    /// the registration path; the per-key verify makes a wrong assumption
    /// fail safe (watch-only).
    pub(crate) async fn derive_key_breadcrumbs(
        &self,
        identity: &Identity,
        identity_index: u32,
        network: key_wallet::Network,
        master: Option<&ExtendedPrivKey>,
    ) -> Result<Vec<crate::changeset::KeyWithBreadcrumb>, PlatformWalletError> {
        use super::identity_handle::{
            derive_ecdsa_identity_auth_keypair_from_master, derive_identity_auth_keypair,
        };

        let mut candidate_scalars: std::collections::BTreeMap<
            dpp::identity::KeyID,
            zeroize::Zeroizing<[u8; 32]>,
        > = std::collections::BTreeMap::new();
        match master {
            Some(master) => {
                for key_id in identity.public_keys().keys() {
                    if let Ok(kp) = derive_ecdsa_identity_auth_keypair_from_master(
                        master,
                        network,
                        identity_index,
                        *key_id,
                    ) {
                        candidate_scalars.insert(*key_id, kp.private_key);
                    }
                }
            }
            None => {
                let wm_read = self.wallet_manager.read().await;
                // The wallet was present for the master probe one step
                // earlier; its absence here is a genuine error, so fail loud
                // (consistent with every other manager lookup in this file)
                // rather than silently leaving every key watch-only.
                let wallet = wm_read.get_wallet(&self.wallet_id).ok_or_else(|| {
                    crate::error::PlatformWalletError::WalletNotFound(
                        "Wallet not found in wallet manager".to_string(),
                    )
                })?;
                for key_id in identity.public_keys().keys() {
                    if let Ok((_, xpriv, _)) =
                        derive_identity_auth_keypair(wallet, network, identity_index, *key_id)
                    {
                        candidate_scalars.insert(
                            *key_id,
                            zeroize::Zeroizing::new(xpriv.private_key.secret_bytes()),
                        );
                    }
                }
            }
        }

        Ok(breadcrumb_decisions(
            identity,
            identity_index,
            self.wallet_id,
            network,
            &candidate_scalars,
        ))
    }

    /// Shared gap-limit scan body for [`Self::discover`] and
    /// [`Self::discover_from_master`]. The only thing the two callers
    /// vary is `source`, which decides how each probe's MASTER auth
    /// pubkey hash is derived (in-memory wallet under a per-index read
    /// lock, vs. a resolved master xpriv). Everything downstream — the
    /// Platform unique-hash lookup, identity folding, derivation
    /// breadcrumb, and DPNS enrichment — is identical, so it lives here
    /// once.
    async fn discover_inner(
        &self,
        opts: IdentityDiscoveryOptions,
        source: KeyHashSource<'_>,
    ) -> Result<Vec<Identity>, PlatformWalletError> {
        use super::identity_handle::{derive_identity_auth_key_hash_from_master, MASTER_KEY_INDEX};
        use crate::wallet::identity::state::managed_identity::key_storage::DpnsNameInfo;
        use crate::wallet::identity::state::managed_identity::key_storage::IdentityStatus;
        use dash_sdk::platform::types::identity::PublicKeyHash;
        use dash_sdk::platform::Fetch;

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

        let mut identity_index = start_index;
        let mut discovered: Vec<Identity> = Vec::new();
        let mut tally = ScanTally::default();

        while tally.should_continue(gap_limit) {
            // Derive the MASTER auth pubkey hash for this identity index
            // from whichever source the caller picked. The per-index read
            // lock is only needed for the wallet-internal derive (it reads
            // the resident key material); the master derive is a pure,
            // lock-free secp256k1 pass.
            let key_hash_array = match source {
                KeyHashSource::ResidentWallet => {
                    let wm = self.wallet_manager.read().await;
                    let wallet = wm.get_wallet(&self.wallet_id).ok_or_else(|| {
                        crate::error::PlatformWalletError::WalletNotFound(
                            "Wallet not found in wallet manager".to_string(),
                        )
                    })?;
                    derive_identity_auth_key_hash(
                        wallet,
                        network,
                        identity_index,
                        MASTER_KEY_INDEX,
                    )?
                }
                KeyHashSource::Master(master) => derive_identity_auth_key_hash_from_master(
                    master,
                    network,
                    identity_index,
                    MASTER_KEY_INDEX,
                )?,
            };

            // Query Platform for an identity registered with this key
            // hash. No locks are held during this network call.
            let fetch_result = Identity::fetch(&self.sdk, PublicKeyHash(key_hash_array)).await;

            match fetch_result {
                Ok(Some(identity)) => {
                    let identity_id = identity.id();

                    // Derive + verify a candidate for every on-chain key
                    // (shared with the index-load path) BEFORE taking the write
                    // lock — candidate derivation borrows the resident wallet /
                    // master xpriv, while breadcrumb emission needs `&mut info`.
                    let key_decisions = self
                        .derive_key_breadcrumbs(
                            &identity,
                            identity_index,
                            network,
                            match source {
                                KeyHashSource::Master(master) => Some(master),
                                KeyHashSource::ResidentWallet => None,
                            },
                        )
                        .await?;

                    // Acquire write lock to add/enrich the identity, then emit
                    // every per-key breadcrumb in one batched changeset.
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
                        // Breadcrumbs for every re-derivable key (not just the
                        // MASTER key) so the client (iOS Keychain) can
                        // re-derive each signing key's private key — without
                        // this only the master key is materialized and the
                        // imported identity cannot sign with its HIGH /
                        // CRITICAL authentication keys. A failed persist here
                        // would silently leave the identity watch-only after
                        // restart, so surface it (matching `add_identity` above).
                        managed
                            .add_keys(key_decisions, &self.persister)
                            .map_err(|e| {
                                PlatformWalletError::Persistence(format!(
                                    "identity keys not persisted during discovery: {e}"
                                ))
                            })?;
                    }
                    drop(wm_guard);

                    if is_new {
                        discovered.push(identity.clone());
                    }
                    tally.record_sighting();
                }
                Ok(None) => {
                    tally.record_miss();
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to query identity at index {}: {}",
                        identity_index,
                        e
                    );
                    tally.record_failure(e);
                }
            }

            identity_index += 1;
        }

        if tally.is_trustworthy() {
            // Found something despite a failed probe: the discovered
            // identities are already persisted, so return them rather than
            // discarding the work. The gap is still worth a line — an identity
            // at the failed index would be missed until the next scan.
            if tally.failed_probes > 0 {
                tracing::warn!(
                    "Identity discovery completed with {} unanswered probe(s); an identity at \
                     a failed index may be missing until the next scan",
                    tally.failed_probes
                );
            }
        } else {
            return Err(tally.into_incomplete_error(start_index, identity_index));
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

/// Running bookkeeping for one gap-limit scan, and the verdict it produces.
///
/// A scan answers "which identities does this seed own", and there are three
/// endings, not two: it found some, it confirmed there are none, or it never
/// got an answer. The third used to be reported as the second — a failed probe
/// incremented the same miss counter as an empty index, so a scan that reached
/// no one at all returned "this seed owns no identity". Callers cannot retry
/// what they were told is a definitive answer, so a few seconds of network
/// trouble after restore-from-seed cost a whole session's DashPay state.
///
/// The counters live here, rather than as locals in `discover_inner`, so tests
/// drive the same bookkeeping production does. Asserting on a bare predicate
/// proved too weak: it kept passing while the scan fed it the wrong number.
#[derive(Default)]
struct ScanTally {
    /// Empty-or-unanswered indices since the last sighting. Terminates the
    /// scan at the gap limit.
    consecutive_misses: u32,
    /// Probes that never reached Platform.
    failed_probes: u32,
    /// Every index Platform answered with an identity — including ones the
    /// manager already tracked, which never reach the returned `discovered`
    /// list. A rescan from index 0 (what the app's "Find identities" command
    /// does) re-confirms known identities without adding to `discovered`, so
    /// judging by that list reports a plainly reachable scan as incomplete the
    /// moment any later index fails.
    identities_seen: usize,
    /// Last probe failure, kept typed so callers can inspect the variant
    /// rather than parse a rendered string.
    last_probe_error: Option<dash_sdk::Error>,
}

impl ScanTally {
    /// Whether the scan should probe another index.
    fn should_continue(&self, gap_limit: u32) -> bool {
        self.consecutive_misses < gap_limit
    }

    /// Platform answered with an identity at this index.
    fn record_sighting(&mut self) {
        self.identities_seen += 1;
        self.consecutive_misses = 0;
    }

    /// Platform answered, definitively, that this index holds no identity.
    fn record_miss(&mut self) {
        self.consecutive_misses += 1;
    }

    /// The probe never got an answer. It still advances the miss counter — the
    /// scan has to terminate when the network is down — but it is remembered
    /// separately, because the verdict depends on telling the two apart.
    fn record_failure(&mut self, error: dash_sdk::Error) {
        self.last_probe_error = Some(error);
        self.failed_probes += 1;
        self.consecutive_misses += 1;
    }

    /// Whether the scan's literal result may be reported as-is.
    ///
    /// Emptiness is only trustworthy when every probe was answered. A scan
    /// that saw an identity is trustworthy either way: it reached Platform and
    /// the seed demonstrably owns one, and the caller re-scans later for
    /// anything a failed index hid.
    fn is_trustworthy(&self) -> bool {
        self.identities_seen > 0 || self.failed_probes == 0
    }

    /// The error an untrustworthy scan returns, carrying the last probe
    /// failure as its `source`.
    fn into_incomplete_error(self, start_index: u32, next_index: u32) -> PlatformWalletError {
        PlatformWalletError::IdentityDiscoveryIncomplete {
            start_index,
            probed: next_index.saturating_sub(start_index),
            failed_probes: self.failed_probes,
            // Unreachable by construction: `is_trustworthy` only returns false
            // when `failed_probes > 0`, and every failure records its error.
            // Named rather than papered over with a plausible-looking cause.
            source: Box::new(self.last_probe_error.unwrap_or_else(|| {
                dash_sdk::Error::Generic(
                    "identity discovery reported a failed probe with no recorded error".to_string(),
                )
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::identity_handle::derive_ecdsa_identity_auth_keypair_from_master;
    use super::{breadcrumb_decisions, PlatformWalletError, ScanTally};
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dpp::identity::v0::IdentityV0;
    use dpp::identity::{Identity, IdentityPublicKey, KeyID, KeyType, Purpose, SecurityLevel};
    use dpp::prelude::Identifier;
    use key_wallet::bip32::ExtendedPrivKey;
    use key_wallet::mnemonic::{Language, Mnemonic};
    use key_wallet::Network;
    use std::collections::BTreeMap;

    const TEST_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon \
         abandon abandon abandon abandon abandon about";

    fn test_master() -> ExtendedPrivKey {
        let mnemonic = Mnemonic::from_phrase(TEST_MNEMONIC, Language::English).expect("mnemonic");
        let seed = mnemonic.to_seed("");
        ExtendedPrivKey::new_master(Network::Testnet, &seed).expect("master xpriv")
    }

    fn ecdsa_auth_key(id: KeyID, data: Vec<u8>) -> IdentityPublicKey {
        IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: dpp::platform_value::BinaryData::new(data),
            disabled_at: None,
        })
    }

    fn identity_with_keys(keys: Vec<IdentityPublicKey>) -> Identity {
        let mut map = BTreeMap::new();
        for k in keys {
            map.insert(k.id(), k);
        }
        Identity::V0(IdentityV0 {
            id: Identifier::from([0x42; 32]),
            public_keys: map,
            balance: 0,
            revision: 0,
        })
    }

    /// Every on-chain key re-derivable at `(identity_index, key_id)` earns a
    /// breadcrumb. This is the fix for "imported identity cannot sign":
    /// before, only the MASTER key was breadcrumbed, so the non-master
    /// signing keys had no private material and signing failed.
    #[test]
    fn breadcrumb_decisions_emits_for_every_reproducible_key() {
        let master = test_master();
        let wallet_id = [0xAB; 32];
        let identity_index = 0u32;

        let mut scalars = BTreeMap::new();
        let mut keys = Vec::new();
        for key_id in 0u32..5 {
            let kp = derive_ecdsa_identity_auth_keypair_from_master(
                &master,
                Network::Testnet,
                identity_index,
                key_id,
            )
            .expect("derive candidate");
            keys.push(ecdsa_auth_key(key_id, kp.public_key.to_vec()));
            scalars.insert(key_id, kp.private_key);
        }
        let identity = identity_with_keys(keys);

        let decisions = breadcrumb_decisions(
            &identity,
            identity_index,
            wallet_id,
            Network::Testnet,
            &scalars,
        );
        assert_eq!(decisions.len(), 5);
        for decision in &decisions {
            assert_eq!(
                decision.breadcrumb,
                Some((wallet_id, identity_index, decision.key.id())),
                "key {} must be breadcrumbed",
                decision.key.id()
            );
        }
    }

    /// A published key whose bytes do NOT match the candidate at its `key_id`
    /// (a foreign / non-wallet key) stays watch-only — verify-before-emit
    /// never hands a wrong breadcrumb that would store an unauthorized key.
    #[test]
    fn breadcrumb_decisions_leaves_non_reproducible_key_watch_only() {
        let master = test_master();
        let wallet_id = [0xAB; 32];

        let kp0 = derive_ecdsa_identity_auth_keypair_from_master(&master, Network::Testnet, 0, 0)
            .expect("derive");
        // A foreign pubkey for key_id 1 (derived at an unrelated slot) — the
        // seed candidate at (0, 1) won't reproduce it.
        let kp_foreign =
            derive_ecdsa_identity_auth_keypair_from_master(&master, Network::Testnet, 9, 9)
                .expect("derive");
        let kp1_candidate =
            derive_ecdsa_identity_auth_keypair_from_master(&master, Network::Testnet, 0, 1)
                .expect("derive");

        let mut scalars = BTreeMap::new();
        scalars.insert(0u32, kp0.private_key);
        scalars.insert(1u32, kp1_candidate.private_key);

        let identity = identity_with_keys(vec![
            ecdsa_auth_key(0, kp0.public_key.to_vec()),
            ecdsa_auth_key(1, kp_foreign.public_key.to_vec()),
        ]);
        let decisions = breadcrumb_decisions(&identity, 0, wallet_id, Network::Testnet, &scalars);
        let by_id: BTreeMap<KeyID, &crate::changeset::KeyWithBreadcrumb> =
            decisions.iter().map(|d| (d.key.id(), d)).collect();
        assert_eq!(
            by_id[&0].breadcrumb,
            Some((wallet_id, 0, 0)),
            "reproducible key breadcrumbed"
        );
        assert_eq!(by_id[&1].breadcrumb, None, "foreign key left watch-only");
    }

    /// An on-chain key typed `ECDSA_HASH160` (data = the 20-byte hash of
    /// the pubkey) is matched by its hash — `validate_private_key_bytes`
    /// covers both ECDSA representations. (Uncompressed 65-byte ECDSA keys
    /// are deliberately NOT tested: the wallet only ever derives the
    /// compressed pubkey form, so an uncompressed externally-registered key
    /// is simply not wallet-derivable and stays watch-only by graceful
    /// non-match — not because the protocol forbids such keys.)
    #[test]
    fn breadcrumb_decisions_matches_hash160_key() {
        use dpp::util::hash::ripemd160_sha256;

        let master = test_master();
        let wallet_id = [0xAB; 32];
        let kp = derive_ecdsa_identity_auth_keypair_from_master(&master, Network::Testnet, 0, 0)
            .expect("derive");
        let hash = ripemd160_sha256(&kp.public_key).to_vec();

        let key = IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: 0,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            key_type: KeyType::ECDSA_HASH160,
            read_only: false,
            data: dpp::platform_value::BinaryData::new(hash),
            disabled_at: None,
        });

        let mut scalars = BTreeMap::new();
        scalars.insert(0u32, kp.private_key);
        let identity = identity_with_keys(vec![key]);

        let decisions = breadcrumb_decisions(&identity, 0, wallet_id, Network::Testnet, &scalars);
        assert_eq!(
            decisions[0].breadcrumb,
            Some((wallet_id, 0, 0)),
            "HASH160 key must verify by hash"
        );
    }

    /// Decide key ownership from the candidate's compressed PUBLIC key alone,
    /// reproducing `IdentityPublicKey::validate_private_key_bytes` without the
    /// secret scalar: an `ECDSA_SECP256K1` key matches when the on-chain data
    /// equals the 33-byte compressed pubkey; an `ECDSA_HASH160` key matches
    /// when `ripemd160_sha256` of that pubkey equals the on-chain 20-byte
    /// hash. Any other key type (BLS/EdDSA, or an uncompressed ECDSA key the
    /// wallet never derives) never matches.
    fn pubkey_reproduces(on_chain_key: &IdentityPublicKey, candidate_pubkey: &[u8; 33]) -> bool {
        use dpp::util::hash::ripemd160_sha256;
        match on_chain_key.key_type() {
            KeyType::ECDSA_SECP256K1 => {
                on_chain_key.data().as_slice() == candidate_pubkey.as_slice()
            }
            KeyType::ECDSA_HASH160 => {
                ripemd160_sha256(candidate_pubkey.as_slice()).as_slice()
                    == on_chain_key.data().as_slice()
            }
            _ => false,
        }
    }

    /// The ownership decision derived from the candidate's PUBLIC key is
    /// byte-for-byte identical to the one `validate_private_key_bytes` derives
    /// from the secret scalar — for a matching `ECDSA_SECP256K1` key, a
    /// matching `ECDSA_HASH160` key, and a foreign (non-reproducible) key.
    /// This pins the equivalence that lets discovery verify ownership without
    /// ever materializing the scalar.
    #[test]
    fn pubkey_verify_matches_scalar_verify_for_every_key() {
        use dpp::identity::identity_public_key::methods::hash::IdentityPublicKeyHashMethodsV0;
        use dpp::util::hash::ripemd160_sha256;

        let master = test_master();
        let network = Network::Testnet;
        let identity_index = 0u32;

        // key_id 1 derives the pubkey behind the on-chain HASH160 key; key_id 9'/9
        // is an unrelated slot standing in for a FOREIGN key the wallet can't own.
        let kp1 =
            derive_ecdsa_identity_auth_keypair_from_master(&master, network, 0, 1).expect("derive");
        let kp_foreign =
            derive_ecdsa_identity_auth_keypair_from_master(&master, network, 9, 9).expect("derive");
        let kp0 =
            derive_ecdsa_identity_auth_keypair_from_master(&master, network, 0, 0).expect("derive");

        let hash160_key = IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: 1,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            key_type: KeyType::ECDSA_HASH160,
            read_only: false,
            data: dpp::platform_value::BinaryData::new(ripemd160_sha256(&kp1.public_key).to_vec()),
            disabled_at: None,
        });

        // (on-chain key, the (0, key_id) slot whose candidate is checked, expected).
        let cases: Vec<(IdentityPublicKey, u32, bool)> = vec![
            (ecdsa_auth_key(0, kp0.public_key.to_vec()), 0, true),
            (hash160_key, 1, true),
            (ecdsa_auth_key(2, kp_foreign.public_key.to_vec()), 2, false),
        ];

        for (on_chain_key, key_id, expected) in &cases {
            let candidate = derive_ecdsa_identity_auth_keypair_from_master(
                &master,
                network,
                identity_index,
                *key_id,
            )
            .expect("derive candidate");

            let scalar_decision = on_chain_key
                .validate_private_key_bytes(&candidate.private_key, network)
                .unwrap_or(false);
            let pubkey_decision = pubkey_reproduces(on_chain_key, &candidate.public_key);

            assert_eq!(
                scalar_decision, pubkey_decision,
                "pubkey-verify diverged from scalar-verify for key {key_id}"
            );
            assert_eq!(
                pubkey_decision, *expected,
                "unexpected ownership decision for key {key_id}"
            );
        }
    }

    fn probe_failure() -> dash_sdk::Error {
        dash_sdk::Error::Generic("dapi unreachable".to_string())
    }

    /// Drive a whole scan through the same `ScanTally` production uses, one
    /// probe outcome per element, stopping at the gap limit exactly as
    /// `discover_inner`'s loop does. `Some(())` is a sighting, `None` a
    /// confirmed miss, `Err` an unanswered probe.
    fn run_scan(
        gap_limit: u32,
        outcomes: impl IntoIterator<Item = Result<Option<()>, ()>>,
    ) -> ScanTally {
        let mut tally = ScanTally::default();
        for outcome in outcomes {
            if !tally.should_continue(gap_limit) {
                break;
            }
            match outcome {
                Ok(Some(())) => tally.record_sighting(),
                Ok(None) => tally.record_miss(),
                Err(()) => tally.record_failure(probe_failure()),
            }
        }
        tally
    }

    /// The regression this whole distinction exists for: a scan that found
    /// nothing because it could not reach Platform must not be reported as a
    /// scan that found nothing because there is nothing.
    #[test]
    fn scan_that_never_reached_platform_is_not_trustworthy() {
        let tally = run_scan(5, [Err(()), Err(()), Err(()), Err(()), Err(())]);
        assert_eq!(tally.failed_probes, 5);
        assert_eq!(tally.identities_seen, 0);
        assert!(!tally.is_trustworthy());
    }

    /// The genuinely-empty wallet: every probe answered, all of them "none".
    #[test]
    fn scan_with_every_probe_answered_empty_is_trustworthy() {
        let tally = run_scan(5, [Ok(None), Ok(None), Ok(None), Ok(None), Ok(None)]);
        assert_eq!(tally.failed_probes, 0);
        assert!(tally.is_trustworthy());
    }

    /// Discovered identities are already persisted, so a partial failure does
    /// not discard them — it only means a later scan may find more.
    #[test]
    fn scan_that_found_something_is_trustworthy_despite_failures() {
        let tally = run_scan(
            5,
            [Ok(Some(())), Err(()), Ok(None), Err(()), Ok(None), Ok(None)],
        );
        assert_eq!(tally.identities_seen, 1);
        assert_eq!(tally.failed_probes, 2);
        assert!(tally.is_trustworthy());
    }

    /// A rescan re-confirms identities the manager already holds, which never
    /// reach the returned `discovered` list. Judging by that list called a
    /// scan that plainly reached Platform "incomplete" as soon as a later
    /// index failed — turning the app's "Find identities" command into an
    /// error for a wallet whose identity was found successfully.
    ///
    /// `discovered` stays empty for the whole scan here, which is exactly the
    /// value the fixed wiring must NOT be judging by.
    #[test]
    fn rescan_of_a_known_identity_is_trustworthy_despite_a_later_failure() {
        let tally = run_scan(
            5,
            [Ok(Some(())), Err(()), Err(()), Err(()), Err(()), Err(())],
        );
        assert_eq!(tally.identities_seen, 1, "the known identity was seen");
        assert!(tally.failed_probes > 0);
        assert!(tally.is_trustworthy());
    }

    /// A sighting resets the gap, so an identity past a run of empties is
    /// still reachable — and the scan does not stop early.
    #[test]
    fn a_sighting_resets_the_consecutive_miss_run() {
        let tally = run_scan(3, [Ok(None), Ok(None), Ok(Some(())), Ok(None), Ok(None)]);
        assert_eq!(tally.identities_seen, 1);
        assert_eq!(tally.consecutive_misses, 2);
        assert!(tally.should_continue(3));
    }

    /// An unanswered probe still has to stop the scan, or an offline device
    /// would walk indices forever.
    #[test]
    fn unanswered_probes_still_terminate_the_scan() {
        let tally = run_scan(3, [Err(()), Err(()), Err(()), Err(()), Err(())]);
        assert!(!tally.should_continue(3));
        assert_eq!(
            tally.failed_probes, 3,
            "stopped at the gap limit, not after 5"
        );
    }

    /// The failure reaches the caller typed, not flattened to a string.
    #[test]
    fn incomplete_error_carries_the_probe_failure_as_its_source() {
        use std::error::Error as _;

        let tally = run_scan(2, [Err(()), Err(())]);
        let error = tally.into_incomplete_error(0, 2);
        match &error {
            PlatformWalletError::IdentityDiscoveryIncomplete {
                start_index,
                probed,
                failed_probes,
                source,
            } => {
                assert_eq!(*start_index, 0);
                assert_eq!(*probed, 2);
                assert_eq!(*failed_probes, 2);
                assert!(matches!(**source, dash_sdk::Error::Generic(_)));
            }
            other => panic!("expected IdentityDiscoveryIncomplete, got {other:?}"),
        }
        assert!(error.source().is_some(), "source must survive for callers");
        assert!(error.to_string().contains("dapi unreachable"));
    }
}
