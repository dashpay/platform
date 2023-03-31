use drive::grovedb::Transaction;
use crate::error::Error;
use crate::error::execution::ExecutionError;
use crate::platform::Platform;

impl<C> Platform<C> {
    pub fn get_genesis_time_or_set_if_genesis(&self, block_height: u64, block_time_ms: u64, transaction: &Transaction) -> Result<u64, Error> {
        if block_height == self.config.abci.genesis_height {
            self.drive.set_genesis_time(block_time_ms);
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