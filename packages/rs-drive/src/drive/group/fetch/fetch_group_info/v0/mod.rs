use std::collections::HashMap;

use crate::drive::group::paths::{group_path, GROUP_INFO_KEY};
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::DirectQueryType;
use crate::util::grove_operations::QueryTarget::QueryTargetValue;
use dpp::data_contract::group::Group;
use dpp::data_contract::GroupContractPosition;
use dpp::identifier::Identifier;
use dpp::serialization::PlatformDeserializable;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg, TreeType};

impl Drive {
    pub(super) fn fetch_group_info_v0(
        &self,
        contract_id: Identifier,
        group_contract_position: GroupContractPosition,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Group>, Error> {
        let group_contract_position_bytes = group_contract_position.to_be_bytes().to_vec();
        // Construct the GroveDB path for the action signers
        let path = group_path(contract_id.as_ref(), &group_contract_position_bytes);

        let maybe_group = self
            .grove_get_raw_optional_item(
                (&path).into(),
                GROUP_INFO_KEY,
                DirectQueryType::StatefulDirectQuery,
                transaction,
                &mut vec![],
                &platform_version.drive,
            )?
            .map(|value| Group::deserialize_from_bytes(&value))
            .transpose()?;

        Ok(maybe_group)
    }

    // TODO: Is not using
    #[allow(dead_code)]
    pub(super) fn fetch_group_info_and_add_operations_v0(
        &self,
        contract_id: Identifier,
        group_contract_position: GroupContractPosition,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Group>, Error> {
        let group_contract_position_bytes = group_contract_position.to_be_bytes().to_vec();
        // Construct the GroveDB path for the action signers
        let path = group_path(contract_id.as_ref(), &group_contract_position_bytes);

        // no estimated_costs_only_with_layer_info, means we want to apply to state
        let direct_query_type = if estimated_costs_only_with_layer_info.is_none() {
            DirectQueryType::StatefulDirectQuery
        } else {
            DirectQueryType::StatelessDirectQuery {
                in_tree_type: TreeType::NormalTree,
                query_target: QueryTargetValue(8),
            }
        };

        let maybe_group = self
            .grove_get_raw_optional_item(
                (&path).into(),
                GROUP_INFO_KEY,
                direct_query_type,
                transaction,
                drive_operations,
                &platform_version.drive,
            )?
            .map(|value| Group::deserialize_from_bytes(&value))
            .transpose()?;

        Ok(maybe_group)
    }
}
