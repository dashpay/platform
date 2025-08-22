mod v0;

use crate::drive::contract::DataContractFetchInfo;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::drive_versions::DriveVersion;
use grovedb::TransactionArg;
use std::sync::Arc;

impl Drive {
    /// Returns the contract fetch info with the given ID if it's in cache.
    ///
    /// # Arguments
    ///
    /// * `contract_id` - A 32-byte array representing the unique identifier of the contract.
    ///
    /// * `transaction` - A transaction that requests the contract.
    ///
    /// * `drive_version` - The version of the drive used to select the correct method version.
    ///
    /// # Returns
    ///
    /// * `Option<Arc<DataContractFetchInfo>>` - An `Option` wrapping an `Arc` to the `ContractFetchInfo`.
    ///   If a contract with the given ID exists in the cache, the function returns `Some(Arc<DataContractFetchInfo>)`,
    ///   otherwise it returns `None`.
    ///
    /// # Errors
    ///
    /// This function will return an `Error` if the drive version does not match any of the implemented method versions.
    pub fn get_cached_contract_with_fetch_info(
        &self,
        contract_id: [u8; 32],
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<Option<Arc<DataContractFetchInfo>>, Error> {
        match drive_version
            .methods
            .contract
            .get
            .get_cached_contract_with_fetch_info
        {
            0 => Ok(self.get_cached_contract_with_fetch_info_v0(contract_id, transaction)),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "get_cached_contract_with_fetch_info".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
