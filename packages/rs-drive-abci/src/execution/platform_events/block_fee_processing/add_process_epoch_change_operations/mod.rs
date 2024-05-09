mod v0;

use dpp::version::PlatformVersion;
use drive::drive::batch::DriveOperation;
use drive::grovedb::Transaction;

use crate::error::Error;

use crate::execution::types::block_fees::BlockFees;

use crate::execution::types::storage_fee_distribution_outcome;

use crate::error::execution::ExecutionError;
use crate::execution::types::block_execution_context::BlockExecutionContext;

use crate::platform_types::platform::Platform;

impl<CoreRPCLike> Platform<CoreRPCLike> {
    /// Adds operations to the GroveDB batch which initialize the current epoch
    /// as well as the current+1000 epoch, then distributes storage fees accumulated
    /// during the previous epoch.
    ///
    /// `DistributionLeftoverCredits` will be returned, except if we are at Genesis Epoch.
    ///
    /// # Arguments
    ///
    /// * `block_info` - A `BlockStateInfo` reference that holds block state information.
    /// * `epoch_info` - A `EpochInfo` reference that holds epoch information.
    /// * `block_fees` - A `BlockFees` reference that holds block fee details.
    /// * `transaction` - A `Transaction` reference.
    /// * `batch` - A mutable reference to `Vec<DriveOperation>`, the batch of drive operations.
    /// * `platform_version` - A `PlatformVersion` reference that dictates which version of
    ///   the method to call.
    ///
    /// # Returns
    ///
    /// * `Result<Option<storage_fee_distribution_outcome::v0::StorageFeeDistributionOutcome>, Error>` -
    ///   If the operation is successful, it returns `Ok(Some(StorageFeeDistributionOutcome))`.
    ///   If there is no update, it returns `Ok(None)`. If there is an error, it returns `Error`.
    ///
    pub fn add_process_epoch_change_operations(
        &self,
        block_execution_context: &BlockExecutionContext,
        block_fees: &BlockFees,
        transaction: &Transaction,
        batch: &mut Vec<DriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<storage_fee_distribution_outcome::v0::StorageFeeDistributionOutcome>, Error>
    {
        match platform_version
            .drive_abci
            .methods
            .block_fee_processing
            .add_process_epoch_change_operations
        {
            0 => self.add_process_epoch_change_operations_v0(
                block_execution_context,
                block_fees,
                transaction,
                batch,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "add_process_epoch_change_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
