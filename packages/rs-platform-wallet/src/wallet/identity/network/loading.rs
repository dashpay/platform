//! Load identities by HD index / DPNS name, refresh state.

use dpp::identity::accessors::IdentityGettersV0;

use dpp::identity::Identity;
use dpp::prelude::Identifier;
use key_wallet::bip32::ExtendedPrivKey;

use crate::error::PlatformWalletError;
use crate::BlockTime;

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
        let (mut identity, block_time) =
            match Identity::fetch_with_metadata(&self.sdk, PublicKeyHash(key_hash_array), None)
                .await
            {
                Ok((Some(identity), metadata)) => (identity, BlockTime::from(metadata)),
                Ok((None, _)) => return Ok(None),
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
            let is_new = info.identity_manager.identity(&identity_id).is_none();
            if is_new {
                // Stored below, with its verified keys, as one changeset.
                info.identity_manager.add_identity_unstored(
                    identity.clone(),
                    identity_index,
                    wallet_id,
                )?;
            }

            if let Some(managed) = info.identity_manager.managed_identity_mut(&identity_id) {
                // Fold the freshly fetched on-chain state into an
                // already-known identity too — the add above only runs for
                // new ones, and without this a re-load kept serving the
                // stale key set (keys added from another device never
                // appeared, and contact requests referencing them failed
                // key-index validation). Same fold `refresh_identity`
                // does: a fetch older than the cache keeps the cache's
                // revision, key set and every disable, and the balance
                // follows the proof's block, not the revision. The keys
                // and the folded scalar snapshot reach the store together.
                managed.status = IdentityStatus::Active;
                managed.wallet_id = Some(wallet_id);
                // Breadcrumbs for every re-derivable key (was MASTER-only). A
                // failed persist would silently leave the identity watch-only
                // after restart, so surface it rather than swallow.
                managed.store_fetched_identity_keys(
                    &identity,
                    key_decisions,
                    block_time,
                    &self.persister,
                )?;
                // The cache as folded.
                identity = managed.identity.clone();
            }
        }

        // Query DPNS names for the discovered identity.
        match self.fetch_owned_dpns_names(identity_id, None).await {
            Ok((names, fetch)) => {
                let mut wm = self.wallet_manager.write().await;
                let info = wm.get_wallet_info_mut(&self.wallet_id).ok_or_else(|| {
                    crate::error::PlatformWalletError::WalletNotFound(
                        "Wallet info not found in wallet manager".to_string(),
                    )
                })?;
                if let Some(managed) = info.identity_manager.managed_identity_mut(&identity_id) {
                    managed.apply_fetched_dpns_names(names, fetch, &self.persister);
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
    /// on-chain state (keys, revision) is replaced with the latest version
    /// from Platform and the status is set to `Active`; a disable the cache
    /// holds stays, and so does a key limit the cache holds higher. The
    /// balance, at any revision, is taken only when the fetch's proof is at a
    /// block above the cache's balance watermark. A fetch at a lower revision
    /// than the cache's was answered before an update the cache already
    /// holds: the cache keeps its revision, key set and every disable, and
    /// takes only a disable or a raised limit the fetch shows
    /// ([`ManagedIdentity::take_fetched_identity`]). The row of every key the
    /// fold adds or changes is stored with the scalar snapshot, without a
    /// derivation breadcrumb (the persisters keep the one they hold, and a
    /// load or discovery gives a new key its own), and so are key rows an
    /// earlier refused store queued; when that store fails, the error
    /// is returned, memory keeps the fold and the rows stay queued for the
    /// identity's next key store (a refresh, load, discovery or landed
    /// update; [`ManagedIdentity::store_fetched_identity`]).
    ///
    /// Returns the identity the cache holds afterwards.
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
        let (identity, metadata) = Identity::fetch_with_metadata(&self.sdk, *identity_id, None)
            .await
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to fetch identity {} from Platform: {}",
                    identity_id, e
                ))
            })?;
        let mut identity = identity.ok_or_else(|| {
            PlatformWalletError::InvalidIdentityData(format!(
                "Identity {} not found on Platform",
                identity_id
            ))
        })?;
        let block_time = BlockTime::from(metadata);

        // Update the managed identity.
        {
            let mut wm = self.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&self.wallet_id).ok_or_else(|| {
                crate::error::PlatformWalletError::WalletNotFound(
                    "Wallet info not found in wallet manager".to_string(),
                )
            })?;
            if let Some(managed) = info.identity_manager.managed_identity_mut(identity_id) {
                managed.status = IdentityStatus::Active;
                managed.store_fetched_identity(&identity, block_time, &self.persister)?;
                // The cache as folded.
                identity = managed.identity.clone();
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
    /// for its current DPNS usernames, and reconciles the stored
    /// `dpns_names` list with them, persisting at most one snapshot per
    /// identity: for an identity the wallet only watches, a complete result
    /// replaces the list (departed names drop out); wallet-owned identities
    /// and a possibly truncated result only add — see
    /// `ManagedIdentity::apply_fetched_dpns_names`.
    pub async fn refresh_dpns_names(&self) -> Result<(), PlatformWalletError> {
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
            match self.fetch_owned_dpns_names(identity_id, None).await {
                Ok((names, fetch)) => {
                    let mut wm = self.wallet_manager.write().await;
                    let info = wm.get_wallet_info_mut(&self.wallet_id).ok_or_else(|| {
                        crate::error::PlatformWalletError::WalletNotFound(
                            "Wallet info not found in wallet manager".to_string(),
                        )
                    })?;
                    if let Some(managed) = info.identity_manager.managed_identity_mut(&identity_id)
                    {
                        managed.apply_fetched_dpns_names(names, fetch, &self.persister);
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
    use super::super::signing_key::tests::{
        identity, wallet_with_signing_keys_on, CapturingStore, FailingStore,
    };
    use super::super::{IdentityDiscoveryOptions, IdentityWallet};
    use super::{derive_load_probe_hash, ResolvedLoadKeyHashSource};
    use crate::error::PlatformWalletError;
    use crate::wallet::persister::NoPlatformPersistence;
    use crate::BlockTime;
    use dash_sdk::platform::types::identity::PublicKeyHash;
    use dpp::identity::accessors::{IdentityGettersV0, IdentitySettersV0};
    use dpp::identity::identity_public_key::accessors::v0::{
        IdentityPublicKeyGettersV0, IdentityPublicKeySettersV0,
    };
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dpp::identity::Identity;
    use dpp::identity::{KeyType, Purpose, SecurityLevel};
    use dpp::prelude::Identifier;
    use key_wallet::bip32::ExtendedPrivKey;
    use key_wallet::mnemonic::Mnemonic;
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;
    use key_wallet::wallet::Wallet;
    use key_wallet::Network;
    use std::sync::Arc;

    /// Platform's answer in the fold tests: keys 1 and 2 live, key 4 (which
    /// the cache never saw), a balance of 777, at `revision`.
    fn answer_at(revision: u64) -> Identity {
        let mut answer = identity(
            Purpose::AUTHENTICATION,
            SecurityLevel::HIGH,
            KeyType::ECDSA_SECP256K1,
        );
        answer.set_revision(revision);
        answer.set_balance(777);
        answer.add_public_key(
            IdentityPublicKeyV0 {
                id: 4,
                purpose: Purpose::AUTHENTICATION,
                security_level: SecurityLevel::MEDIUM,
                key_type: KeyType::ECDSA_SECP256K1,
                data: vec![4; KeyType::ECDSA_SECP256K1.default_size()].into(),
                ..Default::default()
            }
            .into(),
        );
        answer
    }

    /// Put the fixture's cached identity at revision 6, with key 1 disabled
    /// by an update the answer below 6 predates.
    async fn cache_at_6_with_key_1_disabled(wallet: &IdentityWallet) {
        let mut wm = wallet.wallet_manager.write().await;
        let identity = &mut wm
            .get_wallet_info_mut(&wallet.wallet_id)
            .unwrap()
            .identity_manager
            .managed_identity_mut(&Identifier::default())
            .unwrap()
            .identity;
        identity.set_revision(6);
        let mut key_1 = identity.public_keys()[&1].clone();
        key_1.set_disabled_at(9);
        identity.add_public_key(key_1);
    }

    async fn held(wallet: &IdentityWallet) -> Identity {
        let wm = wallet.wallet_manager.read().await;
        wm.get_wallet_info(&wallet.wallet_id)
            .unwrap()
            .identity_manager
            .identity(&Identifier::default())
            .unwrap()
            .identity
            .clone()
    }

    /// A refresh answered before an update the cache already took (the
    /// cache at 6, Platform's answer at 5) keeps the cache's revision, key
    /// set and every disable; an answer at 6 or above replaces the cache but
    /// keeps the disables. The balance follows the proof, not the revision:
    /// the cache holds none, so every answer's balance is taken. Either way
    /// the call returns what the cache holds afterwards.
    #[tokio::test]
    async fn should_not_take_the_cache_back_with_an_older_refresh() {
        let identity_id = Identifier::default();
        for (answered, settled) in [(5, 6), (6, 6), (7, 7)] {
            let mut sdk = dash_sdk::SdkBuilder::new_mock().build().unwrap();
            sdk.mock()
                .expect_fetch(identity_id, Some(answer_at(answered)))
                .await
                .unwrap();
            let wallet = wallet_with_signing_keys_on(sdk, Arc::new(NoPlatformPersistence)).await;
            let wallet = wallet.identity();
            cache_at_6_with_key_1_disabled(wallet).await;
            let cached_balance = held(wallet).await.balance();
            assert_ne!(cached_balance, 777);

            let returned = wallet.refresh_identity(&identity_id).await.unwrap();
            let held = held(wallet).await;
            assert_eq!(returned, held, "answered at {answered}");
            assert_eq!(held.revision(), settled, "answered at {answered}");
            let older = answered < 6;
            assert_eq!(held.balance(), 777, "answered at {answered}");
            assert!(
                held.public_keys()[&1].is_disabled(),
                "answered at {answered}"
            );
            assert_eq!(
                held.public_keys().contains_key(&4),
                !older,
                "answered at {answered}"
            );
        }
    }

    /// The same fold through a load by index, in memory, in the returned
    /// identity and in the key rows it stores, plus the identity's revision.
    #[tokio::test]
    async fn should_keep_a_newer_cache_when_a_load_answers_older() {
        // The probe the loader sends for index 0 of the fixture's seed.
        let master = ExtendedPrivKey::new_master(Network::Testnet, &[7; 64]).unwrap();
        let probe = derive_load_probe_hash(
            ResolvedLoadKeyHashSource::Master(&master),
            Network::Testnet,
            0,
        )
        .unwrap();

        for answered in [5, 6] {
            let mut sdk = dash_sdk::SdkBuilder::new_mock().build().unwrap();
            sdk.mock()
                .expect_fetch(PublicKeyHash(probe), Some(answer_at(answered)))
                .await
                .unwrap();
            let store = Arc::new(CapturingStore::default());
            let wallet = wallet_with_signing_keys_on(sdk, store.clone()).await;
            let wallet = wallet.identity();
            cache_at_6_with_key_1_disabled(wallet).await;
            let cached_balance = held(wallet).await.balance();
            assert_ne!(cached_balance, 777);
            store.clear();

            let returned = wallet
                .load_identity_by_index_from_master(0, &master)
                .await
                .unwrap()
                .unwrap();
            let held = held(wallet).await;
            assert_eq!(returned, held, "answered at {answered}");
            assert_eq!(held.revision(), 6, "answered at {answered}");
            assert_eq!(store.stored_revision(Identifier::default()), Some(6));
            let older = answered < 6;
            assert_eq!(held.balance(), 777, "answered at {answered}");
            assert!(
                held.public_keys()[&1].is_disabled(),
                "answered at {answered}"
            );
            assert_eq!(
                held.public_keys().contains_key(&4),
                !older,
                "answered at {answered}"
            );

            let rows = store.key_rows();
            assert!(!rows.is_empty(), "answered at {answered}");
            for row in &rows {
                assert_eq!(
                    row.public_key,
                    held.public_keys()[&row.key_id],
                    "answered at {answered}"
                );
            }
            assert_eq!(
                rows.iter().any(|row| row.key_id == 4),
                !older,
                "answered at {answered}"
            );
        }
    }

    /// A load whose store fails returns the failure and queues its key rows,
    /// and a refresh, which stores no rows of its own
    /// for keys memory already holds, stores them.
    #[tokio::test]
    async fn should_store_a_failed_load_s_key_rows_with_the_next_refresh() {
        let master = ExtendedPrivKey::new_master(Network::Testnet, &[7; 64]).unwrap();
        let probe = derive_load_probe_hash(
            ResolvedLoadKeyHashSource::Master(&master),
            Network::Testnet,
            0,
        )
        .unwrap();
        let mut answer = answer_at(7);
        let mut key_2 = answer.public_keys()[&2].clone();
        key_2.set_disabled_at(4);
        answer.add_public_key(key_2);
        let mut sdk = dash_sdk::SdkBuilder::new_mock().build().unwrap();
        sdk.mock()
            .expect_fetch(PublicKeyHash(probe), Some(answer.clone()))
            .await
            .unwrap();
        sdk.mock()
            .expect_fetch(Identifier::default(), Some(answer))
            .await
            .unwrap();
        let store = Arc::new(FailingStore::default());
        let wallet = wallet_with_signing_keys_on(sdk, store.clone()).await;
        let wallet = wallet.identity();
        cache_at_6_with_key_1_disabled(wallet).await;
        store.arm();

        let result = wallet.load_identity_by_index_from_master(0, &master).await;
        assert!(
            matches!(result, Err(PlatformWalletError::PersisterStore(_))),
            "{result:?}"
        );
        let after = held(wallet).await;
        assert_eq!(after.public_keys()[&2].disabled_at(), Some(4));
        assert_eq!(after.revision(), 7);
        assert!(after.public_keys().contains_key(&4));

        store.disarm();
        store.taken().clear();
        wallet
            .refresh_identity(&Identifier::default())
            .await
            .unwrap();
        let stored = |key_id| store.taken().stored_key(Identifier::default(), key_id);
        assert_eq!(
            stored(2).map(|row| row.public_key.disabled_at()),
            Some(Some(4))
        );
        let key_4 = stored(4).expect("the new key's row is stored");
        assert_eq!(key_4.public_key, held(wallet).await.public_keys()[&4]);
        assert_eq!(
            store.taken().stored_revision(Identifier::default()),
            Some(7)
        );
    }

    /// A load that finds a new identity stores it and its keys, with the
    /// breadcrumbs verified against the seed, as one changeset: there is no
    /// breadcrumb-less snapshot of its keys written ahead of them. A store
    /// that takes one write and refuses the next is enough for the load (a
    /// second write was the one that could fail after the first committed
    /// watch-only rows), and a store that refuses it leaves nothing behind.
    #[tokio::test]
    async fn should_store_a_newly_loaded_identity_with_its_keys_in_one_changeset() {
        let master = ExtendedPrivKey::new_master(Network::Testnet, &[7; 64]).unwrap();
        let probe = derive_load_probe_hash(
            ResolvedLoadKeyHashSource::Master(&master),
            Network::Testnet,
            0,
        )
        .unwrap();
        // How many writes the store takes before it refuses; `None` takes
        // every write.
        for takes in [None, Some(1), Some(0)] {
            let mut sdk = dash_sdk::SdkBuilder::new_mock().build().unwrap();
            sdk.mock()
                .expect_fetch(PublicKeyHash(probe), Some(answer_at(7)))
                .await
                .unwrap();
            let store = Arc::new(FailingStore::default());
            let wallet = wallet_with_signing_keys_on(sdk, store.clone()).await;
            let wallet = wallet.identity();
            {
                let mut wm = wallet.wallet_manager.write().await;
                wm.get_wallet_info_mut(&wallet.wallet_id)
                    .unwrap()
                    .identity_manager
                    .remove_identity(&Identifier::default(), &wallet.persister)
                    .unwrap();
            }
            store.taken().clear();
            if let Some(takes) = takes {
                store.arm_after(takes);
            }

            let result = wallet.load_identity_by_index_from_master(0, &master).await;
            let stores = store.taken().changeset_count();
            if takes == Some(0) {
                assert!(result.is_err());
                assert_eq!(stores, 0, "nothing of a refused load is stored");
                continue;
            }
            result.unwrap();
            assert_eq!(stores, 1, "the identity and its keys go out together");
            assert_eq!(
                store.taken().stored_revision(Identifier::default()),
                Some(7)
            );
            // The fixture's keys are not derived from this seed, so they
            // carry no breadcrumb; what matters is that they ride the
            // identity's changeset.
            assert_eq!(store.taken().key_rows().len(), 3);
        }
    }

    /// A same-revision answer whose proof is not above the cache's balance
    /// watermark (mock answers are at height 0) takes the answer's keys but
    /// keeps the cache's balance, which the revision does not order.
    #[tokio::test]
    async fn should_keep_a_balance_a_refresh_s_proof_does_not_order_after() {
        let identity_id = Identifier::default();
        let mut sdk = dash_sdk::SdkBuilder::new_mock().build().unwrap();
        sdk.mock()
            .expect_fetch(identity_id, Some(answer_at(6)))
            .await
            .unwrap();
        let wallet = wallet_with_signing_keys_on(sdk, Arc::new(NoPlatformPersistence)).await;
        let wallet = wallet.identity();
        cache_at_6_with_key_1_disabled(wallet).await;
        let watermark = BlockTime::new(1, 1, 1);
        {
            let mut wm = wallet.wallet_manager.write().await;
            wm.get_wallet_info_mut(&wallet.wallet_id)
                .unwrap()
                .identity_manager
                .managed_identity_mut(&identity_id)
                .unwrap()
                .last_updated_balance_block_time = Some(watermark);
        }
        let cached_balance = held(wallet).await.balance();

        let returned = wallet.refresh_identity(&identity_id).await.unwrap();
        assert_eq!(returned, held(wallet).await);
        assert_eq!(returned.balance(), cached_balance);
        assert!(returned.public_keys().contains_key(&4));
        assert!(returned.public_keys()[&1].is_disabled());
    }

    /// A disable a refresh brings, from an answer older or newer than the
    /// cache, is stored with its key row, as is a key a newer answer adds. A
    /// store that refuses them fails the
    /// refresh and queues the row, which the next refresh stores although
    /// memory already holds the disable.
    #[tokio::test]
    async fn should_store_a_refresh_s_disable_or_queue_it_for_the_next_store() {
        let identity_id = Identifier::default();
        for revision in [5, 7] {
            let mut answer = answer_at(revision);
            let mut key_2 = answer.public_keys()[&2].clone();
            key_2.set_disabled_at(4);
            answer.add_public_key(key_2);
            // Key 1 is disabled in the cache at 9, a placeholder; Platform's
            // timestamp is 3.
            let mut key_1 = answer.public_keys()[&1].clone();
            key_1.set_disabled_at(3);
            answer.add_public_key(key_1);
            let mock = || async {
                let mut sdk = dash_sdk::SdkBuilder::new_mock().build().unwrap();
                sdk.mock()
                    .expect_fetch(identity_id, Some(answer.clone()))
                    .await
                    .unwrap();
                sdk
            };

            let store = Arc::new(FailingStore::default());
            let wallet = wallet_with_signing_keys_on(mock().await, store.clone()).await;
            let wallet = wallet.identity();
            cache_at_6_with_key_1_disabled(wallet).await;
            assert_ne!(held(wallet).await.balance(), 777);
            store.arm();
            let result = wallet.refresh_identity(&identity_id).await;
            assert!(
                matches!(result, Err(PlatformWalletError::PersisterStore(_))),
                "answered at {revision}: {result:?}"
            );
            // Memory holds the fold: the disables, the revision, and the
            // answer's balance (the cache holds no watermark), queued for
            // the next store with the refused rows.
            let after = held(wallet).await;
            assert_eq!(after.public_keys()[&2].disabled_at(), Some(4));
            assert_eq!(after.public_keys()[&1].disabled_at(), Some(3));
            assert_eq!(after.revision(), revision.max(6), "answered at {revision}");
            assert_eq!(after.balance(), 777);
            store.disarm();
            store.taken().clear();
            wallet.refresh_identity(&identity_id).await.unwrap();
            let after = held(wallet).await;
            for (key_id, disabled_at) in [(1, 3), (2, 4)] {
                assert_eq!(
                    after.public_keys()[&key_id].disabled_at(),
                    Some(disabled_at)
                );
                assert_eq!(
                    store
                        .taken()
                        .stored_key(identity_id, key_id)
                        .map(|row| row.public_key.disabled_at()),
                    Some(Some(disabled_at)),
                    "key {key_id} retried at {revision}"
                );
            }

            let store = Arc::new(CapturingStore::default());
            let wallet = wallet_with_signing_keys_on(mock().await, store.clone()).await;
            let wallet = wallet.identity();
            cache_at_6_with_key_1_disabled(wallet).await;
            wallet.refresh_identity(&identity_id).await.unwrap();
            for (key_id, disabled_at) in [(1, 3), (2, 4)] {
                assert_eq!(
                    store
                        .stored_key(identity_id, key_id)
                        .map(|row| row.public_key.disabled_at()),
                    Some(Some(disabled_at)),
                    "key {key_id} answered at {revision}"
                );
            }
            // A key a newer answer adds is stored too, without the
            // breadcrumb a refresh cannot know; an older answer adds none.
            let key_4 = store.stored_key(identity_id, 4);
            assert_eq!(key_4.is_some(), revision > 6, "answered at {revision}");
            assert!(key_4.is_none_or(|row| row.derivation_indices.is_none()));
        }
    }

    /// Discovery folds an identity it already knows the way loading does: an
    /// answer at a newer revision brings its revision, keys and balance into
    /// the cache and the store; an older one keeps the cache's.
    #[tokio::test]
    async fn should_fold_a_known_identity_discovery_finds() {
        let master = ExtendedPrivKey::new_master(Network::Testnet, &[7; 64]).unwrap();
        let probe = |index| {
            derive_identity_auth_key_hash_from_master(
                &master,
                Network::Testnet,
                index,
                MASTER_KEY_INDEX,
            )
            .unwrap()
        };
        let identity_id = Identifier::default();
        for (answered, settled) in [(5, 6), (7, 7)] {
            let mut sdk = dash_sdk::SdkBuilder::new_mock().build().unwrap();
            sdk.mock()
                .expect_fetch(PublicKeyHash(probe(0)), Some(answer_at(answered)))
                .await
                .unwrap();
            sdk.mock()
                .expect_fetch(PublicKeyHash(probe(1)), None::<Identity>)
                .await
                .unwrap();
            let store = Arc::new(CapturingStore::default());
            let wallet = wallet_with_signing_keys_on(sdk, store.clone()).await;
            let wallet = wallet.identity();
            cache_at_6_with_key_1_disabled(wallet).await;
            assert_ne!(held(wallet).await.balance(), 777);
            store.clear();

            let found = wallet
                .discover_from_master(
                    IdentityDiscoveryOptions {
                        start_index: Some(0),
                        gap_limit: 1,
                    },
                    &master,
                )
                .await
                .unwrap();
            assert!(found.is_empty(), "the identity was already known");
            let held = held(wallet).await;
            let older = answered < 6;
            assert_eq!(held.revision(), settled, "answered at {answered}");
            assert_eq!(store.stored_revision(identity_id), Some(settled));
            assert_eq!(held.balance(), 777, "answered at {answered}");
            assert!(
                held.public_keys()[&1].is_disabled(),
                "answered at {answered}"
            );
            assert_eq!(
                held.public_keys().contains_key(&4),
                !older,
                "answered at {answered}"
            );
        }
    }

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
        let mnemonic = Mnemonic::from_phrase(TEST_MNEMONIC).expect("valid English test mnemonic");
        Wallet::from_mnemonic(mnemonic, network, WalletAccountCreationOptions::None)
            .expect("from_mnemonic should build a Mnemonic wallet")
    }

    /// The BIP-32 master node for [`TEST_MNEMONIC`] on `network` — the
    /// same node `derive_extended_private_key` reconstructs internally.
    fn master_for(network: Network) -> ExtendedPrivKey {
        let mnemonic = Mnemonic::from_phrase(TEST_MNEMONIC).expect("valid English test mnemonic");
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
