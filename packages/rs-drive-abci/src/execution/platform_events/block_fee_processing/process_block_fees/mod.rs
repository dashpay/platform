mod v0;

use dpp::version::PlatformVersion;

use drive::grovedb::Transaction;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::block_execution_context::BlockExecutionContext;

use crate::execution::types::block_fees::BlockFees;

use crate::execution::types::processed_block_fees_outcome;

use crate::platform_types::platform::Platform;

impl<CoreRPCLike> Platform<CoreRPCLike> {
    /// Adds operations to GroveDB op batch related to processing
    /// and distributing the block fees from the previous block and applies the batch.
    ///
    /// Returns `ProcessedBlockFeesOutcome`.
    ///
    /// # Arguments
    ///
    /// * `block_info` - A `BlockStateInfo` reference that holds block state information.
    /// * `epoch_info` - A `EpochInfo` reference that holds epoch information.
    /// * `block_fees` - A `BlockFees` instance that holds block fee details.
    /// * `transaction` - A `Transaction` reference.
    /// * `platform_version` - A `PlatformVersion` reference that dictates which version of
    ///   the method to call.
    ///
    /// # Returns
    ///
    /// * `Result<processed_block_fees_outcome::v0::ProcessedBlockFeesOutcome, Error>` -
    ///   If the operation is successful, it returns `Ok(ProcessedBlockFeesOutcome)`. If there is an error, it returns `Error`.
    ///
    pub fn process_block_fees(
        &self,
        block_execution_context: &BlockExecutionContext,
        block_fees: BlockFees,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<processed_block_fees_outcome::v0::ProcessedBlockFeesOutcome, Error> {
        match platform_version
            .drive_abci
            .methods
            .block_fee_processing
            .process_block_fees
        {
            0 => self.process_block_fees_v0(
                block_execution_context,
                block_fees,
                transaction,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "process_block_fees".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
