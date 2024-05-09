use crate::drive::Drive;
use crate::error::Error;
use dpp::prelude::Identifier;
use dpp::version::PlatformVersion;

pub enum DriveOperationFinalizeTask {
    RemoveDataContractFromCache { contract_id: Identifier },
}

/// Enable callbacks for drive operations that will be called after successful execution
pub trait DriveOperationFinalizationTasks {
    /// Returns a finalize tasks that will be called after successful execution of the drive operation
    fn finalization_tasks(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Vec<DriveOperationFinalizeTask>>, Error>; // Since we have it only for one operation implemeneted we don't want the extra calls and empty vectors
}

impl DriveOperationFinalizeTask {
    pub fn execute(self, drive: &Drive, _platform_version: &PlatformVersion) {
        match self {
            DriveOperationFinalizeTask::RemoveDataContractFromCache { contract_id } => {
                drive.cache.data_contracts.remove(contract_id.to_buffer());
            }
        }
    }
}
