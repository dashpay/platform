use std::borrow::Cow;
use std::sync::Arc;

use dpp::prelude::{DataContract, Identifier};
use hex::ToHex;

use crate::Error;

/// `ContextProvider` trait provides an interface to fetch information about context of proof verification, like
/// quorum information, data contracts present in the platform, etc.
///
/// Developers should implement this trait to provide required information to [FromProof](crate::FromProof)
/// implementations.
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
    ) -> Result<[u8; 48], crate::Error>; // public key is 48 bytes

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
    fn get_data_contract(&self, id: &Identifier)
        -> Result<Option<Arc<DataContract>>, crate::Error>;
}

/// Mock ContextProvider that can read quorum keys from files.
///
/// Use `dash_platform_sdk::SdkBuilder::with_dump_dir()` to generate quorum keys files.
#[cfg(feature = "mocks")]
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
    /// This directory should contain quorum keys files generated using `dash_platform_sdk::SdkBuilder::with_dump_dir()`.
    pub fn quorum_keys_dir(&mut self, quorum_keys_dir: Option<std::path::PathBuf>) {
        self.quorum_keys_dir = quorum_keys_dir;
    }
}

impl Default for MockContextProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "mocks")]
impl ContextProvider for &MockContextProvider {
    /// Mock implementation of [ContextProvider] that returns keys from files saved on disk.
    ///
    /// Use `dash_platform_sdk::SdkBuilder::with_dump_dir()` to generate quorum keys files.   
    fn get_quorum_public_key(
        &self,
        quorum_type: u32,
        quorum_hash: [u8; 32],
        _core_chain_locked_height: u32,
    ) -> Result<[u8; 48], crate::Error> {
        let path = match &self.quorum_keys_dir {
            Some(p) => p,
            None => {
                return Err(crate::Error::InvalidQuorum {
                    error: "dump dir not set".to_string(),
                })
            }
        };

        let file = path.join(format!(
            "quorum_pubkey-{}-{}.json",
            quorum_type,
            quorum_hash.encode_hex::<String>()
        ));

        let f = match std::fs::File::open(&file) {
            Ok(f) => f,
            Err(e) => {
                return Err(crate::Error::InvalidQuorum {
                    error: format!(
                        "cannot load quorum key file {}: {}",
                        file.to_string_lossy(),
                        e
                    ),
                })
            }
        };

        let key: Vec<u8> = serde_json::from_reader(f).expect("cannot parse quorum key");

        Ok(key.try_into().expect("quorum key format mismatch"))
    }

    fn get_data_contract(
        &self,
        _data_contract_id: &Identifier,
    ) -> Result<Option<Arc<DataContract>>, crate::Error> {
        todo!("not implemented yet");
    }
}
