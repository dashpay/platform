use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::{block_execution_outcome, block_proposal};
use crate::rpc::core::CoreRPCLike;
use dpp::validation::ValidationResult;
use drive::grovedb::Transaction;

mod v0;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Runs a block proposal, either from process proposal or prepare proposal.
    ///
    /// This function takes a `BlockProposal` and a `Transaction` as input and processes the block
    /// proposal. It first validates the block proposal and then processes raw state transitions,
    /// withdrawal transactions, and block fees. It also updates the validator set.
    ///
    /// # Arguments
    ///
    /// * `block_proposal` - The block proposal to be processed.
    /// * `known_from_us` - Do we know that we made this block proposal?
    /// * `transaction` - The transaction associated with the block proposal.
    ///
    /// # Returns
    ///
    /// * `Result<ValidationResult<BlockExecutionOutcome, Error>, Error>` - If the block proposal is
    ///   successfully processed, it returns a `ValidationResult` containing the `BlockExecutionOutcome`.
    ///   If the block proposal processing fails, it returns an `Error`. Consensus errors are returned
    ///   in the `ValidationResult`, while critical system errors are returned in the `Result`.
    ///
    /// # Errors
    ///
    /// This function may return an `Error` variant if there is a problem with processing the block
    /// proposal, updating the core info, processing raw state transitions, or processing block fees.
    ///
    pub fn run_block_proposal(
        &self,
        block_proposal: block_proposal::v0::BlockProposal,
        known_from_us: bool,
        transaction: &Transaction,
    ) -> Result<ValidationResult<block_execution_outcome::v0::BlockExecutionOutcome, Error>, Error>
    {
        let state = self.state.read();
        let platform_version = state.current_platform_version()?;
        drop(state);

        let epoch_info = self.gather_epoch_info(&block_proposal, transaction, platform_version)?;

        match platform_version
            .drive_abci
            .methods
            .engine
            .run_block_proposal
        {
            0 => self.run_block_proposal_v0(
                block_proposal,
                known_from_us,
                epoch_info.into(),
                transaction,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "run_block_proposal".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
