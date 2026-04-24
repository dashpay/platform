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

#[cfg(test)]
mod tests {
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
    use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
    use dpp::data_contract::config::v0::DataContractConfigV0;
    use dpp::data_contract::config::DataContractConfig;
    use dpp::data_contract::group::v0::GroupV0;
    use dpp::data_contract::group::Group;
    use dpp::data_contract::v1::DataContractV1;
    use dpp::data_contract::DataContract;
    use dpp::group::action_event::GroupActionEvent;
    use dpp::group::group_action::v0::GroupActionV0;
    use dpp::group::group_action::GroupAction;
    use dpp::identifier::Identifier;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::Identity;
    use dpp::tokens::token_event::TokenEvent;
    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;

    fn setup_with_action() -> (crate::drive::Drive, Identifier, Identifier) {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let identity_1 = Identity::random_identity(3, Some(14), platform_version).unwrap();
        let identity_2 = Identity::random_identity(3, Some(506), platform_version).unwrap();
        let id_1 = identity_1.id();
        let id_2 = identity_2.id();

        let contract = DataContract::V1(DataContractV1 {
            id: Default::default(),
            version: 0,
            owner_id: Default::default(),
            document_types: Default::default(),
            config: DataContractConfig::V0(DataContractConfigV0 {
                can_be_deleted: false,
                readonly: false,
                keeps_history: false,
                documents_keep_history_contract_default: false,
                documents_mutable_contract_default: false,
                documents_can_be_deleted_contract_default: false,
                requires_identity_encryption_bounded_key: None,
                requires_identity_decryption_bounded_key: None,
            }),
            schema_defs: None,
            created_at: None,
            updated_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            created_at_epoch: None,
            updated_at_epoch: None,
            groups: BTreeMap::from([(
                0,
                Group::V0(GroupV0 {
                    members: [(id_1, 1), (id_2, 2)].into(),
                    required_power: 3,
                }),
            )]),
            tokens: BTreeMap::from([(
                0,
                TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive()),
            )]),
            keywords: Vec::new(),
            description: None,
        });
        let contract_id = contract.id();

        drive
            .insert_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .unwrap();

        let action_id = Identifier::random();
        let action = GroupAction::V0(GroupActionV0 {
            contract_id,
            proposer_id: id_1,
            token_contract_position: 0,
            event: GroupActionEvent::TokenEvent(TokenEvent::Mint(1, id_1, None)),
        });
        drive
            .add_group_action(
                contract_id,
                0,
                Some(action),
                false,
                action_id,
                id_1,
                1,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .unwrap();

        (drive, contract_id, action_id)
    }

    #[test]
    fn fetch_active_action_info_v0_nonexistent_returns_error() {
        // fetch_active_action_info uses grove_get_raw_item, which errors when
        // the item is missing (unlike _raw_optional). This exercises that error
        // path.
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let result = drive.fetch_active_action_info_v0(
            Identifier::random(),
            0,
            Identifier::random(),
            None,
            platform_version,
        );

        assert!(
            result.is_err(),
            "expected an error when action does not exist"
        );
    }

    #[test]
    fn fetch_active_action_info_and_add_operations_v0_stateless_returns_none() {
        // In stateless (approximate_without_state_for_costs=true) mode the
        // function returns Ok(None) without deserializing.
        let (drive, contract_id, action_id) = setup_with_action();
        let platform_version = PlatformVersion::latest();

        let mut ops = vec![];
        let result = drive
            .fetch_active_action_info_and_add_operations_v0(
                contract_id,
                0,
                action_id,
                true, // approximate (stateless)
                None,
                &mut ops,
                platform_version,
            )
            .expect("stateless path must succeed");

        assert!(result.is_none(), "stateless mode yields None");
        assert!(
            !ops.is_empty(),
            "stateless path still records a read operation"
        );
    }

    #[test]
    fn fetch_active_action_info_and_add_operations_v0_stateful_returns_action() {
        // Stateful path must deserialize the stored GroupAction.
        let (drive, contract_id, action_id) = setup_with_action();
        let platform_version = PlatformVersion::latest();

        let mut ops = vec![];
        let result = drive
            .fetch_active_action_info_and_add_operations_v0(
                contract_id,
                0,
                action_id,
                false,
                None,
                &mut ops,
                platform_version,
            )
            .expect("stateful fetch must succeed");

        assert!(result.is_some());
    }
}
