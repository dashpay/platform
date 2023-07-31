mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use dpp::version::PlatformVersion;
use drive::grovedb::Transaction;

impl<C> Platform<C> {
    /// Retrieves the genesis time for the specified block height and block time.
    ///
    /// The genesis time is the timestamp of the first block (the 'genesis block') of the blockchain.
    /// This function uses versioning to allow changes in the implementation while maintaining
    /// backward compatibility. The genesis time is critical for validating the chronological order
    /// of the blockchain and also for timestamping transactions.
    ///
    /// # Arguments
    ///
    /// * `block_height`: The block height for which to retrieve the genesis time.
    /// * `block_time_ms`: The block time in milliseconds.
    /// * `transaction`: A reference to the transaction.
    /// * `platform_version`: The version of the platform.
    ///
    /// # Returns
    ///
    /// * `Result<u64, Error>` - The genesis time as a `u64` value on success, or an `Error` on failure.
    pub(crate) fn get_genesis_time(
        &self,
        block_height: u64,
        block_time_ms: u64,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<u64, Error> {
        match platform_version.drive_abci.methods.epoch.get_genesis_time {
            0 => self.get_genesis_time_v0(block_height, block_time_ms, transaction),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "get_genesis_time".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
