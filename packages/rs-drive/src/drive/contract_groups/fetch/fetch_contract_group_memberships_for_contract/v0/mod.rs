use crate::drive::contract_groups::paths::contract_groups_members_path;
use crate::drive::contract_groups::types::ContractGroupMembershipsForContract;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::util::grove_operations::DirectQueryType;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::query_result_type::QueryResultType;
use grovedb::TransactionArg;

impl Drive {
    pub(super) fn fetch_contract_group_memberships_for_contract_v0(
        &self,
        contract_id: Identifier,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ContractGroupMembershipsForContract, Error> {
        // A path query over a missing index tree is an error in GroveDB, so check first.
        let exists = self.grove_has_raw(
            (&contract_groups_members_path()).into(),
            contract_id.as_slice(),
            DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut vec![],
            &platform_version.drive,
        )?;
        if !exists {
            return Ok(ContractGroupMembershipsForContract::default());
        }

        let path_query =
            Self::contract_group_memberships_for_contract_query(contract_id.to_buffer());
        let (results, _) = self.grove_get_raw_path_query(
            &path_query,
            transaction,
            QueryResultType::QueryPathKeyElementTrioResultType,
            &mut vec![],
            &platform_version.drive,
        )?;

        ContractGroupMembershipsForContract::from_path_key_elements(results.to_path_key_elements())
            .map_err(|description| {
                Error::Drive(DriveError::CorruptedDriveState(format!(
                    "contract group memberships of contract {} are malformed: {}",
                    contract_id, description
                )))
            })
    }
}
