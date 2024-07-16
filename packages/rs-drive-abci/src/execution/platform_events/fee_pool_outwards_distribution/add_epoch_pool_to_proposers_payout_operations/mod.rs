use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::unpaid_epoch::UnpaidEpoch;
use crate::platform_types::platform::Platform;

use dpp::fee::Credits;

use dpp::version::PlatformVersion;
use drive::grovedb::Transaction;
use drive::util::batch::DriveOperation;

mod v0;

impl<C> Platform<C> {
    /// Adds operations to the op batch which distribute the fees from an unpaid epoch pool
    /// to the total fees to be paid out to proposers and divides amongst masternode reward shares.
    ///
    /// This function is a version handler that directs to specific version implementations
    /// of the add_epoch_pool_to_proposers_payout_operations function.
    ///
    /// # Arguments
    ///
    /// * `unpaid_epoch` - A reference to an `UnpaidEpoch`.
    /// * `core_block_rewards` - A `Credits` value representing the core block rewards.
    /// * `transaction` - A `Transaction` reference.
    /// * `batch` - A mutable reference to a vector of `DriveOperation`.
    /// * `platform_version` - A `PlatformVersion` reference that dictates which version of
    ///   the method to call.
    ///
    /// # Returns
    ///
    /// * `Result<u16, Error>` - Returns the number of proposers to be paid out if successful, otherwise returns an `Error`.
    pub(super) fn add_epoch_pool_to_proposers_payout_operations(
        &self,
        unpaid_epoch: &UnpaidEpoch,
        core_block_rewards: Credits,
        transaction: &Transaction,
        batch: &mut Vec<DriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<u16, Error> {
        match platform_version
            .drive_abci
            .methods
            .fee_pool_outwards_distribution
            .add_epoch_pool_to_proposers_payout_operations
        {
            0 => self.add_epoch_pool_to_proposers_payout_operations_v0(
                unpaid_epoch,
                core_block_rewards,
                transaction,
                batch,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "add_epoch_pool_to_proposers_payout_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
