mod v0;

use crate::drive::contract_groups::types::ContractGroup;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::identifier::Identifier;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Fetches a contract group with its information and every member, or `None` when no such
    /// group exists.
    pub fn fetch_contract_group(
        &self,
        contract_group_id: Identifier,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<ContractGroup>, Error> {
        match platform_version
            .drive
            .methods
            .contract_group
            .fetch
            .fetch_contract_group
        {
            0 => self.fetch_contract_group_v0(contract_group_id, transaction, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_contract_group".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
