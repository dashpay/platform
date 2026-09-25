mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Creates the documents expirations tree under `Misc` if it does not exist yet.
    ///
    /// Called when the initial state structure of protocol version 14 is created and on the
    /// first block of protocol version 14, so a new chain and an upgraded one hold the same
    /// tree.
    ///
    /// # Parameters
    /// - `transaction`: the transaction to write in.
    /// - `platform_version`: selects the method version.
    ///
    /// # Returns
    /// `Ok(())` once the tree exists.
    pub fn insert_documents_expirations_tree(
        &self,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .document
            .expiration
            .insert_documents_expirations_tree
        {
            0 => self.insert_documents_expirations_tree_v0(transaction, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "insert_documents_expirations_tree".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
