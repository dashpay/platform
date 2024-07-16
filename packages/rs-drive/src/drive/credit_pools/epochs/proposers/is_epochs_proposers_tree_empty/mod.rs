mod v0;

use grovedb::TransactionArg;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::block::epoch::Epoch;
use dpp::version::drive_versions::DriveVersion;

impl Drive {
    /// Returns true if the Epoch's Proposers Tree is empty
    ///
    /// # Arguments
    ///
    /// * `epoch_tree` - An Epoch instance.
    /// * `transaction` - A TransactionArg instance.
    /// * `drive_version` - A DriveVersion instance representing the version of the drive.
    ///
    /// # Returns
    ///
    /// A Result containing a boolean indicating whether the proposer's tree is empty, or an Error.
    pub fn is_epochs_proposers_tree_empty(
        &self,
        epoch_tree: &Epoch,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<bool, Error> {
        match drive_version
            .methods
            .credit_pools
            .epochs
            .is_epochs_proposers_tree_empty
        {
            0 => self.is_epochs_proposers_tree_empty_v0(epoch_tree, transaction, drive_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "is_epochs_proposers_tree_empty".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::epoch::Epoch;
    use dpp::version::drive_versions::DriveVersion;

    #[test]
    fn test_check_if_empty() {
        let drive = setup_drive_with_initial_state_structure();
        let drive_version = DriveVersion::default();
        let transaction = drive.grove.start_transaction();

        let epoch = Epoch::new(0).unwrap();

        let result = drive
            .is_epochs_proposers_tree_empty(&epoch, Some(&transaction), &drive_version)
            .expect("should check if tree is empty");

        assert!(result);
    }
}
