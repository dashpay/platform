use std::collections::BTreeMap;
use grovedb::TransactionArg;
use dpp::version::drive_versions::DriveVersion;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;

mod v0;

impl Drive {
    /// Fetches a contract along with its history.
    ///
    /// # Arguments
    ///
    /// * `contract_id` - A 32-byte array representing the unique identifier of the contract.
    ///
    /// * `transaction` - A transaction that requests the contract.
    ///
    /// * `start_at_date` - A `u64` representing the timestamp in Unix Epoch format from which to
    /// start fetching the contract's history.
    ///
    /// * `limit` - An `Option<u16>` that sets the maximum number of contract history entries
    /// to return. If `None`, the limit is set to 10. Should be between 1 and 10.
    ///
    /// * `offset` - An `Option<u16>` that sets the number of contract history entries to skip
    /// before starting to return them. If `None`, no entries are skipped.
    ///
    /// * `drive_version` - The version of the drive used to select the correct method version.
    ///
    /// # Returns
    ///
    /// * `Result<BTreeMap<u64, Contract>, Error>` - A `Result` type, where `Ok` variant contains
    /// a `BTreeMap` with Unix timestamp as the key and contract as the value, representing
    /// the contract's history. The `Err` variant contains an `Error` in case of a failure.
    ///
    /// # Errors
    ///
    /// This function will return an `Error` in the following situations:
    ///
    /// * If the drive version does not match any of the implemented method versions.
    ///
    /// * If any of the parameters are invalid for querying contract history.
    ///
    /// * If the contract cannot be deserialized due to protocol errors.
    ///
    /// * If the queried contract path does not refer to a contract element.
    pub fn fetch_contract_with_history(
        &self,
        contract_id: [u8; 32],
        transaction: TransactionArg,
        start_at_date: u64,
        limit: Option<u16>,
        offset: Option<u16>,
        drive_version: &DriveVersion,
    ) -> Result<BTreeMap<u64, Contract>, Error> {
        match drive_version.methods.contract.get.fetch_contract_with_history {
            0 => self.fetch_contract_with_history_v0(contract_id, transaction, start_at_date, limit, offset, drive_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_contract_with_history".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}