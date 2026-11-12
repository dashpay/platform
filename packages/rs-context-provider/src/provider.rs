use crate::error::ContextProviderError;
use crate::{ContextProviderMode, QuorumKeySource, SpvSourceSlot};
use dpp::dashcore::Network;
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

/// The Core height at which Platform activated (its L1 locked height) on
/// `network`.
pub fn platform_activation_height(network: Network) -> CoreBlockHeight {
    match network {
        Network::Mainnet => 2_132_092,
        Network::Testnet => 1_090_319,
        Network::Devnet | Network::Regtest => 1,
    }
}

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
    ///
    /// A provider that overrides it keeps a held contract of a higher version
    /// than `contract`, deciding atomically under its own cache lock: a
    /// registration that waited on a fetch must not replace a newer contract
    /// another operation registered meanwhile, which a proof of that newer
    /// version would then fail to verify against.
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

    /// The network whose quorum keys this provider serves, if it serves only
    /// one. The SDK refuses a provider for another network than its own.
    ///
    /// The default, `None`, makes no claim.
    fn network(&self) -> Option<Network> {
        None
    }

    /// Whether the provider is only sound on an SDK that requests and
    /// verifies proofs, for example because its keys are proven for the
    /// heights in verified responses. The SDK refuses such a provider with
    /// proofs turned off.
    ///
    /// The default is `false`.
    fn requires_proofs(&self) -> bool {
        false
    }

    /// Called by the proof verifier once the Tenderdash signature of a
    /// response verified with a key from this provider, with the quorum that
    /// signed it and the Core ChainLocked height in that response's metadata:
    /// the same arguments, in the same order, as the
    /// [get_quorum_public_key](Self::get_quorum_public_key) call that
    /// returned the key.
    ///
    /// The quorum signed that height as part of the state it committed, so
    /// it is a Core height Platform actually reached, unlike the heights
    /// passed to [get_quorum_public_key](Self::get_quorum_public_key), which
    /// come from responses not verified yet. Providers may use it to track
    /// Platform's progress. It is not called for a response that fails
    /// verification.
    ///
    /// A provider that combines others forwards the call only to the child
    /// that supplied this quorum's key, found by the quorum type and hash:
    /// another child never vouched for that height. Verifications run
    /// concurrently, so the quorum identity, not the order of calls, says
    /// which lookup succeeded.
    ///
    /// Called on the verification path, so it must be cheap and must not
    /// block. The default does nothing.
    fn core_height_verified(
        &self,
        _quorum_type: u32,
        _quorum_hash: [u8; 32],
        _core_chain_locked_height: u32,
    ) {
    }

    /// The source this provider answers quorum lookups from right now, for
    /// diagnostics: its only source, or, for a provider that switches between
    /// sources, the one it would ask now. For one that chooses by whether a
    /// published SPV client is ready, a client that cannot say counts as not
    /// ready, so this reports the trusted source then.
    ///
    /// The default, `None`, makes no claim. A provider wrapping another
    /// forwards this and the two methods below, or the wrapped provider
    /// reads as one that says nothing, has no mode and no SPV slot.
    fn quorum_source(&self) -> Option<QuorumKeySource> {
        None
    }

    /// Change which source later quorum lookups use, on a provider that
    /// switches between sources at run time.
    ///
    /// The default refuses with [`ContextProviderError::Config`]: a provider
    /// with a single source has no mode.
    fn set_quorum_mode(&self, mode: ContextProviderMode) -> Result<(), ContextProviderError> {
        Err(ContextProviderError::Config(format!(
            "the context provider has no quorum mode to set to {mode:?}"
        )))
    }

    /// The slot an SPV client publishes this provider's quorum keys into, if
    /// it takes them from one. The default, `None`, takes none.
    fn spv_source_slot(&self) -> Option<Arc<SpvSourceSlot>> {
        None
    }
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

    fn network(&self) -> Option<Network> {
        self.as_ref().network()
    }

    fn requires_proofs(&self) -> bool {
        self.as_ref().requires_proofs()
    }

    fn core_height_verified(
        &self,
        quorum_type: u32,
        quorum_hash: [u8; 32],
        core_chain_locked_height: u32,
    ) {
        self.as_ref()
            .core_height_verified(quorum_type, quorum_hash, core_chain_locked_height)
    }

    fn quorum_source(&self) -> Option<QuorumKeySource> {
        self.as_ref().quorum_source()
    }

    fn set_quorum_mode(&self, mode: ContextProviderMode) -> Result<(), ContextProviderError> {
        self.as_ref().set_quorum_mode(mode)
    }

    fn spv_source_slot(&self) -> Option<Arc<SpvSourceSlot>> {
        self.as_ref().spv_source_slot()
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

    fn network(&self) -> Option<Network> {
        let lock = self.lock().expect("lock poisoned");
        lock.network()
    }

    fn requires_proofs(&self) -> bool {
        let lock = self.lock().expect("lock poisoned");
        lock.requires_proofs()
    }

    fn core_height_verified(
        &self,
        quorum_type: u32,
        quorum_hash: [u8; 32],
        core_chain_locked_height: u32,
    ) {
        let lock = self.lock().expect("lock poisoned");
        lock.core_height_verified(quorum_type, quorum_hash, core_chain_locked_height)
    }

    fn quorum_source(&self) -> Option<QuorumKeySource> {
        let lock = self.lock().expect("lock poisoned");
        lock.quorum_source()
    }

    fn set_quorum_mode(&self, mode: ContextProviderMode) -> Result<(), ContextProviderError> {
        let lock = self.lock().expect("lock poisoned");
        lock.set_quorum_mode(mode)
    }

    fn spv_source_slot(&self) -> Option<Arc<SpvSourceSlot>> {
        let lock = self.lock().expect("lock poisoned");
        lock.spv_source_slot()
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

    /// A quorum type, quorum hash and Core height, as passed to
    /// [ContextProvider::core_height_verified].
    type Verified = (u32, [u8; 32], u32);

    /// A quorum type and quorum hash.
    type Quorum = (u32, [u8; 32]);

    /// Provider whose `fetch_quorum_public_key` counts its calls and records
    /// the quorum it was asked for, and which records the quorums and
    /// heights reported verified.
    #[derive(Default)]
    struct FetchingProvider {
        calls: AtomicUsize,
        asked: Mutex<Option<(u32, [u8; 32], u32)>>,
        verified: Mutex<Vec<Verified>>,
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

        fn network(&self) -> Option<Network> {
            Some(Network::Testnet)
        }

        fn requires_proofs(&self) -> bool {
            true
        }

        fn core_height_verified(
            &self,
            quorum_type: u32,
            quorum_hash: [u8; 32],
            core_chain_locked_height: u32,
        ) {
            self.verified.lock().unwrap().push((
                quorum_type,
                quorum_hash,
                core_chain_locked_height,
            ));
        }
    }

    /// The SDK checks a provider's requirements through the same wrappers it
    /// reaches the provider through; a wrapper that fell back to the defaults
    /// would let the SDK accept a provider it must refuse.
    #[test]
    fn should_forward_provider_requirements_through_every_wrapper() {
        let inner = Arc::new(FetchingProvider::default());
        let boxed: Arc<Box<dyn ContextProvider>> =
            Arc::new(Box::new(Arc::clone(&inner)) as Box<dyn ContextProvider>);
        let mutex = std::sync::Mutex::new(Arc::clone(&inner));
        let wrappers: [&dyn ContextProvider; 3] = [&inner, &boxed, &mutex];
        for provider in wrappers {
            assert_eq!(provider.network(), Some(Network::Testnet));
            assert!(provider.requires_proofs());
        }
    }

    /// A provider that does not say where its quorum keys come from is
    /// reported as custom, has no mode to change and no SPV slot: the FFI
    /// must not present it as trusted, nor claim a mode change it ignored.
    #[test]
    fn should_report_no_quorum_source_mode_or_slot_by_default() {
        let provider = FetchingProvider::default();
        assert_eq!(provider.quorum_source(), None);
        assert!(matches!(
            provider.set_quorum_mode(ContextProviderMode::Spv),
            Err(ContextProviderError::Config(_))
        ));
        assert!(provider.spv_source_slot().is_none());
    }

    /// The proof verifier reports verified heights through the same
    /// wrappers; a wrapper that fell back to the no-op default would hide
    /// Platform's progress from the provider.
    #[test]
    fn should_forward_verified_core_heights_through_every_wrapper() {
        let inner = Arc::new(FetchingProvider::default());
        let boxed: Arc<Box<dyn ContextProvider>> =
            Arc::new(Box::new(Arc::clone(&inner)) as Box<dyn ContextProvider>);
        let mutex = Mutex::new(Arc::clone(&inner));
        let wrappers: [&dyn ContextProvider; 3] = [&inner, &boxed, &mutex];
        for (height, provider) in (10..).zip(wrappers) {
            provider.core_height_verified(6, [height as u8; 32], height);
        }
        assert_eq!(
            *inner.verified.lock().unwrap(),
            [(6, [10; 32], 10), (6, [11; 32], 11), (6, [12; 32], 12)]
        );
    }

    /// A child of [Combining]: serves the keys of the quorums it holds and
    /// records the verified heights it is told about.
    struct Child {
        quorums: Vec<[u8; 32]>,
        verified: Mutex<Vec<Verified>>,
    }

    impl Child {
        fn holding(quorums: Vec<[u8; 32]>) -> Self {
            Self {
                quorums,
                verified: Mutex::new(Vec::new()),
            }
        }
    }

    impl ContextProvider for Child {
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
            quorum_hash: [u8; 32],
            _core_chain_locked_height: u32,
        ) -> Result<[u8; 48], ContextProviderError> {
            if self.quorums.contains(&quorum_hash) {
                Ok([quorum_hash[0]; 48])
            } else {
                Err(ContextProviderError::QuorumSourceUnavailable(
                    "not held".to_string(),
                ))
            }
        }

        fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
            Ok(1)
        }

        fn core_height_verified(
            &self,
            quorum_type: u32,
            quorum_hash: [u8; 32],
            core_chain_locked_height: u32,
        ) {
            self.verified.lock().unwrap().push((
                quorum_type,
                quorum_hash,
                core_chain_locked_height,
            ));
        }
    }

    /// A provider that asks its children in turn for a quorum's key,
    /// remembers which child supplied each quorum, and forwards a verified
    /// height only to that child.
    struct Combining {
        children: Vec<Child>,
        suppliers: Mutex<Vec<(Quorum, usize)>>,
    }

    impl ContextProvider for Combining {
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
            quorum_type: u32,
            quorum_hash: [u8; 32],
            core_chain_locked_height: u32,
        ) -> Result<[u8; 48], ContextProviderError> {
            let mut last = None;
            for (index, child) in self.children.iter().enumerate() {
                match child.get_quorum_public_key(
                    quorum_type,
                    quorum_hash,
                    core_chain_locked_height,
                ) {
                    Ok(key) => {
                        let mut suppliers = self.suppliers.lock().unwrap();
                        suppliers.retain(|(quorum, _)| *quorum != (quorum_type, quorum_hash));
                        suppliers.push(((quorum_type, quorum_hash), index));
                        return Ok(key);
                    }
                    Err(error) => last = Some(error),
                }
            }
            Err(last.unwrap_or_else(|| ContextProviderError::Config("no child".to_string())))
        }

        fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
            Ok(1)
        }

        fn core_height_verified(
            &self,
            quorum_type: u32,
            quorum_hash: [u8; 32],
            core_chain_locked_height: u32,
        ) {
            let supplier = self
                .suppliers
                .lock()
                .unwrap()
                .iter()
                .find(|(quorum, _)| *quorum == (quorum_type, quorum_hash))
                .map(|(_, index)| *index);
            if let Some(index) = supplier {
                self.children[index].core_height_verified(
                    quorum_type,
                    quorum_hash,
                    core_chain_locked_height,
                );
            }
        }
    }

    /// Illustrates the contract with a stub combiner rather than testing
    /// production code: the notification names the quorum, so a provider
    /// combining others can route it to the child that supplied that
    /// quorum's key, even when lookups and verifications of two quorums at
    /// one height interleave, as concurrent requests do. That the verifier
    /// passes the identity it looked the key up by is tested in
    /// `drive-proof-verifier`.
    #[test]
    fn should_let_a_combining_provider_route_verified_heights_by_quorum() {
        let (a, b) = ([0xaa; 32], [0xbb; 32]);
        let combining = Combining {
            children: vec![Child::holding(vec![a]), Child::holding(vec![b])],
            suppliers: Mutex::new(Vec::new()),
        };
        let provider: &dyn ContextProvider = &combining;

        assert_eq!(
            provider.get_quorum_public_key(6, a, 100).unwrap(),
            [0xaa; 48]
        );
        assert_eq!(
            provider.get_quorum_public_key(6, b, 100).unwrap(),
            [0xbb; 48]
        );
        provider.core_height_verified(6, b, 100);
        provider.core_height_verified(6, a, 100);
        assert_eq!(
            provider.get_quorum_public_key(6, b, 101).unwrap(),
            [0xbb; 48]
        );
        provider.core_height_verified(6, b, 101);
        // A quorum no child supplied reaches no child.
        provider.core_height_verified(6, [0xcc; 32], 100);

        assert_eq!(
            *combining.children[0].verified.lock().unwrap(),
            [(6, a, 100)]
        );
        assert_eq!(
            *combining.children[1].verified.lock().unwrap(),
            [(6, b, 100), (6, b, 101)]
        );
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
        assert_eq!(CacheOnly.network(), None);
        assert!(!CacheOnly.requires_proofs());
    }
}
