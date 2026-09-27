mod v0;

use crate::drive::contract_groups::types::{ContractGroupMembersPage, ContractGroupMembersQuery};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;

impl Drive {
    /// Verifies a proof of one page of a contract group's members of one kind, built for the
    /// same query and limit.
    ///
    /// Returns the root hash and the page, empty when the group is absent or has no members of
    /// that kind after the cursor.
    pub fn verify_contract_group_members(
        proof: &[u8],
        contract_group_id: Identifier,
        query: &ContractGroupMembersQuery,
        limit: u16,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, ContractGroupMembersPage), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .contract_group
            .verify_contract_group_members
        {
            0 => Self::verify_contract_group_members_v0(
                proof,
                contract_group_id,
                query,
                limit,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_contract_group_members".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
