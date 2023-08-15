use grovedb::TransactionArg;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee_pools::epochs::paths::EpochProposers;
use dpp::block::epoch::Epoch;

impl Drive {
    /// Returns true if the Epoch's Proposers Tree is empty
    pub(super) fn is_epochs_proposers_tree_empty_v0(
        &self,
        epoch_tree: &Epoch,
        transaction: TransactionArg,
    ) -> Result<bool, Error> {
        match self
            .grove
            .is_empty_tree(&epoch_tree.get_proposers_path(), transaction)
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
