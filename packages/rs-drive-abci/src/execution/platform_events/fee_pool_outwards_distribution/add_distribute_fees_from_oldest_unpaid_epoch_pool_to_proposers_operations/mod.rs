mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;

use crate::execution::types::proposer_payouts::v0::ProposersPayouts;
use crate::platform_types::platform::Platform;

use dpp::version::PlatformVersion;
use drive::util::batch::DriveOperation;

use drive::grovedb::Transaction;

impl<C> Platform<C> {
    /// Distributes fees from the oldest unpaid epoch pool to proposers.
    ///
    /// This function is a version handler that directs to specific version implementations
    /// of the add_distribute_fees_from_oldest_unpaid_epoch_pool_to_proposers_operations function.
    ///
    /// # Arguments
    ///
    /// * `current_epoch_index` - A u16 indicating the current epoch index.
    /// * `cached_current_epoch_start_block_height` - An Option wrapping a u64 value representing the current epoch start block height.
    /// * `cached_current_epoch_start_block_core_height` - An Option wrapping a u32 value representing the current epoch start block core height.
    /// * `transaction` - A Transaction reference.
    /// * `batch` - A mutable reference to a vector of DriveOperation.
    /// * `platform_version` - A PlatformVersion reference that dictates which version of the method to call.
    ///
    /// # Returns
    ///
    /// * `Result<Option<proposer_payouts::ProposersPayouts>, Error>` - Returns a Result wrapping an optional ProposersPayouts value if the operation is successful, otherwise returns an Error.
    pub(in crate::execution::platform_events) fn add_distribute_fees_from_oldest_unpaid_epoch_pool_to_proposers_operations(
        &self,
        current_epoch_index: u16,
        cached_current_epoch_start_block_height: Option<u64>,
        cached_current_epoch_start_block_core_height: Option<u32>,
        transaction: &Transaction,
        batch: &mut Vec<DriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<ProposersPayouts>, Error> {
        match platform_version
            .drive_abci
            .methods
            .fee_pool_outwards_distribution
            .add_distribute_fees_from_oldest_unpaid_epoch_pool_to_proposers_operations
        {
            0 => self.add_distribute_fees_from_oldest_unpaid_epoch_pool_to_proposers_operations_v0(
                current_epoch_index,
                cached_current_epoch_start_block_height,
                cached_current_epoch_start_block_core_height,
                transaction,
                batch,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "add_distribute_fees_from_oldest_unpaid_epoch_pool_to_proposers_operations"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
