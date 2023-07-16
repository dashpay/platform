mod v0;

use crate::drive::Drive;
use crate::error::{drive::DriveError, Error};
use dpp::version::drive_versions::DriveVersion;
use grovedb::Transaction;

impl Drive {
    /// Handles the rollback of a transaction.
    ///
    /// This is a versioned method that rolls back a given transaction.
    ///
    /// # Arguments
    ///
    /// * `transaction` - A `Transaction` reference that is to be rolled back.
    /// * `drive_version` - A `DriveVersion` reference that dictates which version of the method to call.
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - On success, returns `Ok(())`. On error, returns an `Error`.
    ///
    pub fn rollback_transaction(
        &self,
        transaction: &Transaction,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        match drive_version.methods.operations.rollback_transaction {
            0 => self.rollback_transaction_v0(transaction),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "rollback_transaction".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
