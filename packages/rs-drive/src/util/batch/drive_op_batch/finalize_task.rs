use crate::drive::shielded::paths::{
    shielded_anchors_credit_pool_path, shielded_credit_pool_path, SHIELDED_COMMITMENTS_KEY,
};
use crate::drive::Drive;
use crate::error::Error;
use dpp::prelude::Identifier;
use dpp::version::PlatformVersion;
use grovedb::{Element, TransactionArg};

#[derive(Clone, Debug)]
pub enum DriveOperationFinalizeTask {
    RemoveDataContractFromCache {
        contract_id: Identifier,
    },
    /// Record the current commitment tree root hash as a new anchor
    RecordShieldedAnchor,
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
    pub fn execute(
        self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match self {
            DriveOperationFinalizeTask::RemoveDataContractFromCache { contract_id } => {
                drive.cache.data_contracts.remove(contract_id.to_buffer());
                Ok(())
            }
            DriveOperationFinalizeTask::RecordShieldedAnchor => {
                let grove_version = &platform_version.drive.grove_version;

                // Get the current root hash of the commitment tree
                let root_hash = drive
                    .grove
                    .commitment_tree_root_hash(
                        &shielded_credit_pool_path(),
                        &[SHIELDED_COMMITMENTS_KEY],
                        transaction,
                        grove_version,
                    )
                    .unwrap()
                    .map_err(Error::from)?;

                // Insert the root hash as an empty Item into the anchors tree
                drive
                    .grove
                    .insert(
                        &shielded_anchors_credit_pool_path(),
                        &root_hash,
                        Element::Item(vec![], None),
                        None,
                        transaction,
                        grove_version,
                    )
                    .unwrap()
                    .map_err(Error::from)?;

                Ok(())
            }
        }
    }
}
