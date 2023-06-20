use crate::error::Error;
use crate::platform_types::block_execution_outcome;
use crate::platform_types::cleaned_abci_messages::finalized_block_cleaned_request::v0::FinalizeBlockCleanedRequest;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use drive::grovedb::Transaction;

mod v0;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Finalizes the block proposal by first validating it and then committing it to the state.
    ///
    /// This function first retrieves the block execution context and decomposes the request. It then checks
    /// if the received block matches the expected block information (height, round, hash, etc.). If everything
    /// matches, the function verifies the commit signature (if enabled) and the vote extensions. If all checks
    /// pass, the block is committed to the state.
    ///
    /// # Arguments
    ///
    /// * `request_finalize_block` - A `FinalizeBlockCleanedRequest` object containing the block proposal data.
    /// * `transaction` - A reference to a `Transaction` object.
    ///
    /// # Returns
    ///
    /// * `Result<BlockFinalizationOutcome, Error>` - If the block proposal passes all checks and is committed
    ///   to the state, it returns a `BlockFinalizationOutcome`. If any check fails, it returns an `Error`.
    ///
    pub(crate) fn finalize_block_proposal(
        &self,
        request_finalize_block: FinalizeBlockCleanedRequest,
        transaction: &Transaction,
    ) -> Result<block_execution_outcome::v0::BlockFinalizationOutcome, Error> {
        //todo: use protocol version to decide
        self.finalize_block_proposal_v0(request_finalize_block, transaction)
    }
}
