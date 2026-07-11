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

#[cfg(test)]
mod tests {
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::config::v0::DataContractConfigV0;
    use dpp::data_contract::config::DataContractConfig;
    use dpp::data_contract::group::v0::GroupV0;
    use dpp::data_contract::group::Group;
    use dpp::data_contract::v1::DataContractV1;
    use dpp::data_contract::DataContract;
    use dpp::identifier::Identifier;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::Identity;
    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;

    fn contract_with_single_group(identity_id: Identifier) -> DataContract {
        DataContract::V1(DataContractV1 {
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
                    members: [(identity_id, 1)].into(),
                    required_power: 1,
                }),
            )]),
            tokens: Default::default(),
            keywords: Vec::new(),
            description: None,
        })
    }

    #[test]
    fn fetch_group_info_v0_returns_none_for_nonexistent_contract() {
        // Contract was never inserted; the group actions tree has no entry
        // for this contract id, so fetch must return Ok(None).
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let group = drive
            .fetch_group_info_v0(Identifier::random(), 0, None, platform_version)
            .expect("fetch on missing contract should return Ok(None)");

        assert!(group.is_none());
    }

    #[test]
    fn fetch_group_info_and_add_operations_v0_estimated_costs_populates_ops() {
        // Exercises the StatelessDirectQuery branch used for cost estimation.
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let identity = Identity::random_identity(3, Some(14), platform_version)
            .expect("expected a platform identity");
        let contract = contract_with_single_group(identity.id());
        let contract_id = contract.id();

        drive
            .insert_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to insert contract");

        // Pass a Some(..) HashMap so the stateless query branch is taken.
        let mut estimated = Some(std::collections::HashMap::new());
        let mut ops = vec![];
        let group = drive
            .fetch_group_info_and_add_operations_v0(
                contract_id,
                0,
                &mut estimated,
                None,
                &mut ops,
                platform_version,
            )
            .expect("expected stateless fetch to succeed");

        // In stateless mode the item is not materialized from state, so group is None.
        assert!(group.is_none());
        // The stateless path still records the read operation.
        assert!(
            !ops.is_empty(),
            "stateless query should push at least one op"
        );
    }

    #[test]
    fn fetch_group_info_and_add_operations_v0_stateful_populates_ops() {
        // No estimated-costs hashmap => StatefulDirectQuery branch.
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let identity = Identity::random_identity(3, Some(14), platform_version)
            .expect("expected a platform identity");
        let contract = contract_with_single_group(identity.id());
        let contract_id = contract.id();

        drive
            .insert_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to insert contract");

        let mut estimated: Option<_> = None;
        let mut ops = vec![];
        let group = drive
            .fetch_group_info_and_add_operations_v0(
                contract_id,
                0,
                &mut estimated,
                None,
                &mut ops,
                platform_version,
            )
            .expect("expected stateful fetch to succeed");

        assert!(group.is_some(), "group should exist");
        assert!(!ops.is_empty(), "stateful query should record the read op");
    }
}
