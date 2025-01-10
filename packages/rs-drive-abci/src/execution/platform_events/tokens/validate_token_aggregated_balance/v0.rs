use dpp::block::epoch::Epoch;
use drive::drive::Drive;
use drive::grovedb::Transaction;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::block_execution_context::BlockExecutionContext;
use crate::platform_types::platform::Platform;
use platform_version::version::PlatformVersion;

impl<CoreRPCLike> Platform<CoreRPCLike> {
    /// Adds operations to GroveDB op batch related to processing
    /// and distributing the block fees from the previous block and applies the batch.
    ///
    /// Returns `ProcessedBlockFeesOutcome`.
    #[inline(always)]
    pub(super) fn validate_token_aggregated_balance_v0(
        &self,
        block_execution_context: &BlockExecutionContext,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        if self.config.execution.verify_token_sum_trees {
            // Verify sum trees
            let credits_verified = self
                .drive
                .calculate_total_token_balance(Some(transaction), &platform_version.drive)
                .map_err(Error::Drive)?;

            if !credits_verified.ok()? {
                return Err(Error::Execution(
                    ExecutionError::CorruptedCreditsNotBalanced(format!(
                        "credits are not balanced after block execution {:?} off by {}",
                        credits_verified,
                        credits_verified
                            .total_in_trees()
                            .unwrap()
                            .abs_diff(credits_verified.total_credits_in_platform)
                    )),
                ));
            }
        }

        Ok(outcome)
    }
}
