use crate::drive::Drive;
use crate::error::Error;
use dpp::prelude::Identifier;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

#[derive(Clone, Debug)]
pub enum DriveOperationFinalizeTask {
    /// Re-seeds the data contract cache from what state holds for the contract now that the
    /// batch has written it. See [`Drive::refresh_data_contract_cache_from_state`] for why
    /// evicting the superseded copy is not enough.
    RefreshDataContractCache { contract_id: Identifier },
}

/// Enable callbacks for drive operations that will be called after successful execution
pub trait DriveOperationFinalizationTasks {
    /// Returns a finalize tasks that will be called after successful execution of the drive operation
    fn finalization_tasks(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Vec<DriveOperationFinalizeTask>>, Error>; // Since we have it only for one operation implemented we don't want the extra calls and empty vectors
}

impl DriveOperationFinalizeTask {
    /// Runs the task once the batch is applied.
    ///
    /// `transaction` is the transaction the batch was applied in, or `None` when the batch
    /// was committed on its own: the task reads state through it, so it sees what the batch
    /// wrote.
    pub fn execute(
        self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match self {
            DriveOperationFinalizeTask::RefreshDataContractCache { contract_id } => drive
                .refresh_data_contract_cache_from_state(
                    contract_id.to_buffer(),
                    transaction,
                    platform_version,
                ),
        }
    }
}
