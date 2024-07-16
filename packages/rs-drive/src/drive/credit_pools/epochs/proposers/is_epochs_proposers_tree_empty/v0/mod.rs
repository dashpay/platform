use grovedb::TransactionArg;

use crate::drive::credit_pools::epochs::paths::EpochProposers;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::block::epoch::Epoch;
use platform_version::version::drive_versions::DriveVersion;

impl Drive {
    /// Returns true if the Epoch's Proposers Tree is empty
    pub(super) fn is_epochs_proposers_tree_empty_v0(
        &self,
        epoch_tree: &Epoch,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<bool, Error> {
        match self
            .grove
            .is_empty_tree(
                &epoch_tree.get_proposers_path(),
                transaction,
                &drive_version.grove_version,
            )
            .unwrap()
        {
            Ok(result) => Ok(result),
            Err(grovedb::Error::PathNotFound(_) | grovedb::Error::PathParentLayerNotFound(_)) => {
                Ok(true)
            }
            Err(_) => Err(Error::Drive(DriveError::CorruptedCodeExecution(
                "internal grovedb error",
            ))),
        }
    }
}
