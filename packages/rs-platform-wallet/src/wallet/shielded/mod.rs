//! Feature-gated shielded (Orchard/Halo2) wallet support.
//!
//! This module provides ZK-private transactions on Dash Platform using the
//! Orchard circuit (Halo 2 proving system). It is gated behind the `shielded`
//! Cargo feature because it pulls in heavy cryptographic dependencies.
//!
//! # Architecture
//!
//! - [`OrchardKeySet`] — ZIP-32 key derivation from wallet seed
//! - [`ShieldedStore`] / [`InMemoryShieldedStore`] — storage abstraction
//! - [`CachedOrchardProver`] — lazy-init proving key cache
//! - [`ShieldedWallet`] — top-level coordinator tying keys, store, and SDK together
//!
//! The `ShieldedWallet` is generic over `S: ShieldedStore` so consumers can
//! plug in their own persistence (SQLite, RocksDB, etc.) while tests use the
//! in-memory implementation.

pub mod keys;
pub mod note_selection;
pub mod operations;
pub mod prover;
pub mod store;
pub mod sync;

pub use keys::OrchardKeySet;
pub use prover::CachedOrchardProver;
pub use store::{InMemoryShieldedStore, ShieldedNote, ShieldedStore};

use std::sync::Arc;

use dashcore::Network;
use tokio::sync::RwLock;

use crate::error::PlatformWalletError;

/// Feature-gated shielded wallet.
///
/// Coordinates Orchard key material, a pluggable storage backend, and the
/// Dash SDK for note sync, nullifier checks, and shielded state transitions.
///
/// Generic over `S: ShieldedStore` — consumers provide their persistence
/// layer. For tests, use [`InMemoryShieldedStore`].
///
/// # Thread safety
///
/// The store is wrapped in `Arc<RwLock<S>>` so the wallet can be shared
/// across async tasks. Read operations (balance, address queries) take a
/// read lock; mutating operations (sync, spend) take a write lock.
// Fields and accessors used by sync/operations modules (not yet implemented).
#[allow(dead_code)]
pub struct ShieldedWallet<S: ShieldedStore> {
    /// Dash Platform SDK handle for network operations.
    sdk: Arc<dash_sdk::Sdk>,
    /// ZIP-32 derived Orchard keys.
    keys: OrchardKeySet,
    /// Pluggable storage backend behind a shared async lock.
    store: Arc<RwLock<S>>,
    /// Network (mainnet / testnet / devnet / regtest).
    network: Network,
}

impl<S: ShieldedStore> ShieldedWallet<S> {
    /// Create a shielded wallet from pre-derived keys and a store.
    pub fn new(sdk: Arc<dash_sdk::Sdk>, keys: OrchardKeySet, store: S, network: Network) -> Self {
        Self {
            sdk,
            keys,
            store: Arc::new(RwLock::new(store)),
            network,
        }
    }

    /// Derive Orchard keys from a wallet seed and create a shielded wallet.
    ///
    /// This is the primary constructor for production use. The `seed` should
    /// be the BIP-39 seed bytes (typically 64 bytes).
    ///
    /// # Errors
    ///
    /// Returns an error if key derivation fails (invalid seed or account index).
    pub fn from_seed(
        sdk: Arc<dash_sdk::Sdk>,
        seed: &[u8],
        network: Network,
        account: u32,
        store: S,
    ) -> Result<Self, PlatformWalletError> {
        let keys = OrchardKeySet::from_seed(seed, network, account)?;
        Ok(Self::new(sdk, keys, store, network))
    }

    /// Total unspent shielded balance in credits.
    ///
    /// Reads from the store — does not trigger a sync.
    pub async fn balance(&self) -> Result<u64, PlatformWalletError> {
        let store = self.store.read().await;
        let notes = store.get_unspent_notes().map_err(|e| {
            PlatformWalletError::ShieldedStoreError(e.to_string())
        })?;
        Ok(notes.iter().map(|n| n.value).sum())
    }

    /// The default payment address (diversifier index 0) for receiving
    /// shielded funds.
    pub fn default_address(&self) -> &grovedb_commitment_tree::PaymentAddress {
        &self.keys.default_address
    }

    /// Derive a payment address at the given diversifier index.
    pub fn address_at(&self, index: u32) -> grovedb_commitment_tree::PaymentAddress {
        self.keys.address_at(index)
    }

    // Accessors used by sync and operations modules (not yet implemented).
    #[allow(dead_code)]
    /// Access the SDK handle (for sync and operations modules).
    pub(crate) fn sdk(&self) -> &dash_sdk::Sdk {
        &self.sdk
    }

    #[allow(dead_code)]
    /// Access the key set (for sync and operations modules).
    pub(crate) fn keys(&self) -> &OrchardKeySet {
        &self.keys
    }

    #[allow(dead_code)]
    /// Access the store (for sync and operations modules).
    pub(crate) fn store(&self) -> &Arc<RwLock<S>> {
        &self.store
    }

    #[allow(dead_code)]
    /// Access the network (for sync and operations modules).
    pub(crate) fn network(&self) -> Network {
        self.network
    }
}
