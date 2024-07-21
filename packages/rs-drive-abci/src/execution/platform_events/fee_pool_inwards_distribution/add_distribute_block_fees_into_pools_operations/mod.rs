mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;

use crate::execution::types::block_fees::BlockFees;
use crate::execution::types::fees_in_pools::v0::FeesInPoolsV0;
use crate::platform_types::platform::Platform;
use dpp::block::epoch::Epoch;
use dpp::fee::Credits;
use dpp::version::PlatformVersion;
use drive::util::batch::DriveOperation;

use drive::grovedb::TransactionArg;

impl<C> Platform<C> {
    /// This function is a versioned method that adds operations to a batch that update total storage fees
    /// for an epoch considering fees from a new block.
    ///
    /// # Arguments
    ///
    /// * `current_epoch`: A reference to the current epoch.
    /// * `block_fees`: A reference to the block fees.
    /// * `cached_aggregated_storage_fees`: An optional credits that are cached aggregated storage fees.
    /// * `transaction`: A GroveDB transaction argument.
    /// * `batch`: A mutable reference to the vector of Drive operations.
    /// * `platform_version`: A reference to the current platform version.
    ///
    /// # Returns
    ///
    /// The function returns a `Result` type that wraps the `FeesInPoolsV0` structure
    /// and an `Error` type. The `FeesInPoolsV0` includes information about the processing
    /// and storage fees for the block.
    ///
    /// The function uses the versioning strategy to handle different method versions. It reads the
    /// platform version number and based on that number it decides which specific version of the
    /// `add_distribute_block_fees_into_pools_operations` method to call. If the provided version is
    /// not recognized, an `UnknownVersionMismatch` error is returned.
    ///
    /// # Errors
    ///
    /// This function will return an `Error::Execution` variant with `ExecutionError::UnknownVersionMismatch`
    /// in the case when the provided version number does not match any known versions of the
    /// `add_distribute_block_fees_into_pools_operations` method.
    pub(in crate::execution::platform_events) fn add_distribute_block_fees_into_pools_operations(
        &self,
        current_epoch: &Epoch,
        block_fees: &BlockFees,
        cached_aggregated_storage_fees: Option<Credits>,
        transaction: TransactionArg,
        batch: &mut Vec<DriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<FeesInPoolsV0, Error> {
        match platform_version
            .drive_abci
            .methods
            .fee_pool_inwards_distribution
            .add_distribute_block_fees_into_pools_operations
        {
            0 => self.add_distribute_block_fees_into_pools_operations_v0(
                current_epoch,
                block_fees,
                cached_aggregated_storage_fees,
                transaction,
                batch,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "add_distribute_block_fees_into_pools_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
