use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use drive::grovedb::Transaction;

impl<C> Platform<C> {
    /// Retrieves the genesis time for the specified block height and block time.
    ///
    /// # Arguments
    ///
    /// * `block_height` - The block height for which to retrieve the genesis time.
    /// * `block_time_ms` - The block time in milliseconds.
    /// * `transaction` - A reference to the transaction.
    ///
    /// # Returns
    ///
    /// * `Result<u64, Error>` - The genesis time as a `u64` value on success, or an `Error` on failure.
    pub(super) fn get_genesis_time_v0(
        &self,
        block_height: u64,
        block_time_ms: u64,
        transaction: &Transaction,
    ) -> Result<u64, Error> {
        if block_height == self.config.abci.genesis_height {
            // we do not set the genesis time to the cache here,
            // instead that must be done after finalizing the block
            Ok(block_time_ms)
        } else {
            //todo: lazy load genesis time
            self.drive
                .get_genesis_time(Some(transaction))
                .map_err(Error::Drive)?
                .ok_or(Error::Execution(ExecutionError::DriveIncoherence(
                    "the genesis time must be set",
                )))
        }
    }
}
