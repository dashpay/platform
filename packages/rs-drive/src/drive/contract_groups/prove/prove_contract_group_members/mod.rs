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
    ///
    /// # Parameters
    ///
    /// * `contract_group_id`: The group's id.
    /// * `query`: The kind of member (contracts, document types or tokens) and the cursor to
    ///   continue after.
    /// * `limit`: The most entries the page holds.
    /// * `transaction`: The GroveDB transaction.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<u8>)` with the GroveDB proof of the page.
    /// * `Err(Error)` when the method version is unknown, `limit` is out of range, or proving
    ///   fails.
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
