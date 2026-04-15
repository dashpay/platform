//! Adapter that bridges between [Signer] (sync) and [SignerAsync] (async).
//!
//! A single [SignerAdapter] enum wraps either a sync or async signer and
//! implements **both** traits, so callers only need one object regardless of
//! which direction the bridge goes.
//!
//! Uses trait objects internally so the enum has a single type parameter `K`
//! (the key type), enabling clean `From`/`Into` conversions.

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
///
/// # Construction
///
/// ```ignore
/// // From a sync signer (e.g. SimpleSigner):
/// let adapter: SignerAdapter<IdentityPublicKey> = SignerAdapter::from_sync(my_signer);
/// // or via Into:
/// let arc: Arc<dyn Signer<K> + Send> = Arc::new(my_signer);
/// let adapter: SignerAdapter<K> = arc.into();
///
/// // From an async signer:
/// let adapter: SignerAdapter<K> = SignerAdapter::from_async(my_async_signer);
/// ```
pub enum SignerAdapter<K: Send + Sync + 'static> {
    /// Wraps a synchronous [Signer].
    Sync(Arc<dyn Signer<K> + Send>),
    /// Wraps an async [SignerAsync].
    Async(Arc<dyn SignerAsync<K> + Sync>),
}

impl<K: Send + Sync + 'static> Debug for SignerAdapter<K> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sync(s) => f.debug_tuple("SignerAdapter::Sync").field(s).finish(),
            Self::Async(s) => f.debug_tuple("SignerAdapter::Async").field(s).finish(),
        }
    }
}

impl<K: Send + Sync + 'static> Clone for SignerAdapter<K> {
    fn clone(&self) -> Self {
        match self {
            Self::Sync(s) => Self::Sync(Arc::clone(s)),
            Self::Async(s) => Self::Async(Arc::clone(s)),
        }
    }
}

// -- Constructors --

impl<K: Send + Sync + 'static> SignerAdapter<K> {
    /// Create an adapter from a synchronous [Signer].
    pub fn from_sync<S: Signer<K> + Send + 'static>(signer: S) -> Self {
        Self::Sync(Arc::new(signer))
    }

    /// Create an adapter from an async [SignerAsync].
    pub fn from_async<S: SignerAsync<K> + Sync + 'static>(signer: S) -> Self {
        Self::Async(Arc::new(signer))
    }
}

// -- From / Into --

impl<K: Send + Sync + 'static> From<Arc<dyn Signer<K> + Send>> for SignerAdapter<K> {
    fn from(signer: Arc<dyn Signer<K> + Send>) -> Self {
        Self::Sync(signer)
    }
}

impl<K: Send + Sync + 'static> From<Arc<dyn SignerAsync<K> + Sync>> for SignerAdapter<K> {
    fn from(signer: Arc<dyn SignerAsync<K> + Sync>) -> Self {
        Self::Async(signer)
    }
}

// -- Signer (sync) implementation --

impl<K> Signer<K> for SignerAdapter<K>
where
    K: Send + Sync + Clone + Debug + 'static,
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
impl<K> SignerAsync<K> for SignerAdapter<K>
where
    K: Send + Sync + Debug + 'static,
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
