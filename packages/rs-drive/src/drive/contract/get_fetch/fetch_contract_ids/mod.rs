mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Fetches contract IDs from the DataContractDocuments root tree with pagination.
    ///
    /// # Arguments
    ///
    /// * `start_at` - Optional starting contract ID and whether it is included.
    ///   `None` starts from the beginning.
    /// * `limit` - Maximum number of contract IDs to return.
    /// * `transaction` - The transaction argument.
    /// * `platform_version` - The platform version for version dispatch.
    ///
    /// # Returns
    ///
    /// A `Vec<[u8; 32]>` of contract IDs in lexicographic order.
    pub fn fetch_contract_ids(
        &self,
        start_at: Option<([u8; 32], bool)>,
        limit: u16,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<[u8; 32]>, Error> {
        match platform_version
            .drive
            .methods
            .contract
            .get
            .fetch_contract_ids
        {
            0 => self.fetch_contract_ids_v0(start_at, limit, transaction, &platform_version.drive),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_contract_ids".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
