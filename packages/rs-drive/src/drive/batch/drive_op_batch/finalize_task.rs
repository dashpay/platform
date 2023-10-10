use crate::drive::Drive;
use dpp::prelude::Identifier;
use dpp::version::PlatformVersion;

pub enum DriveOperationFinalizeTask {
    RemoveDataContractFromCache { contract_id: Identifier },
}

/// Enable callbacks for drive operations that will be called after successful execution
pub trait DriveOperationWithFinalizeTasks {
    /// Returns a finalize tasks that will be called after successful execution of the drive operation
    fn finalize_tasks(
        &self,
        platform_version: &PlatformVersion,
    ) -> Option<Vec<DriveOperationFinalizeTask>>; // Since we have it only for one operation implemeneted we don't want to extra calls and empty vectors
}

impl DriveOperationFinalizeTask {
    pub fn execute(self, drive: &Drive, _platform_version: &PlatformVersion) {
        match self {
            DriveOperationFinalizeTask::RemoveDataContractFromCache { contract_id } => {
                let mut drive_cache = drive.cache.write().unwrap();
                drive_cache.cached_contracts.remove(contract_id.to_buffer());
            }
        }
    }
}
