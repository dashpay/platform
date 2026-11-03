mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Reads a contract's version from the version item stored beside it (`[64, id, 2] / 64`)
    /// without deserializing the contract.
    ///
    /// The item exists from protocol version 14 (written on every contract create and
    /// update, and backfilled on the version's first block). Before that, and for an id no
    /// contract has, the result is `None`.
    ///
    /// # Arguments
    ///
    /// * `contract_id` - The contract id.
    /// * `transaction` - The transaction to read in, or `None` for the committed state.
    /// * `platform_version` - The platform version for version dispatch.
    ///
    /// # Returns
    ///
    /// The stored version number, or `None` when there is no version item.
    ///
    /// # Errors
    ///
    /// Returns an error if the item is not a four-byte item, the read fails, or the method
    /// version is unknown.
    pub fn fetch_contract_version(
        &self,
        contract_id: [u8; 32],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<u32>, Error> {
        match platform_version
            .drive
            .methods
            .contract
            .get
            .fetch_contract_version
        {
            0 => self.fetch_contract_version_v0(contract_id, transaction, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_contract_version".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
