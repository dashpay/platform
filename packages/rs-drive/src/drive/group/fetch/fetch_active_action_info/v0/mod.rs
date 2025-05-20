use crate::drive::group::paths::{group_active_action_path, ACTION_INFO_KEY};
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::DirectQueryType;
use crate::util::grove_operations::QueryTarget::QueryTargetValue;
use dpp::data_contract::GroupContractPosition;
use dpp::group::group_action::GroupAction;
use dpp::identifier::Identifier;
use dpp::serialization::PlatformDeserializable;
use dpp::version::PlatformVersion;
use grovedb::{TransactionArg, TreeType};

impl Drive {
    pub(super) fn fetch_active_action_info_v0(
        &self,
        contract_id: Identifier,
        group_contract_position: GroupContractPosition,
        action_id: Identifier,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<GroupAction, Error> {
        let group_contract_position_bytes = group_contract_position.to_be_bytes().to_vec();
        // Construct the GroveDB path for the action signers
        let path = group_active_action_path(
            contract_id.as_ref(),
            &group_contract_position_bytes,
            action_id.as_ref(),
        );

        let value = self.grove_get_raw_item(
            (&path).into(),
            ACTION_INFO_KEY,
            DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut vec![],
            &platform_version.drive,
        )?;

        let group_action = GroupAction::deserialize_from_bytes(&value)?;

        Ok(group_action)
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn fetch_active_action_info_and_add_operations_v0(
        &self,
        contract_id: Identifier,
        group_contract_position: GroupContractPosition,
        action_id: Identifier,
        approximate_without_state_for_costs: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<GroupAction>, Error> {
        let group_contract_position_bytes = group_contract_position.to_be_bytes().to_vec();
        // Construct the GroveDB path for the action signers
        let path = group_active_action_path(
            contract_id.as_ref(),
            &group_contract_position_bytes,
            action_id.as_ref(),
        );

        // no estimated_costs_only_with_layer_info, means we want to apply to state
        let direct_query_type = if !approximate_without_state_for_costs {
            DirectQueryType::StatefulDirectQuery
        } else {
            DirectQueryType::StatelessDirectQuery {
                in_tree_type: TreeType::NormalTree,
                query_target: QueryTargetValue(40),
            }
        };

        let value = self.grove_get_raw_item(
            (&path).into(),
            ACTION_INFO_KEY,
            direct_query_type,
            transaction,
            drive_operations,
            &platform_version.drive,
        )?;

        if !approximate_without_state_for_costs {
            let group_action = GroupAction::deserialize_from_bytes(&value)?;

            Ok(Some(group_action))
        } else {
            Ok(None)
        }
    }
}
