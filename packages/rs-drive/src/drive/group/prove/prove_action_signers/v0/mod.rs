use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::data_contract::GroupContractPosition;
use dpp::group::group_action_status::GroupActionStatus;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    pub(super) fn prove_action_signers_v0(
        &self,
        contract_id: Identifier,
        group_contract_position: GroupContractPosition,
        action_status: GroupActionStatus,
        action_id: Identifier,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        self.prove_action_signers_operations_v0(
            contract_id,
            group_contract_position,
            action_status,
            action_id,
            transaction,
            &mut vec![],
            platform_version,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn prove_action_signers_operations_v0(
        &self,
        contract_id: Identifier,
        group_contract_position: GroupContractPosition,
        action_status: GroupActionStatus,
        action_id: Identifier,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let path_query = Drive::group_action_signers_query(
            contract_id.to_buffer(),
            group_contract_position,
            action_status,
            action_id.to_buffer(),
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
    use dpp::data_contract::group::{Group, GroupMemberPower};
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
    fn should_prove_action_signers_with_multiple_actions() {
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
                        members: [(identity_1_id, 1), (identity_2_id, 1), (identity_3_id, 1)]
                            .into(),
                        required_power: 2,
                    }),
                ),
                (
                    1,
                    Group::V0(GroupV0 {
                        members: [(identity_1_id, 2), (identity_2_id, 1), (identity_3_id, 1)]
                            .into(),
                        required_power: 3,
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
            token_contract_position: 1,
            event: GroupActionEvent::TokenEvent(TokenEvent::Burn(50, identity_2_id, None)),
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
                Some(action_1.clone()),
                false,
                action_id_1,
                identity_2_id,
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
            .fetch_action_signers(
                contract_id,
                group_contract_position,
                GroupActionStatus::ActionActive,
                action_id_1,
                None,
                platform_version,
            )
            .expect("expected to fetch active actions");

        assert_eq!(
            fetched_actions,
            BTreeMap::from([(identity_1_id, 1), (identity_2_id, 1)]),
            "unexpected fetched powers"
        );

        // Prove actions
        let proof = drive
            .prove_action_signers_v0(
                contract_id,
                group_contract_position,
                GroupActionStatus::ActionActive,
                action_id_1,
                None,
                platform_version,
            )
            .expect("should not error when proving active actions");

        // Verify proof
        let proved_actions: BTreeMap<Identifier, GroupMemberPower> = Drive::verify_action_signers(
            proof.as_slice(),
            contract_id,
            group_contract_position,
            GroupActionStatus::ActionActive,
            action_id_1,
            false,
            platform_version,
        )
        .expect("expected proof verification to succeed")
        .1;

        // Assert proved actions match fetched actions
        assert_eq!(proved_actions, fetched_actions, "unexpected proved actions");
    }

    #[test]
    fn should_prove_no_action_signers_for_non_existent_action() {
        let drive = setup_drive_with_initial_state_structure(None);

        let platform_version = PlatformVersion::latest();

        let identity_1 = Identity::random_identity(3, Some(14), platform_version)
            .expect("expected a platform identity");

        let identity_1_id = identity_1.id();

        let identity_2 = Identity::random_identity(3, Some(506), platform_version)
            .expect("expected a platform identity");

        let identity_2_id = identity_2.id();

        // Create a data contract with groups
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
                    members: [(identity_1_id, 1), (identity_2_id, 1)].into(),
                    required_power: 2,
                }),
            )]),
            tokens: BTreeMap::new(),
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
        let non_existent_action_id = Identifier::random();

        // Prove action signers for a non-existent action
        let proof = drive
            .prove_action_signers_v0(
                contract_id,
                group_contract_position,
                GroupActionStatus::ActionActive,
                non_existent_action_id,
                None,
                platform_version,
            )
            .expect("should not error when proving action signers for non-existent action");

        // Verify proof
        let proved_signers: BTreeMap<Identifier, GroupMemberPower> = Drive::verify_action_signers(
            proof.as_slice(),
            contract_id,
            group_contract_position,
            GroupActionStatus::ActionActive,
            non_existent_action_id,
            false,
            platform_version,
        )
        .expect("expected proof verification to succeed")
        .1;

        // Assert no signers exist
        assert!(proved_signers.is_empty(), "expected no signers");
    }

    #[test]
    fn should_prove_action_signers_for_closed_action_if_still_active() {
        let drive = setup_drive_with_initial_state_structure(None);

        let platform_version = PlatformVersion::latest();

        let identity_1 = Identity::random_identity(3, Some(14), platform_version)
            .expect("expected a platform identity");

        let identity_1_id = identity_1.id();

        let identity_2 = Identity::random_identity(3, Some(506), platform_version)
            .expect("expected a platform identity");

        let identity_2_id = identity_2.id();

        // Create a data contract with groups
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
                    members: [(identity_1_id, 1), (identity_2_id, 1)].into(),
                    required_power: 2,
                }),
            )]),
            tokens: BTreeMap::new(),
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

        // Create a closed action
        let action_id = Identifier::random();
        let action = GroupAction::V0(GroupActionV0 {
            contract_id,
            proposer_id: identity_2_id,
            token_contract_position: 0,
            event: GroupActionEvent::TokenEvent(TokenEvent::Burn(50, identity_2_id, None)),
        });

        drive
            .add_group_action(
                contract_id,
                group_contract_position,
                Some(action.clone()),
                false,
                action_id,
                identity_2_id,
                1,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add action");

        // Fetch closed action signers
        let fetched_signers = drive
            .fetch_action_signers(
                contract_id,
                group_contract_position,
                GroupActionStatus::ActionClosed,
                action_id,
                None,
                platform_version,
            )
            .expect("expected to fetch closed action signers");

        assert_eq!(
            fetched_signers,
            BTreeMap::new(),
            "unexpected fetched powers"
        );

        // Prove closed action signers
        let proof = drive
            .prove_action_signers_v0(
                contract_id,
                group_contract_position,
                GroupActionStatus::ActionClosed,
                action_id,
                None,
                platform_version,
            )
            .expect("should not error when proving closed action signers");

        // Verify proof
        let proved_signers: BTreeMap<Identifier, GroupMemberPower> = Drive::verify_action_signers(
            proof.as_slice(),
            contract_id,
            group_contract_position,
            GroupActionStatus::ActionClosed,
            action_id,
            false,
            platform_version,
        )
        .expect("expected proof verification to succeed")
        .1;

        // Assert proved signers match fetched signers
        assert_eq!(proved_signers, fetched_signers, "unexpected proved signers");
    }
}
