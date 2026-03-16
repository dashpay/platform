#[cfg(feature = "server")]
mod estimated_costs;
#[cfg(feature = "server")]
mod fetch;
#[cfg(feature = "server")]
mod insert;
/// Group paths
pub mod paths;
#[cfg(feature = "server")]
mod prove;
mod queries;

#[cfg(test)]
#[cfg(feature = "server")]
mod tests {
    use crate::drive::Drive;
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

    /// Helper to create a standard test contract with groups and tokens.
    fn create_test_contract_with_groups(
        identity_1_id: Identifier,
        identity_2_id: Identifier,
    ) -> DataContract {
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
                    members: [(identity_1_id, 1), (identity_2_id, 2)].into(),
                    required_power: 3,
                }),
            )]),
            tokens: BTreeMap::from([(
                0,
                TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive()),
            )]),
            keywords: Vec::new(),
            description: None,
        })
    }

    /// Helper to insert a contract and add an action, returning IDs.
    fn setup_drive_with_contract_and_action(
    ) -> (Drive, Identifier, Identifier, Identifier, Identifier) {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let identity_1 = Identity::random_identity(3, Some(14), platform_version)
            .expect("expected a platform identity");
        let identity_2 = Identity::random_identity(3, Some(506), platform_version)
            .expect("expected a platform identity");
        let identity_1_id = identity_1.id();
        let identity_2_id = identity_2.id();

        let contract = create_test_contract_with_groups(identity_1_id, identity_2_id);
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

        let action_id = Identifier::random();
        let action = GroupAction::V0(GroupActionV0 {
            contract_id,
            proposer_id: identity_1_id,
            token_contract_position: 0,
            event: GroupActionEvent::TokenEvent(TokenEvent::Mint(100, identity_1_id, None)),
        });

        drive
            .add_group_action(
                contract_id,
                0,
                Some(action),
                false,
                action_id,
                identity_1_id,
                1,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add group action");

        (drive, contract_id, identity_1_id, identity_2_id, action_id)
    }

    // ========================================================================
    // fetch_group_info tests (covers fetch_group_info mod.rs + v0/mod.rs)
    // ========================================================================

    #[test]
    fn should_fetch_group_info_for_existing_group() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let identity_1 = Identity::random_identity(3, Some(14), platform_version)
            .expect("expected a platform identity");
        let identity_2 = Identity::random_identity(3, Some(506), platform_version)
            .expect("expected a platform identity");
        let identity_1_id = identity_1.id();
        let identity_2_id = identity_2.id();

        let contract = create_test_contract_with_groups(identity_1_id, identity_2_id);
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

        let group = drive
            .fetch_group_info(contract_id, 0, None, platform_version)
            .expect("expected to fetch group info");

        assert!(group.is_some(), "group should exist");
        let group = group.unwrap();
        assert_eq!(
            group,
            Group::V0(GroupV0 {
                members: [(identity_1_id, 1), (identity_2_id, 2)].into(),
                required_power: 3,
            })
        );
    }

    #[test]
    fn should_return_none_for_nonexistent_group_position() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let identity_1 = Identity::random_identity(3, Some(14), platform_version)
            .expect("expected a platform identity");
        let identity_2 = Identity::random_identity(3, Some(506), platform_version)
            .expect("expected a platform identity");

        let contract = create_test_contract_with_groups(identity_1.id(), identity_2.id());
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

        // Position 99 does not exist
        let group = drive
            .fetch_group_info(contract_id, 99, None, platform_version)
            .expect("expected to fetch group info");

        assert!(group.is_none(), "group should not exist at position 99");
    }

    // ========================================================================
    // fetch_group_infos tests (covers fetch_group_infos mod.rs + v0/mod.rs)
    // ========================================================================

    #[test]
    fn should_fetch_multiple_group_infos() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let identity_1 = Identity::random_identity(3, Some(14), platform_version)
            .expect("expected a platform identity");
        let identity_2 = Identity::random_identity(3, Some(506), platform_version)
            .expect("expected a platform identity");
        let identity_1_id = identity_1.id();
        let identity_2_id = identity_2.id();

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
                        members: [(identity_1_id, 1), (identity_2_id, 2)].into(),
                        required_power: 3,
                    }),
                ),
                (
                    1,
                    Group::V0(GroupV0 {
                        members: [(identity_1_id, 3)].into(),
                        required_power: 3,
                    }),
                ),
            ]),
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
            .expect("expected to insert contract");

        let groups = drive
            .fetch_group_infos(contract_id, None, Some(10), None, platform_version)
            .expect("expected to fetch group infos");

        assert_eq!(groups.len(), 2);
        assert!(groups.contains_key(&0));
        assert!(groups.contains_key(&1));
    }

    #[test]
    fn should_fetch_group_infos_with_start_position() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let identity_1 = Identity::random_identity(3, Some(14), platform_version)
            .expect("expected a platform identity");
        let identity_2 = Identity::random_identity(3, Some(506), platform_version)
            .expect("expected a platform identity");
        let identity_1_id = identity_1.id();
        let identity_2_id = identity_2.id();

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
                        members: [(identity_1_id, 1)].into(),
                        required_power: 1,
                    }),
                ),
                (
                    1,
                    Group::V0(GroupV0 {
                        members: [(identity_2_id, 2)].into(),
                        required_power: 2,
                    }),
                ),
                (
                    2,
                    Group::V0(GroupV0 {
                        members: [(identity_1_id, 1), (identity_2_id, 2)].into(),
                        required_power: 3,
                    }),
                ),
            ]),
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
            .expect("expected to insert contract");

        // Start at position 1 (exclusive), should return positions 2
        let groups = drive
            .fetch_group_infos(
                contract_id,
                Some((1, false)),
                Some(10),
                None,
                platform_version,
            )
            .expect("expected to fetch group infos");

        assert_eq!(groups.len(), 1);
        assert!(groups.contains_key(&2));
    }

    // ========================================================================
    // fetch_active_action_info tests
    // ========================================================================

    #[test]
    fn should_fetch_active_action_info() {
        let (drive, contract_id, identity_1_id, _, action_id) =
            setup_drive_with_contract_and_action();
        let platform_version = PlatformVersion::latest();

        let action = drive
            .fetch_active_action_info(contract_id, 0, action_id, None, platform_version)
            .expect("expected to fetch action info");

        match action {
            GroupAction::V0(ref action_v0) => {
                assert_eq!(action_v0.contract_id, contract_id);
                assert_eq!(action_v0.proposer_id, identity_1_id);
                assert_eq!(action_v0.token_contract_position, 0);
            }
        }
    }

    // ========================================================================
    // fetch_action_id_signers_power tests
    // ========================================================================

    #[test]
    fn should_fetch_action_signers_power() {
        let (drive, contract_id, _, _, action_id) = setup_drive_with_contract_and_action();
        let platform_version = PlatformVersion::latest();

        let power = drive
            .fetch_action_id_signers_power(contract_id, 0, action_id, None, platform_version)
            .expect("expected to fetch signers power");

        assert!(power.is_some(), "signers power should exist");
        assert_eq!(power.unwrap(), 1, "signer power should be 1");
    }

    #[test]
    fn should_return_none_signers_power_for_nonexistent_action() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let identity_1 = Identity::random_identity(3, Some(14), platform_version)
            .expect("expected a platform identity");
        let identity_2 = Identity::random_identity(3, Some(506), platform_version)
            .expect("expected a platform identity");

        let contract = create_test_contract_with_groups(identity_1.id(), identity_2.id());
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

        let power = drive
            .fetch_action_id_signers_power(
                contract_id,
                0,
                Identifier::random(),
                None,
                platform_version,
            )
            .expect("expected to fetch signers power");

        assert!(power.is_none(), "no power for nonexistent action");
    }

    // ========================================================================
    // fetch_action_id_has_signer tests
    // ========================================================================

    #[test]
    fn should_detect_existing_signer() {
        let (drive, contract_id, identity_1_id, _, action_id) =
            setup_drive_with_contract_and_action();
        let platform_version = PlatformVersion::latest();

        let has_signer = drive
            .fetch_action_id_has_signer(
                contract_id,
                0,
                action_id,
                identity_1_id,
                None,
                platform_version,
            )
            .expect("expected to check signer");

        assert!(has_signer, "identity_1 should be a signer");
    }

    #[test]
    fn should_detect_missing_signer() {
        let (drive, contract_id, _, identity_2_id, action_id) =
            setup_drive_with_contract_and_action();
        let platform_version = PlatformVersion::latest();

        // identity_2 has NOT signed
        let has_signer = drive
            .fetch_action_id_has_signer(
                contract_id,
                0,
                action_id,
                identity_2_id,
                None,
                platform_version,
            )
            .expect("expected to check signer");

        assert!(!has_signer, "identity_2 should not be a signer");
    }

    // ========================================================================
    // fetch_action_id_info_keep_serialized tests
    // ========================================================================

    #[test]
    fn should_fetch_serialized_action_info() {
        let (drive, contract_id, _, _, action_id) = setup_drive_with_contract_and_action();
        let platform_version = PlatformVersion::latest();

        let serialized = drive
            .fetch_action_id_info_keep_serialized(contract_id, 0, action_id, None, platform_version)
            .expect("expected to fetch serialized action info");

        assert!(
            !serialized.is_empty(),
            "serialized data should not be empty"
        );
    }

    // ========================================================================
    // fetch_action_is_closed tests
    // ========================================================================

    #[test]
    fn active_action_should_not_be_closed() {
        let (drive, contract_id, _, _, action_id) = setup_drive_with_contract_and_action();
        let platform_version = PlatformVersion::latest();

        let is_closed = drive
            .fetch_action_is_closed(
                contract_id,
                0,
                action_id,
                true,
                None,
                &mut vec![],
                platform_version,
            )
            .expect("expected to check if action is closed");

        assert!(!is_closed, "active action should not be closed");
    }

    // ========================================================================
    // add_group_action with closing (covers the closing branch in v0)
    // ========================================================================

    #[test]
    fn should_close_group_action_and_move_signers() {
        let (drive, contract_id, identity_1_id, identity_2_id, action_id) =
            setup_drive_with_contract_and_action();
        let platform_version = PlatformVersion::latest();

        // Add second signer to bring total power to 3 (meets required_power)
        // and close the action
        drive
            .add_group_action(
                contract_id,
                0,
                None, // no new action info, existing one will be moved
                true, // closes_group_action
                action_id,
                identity_2_id,
                2,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to close group action");

        // Verify the action is now closed
        let is_closed = drive
            .fetch_action_is_closed(
                contract_id,
                0,
                action_id,
                true,
                None,
                &mut vec![],
                platform_version,
            )
            .expect("expected to check if action is closed");

        assert!(is_closed, "action should now be closed");

        // Verify signers were moved to closed
        let closed_signers = drive
            .fetch_action_signers(
                contract_id,
                0,
                GroupActionStatus::ActionClosed,
                action_id,
                None,
                platform_version,
            )
            .expect("expected to fetch closed signers");

        // Should have both signers in the closed state
        assert!(
            closed_signers.contains_key(&identity_1_id),
            "identity_1 should be in closed signers"
        );
        assert!(
            closed_signers.contains_key(&identity_2_id),
            "identity_2 should be in closed signers"
        );
    }

    #[test]
    fn should_close_group_action_with_new_action_info() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let identity_1 = Identity::random_identity(3, Some(14), platform_version)
            .expect("expected a platform identity");
        let identity_2 = Identity::random_identity(3, Some(506), platform_version)
            .expect("expected a platform identity");
        let identity_1_id = identity_1.id();
        let identity_2_id = identity_2.id();

        // Contract with required_power = 1 so single signer can close
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
                    members: [(identity_1_id, 5), (identity_2_id, 5)].into(),
                    required_power: 5,
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
            .expect("expected to insert contract");

        let action_id = Identifier::random();
        let action = GroupAction::V0(GroupActionV0 {
            contract_id,
            proposer_id: identity_1_id,
            token_contract_position: 0,
            event: GroupActionEvent::TokenEvent(TokenEvent::Mint(500, identity_1_id, None)),
        });

        // Close immediately with action info provided
        drive
            .add_group_action(
                contract_id,
                0,
                Some(action.clone()),
                true, // close immediately
                action_id,
                identity_1_id,
                5,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to close group action immediately");

        let is_closed = drive
            .fetch_action_is_closed(
                contract_id,
                0,
                action_id,
                true,
                None,
                &mut vec![],
                platform_version,
            )
            .expect("expected to check if action is closed");

        assert!(is_closed, "action should be closed");
    }

    // ========================================================================
    // add_new_groups with existing contract (covers the "not inserted" branch)
    // ========================================================================

    #[test]
    fn should_add_groups_to_existing_contract_tree() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let identity_1 = Identity::random_identity(3, Some(14), platform_version)
            .expect("expected a platform identity");
        let identity_2 = Identity::random_identity(3, Some(506), platform_version)
            .expect("expected a platform identity");
        let identity_1_id = identity_1.id();
        let identity_2_id = identity_2.id();

        let contract = create_test_contract_with_groups(identity_1_id, identity_2_id);
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

        // Now add a second group to the same contract
        let new_groups = BTreeMap::from([(
            1u16,
            Group::V0(GroupV0 {
                members: [(identity_1_id, 2)].into(),
                required_power: 2,
            }),
        )]);

        drive
            .add_new_groups(
                contract_id,
                &new_groups,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add new groups to existing contract");

        // Verify both groups exist
        let fetched = drive
            .fetch_group_infos(contract_id, None, Some(10), None, platform_version)
            .expect("expected to fetch group infos");

        assert_eq!(fetched.len(), 2);
        assert!(fetched.contains_key(&0));
        assert!(fetched.contains_key(&1));
    }

    // ========================================================================
    // Cost estimation tests (apply=false covers estimated_costs modules)
    // ========================================================================

    #[test]
    fn should_estimate_costs_for_add_group_action() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let identity_1 = Identity::random_identity(3, Some(14), platform_version)
            .expect("expected a platform identity");
        let identity_2 = Identity::random_identity(3, Some(506), platform_version)
            .expect("expected a platform identity");

        let contract = create_test_contract_with_groups(identity_1.id(), identity_2.id());
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

        let action_id = Identifier::random();
        let action = GroupAction::V0(GroupActionV0 {
            contract_id,
            proposer_id: identity_1.id(),
            token_contract_position: 0,
            event: GroupActionEvent::TokenEvent(TokenEvent::Mint(100, identity_1.id(), None)),
        });

        // apply=false triggers cost estimation
        let fee_result = drive
            .add_group_action(
                contract_id,
                0,
                Some(action),
                false, // not closing
                action_id,
                identity_1.id(),
                1,
                &BlockInfo::default(),
                false, // apply=false = cost estimation only
                None,
                platform_version,
            )
            .expect("expected cost estimation to succeed");

        assert!(
            fee_result.storage_fee > 0 || fee_result.processing_fee > 0,
            "fee result should contain fees"
        );
    }

    #[test]
    fn should_estimate_costs_for_add_new_groups() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let identity_1 = Identity::random_identity(3, Some(14), platform_version)
            .expect("expected a platform identity");
        let identity_1_id = identity_1.id();

        let groups = BTreeMap::from([(
            0u16,
            Group::V0(GroupV0 {
                members: [(identity_1_id, 1)].into(),
                required_power: 1,
            }),
        )]);

        let contract_id = Identifier::random();

        let fee_result = drive
            .add_new_groups(
                contract_id,
                &groups,
                &BlockInfo::default(),
                false, // apply=false = cost estimation
                None,
                platform_version,
            )
            .expect("expected cost estimation to succeed");

        assert!(
            fee_result.storage_fee > 0 || fee_result.processing_fee > 0,
            "fee result should contain fees"
        );
    }

    #[test]
    fn should_estimate_costs_for_closing_group_action() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let identity_1 = Identity::random_identity(3, Some(14), platform_version)
            .expect("expected a platform identity");
        let identity_2 = Identity::random_identity(3, Some(506), platform_version)
            .expect("expected a platform identity");

        let contract = create_test_contract_with_groups(identity_1.id(), identity_2.id());
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

        let action_id = Identifier::random();
        let action = GroupAction::V0(GroupActionV0 {
            contract_id,
            proposer_id: identity_1.id(),
            token_contract_position: 0,
            event: GroupActionEvent::TokenEvent(TokenEvent::Mint(100, identity_1.id(), None)),
        });

        // Estimate costs for closing action (apply=false, closes_group_action=true)
        let fee_result = drive
            .add_group_action(
                contract_id,
                0,
                Some(action),
                true, // closing
                action_id,
                identity_1.id(),
                3,
                &BlockInfo::default(),
                false, // apply=false = cost estimation only
                None,
                platform_version,
            )
            .expect("expected cost estimation to succeed");

        assert!(
            fee_result.storage_fee > 0 || fee_result.processing_fee > 0,
            "fee result should contain fees for closing"
        );
    }

    // ========================================================================
    // prove through public API (covers prove mod.rs version dispatch)
    // ========================================================================

    #[test]
    fn should_prove_group_info_through_public_api() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let identity_1 = Identity::random_identity(3, Some(14), platform_version)
            .expect("expected a platform identity");
        let identity_2 = Identity::random_identity(3, Some(506), platform_version)
            .expect("expected a platform identity");

        let contract = create_test_contract_with_groups(identity_1.id(), identity_2.id());
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

        let proof = drive
            .prove_group_info(contract_id, 0, None, platform_version)
            .expect("should prove group info through public API");

        let (root_hash, group) =
            Drive::verify_group_info(proof.as_slice(), contract_id, 0, false, platform_version)
                .expect("expected proof verification to succeed");

        assert!(!root_hash.is_empty());
        assert!(group.is_some());
    }

    #[test]
    fn should_prove_group_infos_through_public_api() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let identity_1 = Identity::random_identity(3, Some(14), platform_version)
            .expect("expected a platform identity");
        let identity_2 = Identity::random_identity(3, Some(506), platform_version)
            .expect("expected a platform identity");

        let contract = create_test_contract_with_groups(identity_1.id(), identity_2.id());
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

        let proof = drive
            .prove_group_infos(contract_id, None, Some(10), None, platform_version)
            .expect("should prove group infos through public API");

        let (_, groups): (
            _,
            BTreeMap<dpp::data_contract::GroupContractPosition, Group>,
        ) = Drive::verify_group_infos_in_contract(
            proof.as_slice(),
            contract_id,
            None,
            Some(10),
            false,
            platform_version,
        )
        .expect("expected proof verification to succeed");

        assert_eq!(groups.len(), 1);
    }

    #[test]
    fn should_prove_action_signers_through_public_api() {
        let (drive, contract_id, identity_1_id, _, action_id) =
            setup_drive_with_contract_and_action();
        let platform_version = PlatformVersion::latest();

        let proof = drive
            .prove_action_signers(
                contract_id,
                0,
                GroupActionStatus::ActionActive,
                action_id,
                None,
                platform_version,
            )
            .expect("should prove action signers through public API");

        let (_, signers): (
            _,
            BTreeMap<Identifier, dpp::data_contract::group::GroupMemberPower>,
        ) = Drive::verify_action_signers(
            proof.as_slice(),
            contract_id,
            0,
            GroupActionStatus::ActionActive,
            action_id,
            false,
            platform_version,
        )
        .expect("expected proof verification to succeed");

        assert_eq!(signers.len(), 1);
        assert!(signers.contains_key(&identity_1_id));
    }

    // ========================================================================
    // Second signer addition (non-initialization, existing action)
    // ========================================================================

    #[test]
    fn should_add_second_signer_to_existing_action() {
        let (drive, contract_id, identity_1_id, identity_2_id, action_id) =
            setup_drive_with_contract_and_action();
        let platform_version = PlatformVersion::latest();

        // Add second signer without closing
        drive
            .add_group_action(
                contract_id,
                0,
                None,  // action already exists
                false, // not closing
                action_id,
                identity_2_id,
                2,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add second signer");

        let signers = drive
            .fetch_action_signers(
                contract_id,
                0,
                GroupActionStatus::ActionActive,
                action_id,
                None,
                platform_version,
            )
            .expect("expected to fetch signers");

        assert_eq!(signers.len(), 2);
        assert_eq!(signers[&identity_1_id], 1);
        assert_eq!(signers[&identity_2_id], 2);

        // Verify total power
        let power = drive
            .fetch_action_id_signers_power(contract_id, 0, action_id, None, platform_version)
            .expect("expected to fetch signers power");

        assert_eq!(power.unwrap(), 3);
    }

    // ========================================================================
    // fetch_action_id_has_signer_with_costs
    // ========================================================================

    #[test]
    fn should_fetch_has_signer_with_costs() {
        let (drive, contract_id, identity_1_id, _, action_id) =
            setup_drive_with_contract_and_action();
        let platform_version = PlatformVersion::latest();

        let (has_signer, fee_result) = drive
            .fetch_action_id_has_signer_with_costs(
                contract_id,
                0,
                action_id,
                identity_1_id,
                &BlockInfo::default(),
                None,
                platform_version,
            )
            .expect("expected to check signer with costs");

        assert!(has_signer, "identity_1 should be a signer");
        // Fee result should be computed
        assert!(fee_result.processing_fee > 0);
    }

    // ========================================================================
    // fetch_action_is_closed with apply=false (stateless)
    // ========================================================================

    #[test]
    fn should_check_action_is_closed_stateless() {
        let (drive, contract_id, _, _, action_id) = setup_drive_with_contract_and_action();
        let platform_version = PlatformVersion::latest();

        let is_closed = drive
            .fetch_action_is_closed(
                contract_id,
                0,
                action_id,
                false, // stateless
                None,
                &mut vec![],
                platform_version,
            )
            .expect("expected to check if action is closed");

        // Stateless mode returns false since we don't check active state
        assert!(!is_closed);
    }

    // ========================================================================
    // prove_action_infos through public API
    // ========================================================================

    #[test]
    fn should_prove_action_infos_through_public_api() {
        let (drive, contract_id, _, _, action_id) = setup_drive_with_contract_and_action();
        let platform_version = PlatformVersion::latest();

        let proof = drive
            .prove_action_infos(
                contract_id,
                0,
                GroupActionStatus::ActionActive,
                None,
                Some(10),
                None,
                platform_version,
            )
            .expect("should prove action infos through public API");

        let (_, actions): (_, BTreeMap<Identifier, GroupAction>) =
            Drive::verify_action_infos_in_contract(
                proof.as_slice(),
                contract_id,
                0,
                GroupActionStatus::ActionActive,
                None,
                Some(10),
                false,
                platform_version,
            )
            .expect("expected proof verification to succeed");

        assert_eq!(actions.len(), 1);
        assert!(actions.contains_key(&action_id));
    }
}
