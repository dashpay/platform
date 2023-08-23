mod v0;

use crate::drive::Drive;
use crate::error::{drive::DriveError, Error};
use dpp::version::drive_versions::DriveVersion;
use grovedb::Transaction;

impl Drive {
    /// Commits a transaction.
    ///
    /// This method checks the drive version and calls the appropriate versioned method.
    /// If an unsupported version is passed, the function will return an `Error::Drive` with a `DriveError::UnknownVersionMismatch` error.
    ///
    /// # Arguments
    ///
    /// * `transaction` - The transaction to be committed.
    /// * `drive_version` - A `DriveVersion` reference that dictates which version of the method to call.
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - On success, returns `Ok(())`. On error, returns an `Error`.
    ///
    pub fn commit_transaction(
        &self,
        transaction: Transaction,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        match drive_version.methods.operations.commit_transaction {
            0 => self.commit_transaction_v0(transaction),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "commit_transaction".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
