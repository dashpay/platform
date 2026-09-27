mod v0;

use crate::drive::contract_groups::types::{ContractGroupMembersPage, ContractGroupMembersQuery};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::identifier::Identifier;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Fetches one page of a contract group's members of one kind: at most `limit` entries in
    /// key order, continuing after the query's cursor. An absent group yields an empty page;
    /// use [`Drive::fetch_contract_group_info`] to tell an absent group from an empty one.
    ///
    /// `limit` must be between 1 and the configured maximum query limit.
    pub fn fetch_contract_group_members(
        &self,
        contract_group_id: Identifier,
        query: &ContractGroupMembersQuery,
        limit: u16,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ContractGroupMembersPage, Error> {
        match platform_version
            .drive
            .methods
            .contract_group
            .fetch
            .fetch_contract_group_members
        {
            0 => self.fetch_contract_group_members_v0(
                contract_group_id,
                query,
                limit,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_contract_group_members".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
