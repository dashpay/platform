//! Adapter that bridges between [Signer] (sync) and [SignerAsync] (async).
//!
//! A single [SignerAdapter] enum wraps either a sync or async signer and
//! implements **both** traits, so callers only need one object regardless of
//! which direction the bridge goes.

use dpp::address_funds::AddressWitness;
use dpp::identity::signer::{Signer, SignerAsync};
use dpp::platform_value::BinaryData;
use dpp::ProtocolError;
use std::fmt::Debug;
use std::sync::Arc;

use crate::sync::block_on;

/// A single adapter that wraps either a sync [Signer] or an async [SignerAsync]
/// and implements both traits.
///
/// - **`Sync` variant**: delegates directly for sync calls; trivially wraps for
///   async calls (sync work is non-blocking).
/// - **`Async` variant**: delegates directly for async calls; uses [`block_on`]
///   for sync calls. Requires `K: Clone` so the key can be moved into the
///   spawned future safely.
pub enum SignerAdapter<Sy, As> {
    /// Wraps a synchronous [Signer].
    Sync(Arc<Sy>),
    /// Wraps an async [SignerAsync].
    Async(Arc<As>),
}

// -- Manual impls because #[derive] would require both Sy and As to be Clone/Debug --

impl<Sy: Debug, As: Debug> Debug for SignerAdapter<Sy, As> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sync(s) => f.debug_tuple("SignerAdapter::Sync").field(s).finish(),
            Self::Async(s) => f.debug_tuple("SignerAdapter::Async").field(s).finish(),
        }
    }
}

impl<Sy, As> Clone for SignerAdapter<Sy, As> {
    fn clone(&self) -> Self {
        match self {
            Self::Sync(s) => Self::Sync(Arc::clone(s)),
            Self::Async(s) => Self::Async(Arc::clone(s)),
        }
    }
}

// -- Constructors --

impl<Sy, As> SignerAdapter<Sy, As> {
    /// Create an adapter from a synchronous [Signer].
    pub fn from_sync(signer: Sy) -> Self {
        Self::Sync(Arc::new(signer))
    }

    /// Create an adapter from a synchronous [Signer] already wrapped in [Arc].
    pub fn from_sync_arc(signer: Arc<Sy>) -> Self {
        Self::Sync(signer)
    }

    /// Create an adapter from an async [SignerAsync].
    pub fn from_async(signer: As) -> Self {
        Self::Async(Arc::new(signer))
    }

    /// Create an adapter from an async [SignerAsync] already wrapped in [Arc].
    pub fn from_async_arc(signer: Arc<As>) -> Self {
        Self::Async(signer)
    }
}

// -- Signer (sync) implementation --

impl<K, Sy, As> Signer<K> for SignerAdapter<Sy, As>
where
    K: Send + Sync + Clone + Debug + 'static,
    Sy: Signer<K> + Send + 'static,
    As: SignerAsync<K> + Sync + 'static,
{
    fn sign(&self, key: &K, data: &[u8]) -> Result<BinaryData, ProtocolError> {
        match self {
            Self::Sync(s) => s.sign(key, data),
            Self::Async(s) => {
                let signer = Arc::clone(s);
                let key = key.clone();
                let data = data.to_vec();
                block_on(async move { signer.sign(&key, &data).await })
                    .map_err(|e| ProtocolError::Generic(format!("async-sync bridge error: {e}")))?
            }
        }
    }

    fn sign_create_witness(&self, key: &K, data: &[u8]) -> Result<AddressWitness, ProtocolError> {
        match self {
            Self::Sync(s) => s.sign_create_witness(key, data),
            Self::Async(s) => {
                let signer = Arc::clone(s);
                let key = key.clone();
                let data = data.to_vec();
                block_on(async move { signer.sign_create_witness(&key, &data).await })
                    .map_err(|e| ProtocolError::Generic(format!("async-sync bridge error: {e}")))?
            }
        }
    }

    fn can_sign_with(&self, key: &K) -> bool {
        match self {
            Self::Sync(s) => s.can_sign_with(key),
            Self::Async(s) => s.can_sign_with(key),
        }
    }
}

// -- SignerAsync implementation --

#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
impl<K, Sy, As> SignerAsync<K> for SignerAdapter<Sy, As>
where
    K: Send + Sync + Debug + 'static,
    Sy: Signer<K> + Send + 'static,
    As: SignerAsync<K> + Sync + 'static,
{
    async fn sign(&self, key: &K, data: &[u8]) -> Result<BinaryData, ProtocolError> {
        match self {
            Self::Sync(s) => s.sign(key, data),
            Self::Async(s) => s.sign(key, data).await,
        }
    }

    async fn sign_create_witness(
        &self,
        key: &K,
        data: &[u8],
    ) -> Result<AddressWitness, ProtocolError> {
        match self {
            Self::Sync(s) => s.sign_create_witness(key, data),
            Self::Async(s) => s.sign_create_witness(key, data).await,
        }
    }

    fn can_sign_with(&self, key: &K) -> bool {
        match self {
            Self::Sync(s) => s.can_sign_with(key),
            Self::Async(s) => s.can_sign_with(key),
        }
    }
}
