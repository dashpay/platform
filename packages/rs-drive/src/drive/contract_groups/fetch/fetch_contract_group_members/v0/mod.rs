use crate::drive::contract_groups::paths::contract_groups_groups_path;
use crate::drive::contract_groups::types::{ContractGroupMembersPage, ContractGroupMembersQuery};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::util::grove_operations::DirectQueryType;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::query_result_type::QueryResultType;
use grovedb::TransactionArg;

impl Drive {
    pub(super) fn fetch_contract_group_members_v0(
        &self,
        contract_group_id: Identifier,
        query: &ContractGroupMembersQuery,
        limit: u16,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ContractGroupMembersPage, Error> {
        self.check_contract_group_members_limit(limit)?;

        // A path query over a missing group tree is an error in GroveDB, so check first.
        let exists = self.grove_has_raw(
            (&contract_groups_groups_path()).into(),
            contract_group_id.as_slice(),
            DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut vec![],
            &platform_version.drive,
        )?;
        if !exists {
            return Ok(ContractGroupMembersPage::empty_for(query));
        }

        let path_query =
            Self::contract_group_members_query(contract_group_id.to_buffer(), query, limit);
        let (results, _) = self.grove_get_raw_path_query(
            &path_query,
            transaction,
            QueryResultType::QueryPathKeyElementTrioResultType,
            &mut vec![],
            &platform_version.drive,
        )?;

        ContractGroupMembersPage::from_path_key_elements(query, results.to_path_key_elements())
            .map_err(|description| {
                Error::Drive(DriveError::CorruptedDriveState(format!(
                    "contract group {} members are malformed: {}",
                    contract_group_id, description
                )))
            })
    }
}
