use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::data_contract::GroupContractPosition;
use dpp::group::group_action::GroupAction;
use dpp::group::group_action_status::GroupActionStatus;
use dpp::identifier::Identifier;
use dpp::prelude::StartAtIncluded;
use dpp::serialization::PlatformDeserializable;
use dpp::version::PlatformVersion;
use grovedb::query_result_type::QueryResultType;
use grovedb::Element::Item;
use grovedb::TransactionArg;
use std::collections::BTreeMap;

impl Drive {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn fetch_action_infos_v0(
        &self,
        contract_id: Identifier,
        group_contract_position: GroupContractPosition,
        action_status: GroupActionStatus,
        start_action_id: Option<(Identifier, StartAtIncluded)>,
        limit: Option<u16>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<Identifier, GroupAction>, Error> {
        self.fetch_action_infos_and_add_operations_v0(
            contract_id,
            group_contract_position,
            action_status,
            start_action_id,
            limit,
            transaction,
            &mut vec![],
            platform_version,
        )
    }
    #[allow(clippy::too_many_arguments)]
    pub(super) fn fetch_action_infos_and_add_operations_v0(
        &self,
        contract_id: Identifier,
        group_contract_position: GroupContractPosition,
        action_status: GroupActionStatus,
        start_action_id: Option<(Identifier, StartAtIncluded)>,
        limit: Option<u16>,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<Identifier, GroupAction>, Error> {
        let path_query = Drive::group_action_infos_query(
            contract_id.to_buffer(),
            group_contract_position,
            action_status,
            start_action_id.map(|(s, i)| (s.to_buffer(), i)),
            limit,
        );

        self.grove_get_raw_path_query(
            &path_query,
            transaction,
            QueryResultType::QueryPathKeyElementTrioResultType,
            drive_operations,
            &platform_version.drive,
        )?
        .0
        .to_path_key_elements()
        .into_iter()
        .map(|(path, _, element)| {
            let Some(last_path_component) = path.last() else {
                return Err(Error::Drive(DriveError::CorruptedDriveState(
                    "we should always have a path not be empty".to_string(),
                )));
            };
            let action_id = Identifier::from_bytes(last_path_component)?;

            match element {
                Item(value, ..) => Ok((action_id, GroupAction::deserialize_from_bytes(&value)?)),
                _ => Err(Error::Drive(DriveError::CorruptedDriveState(
                    "element should be an item representing the group action".to_string(),
                ))),
            }
        })
        .collect()
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
    use dpp::group::group_action_status::GroupActionStatus;
    use dpp::identifier::Identifier;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::Identity;
    use dpp::tokens::token_event::TokenEvent;
    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;

    fn setup_with_one_active_action() -> (crate::drive::Drive, Identifier, Identifier, Identifier) {
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
            .expect("insert contract");

        let action_id = Identifier::random();
        let action = GroupAction::V0(GroupActionV0 {
            contract_id,
            proposer_id: id_1,
            token_contract_position: 0,
            event: GroupActionEvent::TokenEvent(TokenEvent::Mint(100, id_1, None)),
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
            .expect("add group action");

        (drive, contract_id, action_id, id_1)
    }

    #[test]
    fn fetch_action_infos_v0_closed_is_empty_when_action_is_active() {
        // Only the Active branch has data — the Closed tree should be empty.
        let (drive, contract_id, _action_id, _) = setup_with_one_active_action();
        let platform_version = PlatformVersion::latest();

        let infos = drive
            .fetch_action_infos_v0(
                contract_id,
                0,
                GroupActionStatus::ActionClosed,
                None,
                Some(10),
                None,
                platform_version,
            )
            .expect("expected empty fetch");

        assert!(infos.is_empty());
    }

    #[test]
    fn fetch_action_infos_v0_nonexistent_contract_returns_empty() {
        // Query against a contract that was never inserted must return empty.
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let infos = drive
            .fetch_action_infos_v0(
                Identifier::random(),
                0,
                GroupActionStatus::ActionActive,
                None,
                Some(10),
                None,
                platform_version,
            )
            .expect("expected empty fetch");

        assert!(infos.is_empty());
    }

    #[test]
    fn fetch_action_infos_v0_zero_limit_returns_empty_with_active_data() {
        // Data exists, but limit=0 should short-circuit in the query layer.
        let (drive, contract_id, _action_id, _) = setup_with_one_active_action();
        let platform_version = PlatformVersion::latest();

        let infos = drive
            .fetch_action_infos_v0(
                contract_id,
                0,
                GroupActionStatus::ActionActive,
                None,
                Some(0),
                None,
                platform_version,
            )
            .expect("expected zero-limit fetch");

        assert!(infos.is_empty());
    }

    #[test]
    fn fetch_action_infos_and_add_operations_v0_records_ops() {
        // Ensure the _and_add_operations variant appends ops.
        let (drive, contract_id, _action_id, _) = setup_with_one_active_action();
        let platform_version = PlatformVersion::latest();

        let mut ops = vec![];
        let infos = drive
            .fetch_action_infos_and_add_operations_v0(
                contract_id,
                0,
                GroupActionStatus::ActionActive,
                None,
                Some(10),
                None,
                &mut ops,
                platform_version,
            )
            .expect("expected fetch to succeed");

        assert_eq!(infos.len(), 1);
        assert!(!ops.is_empty(), "path query should record operations");
    }
}
