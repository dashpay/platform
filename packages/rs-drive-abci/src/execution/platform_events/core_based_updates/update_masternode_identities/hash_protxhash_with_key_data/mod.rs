mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::version::PlatformVersion;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Hashes transaction hash with key data.
    ///
    /// This function performs the hash operation and delegates to a version-specific method depending on the platform version.
    ///
    /// # Arguments
    ///
    /// * pro_tx_hash - The provided transaction hash.
    /// * key_data - The key data.
    /// * platform_version - The version of the platform to determine which method to delegate to.
    ///
    /// # Returns
    ///
    /// * Result<[u8; 32], Error> - Returns a 32 byte hash if successful. Otherwise, returns an error.
    pub(in crate::execution::platform_events::core_based_updates) fn hash_protxhash_with_key_data(
        pro_tx_hash: &[u8; 32],
        key_data: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<[u8; 32], Error> {
        match platform_version
            .drive_abci
            .methods
            .core_based_updates
            .masternode_updates
            .hash_protxhash_with_key_data
        {
            0 => Self::hash_protxhash_with_key_data_v0(pro_tx_hash, key_data),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "hash_protxhash_with_key_data".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
