use hex::ToHex;

/// `QuorumInfoProvider` trait provides an interface to fetch quorum related information, required to verify the proof.
///
/// Developers should implement this trait to provide required quorum details to [FromProof](crate::FromProof)
/// implementations.
///
/// It defines a single method [`get_quorum_public_key()`](QuorumInfoProvider::get_quorum_public_key())
/// which retrieves the public key of a given quorum.
pub trait QuorumInfoProvider: Send + Sync {
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
}

/// Mock QuorumInfoProvider that can read quorum keys from files.
///
/// Use `rs_sdk::SdkBuilder::with_dump_dir()` to generate quorum keys files.
#[cfg(feature = "mocks")]
pub struct MockQuorumInfoProvider {
    quorum_keys_dir: Option<std::path::PathBuf>,
}

#[cfg(feature = "mocks")]
impl MockQuorumInfoProvider {
    /// Create a new instance of [MockQuorumInfoProvider].
    ///
    /// This instance can be used to read quorum keys from files.
    /// You need to configure quorum keys dir using
    /// [MockQuorumInfoProvider::quorum_keys_dir()](MockQuorumInfoProvider::quorum_keys_dir())
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
    /// This directory should contain quorum keys files generated using `rs_sdk::SdkBuilder::with_dump_dir()`.
    pub fn quorum_keys_dir(&mut self, quorum_keys_dir: Option<std::path::PathBuf>) {
        self.quorum_keys_dir = quorum_keys_dir;
    }
}

impl Default for MockQuorumInfoProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "mocks")]
impl QuorumInfoProvider for MockQuorumInfoProvider {
    /// Mock implementation of [QuorumInfoProvider] that returns keys from files saved on disk.
    ///
    /// Use `rs_sdk::SdkBuilder::with_dump_dir()` to generate quorum keys files.   
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
}
