mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;

use crate::execution::types::storage_fee_distribution_outcome::v0::StorageFeeDistributionOutcome;
use crate::platform_types::platform::Platform;
use dpp::block::epoch::EpochIndex;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;
use drive::util::batch::GroveDbOpBatch;

impl<C> Platform<C> {
    /// This function is a versioned method that adds operations to the GroveDB operation batch.
    /// It distributes storage fees from the distribution pool and subtracts any pending refunds.
    ///
    /// # Arguments
    ///
    /// * `current_epoch_index`: An index indicating the current epoch.
    /// * `transaction`: A GroveDB transaction argument.
    /// * `batch`: A mutable reference to the GroveDB operation batch.
    /// * `platform_version`: A reference to the current platform version.
    ///
    ///
    /// # Returns
    ///
    /// The function returns a `Result` type that wraps the `StorageFeeDistributionOutcome` enumeration
    /// and an `Error` type. The `StorageFeeDistributionOutcome` includes information about the
    /// distribution leftovers and the number of refunded epochs, if any.
    ///
    /// The function uses the versioning strategy to handle different method versions. It reads the
    /// platform version number and based on that number it decides which specific version of the
    /// `add_distribute_storage_fee_to_epochs_operations` method to call. If the provided version is
    /// not recognized, an `UnknownVersionMismatch` error is returned.
    ///
    /// # Errors
    ///
    /// This function will return an `Error::Execution` variant with `ExecutionError::UnknownVersionMismatch`
    /// in the case when the provided version number does not match any known versions of the
    /// `add_distribute_storage_fee_to_epochs_operations` method.
    pub(in crate::execution::platform_events) fn add_distribute_storage_fee_to_epochs_operations(
        &self,
        current_epoch_index: EpochIndex,
        transaction: TransactionArg,
        batch: &mut GroveDbOpBatch,
        platform_version: &PlatformVersion,
    ) -> Result<StorageFeeDistributionOutcome, Error> {
        match platform_version
            .drive_abci
            .methods
            .fee_pool_inwards_distribution
            .add_distribute_storage_fee_to_epochs_operations
        {
            0 => self.add_distribute_storage_fee_to_epochs_operations_v0(
                current_epoch_index,
                transaction,
                batch,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "add_distribute_storage_fee_to_epochs_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
