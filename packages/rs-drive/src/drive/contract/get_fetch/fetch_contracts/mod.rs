mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::data_contract::DataContract;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Fetches data contracts from state with pagination.
    ///
    /// Returns deserialized `DataContract` objects in lexicographic order of their IDs.
    /// Handles both non-historical and historical contract storage layouts.
    ///
    /// # Arguments
    ///
    /// * `start_at` - Optional starting contract ID and whether it is included.
    ///   `None` starts from the beginning.
    /// * `limit` - Maximum number of contracts to return.
    /// * `transaction` - The transaction argument.
    /// * `platform_version` - The platform version for version dispatch.
    pub fn fetch_contracts(
        &self,
        start_at: Option<([u8; 32], bool)>,
        limit: u16,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<DataContract>, Error> {
        match platform_version.drive.methods.contract.get.fetch_contracts {
            0 => self.fetch_contracts_v0(start_at, limit, transaction, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_contracts".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
