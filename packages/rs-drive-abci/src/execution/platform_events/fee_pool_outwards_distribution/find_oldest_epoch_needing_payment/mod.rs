mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::unpaid_epoch;
use crate::platform_types::platform::Platform;

use dpp::version::PlatformVersion;

use drive::grovedb::TransactionArg;

impl<C> Platform<C> {
    /// Finds and returns the oldest epoch that hasn't been paid out yet.
    /// This function is a version handler that directs to specific version implementations
    /// of the `find_oldest_epoch_needing_payment` function.
    ///
    /// # Arguments
    ///
    /// * `current_epoch_index` - An index representing the current epoch.
    /// * `cached_current_epoch_start_block_height` - An optional starting block height of the current cached epoch.
    /// * `cached_current_epoch_start_block_core_height` - An optional starting core block height of the current cached epoch.
    /// * `transaction` - A `TransactionArg` reference.
    /// * `platform_version` - A `PlatformVersion` reference that dictates which version of
    ///   the method to call.
    ///
    /// # Returns
    ///
    /// * `Result<Option<unpaid_epoch::v0::UnpaidEpochV0>, Error>` - Returns the unpaid epoch needing payment if found, otherwise returns an `Error`.
    pub(in crate::execution::platform_events::fee_pool_outwards_distribution) fn find_oldest_epoch_needing_payment(
        &self,
        current_epoch_index: u16,
        cached_current_epoch_start_block_height: Option<u64>,
        cached_current_epoch_start_block_core_height: Option<u32>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<unpaid_epoch::v0::UnpaidEpochV0>, Error> {
        match platform_version
            .drive_abci
            .methods
            .fee_pool_outwards_distribution
            .find_oldest_epoch_needing_payment
        {
            0 => self.find_oldest_epoch_needing_payment_v0(
                current_epoch_index,
                cached_current_epoch_start_block_height,
                cached_current_epoch_start_block_core_height,
                transaction,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "find_oldest_epoch_needing_payment".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
