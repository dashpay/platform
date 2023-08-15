mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::util::deserializer::ProtocolVersion;
use dpp::version::drive_versions::DriveVersion;

use grovedb::TransactionArg;

use nohash_hasher::IntMap;

impl Drive {
    /// Fetch versions by count for the upgrade window
    ///
    /// # Arguments
    ///
    /// * `transaction` - A `TransactionArg` object representing the transaction.
    ///
    /// # Returns
    ///
    /// * `Result<IntMap<ProtocolVersion, u64>, Error>` - If successful, returns an `Ok(IntMap<ProtocolVersion, u64>)` which contains versions and their counters. If an error occurs during the operation, returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function will return an error if the Drive version is unknown or any issue with the data reading process.
    pub fn fetch_versions_with_counter(
        &self,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<IntMap<ProtocolVersion, u64>, Error> {
        match drive_version
            .methods
            .protocol_upgrade
            .fetch_versions_with_counter
        {
            0 => self.fetch_versions_with_counter_v0(transaction, drive_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_versions_with_counter".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
