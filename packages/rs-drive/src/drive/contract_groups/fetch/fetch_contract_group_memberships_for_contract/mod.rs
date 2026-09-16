mod v0;

use crate::drive::contract_groups::types::ContractGroupMembershipsForContract;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::identifier::Identifier;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Fetches the contract groups a contract belongs to, as a whole, through its document
    /// types and through its tokens. Empty when the contract belongs to no group.
    pub fn fetch_contract_group_memberships_for_contract(
        &self,
        contract_id: Identifier,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ContractGroupMembershipsForContract, Error> {
        match platform_version
            .drive
            .methods
            .contract_group
            .fetch
            .fetch_contract_group_memberships_for_contract
        {
            0 => self.fetch_contract_group_memberships_for_contract_v0(
                contract_id,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_contract_group_memberships_for_contract".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
