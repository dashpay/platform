use crate::abci::AbciError;
use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dashcore::signer::sign;
use dpp::bls_signatures;
use dpp::bls_signatures::Serialize;
use dpp::validation::{SimpleValidationResult, ValidationResult};
use drive::grovedb::Transaction;
use tenderdash_abci::proto::abci::CommitInfo;
use tenderdash_abci::proto::types::BlockId;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    pub(crate) fn get_genesis_time(
        &self,
        block_height: u64,
        block_time_ms: u64,
        transaction: &Transaction,
    ) -> Result<u64, Error> {
        if block_height == self.config.abci.genesis_height as u64 {
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
