use crate::drive::contract_groups::paths::contract_groups_groups_path;
use crate::drive::contract_groups::types::{ContractGroup, DecodeTrust};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::util::grove_operations::DirectQueryType;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::query_result_type::QueryResultType;
use grovedb::TransactionArg;

impl Drive {
    pub(super) fn fetch_contract_group_v0(
        &self,
        contract_group_id: Identifier,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<ContractGroup>, Error> {
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
            return Ok(None);
        }

        let path_query = Self::contract_group_query(contract_group_id.to_buffer());
        let (results, _) = self.grove_get_raw_path_query(
            &path_query,
            transaction,
            QueryResultType::QueryPathKeyElementTrioResultType,
            &mut vec![],
            &platform_version.drive,
        )?;

        ContractGroup::from_path_key_elements(
            contract_group_id,
            results.to_path_key_elements(),
            DecodeTrust::Trusted,
        )
        .map_err(|description| {
            Error::Drive(DriveError::CorruptedDriveState(format!(
                "contract group {} is malformed: {}",
                contract_group_id, description
            )))
        })
    }
}
