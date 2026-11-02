mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Creates the trees document time to live needs if they do not exist yet: the documents
    /// expirations tree under `Misc`, and the lifetime storage fee pools under `Pools`.
    ///
    /// Called when the initial state structure of protocol version 14 is created and on the
    /// first block of protocol version 14, so a new chain and an upgraded one hold the same
    /// trees.
    ///
    /// # Parameters
    /// - `transaction`: the transaction to write in.
    /// - `platform_version`: selects the method version.
    ///
    /// # Returns
    /// `Ok(())` once the trees exist.
    pub fn insert_document_ttl_trees(
        &self,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .document
            .expiration
            .insert_document_ttl_trees
        {
            0 => self.insert_document_ttl_trees_v0(transaction, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "insert_document_ttl_trees".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
