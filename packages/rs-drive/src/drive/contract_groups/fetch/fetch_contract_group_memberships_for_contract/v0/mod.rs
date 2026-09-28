use crate::drive::contract_groups::paths::contract_groups_members_path;
use crate::drive::contract_groups::types::ContractGroupMembershipsForContract;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::DirectQueryType;
use dpp::block::epoch::Epoch;
use dpp::fee::fee_result::FeeResult;
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
        self.fetch_contract_group_memberships_for_contract_add_to_operations_v0(
            contract_id,
            transaction,
            &mut vec![],
            platform_version,
        )
    }

    pub(super) fn fetch_contract_group_memberships_for_contract_with_fee_v0(
        &self,
        contract_id: Identifier,
        epoch: &Epoch,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(FeeResult, ContractGroupMembershipsForContract), Error> {
        let mut drive_operations = vec![];
        let memberships = self.fetch_contract_group_memberships_for_contract_add_to_operations_v0(
            contract_id,
            transaction,
            &mut drive_operations,
            platform_version,
        )?;
        let fee = Drive::calculate_fee(
            None,
            Some(drive_operations),
            epoch,
            self.config.epochs_per_era,
            platform_version,
            None,
        )?;
        Ok((fee, memberships))
    }

    pub(super) fn fetch_contract_group_memberships_for_contract_add_to_operations_v0(
        &self,
        contract_id: Identifier,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<ContractGroupMembershipsForContract, Error> {
        // A path query over a missing index tree is an error in GroveDB, so check first.
        let exists = self.grove_has_raw(
            (&contract_groups_members_path()).into(),
            contract_id.as_slice(),
            DirectQueryType::StatefulDirectQuery,
            transaction,
            drive_operations,
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
            drive_operations,
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
