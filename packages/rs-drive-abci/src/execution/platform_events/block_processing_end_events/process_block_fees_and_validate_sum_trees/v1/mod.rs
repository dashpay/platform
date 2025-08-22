//! Block Fees Processing.
//!
//! This module defines functions related to processing block fees upon block and
//! epoch changes.
//!
//! From the Dash Improvement Proposal:
//! For the purpose of this explanation we can trivialize that the execution of a block comprises
//! the sum of the execution of all state transitions contained within the block. In order to
//! avoid altering participating masternode identity balances every block and distribute fees
//! evenly, the concept of pools is introduced. We will also introduce the concepts of an Epoch
//! and the Epoch Era that are both covered later in this document. As the block executes state
//! transitions, processing and storage fees are accumulated, as well as a list of refunded fees
//! from various Epochs and fee multipliers. When there are no more state transitions to execute
//! we can say the block has ended its state transition execution phase. The system will then add
//! the accumulated fees to their corresponding pools, and in the case of deletion of data, remove
//! storage fees from future Epoch storage pools.

use crate::error::Error;
use crate::execution::types::block_execution_context::BlockExecutionContext;
use crate::execution::types::block_fees::BlockFees;
use crate::execution::types::processed_block_fees_outcome;
use crate::platform_types::platform::Platform;
use dpp::version::PlatformVersion;
use drive::grovedb::Transaction;

impl<CoreRPCLike> Platform<CoreRPCLike> {
    /// Adds operations to GroveDB op batch related to processing
    /// and distributing the block fees from the previous block and applies the batch.
    ///
    /// Returns `ProcessedBlockFeesOutcome`.
    ///
    /// V1 adds the validation of the token aggregated balance
    #[inline(always)]
    pub(super) fn process_block_fees_and_validate_sum_trees_v1(
        &self,
        block_execution_context: &BlockExecutionContext,
        block_fees: BlockFees,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<processed_block_fees_outcome::v0::ProcessedBlockFeesOutcome, Error> {
        let outcome = self.process_block_fees_and_validate_sum_trees_v0(
            block_execution_context,
            block_fees,
            transaction,
            platform_version,
        )?;

        self.validate_token_aggregated_balance(transaction, platform_version)?;

        Ok(outcome)
    }
}
