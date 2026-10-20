use crate::error::ContextProviderError;
use dpp::data_contract::TokenConfiguration;
use dpp::prelude::{CoreBlockHeight, DataContract, Identifier};
use dpp::version::PlatformVersion;
use drive::{error::proof::ProofError, query::ContractLookupFn};
use std::{future::Future, ops::Deref, pin::Pin, sync::Arc};

#[cfg(feature = "mocks")]
use {
    dpp::data_contract::serialized_version::DataContractInSerializationFormat, hex::ToHex,
    std::io::ErrorKind,
};

/// Future returned by [ContextProvider::fetch_quorum_public_key].
///
/// `Send` on every target because the SDK's request futures are `Send`
/// everywhere, and `'static` so that wrappers such as `Mutex<T>` can forward
/// the call without holding a guard across an await.
pub type QuorumKeyFuture =
    Pin<Box<dyn Future<Output = Result<Option<[u8; 48]>, ContextProviderError>> + Send + 'static>>;

/// Interface between the Sdk and state of the application.
///
/// ContextProvider is called by the [FromProof](crate::FromProof) trait (and similar) to get information about
/// the application and/or network state, including data contracts that might be cached by the application or
/// quorum public keys.
///
/// Developers using the Dash Platform SDK should implement this trait to provide required information
/// to the Sdk, especially implementation of [FromProof](crate::FromProof) trait.
///
/// A ContextProvider should be thread-safe and manage timeouts and other concurrency-related issues internally,
/// as the [FromProof](crate::FromProof) implementations can block on ContextProvider calls.
pub trait ContextProvider: Send + Sync {
    /// Fetches the data contract for a specified data contract ID.
    /// This method is used by [FromProof](crate::FromProof) implementations to fetch data contracts
    /// referenced in proofs.
    ///
    /// # Arguments
    ///
    /// * `data_contract_id`: The ID of the data contract to fetch.
    /// * `platform_version`: The platform version to use.
    ///
    /// # Returns
    ///
    /// * `Ok(Option<Arc<DataContract>>)`: On success, returns the data contract if it exists, or `None` if it does not.
    ///   We use Arc to avoid copying the data contract.
    /// * `Err(Error)`: On failure, returns an error indicating why the operation failed.
    fn get_data_contract(
        &self,
        id: &Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError>;

    /// Register a data contract into the provider's cache so that a
    /// subsequent proof verification (e.g. the returned-proof check after a
    /// document or token state-transition broadcast) can resolve it without
    /// a network fetch.
    ///
    /// Providers that maintain a writable known-contracts cache (e.g. the
    /// mobile `TrustedHttpContextProvider`) override this; the default is a
    /// no-op for providers that fetch on demand or don't cache.
    fn register_data_contract(&self, _contract: Arc<DataContract>) {}

    /// Fetches the token configuration for a specified token ID.
    /// This method is used by [FromProof](crate::FromProof) implementations to fetch token configurations
    /// referenced in proofs.
    ///
    /// # Arguments
    ///
    /// * `token_id`: The ID of the token to fetch.
    /// * `platform_version`: The platform version to use.
    ///
    /// # Returns
    ///
    /// * `Ok(Option<TokenConfiguration>)`: On success, returns the token configuration if it exists, or `None` if it does not.
    ///   We use Arc to avoid copying the token configuration.
    /// * `Err(Error)`: On failure, returns an error indicating why the operation failed.
    fn get_token_configuration(
        &self,
        token_id: &Identifier,
    ) -> Result<Option<TokenConfiguration>, ContextProviderError>;

    /// Fetches the public key for a specified quorum.
    ///
    /// # Arguments
    ///
    /// * `quorum_type`: The type of the quorum.
    /// * `quorum_hash`: The hash of the quorum. This is used to determine which quorum's public key to fetch.
    /// * `core_chain_locked_height`: Core chain locked height for which the quorum must be valid
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<u8>)`: On success, returns a byte vector representing the public key of the quorum.
    /// * `Err(Error)`: On failure, returns an error indicating why the operation failed.
    fn get_quorum_public_key(
        &self,
        quorum_type: u32,
        quorum_hash: [u8; 32], // quorum hash is 32 bytes
        core_chain_locked_height: u32,
    ) -> Result<[u8; 48], ContextProviderError>; // public key is 48 bytes

    /// Fetches the public key of a quorum this provider does not hold yet.
    ///
    /// The SDK calls this from async code when proof verification failed
    /// because [get_quorum_public_key](Self::get_quorum_public_key) had no key
    /// for the quorum, and verifies the same response again when a key comes
    /// back. This lets a provider that caches quorum keys, and cannot block
    /// inside the synchronous lookup, fetch a key that is newer than its cache.
    ///
    /// The arguments come from a response that has not been verified yet. The
    /// key must come from the provider's own trusted source, never from the
    /// arguments, and the work done must stay bounded whatever they are.
    ///
    /// # Returns
    ///
    /// * `None`: an asynchronous fetch would not help, for example because the
    ///   provider cannot fetch keys or its synchronous lookup already fetched.
    ///   This is the default.
    /// * `Some(future)`, resolving to:
    ///   * `Ok(Some(key))`: the key, which `get_quorum_public_key` now returns
    ///     as well.
    ///   * `Ok(None)`: the trusted source, asked after this call was made,
    ///     answered and does not know this quorum.
    ///   * `Err(_)`: the provider could not find out, for example because its
    ///     trusted source was unreachable.
    fn fetch_quorum_public_key(
        &self,
        _quorum_type: u32,
        _quorum_hash: [u8; 32],
        _core_chain_locked_height: u32,
    ) -> Option<QuorumKeyFuture> {
        None
    }

    /// Gets the platform activation height from core. Once this has happened this can be hardcoded.
    ///
    /// # Returns
    ///
    /// * `Ok(CoreBlockHeight)`: On success, returns the platform activation height as defined by mn_rr
    /// * `Err(Error)`: On failure, returns an error indicating why the operation failed.
    fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError>;
}

impl<C: AsRef<dyn ContextProvider> + Send + Sync> ContextProvider for C {
    fn get_data_contract(
        &self,
        id: &Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        self.as_ref().get_data_contract(id, platform_version)
    }

    fn register_data_contract(&self, contract: Arc<DataContract>) {
        self.as_ref().register_data_contract(contract)
    }

    fn get_token_configuration(
        &self,
        token_id: &Identifier,
    ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
        self.as_ref().get_token_configuration(token_id)
    }

    fn get_quorum_public_key(
        &self,
        quorum_type: u32,
        quorum_hash: [u8; 32],
        core_chain_locked_height: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        self.as_ref()
            .get_quorum_public_key(quorum_type, quorum_hash, core_chain_locked_height)
    }

    fn fetch_quorum_public_key(
        &self,
        quorum_type: u32,
        quorum_hash: [u8; 32],
        core_chain_locked_height: u32,
    ) -> Option<QuorumKeyFuture> {
        self.as_ref()
            .fetch_quorum_public_key(quorum_type, quorum_hash, core_chain_locked_height)
    }

    fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
        self.as_ref().get_platform_activation_height()
    }
}

impl<T: ContextProvider> ContextProvider for std::sync::Mutex<T>
where
    Self: Sync + Send,
{
    fn get_data_contract(
        &self,
        id: &Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        let lock = self.lock().expect("lock poisoned");
        lock.get_data_contract(id, platform_version)
    }

    fn register_data_contract(&self, contract: Arc<DataContract>) {
        let lock = self.lock().expect("lock poisoned");
        lock.register_data_contract(contract)
    }

    fn get_token_configuration(
        &self,
        token_id: &Identifier,
    ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
        let lock = self.lock().expect("lock poisoned");
        lock.get_token_configuration(token_id)
    }

    fn get_quorum_public_key(
        &self,
        quorum_type: u32,
        quorum_hash: [u8; 32], // quorum hash is 32 bytes
        core_chain_locked_height: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        let lock = self.lock().expect("lock poisoned");
        lock.get_quorum_public_key(quorum_type, quorum_hash, core_chain_locked_height)
    }

    fn fetch_quorum_public_key(
        &self,
        quorum_type: u32,
        quorum_hash: [u8; 32],
        core_chain_locked_height: u32,
    ) -> Option<QuorumKeyFuture> {
        // The future is 'static, so the guard is released before it is awaited.
        let lock = self.lock().expect("lock poisoned");
        lock.fetch_quorum_public_key(quorum_type, quorum_hash, core_chain_locked_height)
    }

    fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
        let lock = self.lock().expect("lock poisoned");
        lock.get_platform_activation_height()
    }
}

/// A trait that provides a function that can be used to look up a [DataContract] by its [Identifier].
///
/// This trait is automatically implemented for any type that implements [ContextProvider].
/// It is used internally by the Drive proof verification functions to look up data contracts.
pub trait DataContractProvider: Send + Sync {
    /// Returns [ContractLookupFn] function that can be used to look up a [DataContract] by its [Identifier].
    fn as_contract_lookup_fn<'a>(
        &'a self,
        platform_version: &'a PlatformVersion,
    ) -> Box<ContractLookupFn<'a>>;
}
impl<C: ContextProvider + ?Sized> DataContractProvider for C {
    /// Returns function that uses [ContextProvider] to provide a [DataContract] to Drive proof verification functions
    fn as_contract_lookup_fn<'a>(
        &'a self,
        platform_version: &'a PlatformVersion,
    ) -> Box<ContractLookupFn<'a>> {
        let f = |id: &Identifier| -> Result<Option<Arc<DataContract>>, drive::error::Error> {
            self.get_data_contract(id, platform_version).map_err(|e| {
                drive::error::Error::Proof(ProofError::ErrorRetrievingContract(e.to_string()))
            })
        };

        Box::new(f)
    }
}

/// Mock ContextProvider that can read quorum keys from files.
///
/// Use [dash_sdk::SdkBuilder::with_dump_dir()] to generate quorum keys files.
#[cfg(feature = "mocks")]
#[derive(Debug)]
pub struct MockContextProvider {
    quorum_keys_dir: Option<std::path::PathBuf>,
}

#[cfg(feature = "mocks")]
impl MockContextProvider {
    /// Create a new instance of [MockContextProvider].
    ///
    /// This instance can be used to read quorum keys from files.
    /// You need to configure quorum keys dir using
    /// [MockContextProvider::quorum_keys_dir()](MockContextProvider::quorum_keys_dir())
    /// before using this instance.
    ///
    /// In future, we may add more methods to this struct to allow setting expectations.
    pub fn new() -> Self {
        Self {
            quorum_keys_dir: None,
        }
    }

    /// Set the directory where quorum keys are stored.
    ///
    /// This directory should contain quorum keys files generated using [dash_sdk::SdkBuilder::with_dump_dir()].
    pub fn quorum_keys_dir(&mut self, quorum_keys_dir: Option<std::path::PathBuf>) {
        self.quorum_keys_dir = quorum_keys_dir;
    }
}

#[cfg(feature = "mocks")]
impl Default for MockContextProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "mocks")]
impl ContextProvider for MockContextProvider {
    /// Mock implementation of [ContextProvider] that returns keys from files saved on disk.
    ///
    /// Use [dash_sdk::SdkBuilder::with_dump_dir()] to generate quorum keys files.
    fn get_quorum_public_key(
        &self,
        quorum_type: u32,
        quorum_hash: [u8; 32],
        _core_chain_locked_height: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        let path = match &self.quorum_keys_dir {
            Some(p) => p,
            None => return Err(ContextProviderError::Config("dump dir not set".to_string())),
        };

        let file = path.join(format!(
            "quorum_pubkey-{}-{}.json",
            quorum_type,
            quorum_hash.encode_hex::<String>()
        ));

        let f = match std::fs::File::open(&file) {
            Ok(f) => f,
            Err(e) => {
                return Err(ContextProviderError::InvalidQuorum(format!(
                    "cannot load quorum key file {}: {}",
                    file.to_string_lossy(),
                    e
                )))
            }
        };

        let data = std::io::read_to_string(f).expect("cannot read quorum key file");
        let key: Vec<u8> = hex::decode(data).expect("cannot parse quorum key");

        Ok(key.try_into().expect("quorum key format mismatch"))
    }

    fn get_data_contract(
        &self,
        data_contract_id: &Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        let path = match &self.quorum_keys_dir {
            Some(p) => p,
            None => return Err(ContextProviderError::Config("dump dir not set".to_string())),
        };

        let file = path.join(format!(
            "data_contract-{}.json",
            data_contract_id.encode_hex::<String>()
        ));

        let f = match std::fs::File::open(&file) {
            Ok(f) => f,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
            Err(e) => {
                return Err(ContextProviderError::DataContractFailure(format!(
                    "cannot load data contract file {}: {}",
                    file.to_string_lossy(),
                    e
                )))
            }
        };

        let serialized_form: DataContractInSerializationFormat = serde_json::from_reader(f)
            .map_err(|e| {
                ContextProviderError::DataContractFailure(format!(
                    "cannot deserialized data contract with id {}: {}",
                    data_contract_id, e
                ))
            })?;
        let dc = DataContract::try_from_platform_versioned(
            serialized_form,
            false,
            &mut vec![],
            platform_version,
        )
        .map_err(|e| {
            ContextProviderError::DataContractFailure(format!(
                "cannot use serialized version of data contract with id {}: {}",
                data_contract_id, e
            ))
        })?;

        Ok(Some(Arc::new(dc)))
    }

    fn get_token_configuration(
        &self,
        _token_id: &Identifier,
    ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
        // Token configuration files are never generated
        Ok(None)
    }

    fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
        Ok(1320) // This is the default activation height for a Regtest network
    }
}

// the trait `std::convert::AsRef<(dyn drive_proof_verifier::ContextProvider + 'static)>`
// is not implemented for `std::sync::Arc<mock::provider::GrpcContextProvider<'_>>`
impl<'a, T: ContextProvider + 'a> AsRef<dyn ContextProvider + 'a> for Arc<T> {
    fn as_ref(&self) -> &(dyn ContextProvider + 'a) {
        self.deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    /// Provider whose `fetch_quorum_public_key` counts its calls and records
    /// the quorum it was asked for.
    #[derive(Default)]
    struct FetchingProvider {
        calls: AtomicUsize,
        asked: Mutex<Option<(u32, [u8; 32], u32)>>,
    }

    impl ContextProvider for FetchingProvider {
        fn get_data_contract(
            &self,
            _id: &Identifier,
            _platform_version: &PlatformVersion,
        ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
            Ok(None)
        }

        fn get_token_configuration(
            &self,
            _token_id: &Identifier,
        ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
            Ok(None)
        }

        fn get_quorum_public_key(
            &self,
            _quorum_type: u32,
            _quorum_hash: [u8; 32],
            _core_chain_locked_height: u32,
        ) -> Result<[u8; 48], ContextProviderError> {
            Err(ContextProviderError::InvalidQuorum(
                "not cached".to_string(),
            ))
        }

        fn fetch_quorum_public_key(
            &self,
            quorum_type: u32,
            quorum_hash: [u8; 32],
            core_chain_locked_height: u32,
        ) -> Option<QuorumKeyFuture> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            *self.asked.lock().unwrap() =
                Some((quorum_type, quorum_hash, core_chain_locked_height));
            Some(Box::pin(async { Ok(Some([7u8; 48])) }))
        }

        fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
            Ok(1)
        }
    }

    /// The SDK reaches its provider through `Arc`, `Box<dyn ContextProvider>`
    /// and `Mutex` wrappers. A wrapper that fell back to the default `None`
    /// would silently turn off fetching of keys newer than the provider's
    /// cache, so every wrapper must reach the inner provider with the same
    /// arguments.
    #[test]
    fn should_forward_quorum_key_fetches_through_every_wrapper() {
        fn assert_forwards(provider: &dyn ContextProvider, inner: &FetchingProvider, call: usize) {
            assert!(provider
                .fetch_quorum_public_key(6, [0xab; 32], 42)
                .is_some());
            assert_eq!(inner.calls.load(Ordering::SeqCst), call);
            assert_eq!(*inner.asked.lock().unwrap(), Some((6, [0xab; 32], 42)));
        }

        let inner = Arc::new(FetchingProvider::default());

        let arc: Arc<FetchingProvider> = Arc::clone(&inner);
        assert_forwards(&arc, &inner, 1);

        let boxed: Arc<Box<dyn ContextProvider>> =
            Arc::new(Box::new(Arc::clone(&inner)) as Box<dyn ContextProvider>);
        assert_forwards(&boxed, &inner, 2);

        let mutex = std::sync::Mutex::new(Arc::clone(&inner));
        assert_forwards(&mutex, &inner, 3);
    }

    /// Providers that do not override the method keep their behaviour: the
    /// SDK sees `None` and reports the original verification error.
    #[test]
    fn should_not_fetch_quorum_keys_by_default() {
        struct CacheOnly;
        impl ContextProvider for CacheOnly {
            fn get_data_contract(
                &self,
                _id: &Identifier,
                _platform_version: &PlatformVersion,
            ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
                Ok(None)
            }
            fn get_token_configuration(
                &self,
                _token_id: &Identifier,
            ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
                Ok(None)
            }
            fn get_quorum_public_key(
                &self,
                _quorum_type: u32,
                _quorum_hash: [u8; 32],
                _core_chain_locked_height: u32,
            ) -> Result<[u8; 48], ContextProviderError> {
                Err(ContextProviderError::InvalidQuorum(
                    "not cached".to_string(),
                ))
            }
            fn get_platform_activation_height(
                &self,
            ) -> Result<CoreBlockHeight, ContextProviderError> {
                Ok(1)
            }
        }

        assert!(CacheOnly.fetch_quorum_public_key(6, [0; 32], 1).is_none());
    }
}
