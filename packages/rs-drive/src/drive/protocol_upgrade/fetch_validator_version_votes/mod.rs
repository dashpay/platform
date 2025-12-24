mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::drive_versions::DriveVersion;
use std::collections::BTreeMap;

use dpp::util::deserializer::ProtocolVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Fetch validator votes for versions
    ///
    /// # Arguments
    ///
    /// * `start_protx_hash` - The first identifier to get votes from. If none is set start from the
    ///   first item by ordered hash.
    /// * `count` - How many max items to retrieve.
    /// * `transaction` - A `TransactionArg` object representing the transaction.
    ///
    /// # Returns
    ///
    /// * `Result<BTreeMap<[u8;32], ProtocolVersion>, Error>` - If successful, returns an `Ok(BTreeMap<[u8;32], ProtocolVersion>)` which contains the versions votes by validators. If an error occurs during the operation, returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function will return an error if the Drive version is unknown or any issue with the data reading process.
    pub fn fetch_validator_version_votes(
        &self,
        start_protx_hash: Option<[u8; 32]>,
        count: u16,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<BTreeMap<[u8; 32], ProtocolVersion>, Error> {
        match drive_version
            .methods
            .protocol_upgrade
            .fetch_validator_version_votes
        {
            0 => self.fetch_validator_version_votes_v0(
                start_protx_hash,
                count,
                transaction,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_validator_version_votes".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
