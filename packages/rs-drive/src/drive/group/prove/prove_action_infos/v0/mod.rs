use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::data_contract::GroupContractPosition;
use dpp::group::group_action_status::GroupActionStatus;
use dpp::identifier::Identifier;
use dpp::prelude::StartAtIncluded;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn prove_action_infos_v0(
        &self,
        contract_id: Identifier,
        group_contract_position: GroupContractPosition,
        action_status: GroupActionStatus,
        start_action_id: Option<(Identifier, StartAtIncluded)>,
        limit: Option<u16>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        self.prove_action_infos_operations_v0(
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
    pub(super) fn prove_action_infos_operations_v0(
        &self,
        contract_id: Identifier,
        group_contract_position: GroupContractPosition,
        action_status: GroupActionStatus,
        start_action_id: Option<(Identifier, StartAtIncluded)>,
        limit: Option<u16>,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let path_query = Drive::group_action_infos_query(
            contract_id.to_buffer(),
            group_contract_position,
            action_status,
            start_action_id.map(|(s, i)| (s.to_buffer(), i)),
            limit,
        );
        self.grove_get_proved_path_query(
            &path_query,
            transaction,
            drive_operations,
            &platform_version.drive,
        )
    }
}
#[cfg(test)]
mod tests {
    use super::*;
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

    #[test]
    fn should_prove_action_infos_with_multiple_actions() {
        let drive = setup_drive_with_initial_state_structure(None);

        let platform_version = PlatformVersion::latest();

        let identity_1 = Identity::random_identity(3, Some(14), platform_version)
            .expect("expected a platform identity");

        let identity_1_id = identity_1.id();

        let identity_2 = Identity::random_identity(3, Some(506), platform_version)
            .expect("expected a platform identity");

        let identity_2_id = identity_2.id();

        let identity_3 = Identity::random_identity(3, Some(9), platform_version)
            .expect("expected a platform identity");

        let identity_3_id = identity_3.id();

        // Create a data contract with multiple tokens
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
            groups: BTreeMap::from([
                (
                    0,
                    Group::V0(GroupV0 {
                        members: [(identity_1_id, 1), (identity_2_id, 1)].into(),
                        required_power: 2,
                    }),
                ),
                (
                    1,
                    Group::V0(GroupV0 {
                        members: [(identity_1_id, 2), (identity_2_id, 1), (identity_3_id, 1)]
                            .into(),
                        required_power: 2,
                    }),
                ),
            ]),
            tokens: BTreeMap::from([
                (
                    0,
                    TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive()),
                ),
                (
                    1,
                    TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive()),
                ),
            ]),
            keywords: Vec::new(),
            description: None,
        });

        drive
            .insert_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to insert contract");

        // Contract and group details
        let contract_id = contract.id();
        let group_contract_position = 0;

        // Create actions
        let action_id_1 = Identifier::random();
        let action_id_2 = Identifier::random();

        let action_1 = GroupAction::V0(GroupActionV0 {
            contract_id,
            proposer_id: identity_1_id,
            token_contract_position: 0,
            event: GroupActionEvent::TokenEvent(TokenEvent::Mint(100, identity_1_id, None)),
        });

        let action_2 = GroupAction::V0(GroupActionV0 {
            contract_id,
            proposer_id: identity_2_id,
            token_contract_position: 0,
            event: GroupActionEvent::TokenEvent(TokenEvent::Burn(50, identity_1_id, None)),
        });

        // Add actions using `add_group_action`
        drive
            .add_group_action(
                contract_id,
                group_contract_position,
                Some(action_1.clone()),
                false,
                action_id_1,
                identity_1_id,
                1,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add action 1");

        drive
            .add_group_action(
                contract_id,
                group_contract_position,
                Some(action_2.clone()),
                false,
                action_id_2,
                identity_2_id,
                1,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add action 2");

        // Fetch actions directly
        let fetched_actions = drive
            .fetch_action_infos(
                contract_id,
                group_contract_position,
                GroupActionStatus::ActionActive,
                None,
                Some(10),
                None,
                platform_version,
            )
            .expect("expected to fetch active actions");

        assert_eq!(
            fetched_actions,
            BTreeMap::from([
                (action_id_1, action_1.clone()),
                (action_id_2, action_2.clone())
            ]),
            "unexpected fetched actions"
        );

        // Prove actions
        let proof = drive
            .prove_action_infos_v0(
                contract_id,
                group_contract_position,
                GroupActionStatus::ActionActive,
                None,
                Some(10),
                None,
                platform_version,
            )
            .expect("should not error when proving active actions");

        // Verify proof
        let proved_actions: BTreeMap<Identifier, GroupAction> =
            Drive::verify_action_infos_in_contract(
                proof.as_slice(),
                contract_id,
                group_contract_position,
                GroupActionStatus::ActionActive,
                None,
                Some(10),
                false,
                platform_version,
            )
            .expect("expected proof verification to succeed")
            .1;

        // Assert proved actions match fetched actions
        assert_eq!(proved_actions, fetched_actions, "unexpected proved actions");
    }

    #[test]
    fn should_prove_no_action_infos_for_non_existent_contract() {
        let drive = setup_drive_with_initial_state_structure(None);

        let platform_version = PlatformVersion::latest();

        let non_existent_contract_id = Identifier::random();
        let group_contract_position = 0;

        // Prove actions for non-existent contract
        let proof = drive
            .prove_action_infos_v0(
                non_existent_contract_id,
                group_contract_position,
                GroupActionStatus::ActionActive,
                None,
                Some(10),
                None,
                platform_version,
            )
            .expect("should not error when proving active actions for non-existent contract");

        // Verify proof
        let proved_actions: BTreeMap<Identifier, GroupAction> =
            Drive::verify_action_infos_in_contract(
                proof.as_slice(),
                non_existent_contract_id,
                group_contract_position,
                GroupActionStatus::ActionActive,
                None,
                Some(10),
                false,
                platform_version,
            )
            .expect("expected proof verification to succeed")
            .1;

        // Assert no actions exist
        assert_eq!(
            proved_actions,
            BTreeMap::new(),
            "expected no proved actions"
        );
    }

    #[test]
    fn should_fetch_and_prove_action_infos_with_start_action_id() {
        let drive = setup_drive_with_initial_state_structure(None);

        let platform_version = PlatformVersion::latest();

        let identity_1 = Identity::random_identity(3, Some(14), platform_version)
            .expect("expected a platform identity");
        let identity_1_id = identity_1.id();

        let identity_2 = Identity::random_identity(3, Some(506), platform_version)
            .expect("expected a platform identity");
        let identity_2_id = identity_2.id();

        let identity_3 = Identity::random_identity(3, Some(9), platform_version)
            .expect("expected a platform identity");

        let identity_3_id = identity_3.id();

        // Create a data contract with multiple tokens
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
            groups: BTreeMap::from([
                (
                    0,
                    Group::V0(GroupV0 {
                        members: [(identity_1_id, 1), (identity_2_id, 1)].into(),
                        required_power: 2,
                    }),
                ),
                (
                    1,
                    Group::V0(GroupV0 {
                        members: [(identity_1_id, 2), (identity_2_id, 1), (identity_3_id, 1)]
                            .into(),
                        required_power: 2,
                    }),
                ),
            ]),
            tokens: BTreeMap::from([
                (
                    0,
                    TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive()),
                ),
                (
                    1,
                    TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive()),
                ),
            ]),
            keywords: Vec::new(),
            description: None,
        });

        drive
            .insert_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to insert contract");

        // Contract and group details
        let contract_id = contract.id();
        let group_contract_position = 0;

        // Create actions
        let action_id_1 = Identifier::new([1; 32]);
        let action_id_2 = Identifier::new([2; 32]);
        let action_id_3 = Identifier::new([3; 32]);
        let action_id_4 = Identifier::new([4; 32]);

        let action_1 = GroupAction::V0(GroupActionV0 {
            contract_id,
            proposer_id: identity_1_id,
            token_contract_position: 0,
            event: GroupActionEvent::TokenEvent(TokenEvent::Mint(200, identity_1_id, None)),
        });

        let action_2 = GroupAction::V0(GroupActionV0 {
            contract_id,
            proposer_id: identity_1_id,
            token_contract_position: 0,
            event: GroupActionEvent::TokenEvent(TokenEvent::Burn(50, identity_1_id, None)),
        });

        let action_3 = GroupAction::V0(GroupActionV0 {
            contract_id,
            proposer_id: identity_2_id,
            token_contract_position: 0,
            event: GroupActionEvent::TokenEvent(TokenEvent::Transfer(
                identity_2_id,
                None,
                None,
                None,
                150,
            )),
        });

        let action_4 = GroupAction::V0(GroupActionV0 {
            contract_id,
            proposer_id: identity_3_id,
            token_contract_position: 0,
            event: GroupActionEvent::TokenEvent(TokenEvent::Transfer(
                identity_3_id,
                None,
                None,
                None,
                150,
            )),
        });

        // Add actions using `add_group_action`
        drive
            .add_group_action(
                contract_id,
                group_contract_position,
                Some(action_1.clone()),
                false,
                action_id_1,
                identity_1_id,
                1,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add action 1");

        drive
            .add_group_action(
                contract_id,
                group_contract_position,
                Some(action_2.clone()),
                false,
                action_id_2,
                identity_1_id,
                1,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add action 2");

        drive
            .add_group_action(
                contract_id,
                group_contract_position,
                Some(action_3.clone()),
                false,
                action_id_3,
                identity_2_id,
                1,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add action 3");

        drive
            .add_group_action(
                contract_id,
                group_contract_position,
                Some(action_4.clone()),
                false,
                action_id_4,
                identity_3_id,
                1,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add action 3");

        // Fetch actions starting from action_id_2
        let fetched_actions = drive
            .fetch_action_infos(
                contract_id,
                group_contract_position,
                GroupActionStatus::ActionActive,
                Some((action_id_2, true)),
                Some(2),
                None,
                platform_version,
            )
            .expect("expected to fetch active actions");

        assert_eq!(
            fetched_actions,
            BTreeMap::from([
                (action_id_2, action_2.clone()),
                (action_id_3, action_3.clone())
            ]),
            "unexpected fetched actions"
        );

        // Prove actions starting from action_id_2
        let proof = drive
            .prove_action_infos_v0(
                contract_id,
                group_contract_position,
                GroupActionStatus::ActionActive,
                Some((action_id_2, true)),
                Some(2),
                None,
                platform_version,
            )
            .expect("should not error when proving active actions");

        // Verify proof
        let proved_actions: BTreeMap<Identifier, GroupAction> =
            Drive::verify_action_infos_in_contract(
                proof.as_slice(),
                contract_id,
                group_contract_position,
                GroupActionStatus::ActionActive,
                Some((action_id_2, true)),
                Some(2),
                false,
                platform_version,
            )
            .expect("expected proof verification to succeed")
            .1;

        // Assert proved actions match fetched actions
        assert_eq!(proved_actions, fetched_actions, "unexpected proved actions");
    }
}
