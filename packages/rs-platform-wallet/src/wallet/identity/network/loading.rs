//! Load identities by HD index / DPNS name, refresh state.

use dpp::identity::accessors::IdentityGettersV0;

use dpp::identity::Identity;
use dpp::prelude::Identifier;
use key_wallet::bip32::ExtendedPrivKey;

use crate::error::PlatformWalletError;

use super::*;

/// Where the MASTER auth pubkey hash that probes a single identity
/// index comes from. The two `load_identity_by_index*` entry points
/// differ *only* in this: everything else (Platform unique-hash lookup,
/// identity folding, DPNS enrichment) is shared in
/// [`IdentityWallet::load_identity_by_index_inner`].
///
/// This mirrors the discovery scan's `KeyHashSource` (in `discovery.rs`)
/// but is kept loading-local on purpose: that enum is module-private and
/// the two-variant shape is small enough that duplicating it beats
/// widening its visibility just to share it.
///
/// - [`LoadKeyHashSource::ResidentWallet`] derives from the in-process
///   `Wallet` key material (the historical path). Only valid for wallets
///   that actually hold a private key in memory
///   (`WalletType::Mnemonic` / `Seed` / `ExtendedPrivKey`).
/// - [`LoadKeyHashSource::Master`] derives from a master
///   `ExtendedPrivKey` the caller resolved from the wallet's mnemonic on
///   demand. This is the path the iOS Keychain-backed
///   `WalletType::ExternalSignable` shape uses: its seed lives outside
///   the in-process wallet manager, so the resident-wallet derive would
///   fail with `External signable wallet has no private key`.
enum LoadKeyHashSource<'a> {
    /// Derive the probe hash from the in-memory wallet under a read lock
    /// on the shared `WalletManager`.
    ResidentWallet,
    /// Derive the probe hash from this master xpriv (pure, no lock).
    Master(&'a ExtendedPrivKey),
}

/// Where the load probe's MASTER auth pubkey hash is derived from,
/// resolved to a concrete `Wallet` / `master` once the read lock has
/// pinned the resident key material. This is the lock-free,
/// `IdentityWallet`-free core of the key-hash-source selection so it can
/// be exercised by a real unit test (the surrounding async Platform
/// fetch obviously cannot run in one).
///
/// - [`ResolvedLoadKeyHashSource::ResidentWallet`] carries the in-memory
///   `Wallet`; the derive reads its resident key material and fails for
///   `WalletType::ExternalSignable` with
///   `External signable wallet has no private key`.
/// - [`ResolvedLoadKeyHashSource::Master`] carries a resolved master
///   xpriv; the derive is a pure secp256k1 pass that works for
///   `ExternalSignable` wallets too.
enum ResolvedLoadKeyHashSource<'a> {
    ResidentWallet(&'a key_wallet::wallet::Wallet),
    Master(&'a ExtendedPrivKey),
}

/// Derive the 20-byte MASTER auth pubkey hash that probes
/// `identity_index` at [`MASTER_KEY_INDEX`] on `network`, picking the
/// derive path from `source`.
///
/// Factored out of [`IdentityWallet::load_identity_by_index_inner`] so
/// the resident-wallet-vs-master branch is a single pure function: the
/// inner method resolves the read lock + network, then calls this; the
/// unit tests call it directly. The resident path fails for
/// `WalletType::ExternalSignable` (the seed lives outside the in-process
/// wallet — iOS Keychain), which is the exact bug the master path fixes.
fn derive_load_probe_hash(
    source: ResolvedLoadKeyHashSource<'_>,
    network: key_wallet::Network,
    identity_index: u32,
) -> Result<[u8; 20], PlatformWalletError> {
    use super::identity_handle::{
        derive_identity_auth_key_hash, derive_identity_auth_key_hash_from_master, MASTER_KEY_INDEX,
    };

    match source {
        ResolvedLoadKeyHashSource::ResidentWallet(wallet) => {
            derive_identity_auth_key_hash(wallet, network, identity_index, MASTER_KEY_INDEX)
        }
        ResolvedLoadKeyHashSource::Master(master) => derive_identity_auth_key_hash_from_master(
            master,
            network,
            identity_index,
            MASTER_KEY_INDEX,
        ),
    }
}

// ---------------------------------------------------------------------------
// Identity loading & refresh
// ---------------------------------------------------------------------------

impl IdentityWallet {
    /// Load a single identity by its BIP-9 HD identity index.
    ///
    /// Derives the MASTER authentication key hash at the given
    /// `identity_index` (key_index [`MASTER_KEY_INDEX`]) from the
    /// in-process wallet and queries Platform for an identity registered
    /// with that key. If found the identity is added to the local
    /// [`IdentityManager`] with its derivation-path key storage, status
    /// set to `Active`, DPNS names queried, and wallet seed hash
    /// recorded.
    ///
    /// Returns the identity if one was found, or `None` if no identity is
    /// registered at that index.
    ///
    /// This is the historical resident-wallet path: it derives from the
    /// in-memory `Wallet` key material, so it is valid only for wallets
    /// that hold a private key in process
    /// (`WalletType::Mnemonic` / `Seed` / `ExtendedPrivKey`). For the iOS
    /// Keychain-backed `WalletType::ExternalSignable` shape — whose seed
    /// lives outside the in-process wallet manager — this would fail with
    /// `External signable wallet has no private key`; use
    /// [`Self::load_identity_by_index_from_master`] instead.
    pub async fn load_identity_by_index(
        &self,
        identity_index: u32,
    ) -> Result<Option<Identity>, PlatformWalletError> {
        self.load_identity_by_index_inner(identity_index, LoadKeyHashSource::ResidentWallet)
            .await
    }

    /// Master-xpriv variant of [`Self::load_identity_by_index`]: run the
    /// identical Platform unique-hash lookup / identity-folding /
    /// DPNS-enrichment logic, but derive the probe's MASTER auth pubkey
    /// hash from the supplied master `ExtendedPrivKey` instead of from
    /// the in-memory wallet.
    ///
    /// This is the path the iOS Keychain-backed
    /// `WalletType::ExternalSignable` wallets must take: their seed lives
    /// outside the in-process wallet manager, so
    /// [`Self::load_identity_by_index`]'s resident-wallet derive fails
    /// with `External signable wallet has no private key`. The caller
    /// resolves the wallet's mnemonic into `master` on demand (see the
    /// FFI resolver path) and hands it in here; the derivation goes
    /// through the same [`derive_identity_auth_key_hash_from_master`]
    /// the registration / discovery paths use, so a load derives exactly
    /// the key material a key-resident wallet would.
    ///
    /// `master` must be the BIP-32 master node for this wallet on its
    /// network (`ExtendedPrivKey::new_master(network, mnemonic.to_seed(""))`),
    /// same as [`Self::discover_from_master`].
    pub async fn load_identity_by_index_from_master(
        &self,
        identity_index: u32,
        master: &ExtendedPrivKey,
    ) -> Result<Option<Identity>, PlatformWalletError> {
        self.load_identity_by_index_inner(identity_index, LoadKeyHashSource::Master(master))
            .await
    }

    /// Shared body for [`Self::load_identity_by_index`] and
    /// [`Self::load_identity_by_index_from_master`]. The only thing the
    /// two callers vary is `source`, which decides how the probe's
    /// MASTER auth pubkey hash is derived (in-memory wallet under a read
    /// lock, vs. a resolved master xpriv). Everything downstream — the
    /// Platform unique-hash lookup, identity folding, derivation
    /// breadcrumb, and DPNS enrichment — is identical, so it lives here
    /// once.
    async fn load_identity_by_index_inner(
        &self,
        identity_index: u32,
        source: LoadKeyHashSource<'_>,
    ) -> Result<Option<Identity>, PlatformWalletError> {
        use crate::wallet::identity::state::managed_identity::key_storage::DpnsNameInfo;
        use crate::wallet::identity::state::managed_identity::key_storage::IdentityStatus;
        use dash_sdk::platform::types::identity::PublicKeyHash;
        use dash_sdk::platform::Fetch;

        let wallet_id = self.wallet_id;

        // Derive the MASTER auth pubkey hash for this identity index from
        // whichever source the caller picked. The read lock is only
        // needed for the wallet-internal derive (it reads the resident
        // key material); the master derive is a pure, lock-free
        // secp256k1 pass. The actual branch lives in the pure,
        // unit-testable `derive_load_probe_hash` helper, which probes
        // `MASTER_KEY_INDEX` (not a hardcoded `0`) so loading and the
        // discovery scan visibly target the same slot.
        let (key_hash_array, network) = {
            let wm = self.wallet_manager.read().await;
            let wallet = wm.get_wallet(&self.wallet_id).ok_or_else(|| {
                crate::error::PlatformWalletError::WalletNotFound(
                    "Wallet not found in wallet manager".to_string(),
                )
            })?;
            let network = wallet.network;
            // For the resident-wallet path the derive borrows `wallet`'s
            // key material; for the master path the `wallet` borrow is
            // only used to read the (non-secret) network tag above and
            // the resolved master does the work. Both run under the same
            // read lock — no extra lock juggling.
            let resolved = match source {
                LoadKeyHashSource::ResidentWallet => {
                    ResolvedLoadKeyHashSource::ResidentWallet(wallet)
                }
                LoadKeyHashSource::Master(master) => ResolvedLoadKeyHashSource::Master(master),
            };
            (
                derive_load_probe_hash(resolved, network, identity_index)?,
                network,
            )
        };

        // Query Platform for an identity registered with this key hash.
        let identity = match Identity::fetch(&self.sdk, PublicKeyHash(key_hash_array)).await {
            Ok(Some(identity)) => identity,
            Ok(None) => return Ok(None),
            Err(e) => {
                return Err(PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to fetch identity at index {}: {}",
                    identity_index, e
                )));
            }
        };

        let identity_id = identity.id();

        // Derive + verify a candidate for EVERY on-chain key (shared with
        // the discovery scan) so the imported identity can sign with all its
        // re-derivable keys, not just MASTER. Runs before the write lock.
        let key_decisions = self
            .derive_key_breadcrumbs(
                &identity,
                identity_index,
                network,
                match source {
                    LoadKeyHashSource::Master(master) => Some(master),
                    LoadKeyHashSource::ResidentWallet => None,
                },
            )
            .await?;

        // Add the identity to the manager and enrich it.
        {
            let mut wm = self.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&self.wallet_id).ok_or_else(|| {
                crate::error::PlatformWalletError::WalletNotFound(
                    "Wallet info not found in wallet manager".to_string(),
                )
            })?;
            if info.identity_manager.identity(&identity_id).is_none() {
                info.identity_manager.add_identity(
                    identity.clone(),
                    identity_index,
                    wallet_id,
                    &self.persister,
                )?;
            }

            if let Some(managed) = info.identity_manager.managed_identity_mut(&identity_id) {
                managed.set_status(IdentityStatus::Active, &self.persister);
                managed.wallet_id = Some(wallet_id);
                // Breadcrumbs for every re-derivable key (was MASTER-only). A
                // failed persist would silently leave the identity watch-only
                // after restart, so surface it rather than swallow.
                managed
                    .add_keys(key_decisions, &self.persister)
                    .map_err(|e| {
                        PlatformWalletError::Persistence(format!(
                            "identity keys not persisted during load: {e}"
                        ))
                    })?;
            }
        }

        // Query DPNS names for the discovered identity.
        match self
            .sdk
            .get_dpns_usernames_by_identity(identity_id, None)
            .await
        {
            Ok(usernames) => {
                let mut wm = self.wallet_manager.write().await;
                let info = wm.get_wallet_info_mut(&self.wallet_id).ok_or_else(|| {
                    crate::error::PlatformWalletError::WalletNotFound(
                        "Wallet info not found in wallet manager".to_string(),
                    )
                })?;
                if let Some(managed) = info.identity_manager.managed_identity_mut(&identity_id) {
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

        Ok(Some(identity))
    }

    /// Refresh an identity that is already in the local manager by
    /// re-fetching it from Platform.
    ///
    /// The identity must already exist in the [`IdentityManager`]. Its
    /// on-chain state (keys, balance, revision) is replaced with the latest
    /// version from Platform and the status is set to `Active`.
    ///
    /// Returns the refreshed identity.
    ///
    /// # Errors
    ///
    /// * [`PlatformWalletError::IdentityNotFound`] if the identity is not in
    ///   the manager.
    /// * An error if Platform does not return the identity (e.g. it was
    ///   deleted).
    pub async fn refresh_identity(
        &self,
        identity_id: &Identifier,
    ) -> Result<Identity, PlatformWalletError> {
        use crate::wallet::identity::state::managed_identity::key_storage::IdentityStatus;
        use dash_sdk::platform::Fetch;

        // Verify identity exists in the manager.
        {
            let wm = self.wallet_manager.read().await;
            let info = wm.get_wallet_info(&self.wallet_id).ok_or_else(|| {
                crate::error::PlatformWalletError::WalletNotFound(
                    "Wallet info not found in wallet manager".to_string(),
                )
            })?;
            if info.identity_manager.identity(identity_id).is_none() {
                return Err(PlatformWalletError::IdentityNotFound(*identity_id));
            }
        }

        // Fetch the latest state from Platform.
        let identity = Identity::fetch(&self.sdk, *identity_id)
            .await
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to fetch identity {} from Platform: {}",
                    identity_id, e
                ))
            })?
            .ok_or_else(|| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Identity {} not found on Platform",
                    identity_id
                ))
            })?;

        // Update the managed identity.
        {
            let mut wm = self.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&self.wallet_id).ok_or_else(|| {
                crate::error::PlatformWalletError::WalletNotFound(
                    "Wallet info not found in wallet manager".to_string(),
                )
            })?;
            if let Some(managed) = info.identity_manager.managed_identity_mut(identity_id) {
                managed.identity = identity.clone();
                managed.set_status(IdentityStatus::Active, &self.persister);
            }
        }

        Ok(identity)
    }

    /// Refresh an identity using an externally-provided identity ID.
    ///
    /// Unlike [`refresh_identity`](Self::refresh_identity), this method does
    /// **not** look up or update the internal `IdentityManager`. It simply
    /// fetches the latest identity from Platform and returns it. This is
    /// useful when the caller manages identities outside of the
    /// platform-wallet `IdentityManager` (e.g. evo-tool's
    /// `QualifiedIdentity`).
    ///
    /// Returns the refreshed identity, or an error if not found on Platform.
    pub async fn refresh_identity_with_signer(
        &self,
        identity_id: &Identifier,
    ) -> Result<Identity, dash_sdk::Error> {
        use dash_sdk::platform::Fetch;

        Identity::fetch(&self.sdk, *identity_id)
            .await?
            .ok_or_else(|| {
                dash_sdk::Error::Generic(format!("Identity {} not found on Platform", identity_id))
            })
    }

    /// Refresh DPNS names for all identities in the manager.
    ///
    /// Iterates every identity in the [`IdentityManager`], queries Platform
    /// for its current DPNS usernames, and replaces the stored
    /// `dpns_names` list with the fresh results.
    pub async fn refresh_dpns_names(&self) -> Result<(), PlatformWalletError> {
        use crate::wallet::identity::state::managed_identity::key_storage::DpnsNameInfo;

        // Collect identity IDs so we don't hold the lock during network calls.
        let identity_ids: Vec<Identifier> = {
            let wm = self.wallet_manager.read().await;
            let info = wm.get_wallet_info(&self.wallet_id).ok_or_else(|| {
                crate::error::PlatformWalletError::WalletNotFound(
                    "Wallet info not found in wallet manager".to_string(),
                )
            })?;
            info.identity_manager
                .all_identities()
                .into_iter()
                .map(|i| i.id())
                .collect()
        };

        for identity_id in identity_ids {
            match self
                .sdk
                .get_dpns_usernames_by_identity(identity_id, None)
                .await
            {
                Ok(usernames) => {
                    let mut wm = self.wallet_manager.write().await;
                    let info = wm.get_wallet_info_mut(&self.wallet_id).ok_or_else(|| {
                        crate::error::PlatformWalletError::WalletNotFound(
                            "Wallet info not found in wallet manager".to_string(),
                        )
                    })?;
                    if let Some(managed) = info.identity_manager.managed_identity_mut(&identity_id)
                    {
                        managed.dpns_names = usernames
                            .into_iter()
                            .map(|u| DpnsNameInfo {
                                label: u.label,
                                acquired_at: None,
                            })
                            .collect();
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

        Ok(())
    }

    /// Load an identity by resolving a DPNS name.
    ///
    /// Resolves the given `name` to an identity identifier via
    /// [`resolve_name`](Self::resolve_name), fetches the identity from
    /// Platform, and adds it to the **watched** identities collection (since
    /// the wallet derivation index is unknown for externally-resolved names
    /// and we cannot sign on their behalf).
    ///
    /// Returns the identity if the name resolves successfully, or `None` if
    /// the name does not exist.
    pub async fn load_identity_by_dpns_name(
        &self,
        name: &str,
    ) -> Result<Option<Identity>, PlatformWalletError> {
        use dash_sdk::platform::Fetch;

        // Resolve the DPNS name to an identity ID.
        let identity_id = match self.resolve_name(name).await? {
            Some(id) => id,
            None => return Ok(None),
        };

        // Fetch the identity from Platform.
        let identity = Identity::fetch(&self.sdk, identity_id)
            .await
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to fetch identity {} for DPNS name '{}': {}",
                    identity_id, name, e
                ))
            })?
            .ok_or_else(|| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "DPNS name '{}' resolved to identity {} but it was not found on Platform",
                    name, identity_id
                ))
            })?;

        // Add to the out-of-wallet bucket (observed read-only — we
        // don't know the wallet index and cannot sign).
        {
            let mut wm = self.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&self.wallet_id).ok_or_else(|| {
                crate::error::PlatformWalletError::WalletNotFound(
                    "Wallet info not found in wallet manager".to_string(),
                )
            })?;
            info.identity_manager
                .add_out_of_wallet_identity(identity.clone(), &self.persister)?;
        }

        Ok(Some(identity))
    }
}

#[cfg(test)]
mod tests {
    use super::super::identity_handle::{
        derive_identity_auth_key_hash, derive_identity_auth_key_hash_from_master, MASTER_KEY_INDEX,
    };
    use super::{derive_load_probe_hash, ResolvedLoadKeyHashSource};
    use key_wallet::bip32::ExtendedPrivKey;
    use key_wallet::mnemonic::{Language, Mnemonic};
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;
    use key_wallet::wallet::Wallet;
    use key_wallet::Network;

    /// English BIP-39 test vector (all-zero entropy). Same fixture the
    /// FFI-side derive tests and the `identity_handle` tests use, so the
    /// derivations here can be cross-checked against those if a
    /// regression ever appears on one side only.
    const TEST_MNEMONIC: &str =
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    /// Build a key-resident `WalletType::Mnemonic` wallet on `network`
    /// from [`TEST_MNEMONIC`]. `WalletAccountCreationOptions::None`
    /// skips the BLS/EdDSA provider accounts the identity-auth derive
    /// never touches — it walks the master xpriv, not the per-account
    /// collection, so no accounts are needed.
    fn mnemonic_wallet(network: Network) -> Wallet {
        let mnemonic = Mnemonic::from_phrase(TEST_MNEMONIC, Language::English)
            .expect("valid English test mnemonic");
        Wallet::from_mnemonic(mnemonic, network, WalletAccountCreationOptions::None)
            .expect("from_mnemonic should build a Mnemonic wallet")
    }

    /// The BIP-32 master node for [`TEST_MNEMONIC`] on `network` — the
    /// same node `derive_extended_private_key` reconstructs internally.
    fn master_for(network: Network) -> ExtendedPrivKey {
        let mnemonic = Mnemonic::from_phrase(TEST_MNEMONIC, Language::English)
            .expect("valid English test mnemonic");
        let seed = mnemonic.to_seed("");
        ExtendedPrivKey::new_master(network, &seed).expect("master xpriv from test seed")
    }

    /// The loading probe's resident-wallet and master derive sources
    /// must yield the SAME 20-byte hash at the slot loading actually
    /// probes (`MASTER_KEY_INDEX`), for both networks and several
    /// identity indices. This is the core correctness guarantee: a load
    /// driven through the resolved master probes exactly the on-chain
    /// pubkey-hash a key-resident wallet would, so both entry points
    /// resolve the same identity.
    #[test]
    fn load_probe_master_matches_resident_across_networks_and_indices() {
        for network in [Network::Mainnet, Network::Testnet] {
            let wallet = mnemonic_wallet(network);
            let master = master_for(network);

            for identity_index in 0..6u32 {
                let resident = derive_load_probe_hash(
                    ResolvedLoadKeyHashSource::ResidentWallet(&wallet),
                    network,
                    identity_index,
                )
                .expect("resident-wallet load probe should succeed for a Mnemonic wallet");

                let from_master = derive_load_probe_hash(
                    ResolvedLoadKeyHashSource::Master(&master),
                    network,
                    identity_index,
                )
                .expect("master load probe should succeed");

                assert_eq!(
                    resident, from_master,
                    "master-based load probe must equal resident-wallet probe \
                     (network={network:?}, identity_index={identity_index})"
                );

                // And both must equal the slot `MASTER_KEY_INDEX` the
                // discovery scan probes — pinning that loading and
                // discovery target the same key.
                let direct = derive_identity_auth_key_hash(
                    &wallet,
                    network,
                    identity_index,
                    MASTER_KEY_INDEX,
                )
                .expect("direct resident derive at MASTER_KEY_INDEX");
                assert_eq!(
                    resident, direct,
                    "load probe must target MASTER_KEY_INDEX \
                     (network={network:?}, identity_index={identity_index})"
                );
            }
        }
    }

    /// Pin the bug and its fix at loading's probed slot
    /// `(identity_index, MASTER_KEY_INDEX)`: the resident-wallet load
    /// probe on a `WalletType::ExternalSignable` wallet ERRORS with
    /// `External signable wallet has no private key` (its seed lives
    /// outside the in-process wallet — the exact failure
    /// `load_identity_by_index` surfaced for app wallets), while the
    /// master-based load probe SUCCEEDS for the same slot and yields the
    /// byte-identical hash a key-resident `WalletType::Mnemonic` wallet
    /// produces — across both Mainnet and Testnet and a few identity
    /// indices.
    #[test]
    fn external_signable_load_probe_errors_but_master_succeeds() {
        for network in [Network::Mainnet, Network::Testnet] {
            let master = master_for(network);

            for identity_index in 0..3u32 {
                // Reference hash from a key-resident wallet at the
                // probed slot.
                let resident_wallet = mnemonic_wallet(network);
                let expected = derive_load_probe_hash(
                    ResolvedLoadKeyHashSource::ResidentWallet(&resident_wallet),
                    network,
                    identity_index,
                )
                .expect("resident-wallet load probe should succeed");

                // Downgrade a clone to ExternalSignable: same wallet id,
                // but the key material is dropped — exactly the iOS
                // Keychain-backed shape loaded into the in-process
                // `WalletManager`.
                let mut external = mnemonic_wallet(network);
                external.downgrade_to_external_signable();

                let err = derive_load_probe_hash(
                    ResolvedLoadKeyHashSource::ResidentWallet(&external),
                    network,
                    identity_index,
                )
                .expect_err("ExternalSignable wallet has no resident key — probe must error");
                let msg = err.to_string();
                assert!(
                    msg.contains("External signable wallet has no private key"),
                    "error should be the External-signable no-private-key failure, got: {msg}"
                );

                // The master-based load probe succeeds for the same slot
                // and matches the key-resident reference hash — this is
                // the loading fix.
                let from_master = derive_load_probe_hash(
                    ResolvedLoadKeyHashSource::Master(&master),
                    network,
                    identity_index,
                )
                .expect("master load probe must succeed where the resident probe failed");
                assert_eq!(
                    from_master, expected,
                    "master load probe on an ExternalSignable wallet must reproduce the \
                     key-resident hash for the same slot \
                     (network={network:?}, identity_index={identity_index})"
                );

                // Sanity: the master load probe is byte-identical to the
                // `_from_master` helper the discovery scan uses at the
                // same slot — loading and discovery never drift.
                let discovery_hash = derive_identity_auth_key_hash_from_master(
                    &master,
                    network,
                    identity_index,
                    MASTER_KEY_INDEX,
                )
                .expect("discovery-shaped master hash should succeed");
                assert_eq!(
                    from_master, discovery_hash,
                    "load master probe must equal the discovery master probe at the same slot"
                );
            }
        }
    }
}
