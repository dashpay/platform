mod v0;

use crate::drive::contract_groups::types::ContractGroupMembersQuery;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::identifier::Identifier;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Proves one page of a contract group's members of one kind: at most `limit` entries in
    /// key order, continuing after the query's cursor. An absent group proves as an empty page.
    ///
    /// `limit` must be between 1 and the configured maximum query limit, so the proof cannot
    /// grow with the size of the group.
    pub fn prove_contract_group_members(
        &self,
        contract_group_id: Identifier,
        query: &ContractGroupMembersQuery,
        limit: u16,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        match platform_version
            .drive
            .methods
            .contract_group
            .prove
            .prove_contract_group_members
        {
            0 => self.prove_contract_group_members_v0(
                contract_group_id,
                query,
                limit,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_contract_group_members".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
