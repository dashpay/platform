use crate::error::ContextProviderError;
use dpp::prelude::{DataContract, Identifier};
use drive::{error::proof::ProofError, query::ContractLookupFn};
#[cfg(feature = "mocks")]
use hex::ToHex;
use std::{io::ErrorKind, ops::Deref, sync::Arc};

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

    /// Fetches the data contract for a specified data contract ID.
    /// This method is used by [FromProof](crate::FromProof) implementations to fetch data contracts
    /// referenced in proofs.
    ///
    /// # Arguments
    ///
    /// * `data_contract_id`: The ID of the data contract to fetch.
    ///
    /// # Returns
    ///
    /// * `Ok(Option<Arc<DataContract>>)`: On success, returns the data contract if it exists, or `None` if it does not.
    /// We use Arc to avoid copying the data contract.
    /// * `Err(Error)`: On failure, returns an error indicating why the operation failed.
    fn get_data_contract(
        &self,
        id: &Identifier,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError>;
}

impl<C: AsRef<dyn ContextProvider> + Send + Sync> ContextProvider for C {
    fn get_quorum_public_key(
        &self,
        quorum_type: u32,
        quorum_hash: [u8; 32],
        core_chain_locked_height: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        self.as_ref()
            .get_quorum_public_key(quorum_type, quorum_hash, core_chain_locked_height)
    }

    fn get_data_contract(
        &self,
        id: &Identifier,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        self.as_ref().get_data_contract(id)
    }
}

impl<'a, T: ContextProvider + 'a> ContextProvider for std::sync::Mutex<T>
where
    Self: Sync + Send,
{
    fn get_data_contract(
        &self,
        id: &Identifier,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        let lock = self.lock().expect("lock poisoned");
        lock.get_data_contract(id)
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
}

/// A trait that provides a function that can be used to look up a [DataContract] by its [Identifier].
///
/// This trait is automatically implemented for any type that implements [ContextProvider].
/// It is used internally by the Drive proof verification functions to look up data contracts.
pub trait DataContractProvider: Send + Sync {
    /// Returns [ContractLookupFn] function that can be used to look up a [DataContract] by its [Identifier].
    fn as_contract_lookup_fn(&self) -> Box<ContractLookupFn>;
}
impl<C: ContextProvider + ?Sized> DataContractProvider for C {
    /// Returns function that uses [ContextProvider] to provide a [DataContract] to Drive proof verification functions
    fn as_contract_lookup_fn(&self) -> Box<ContractLookupFn> {
        let f = |id: &Identifier| -> Result<Option<Arc<DataContract>>, drive::error::Error> {
            self.get_data_contract(id).map_err(|e| {
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

        let dc: DataContract = serde_json::from_reader(f).expect("cannot parse data contract");

        Ok(Some(Arc::new(dc)))
    }
}

// the trait `std::convert::AsRef<(dyn drive_proof_verifier::ContextProvider + 'static)>`
// is not implemented for `std::sync::Arc<mock::provider::GrpcContextProvider<'_>>`
impl<'a, T: ContextProvider + 'a> AsRef<dyn ContextProvider + 'a> for Arc<T> {
    fn as_ref(&self) -> &(dyn ContextProvider + 'a) {
        self.deref()
    }
}
