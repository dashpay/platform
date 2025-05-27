#[cfg(test)]
mod tests {
    use crate::execution::{continue_chain_for_strategy, run_chain_for_strategy};
    use crate::strategy::{ChainExecutionOutcome, ChainExecutionParameters, NetworkStrategy, StrategyRandomness, UpgradingInfo};
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::document_type::random_document::{
        DocumentFieldFillSize, DocumentFieldFillType,
    };
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::Identity;
    use dpp::platform_value::Value;
    use drive_abci::config::{ChainLockConfig, ExecutionConfig, InstantLockConfig, PlatformConfig, PlatformTestConfig, ValidatorSetConfig};
    use drive_abci::test::helpers::setup::TestPlatformBuilder;
    use platform_version::version::PlatformVersion;
    use rand::prelude::StdRng;
    use rand::SeedableRng;
    use simple_signer::signer::SimpleSigner;
    use std::collections::BTreeMap;
    use assert_matches::assert_matches;
    use dapi_grpc::platform::v0::{get_contested_resource_vote_state_request, get_contested_resource_vote_state_response, GetContestedResourceVoteStateRequest};
    use dapi_grpc::platform::v0::get_contested_resource_vote_state_request::get_contested_resource_vote_state_request_v0::ResultType;
    use dapi_grpc::platform::v0::get_contested_resource_vote_state_request::GetContestedResourceVoteStateRequestV0;
    use dapi_grpc::platform::v0::get_contested_resource_vote_state_response::{get_contested_resource_vote_state_response_v0, GetContestedResourceVoteStateResponseV0};
    use dapi_grpc::platform::v0::get_contested_resource_vote_state_response::get_contested_resource_vote_state_response_v0::FinishedVoteInfo;
    use dapi_grpc::platform::v0::get_contested_resource_vote_state_response::get_contested_resource_vote_state_response_v0::finished_vote_info::FinishedVoteOutcome;
    use dpp::block::epoch::Epoch;
    use dpp::block::extended_block_info::v0::ExtendedBlockInfoV0Getters;
    use dpp::dash_to_duffs;
    use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
    use dpp::state_transition::StateTransition;
    use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
    use drive::util::object_size_info::DataContractOwnedResolvedInfo;
    use drive::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
    use drive_abci::platform_types::platform_state::v0::PlatformStateV0Methods;
    use strategy_tests::frequency::Frequency;
    use strategy_tests::operations::{DocumentAction, DocumentOp, Operation, OperationType, ResourceVoteOp, VoteAction};
    use strategy_tests::transitions::create_state_transitions_for_identities;
    use strategy_tests::{StartIdentities, Strategy};

    #[test]
    fn run_chain_with_temporarily_disabled_contested_documents() {
        let epoch_time_length_s = 60;

        let config = PlatformConfig {
            testing_configs: PlatformTestConfig {
                block_signing: false,
                store_platform_state: false,
                block_commit_signature_verification: false,
                disable_instant_lock_signature_verification: true,
                disable_contested_documents_is_allowed_validation: false,
            },
            chain_lock: ChainLockConfig::default_100_67(),
            instant_lock: InstantLockConfig::default_100_67(),
            execution: ExecutionConfig {
                //we disable document triggers because we are using dpns and dpns needs a preorder
                use_document_triggers: false,
                epoch_time_length_s,
                ..Default::default()
            },
            block_spacing_ms: epoch_time_length_s * 1000,
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let platform_version = PlatformVersion::latest();

        let mut rng = StdRng::seed_from_u64(567);

        let mut simple_signer = SimpleSigner::default();

        let (mut identity1, keys1) = Identity::random_identity_with_main_keys_with_private_key::<
            Vec<_>,
        >(2, &mut rng, platform_version)
        .unwrap();

        simple_signer.add_keys(keys1);

        let start_identities: Vec<(Identity, Option<StateTransition>)> =
            create_state_transitions_for_identities(
                vec![&mut identity1],
                &(dash_to_duffs!(1)..=dash_to_duffs!(1)),
                &simple_signer,
                &mut rng,
                platform_version,
            )
            .into_iter()
            .map(|(identity, transition)| (identity, Some(transition)))
            .collect();

        let dpns_contract = platform
            .drive
            .cache
            .system_data_contracts
            .load_dpns()
            .as_ref()
            .clone();

        let document_type = dpns_contract
            .document_type_for_name("domain")
            .expect("expected a profile document type")
            .to_owned_document_type();

        let identity1_id = start_identities.first().unwrap().0.id();
        let document_op_1 = DocumentOp {
            contract: dpns_contract.clone(),
            action: DocumentAction::DocumentActionInsertSpecific(
                BTreeMap::from([
                    ("label".into(), "quantum".into()),
                    ("normalizedLabel".into(), "quantum".into()),
                    ("normalizedParentDomainName".into(), "dash".into()),
                    (
                        "records".into(),
                        BTreeMap::from([("identity", Value::from(identity1_id))]).into(),
                    ),
                ]),
                Some(start_identities.first().unwrap().0.id()),
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
            ),
            document_type: document_type.clone(),
        };

        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![Operation {
                    op_type: OperationType::Document(document_op_1.clone()),
                    frequency: Frequency {
                        times_per_block_range: 1..2,
                        chance_per_block: None,
                    },
                }],
                start_identities: StartIdentities {
                    hard_coded: start_identities.clone(),
                    ..Default::default()
                },
                identity_inserts: Default::default(),

                identity_contract_nonce_gaps: None,
                signer: Some(simple_signer.clone()),
            },
            total_hpmns: 100,
            extra_normal_mns: 0,
            validator_quorum_count: 24,
            chain_lock_quorum_count: 24,
            upgrading_info: None,
            proposer_strategy: Default::default(),
            rotate_quorums: false,
            failure_testing: None,
            query_testing: None,
            verify_state_transition_results: true,
            ..Default::default()
        };

        let mut voting_signer = Some(SimpleSigner::default());

        let ChainExecutionOutcome {
            abci_app,
            proposers,
            validator_quorums,
            current_validator_quorum_hash,
            instant_lock_quorums,
            current_proposer_versions,
            end_time_ms,
            identity_nonce_counter,
            identity_contract_nonce_counter,
            state_transition_results_per_block,
            identities,
            ..
        } = run_chain_for_strategy(
            &mut platform,
            2,
            strategy.clone(),
            config.clone(),
            15,
            &mut voting_signer,
            &mut None,
        );

        let platform_state = abci_app.platform.state.load();

        // On first block we have identity
        // On second block we have should have documents
        // but not in our case because we disabled contested documents
        let state_transitions_block_2 = state_transition_results_per_block
            .get(&2)
            .expect("expected to get block 2");

        // Document transaction was rejected
        assert!(state_transitions_block_2.is_empty());

        assert_eq!(platform_state.last_committed_block_epoch().index, 1);

        // Move over 2nd epochs

        let block_start = platform_state
            .last_committed_block_info()
            .as_ref()
            .unwrap()
            .basic_info()
            .height
            + 1;

        let ChainExecutionOutcome {
            abci_app,
            proposers,
            validator_quorums,
            current_validator_quorum_hash,
            instant_lock_quorums,
            current_proposer_versions,
            end_time_ms,
            identity_nonce_counter,
            identity_contract_nonce_counter,
            ..
        } = continue_chain_for_strategy(
            abci_app,
            ChainExecutionParameters {
                block_start,
                core_height_start: 1,
                block_count: 3,
                proposers,
                validator_quorums,
                current_validator_quorum_hash,
                instant_lock_quorums,
                current_proposer_versions: Some(current_proposer_versions.clone()),
                current_identity_nonce_counter: identity_nonce_counter,
                current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                current_votes: BTreeMap::default(),
                start_time_ms: 1681094380000,
                current_time_ms: end_time_ms,
                current_identities: Vec::new(),
            },
            NetworkStrategy::default(),
            config.clone(),
            StrategyRandomness::SeedEntropy(7),
        );

        let platform_state = abci_app.platform.state.load();

        assert_eq!(platform_state.last_committed_block_epoch().index, 4);

        // Insert successfully contested document

        let block_start = platform_state
            .last_committed_block_info()
            .as_ref()
            .unwrap()
            .basic_info()
            .height
            + 1;

        let strategy = NetworkStrategy {
            strategy: Strategy {
                operations: vec![Operation {
                    op_type: OperationType::Document(document_op_1.clone()),
                    frequency: Frequency {
                        times_per_block_range: 1..2,
                        chance_per_block: None,
                    },
                }],
                signer: Some(simple_signer),
                ..Default::default()
            },
            total_hpmns: 100,
            extra_normal_mns: 0,
            validator_quorum_count: 24,
            chain_lock_quorum_count: 24,
            upgrading_info: None,

            proposer_strategy: Default::default(),
            rotate_quorums: false,
            failure_testing: None,
            query_testing: None,
            verify_state_transition_results: true,
            ..Default::default()
        };

        let ChainExecutionOutcome {
            state_transition_results_per_block,
            ..
        } = continue_chain_for_strategy(
            abci_app,
            ChainExecutionParameters {
                block_start,
                core_height_start: 1,
                block_count: 1,
                proposers,
                validator_quorums,
                current_validator_quorum_hash,
                instant_lock_quorums,
                current_proposer_versions: Some(current_proposer_versions.clone()),
                current_identity_nonce_counter: identity_nonce_counter,
                current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                current_votes: BTreeMap::default(),
                start_time_ms: 1681094380000,
                current_time_ms: end_time_ms,
                current_identities: identities,
            },
            strategy,
            config.clone(),
            StrategyRandomness::SeedEntropy(7),
        );

        let state_transitions_block_6 = state_transition_results_per_block
            .get(&6)
            .expect("expected to get block 6");

        // Contested document was created
        assert_eq!(state_transitions_block_6.len(), 1);

        let (state_transition, execution_result) = state_transitions_block_6
            .first()
            .expect("expected a document insert");

        assert_matches!(state_transition, StateTransition::Batch(_));

        assert_eq!(execution_result.code, 0);
    }

    #[test]
    fn run_chain_block_two_state_transitions_conflicting_unique_index_inserted_same_block_version_8(
    ) {
        // In this test we try to insert two state transitions with the same unique index
        // We use the DPNS contract, and we insert two documents both with the same "name"
        // This is a common scenario we should see quite often
        let config = PlatformConfig {
            testing_configs: PlatformTestConfig::default_minimal_verifications(),
            chain_lock: ChainLockConfig::default_100_67(),
            instant_lock: InstantLockConfig::default_100_67(),
            execution: ExecutionConfig {
                //we disable document triggers because we are using dpns and dpns needs a preorder
                use_document_triggers: false,
                ..Default::default()
            },
            block_spacing_ms: 3000,
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .with_initial_protocol_version(8)
            .build_with_mock_rpc();

        let platform_version = PlatformVersion::get(8).expect("failed to get platform_version");

        let mut rng = StdRng::seed_from_u64(567);

        let mut simple_signer = SimpleSigner::default();

        let (mut identity1, keys1) = Identity::random_identity_with_main_keys_with_private_key::<
            Vec<_>,
        >(2, &mut rng, platform_version)
        .unwrap();

        simple_signer.add_keys(keys1);

        let (mut identity2, keys2) = Identity::random_identity_with_main_keys_with_private_key::<
            Vec<_>,
        >(2, &mut rng, platform_version)
        .unwrap();

        simple_signer.add_keys(keys2);

        let start_identities: Vec<(Identity, Option<StateTransition>)> =
            create_state_transitions_for_identities(
                vec![&mut identity1, &mut identity2],
                &(dash_to_duffs!(1)..=dash_to_duffs!(1)),
                &simple_signer,
                &mut rng,
                platform_version,
            )
            .into_iter()
            .map(|(identity, transition)| (identity, Some(transition)))
            .collect();

        let dpns_contract = platform
            .drive
            .cache
            .system_data_contracts
            .load_dpns()
            .as_ref()
            .clone();

        let document_type = dpns_contract
            .document_type_for_name("domain")
            .expect("expected a profile document type")
            .to_owned_document_type();

        let identity1_id = start_identities.first().unwrap().0.id();
        let identity2_id = start_identities.last().unwrap().0.id();
        let document_op_1 = DocumentOp {
            contract: dpns_contract.clone(),
            action: DocumentAction::DocumentActionInsertSpecific(
                BTreeMap::from([
                    ("label".into(), "quantum".into()),
                    ("normalizedLabel".into(), "quantum".into()),
                    ("normalizedParentDomainName".into(), "dash".into()),
                    (
                        "records".into(),
                        BTreeMap::from([("identity", Value::from(identity1_id))]).into(),
                    ),
                ]),
                Some(start_identities.first().unwrap().0.id()),
                DocumentFieldFillType::DoNotFillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
            ),
            document_type: document_type.clone(),
        };

        let document_op_2 = DocumentOp {
            contract: dpns_contract.clone(),
            action: DocumentAction::DocumentActionInsertSpecific(
                BTreeMap::from([
                    ("label".into(), "quantum".into()),
                    ("normalizedLabel".into(), "quantum".into()),
                    ("normalizedParentDomainName".into(), "dash".into()),
                    (
                        "records".into(),
                        BTreeMap::from([(
                            "identity",
                            Value::from(start_identities.last().unwrap().0.id()),
                        )])
                        .into(),
                    ),
                ]),
                Some(start_identities.last().unwrap().0.id()),
                DocumentFieldFillType::DoNotFillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
            ),
            document_type: document_type.clone(),
        };

        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![
                    Operation {
                        op_type: OperationType::Document(document_op_1),
                        frequency: Frequency {
                            times_per_block_range: 1..2,
                            chance_per_block: None,
                        },
                    },
                    Operation {
                        op_type: OperationType::Document(document_op_2),
                        frequency: Frequency {
                            times_per_block_range: 1..2,
                            chance_per_block: None,
                        },
                    },
                ],
                start_identities: StartIdentities {
                    hard_coded: start_identities,
                    ..Default::default()
                },
                identity_inserts: Default::default(),

                identity_contract_nonce_gaps: None,
                signer: Some(simple_signer),
            },
            total_hpmns: 100,
            extra_normal_mns: 0,
            validator_quorum_count: 24,
            chain_lock_quorum_count: 24,
            upgrading_info: None,

            proposer_strategy: Default::default(),
            rotate_quorums: false,
            failure_testing: None,
            query_testing: None,
            verify_state_transition_results: true,
            ..Default::default()
        };

        // On the first block we only have identities and contracts
        let outcome = run_chain_for_strategy(
            &mut platform,
            2,
            strategy.clone(),
            config.clone(),
            15,
            &mut None,
            &mut None,
        );

        let platform = outcome.abci_app.platform;

        let platform_state = platform.state.load();

        let state_transitions_block_2 = outcome
            .state_transition_results_per_block
            .get(&2)
            .expect("expected to get block 2");

        let first_document_insert_result = &state_transitions_block_2
            .first()
            .as_ref()
            .expect("expected a document insert")
            .1;
        assert_eq!(first_document_insert_result.code, 0);

        let second_document_insert_result = &state_transitions_block_2
            .get(1)
            .as_ref()
            .expect("expected a document insert")
            .1;

        assert_eq!(second_document_insert_result.code, 0); // we expect the second to also be insertable as they are both contested

        // Now let's run a query for the vote totals

        let config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();

        let dash_encoded = bincode::encode_to_vec(Value::Text("dash".to_string()), config)
            .expect("expected to encode the word dash");

        let quantum_encoded = bincode::encode_to_vec(Value::Text("quantum".to_string()), config)
            .expect("expected to encode the word quantum");

        let index_name = "parentNameAndLabel".to_string();

        let query_validation_result = platform
            .query_contested_resource_vote_state(
                GetContestedResourceVoteStateRequest {
                    version: Some(get_contested_resource_vote_state_request::Version::V0(
                        GetContestedResourceVoteStateRequestV0 {
                            contract_id: dpns_contract.id().to_vec(),
                            document_type_name: document_type.name().clone(),
                            index_name: index_name.clone(),
                            index_values: vec![dash_encoded.clone(), quantum_encoded.clone()],
                            result_type: ResultType::DocumentsAndVoteTally as i32,
                            allow_include_locked_and_abstaining_vote_tally: true,
                            start_at_identifier_info: None,
                            count: None,
                            prove: false,
                        },
                    )),
                },
                &platform_state,
                platform_version,
            )
            .expect("expected to execute query")
            .into_data()
            .expect("expected query to be valid");

        let get_contested_resource_vote_state_response::Version::V0(
            GetContestedResourceVoteStateResponseV0 {
                metadata: _,
                result,
            },
        ) = query_validation_result.version.expect("expected a version");

        let Some(
            get_contested_resource_vote_state_response_v0::Result::ContestedResourceContenders(
                get_contested_resource_vote_state_response_v0::ContestedResourceContenders {
                    contenders,
                    abstain_vote_tally: _,
                    lock_vote_tally: _,
                    finished_vote_info: _,
                },
            ),
        ) = result
        else {
            panic!("expected contenders")
        };

        assert_eq!(contenders.len(), 2);

        let first_contender = contenders.first().unwrap();

        let second_contender = contenders.last().unwrap();

        assert_eq!(
            first_contender.document.as_ref().map(hex::encode),
            Some("00177f2479090a0286a67d6a1f67b563b51518edd6eea0461829f7d630fd65708d29124be7e86f97e959894a67a9cc078c3e0106d4bfcfbf34bc403a4f099925b401000700000187690895980000018769089598000001876908959800077175616e74756d077175616e74756d00046461736800210129124be7e86f97e959894a67a9cc078c3e0106d4bfcfbf34bc403a4f099925b40101".to_string())
        );

        assert_eq!(
            second_contender.document.as_ref().map(hex::encode),
            Some("00490e212593a1d3cc6ae17bf107ab9cb465175e7877fcf7d085ed2fce27be11d68b8948a6801501bbe0431e3d994dcf71cf5a2a0939fe51b0e600076199aba4fb01000700000187690895980000018769089598000001876908959800077175616e74756d077175616e74756d0004646173680021018b8948a6801501bbe0431e3d994dcf71cf5a2a0939fe51b0e600076199aba4fb0100".to_string())
        );

        assert_eq!(first_contender.identifier, identity2_id.to_vec());

        assert_eq!(second_contender.identifier, identity1_id.to_vec());

        assert_eq!(first_contender.vote_count, Some(0));

        assert_eq!(second_contender.vote_count, Some(0));
    }

    #[test]
    fn run_chain_with_voting_on_conflicting_index_just_abstain_votes() {
        // In this test we try to insert two state transitions with the same unique index
        // We use the DPNS contract, and we insert two documents both with the same "name"
        // This is a common scenario we should see quite often
        let config = PlatformConfig {
            testing_configs: PlatformTestConfig::default_minimal_verifications(),
            chain_lock: ChainLockConfig::default_100_67(),
            instant_lock: InstantLockConfig::default_100_67(),
            execution: ExecutionConfig {
                //we disable document triggers because we are using dpns and dpns needs a preorder
                use_document_triggers: false,
                ..Default::default()
            },
            block_spacing_ms: 3000,
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let platform_version = PlatformVersion::latest();

        let mut rng = StdRng::seed_from_u64(567);

        let mut simple_signer = SimpleSigner::default();

        let (mut identity1, keys1) = Identity::random_identity_with_main_keys_with_private_key::<
            Vec<_>,
        >(2, &mut rng, platform_version)
        .unwrap();

        simple_signer.add_keys(keys1);

        let (mut identity2, keys2) = Identity::random_identity_with_main_keys_with_private_key::<
            Vec<_>,
        >(2, &mut rng, platform_version)
        .unwrap();

        simple_signer.add_keys(keys2);

        let start_identities: Vec<(Identity, Option<StateTransition>)> =
            create_state_transitions_for_identities(
                vec![&mut identity1, &mut identity2],
                &(dash_to_duffs!(1)..=dash_to_duffs!(1)),
                &simple_signer,
                &mut rng,
                platform_version,
            )
            .into_iter()
            .map(|(identity, transition)| (identity, Some(transition)))
            .collect();

        let dpns_contract = platform
            .drive
            .cache
            .system_data_contracts
            .load_dpns()
            .as_ref()
            .clone();

        let document_type = dpns_contract
            .document_type_for_name("domain")
            .expect("expected a profile document type")
            .to_owned_document_type();

        let identity1_id = start_identities.first().unwrap().0.id();
        let identity2_id = start_identities.last().unwrap().0.id();
        let document_op_1 = DocumentOp {
            contract: dpns_contract.clone(),
            action: DocumentAction::DocumentActionInsertSpecific(
                BTreeMap::from([
                    ("label".into(), "quantum".into()),
                    ("normalizedLabel".into(), "quantum".into()),
                    ("normalizedParentDomainName".into(), "dash".into()),
                    (
                        "records".into(),
                        BTreeMap::from([("identity", Value::from(identity1_id))]).into(),
                    ),
                ]),
                Some(start_identities.first().unwrap().0.id()),
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
            ),
            document_type: document_type.clone(),
        };

        let document_op_2 = DocumentOp {
            contract: dpns_contract.clone(),
            action: DocumentAction::DocumentActionInsertSpecific(
                BTreeMap::from([
                    ("label".into(), "quantum".into()),
                    ("normalizedLabel".into(), "quantum".into()),
                    ("normalizedParentDomainName".into(), "dash".into()),
                    (
                        "records".into(),
                        BTreeMap::from([(
                            "identity",
                            Value::from(start_identities.last().unwrap().0.id()),
                        )])
                        .into(),
                    ),
                ]),
                Some(start_identities.last().unwrap().0.id()),
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
            ),
            document_type: document_type.clone(),
        };

        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![
                    Operation {
                        op_type: OperationType::Document(document_op_1),
                        frequency: Frequency {
                            times_per_block_range: 1..2,
                            chance_per_block: None,
                        },
                    },
                    Operation {
                        op_type: OperationType::Document(document_op_2),
                        frequency: Frequency {
                            times_per_block_range: 1..2,
                            chance_per_block: None,
                        },
                    },
                ],
                start_identities: StartIdentities {
                    hard_coded: start_identities,
                    ..Default::default()
                },
                identity_inserts: Default::default(),

                identity_contract_nonce_gaps: None,
                signer: Some(simple_signer),
            },
            total_hpmns: 100,
            extra_normal_mns: 0,
            validator_quorum_count: 24,
            chain_lock_quorum_count: 24,
            upgrading_info: None,

            proposer_strategy: Default::default(),
            rotate_quorums: false,
            failure_testing: None,
            query_testing: None,
            verify_state_transition_results: true,
            ..Default::default()
        };

        let mut voting_signer = Some(SimpleSigner::default());

        // On the first block we only have identities and contracts
        let ChainExecutionOutcome {
            abci_app,
            proposers,
            validator_quorums,
            current_validator_quorum_hash,
            instant_lock_quorums,
            current_proposer_versions,
            end_time_ms,
            identity_nonce_counter,
            identity_contract_nonce_counter,
            state_transition_results_per_block,
            ..
        } = run_chain_for_strategy(
            &mut platform,
            2,
            strategy.clone(),
            config.clone(),
            15,
            &mut voting_signer,
            &mut None,
        );

        let platform = abci_app.platform;

        let platform_state = platform.state.load();

        let state_transitions_block_2 = state_transition_results_per_block
            .get(&2)
            .expect("expected to get block 2");

        let first_document_insert_result = &state_transitions_block_2
            .first()
            .as_ref()
            .expect("expected a document insert")
            .1;
        assert_eq!(first_document_insert_result.code, 0);

        let second_document_insert_result = &state_transitions_block_2
            .get(1)
            .as_ref()
            .expect("expected a document insert")
            .1;

        assert_eq!(second_document_insert_result.code, 0); // we expect the second to also be insertable as they are both contested

        let block_start = platform_state
            .last_committed_block_info()
            .as_ref()
            .unwrap()
            .basic_info()
            .height
            + 1;
        let outcome = continue_chain_for_strategy(
            abci_app,
            ChainExecutionParameters {
                block_start,
                core_height_start: 1,
                block_count: 30,
                proposers,
                validator_quorums,
                current_validator_quorum_hash,
                instant_lock_quorums,
                current_proposer_versions: Some(current_proposer_versions.clone()),
                current_identity_nonce_counter: identity_nonce_counter,
                current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                current_votes: BTreeMap::default(),
                start_time_ms: 1681094380000,
                current_time_ms: end_time_ms,
                current_identities: Vec::new(),
            },
            NetworkStrategy {
                strategy: Strategy {
                    start_contracts: vec![],
                    operations: vec![Operation {
                        op_type: OperationType::ResourceVote(ResourceVoteOp {
                            resolved_vote_poll: ContestedDocumentResourceVotePollWithContractInfo {
                                contract: DataContractOwnedResolvedInfo::OwnedDataContract(
                                    dpns_contract.clone(),
                                ),
                                document_type_name: "domain".to_string(),
                                index_name: "parentNameAndLabel".to_string(),
                                index_values: vec!["dash".into(), "quantum".into()],
                            },
                            action: VoteAction {
                                vote_choices_with_weights: vec![(ResourceVoteChoice::Abstain, 1)],
                            },
                        }),
                        frequency: Frequency {
                            times_per_block_range: 1..3,
                            chance_per_block: None,
                        },
                    }],
                    start_identities: StartIdentities::default(),
                    identity_inserts: Default::default(),

                    identity_contract_nonce_gaps: None,
                    signer: voting_signer,
                },
                total_hpmns: 100,
                extra_normal_mns: 0,
                validator_quorum_count: 24,
                chain_lock_quorum_count: 24,
                upgrading_info: None,

                proposer_strategy: Default::default(),
                rotate_quorums: false,
                failure_testing: None,
                query_testing: None,
                verify_state_transition_results: true,
                ..Default::default()
            },
            config.clone(),
            StrategyRandomness::SeedEntropy(7),
        );

        let platform = outcome.abci_app.platform;

        // Now let's run a query for the vote totals

        let config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();

        let dash_encoded = bincode::encode_to_vec(Value::Text("dash".to_string()), config)
            .expect("expected to encode the word dash");

        let quantum_encoded = bincode::encode_to_vec(Value::Text("quantum".to_string()), config)
            .expect("expected to encode the word quantum");

        let index_name = "parentNameAndLabel".to_string();

        let query_validation_result = platform
            .query_contested_resource_vote_state(
                GetContestedResourceVoteStateRequest {
                    version: Some(get_contested_resource_vote_state_request::Version::V0(
                        GetContestedResourceVoteStateRequestV0 {
                            contract_id: dpns_contract.id().to_vec(),
                            document_type_name: document_type.name().clone(),
                            index_name: index_name.clone(),
                            index_values: vec![dash_encoded.clone(), quantum_encoded.clone()],
                            result_type: ResultType::DocumentsAndVoteTally as i32,
                            allow_include_locked_and_abstaining_vote_tally: true,
                            start_at_identifier_info: None,
                            count: None,
                            prove: false,
                        },
                    )),
                },
                &platform_state,
                platform_version,
            )
            .expect("expected to execute query")
            .into_data()
            .expect("expected query to be valid");

        let get_contested_resource_vote_state_response::Version::V0(
            GetContestedResourceVoteStateResponseV0 {
                metadata: _,
                result,
            },
        ) = query_validation_result.version.expect("expected a version");

        let Some(
            get_contested_resource_vote_state_response_v0::Result::ContestedResourceContenders(
                get_contested_resource_vote_state_response_v0::ContestedResourceContenders {
                    contenders,
                    abstain_vote_tally,
                    lock_vote_tally: _,
                    finished_vote_info: _,
                },
            ),
        ) = result
        else {
            panic!("expected contenders")
        };

        assert_eq!(contenders.len(), 2);

        let first_contender = contenders.first().unwrap();

        let second_contender = contenders.last().unwrap();

        assert!(first_contender.document.is_some());

        assert!(second_contender.document.is_some());

        assert_eq!(first_contender.identifier, identity2_id.to_vec());

        assert_eq!(second_contender.identifier, identity1_id.to_vec());

        assert_eq!(first_contender.vote_count, Some(0));

        assert_eq!(second_contender.vote_count, Some(0));

        assert_eq!(abstain_vote_tally, Some(124));
    }

    #[test]
    fn run_chain_with_voting_on_conflicting_index_various_votes() {
        // In this test we try to insert two state transitions with the same unique index
        // We use the DPNS contract, and we insert two documents both with the same "name"
        // This is a common scenario we should see quite often
        let config = PlatformConfig {
            testing_configs: PlatformTestConfig::default_minimal_verifications(),
            chain_lock: ChainLockConfig::default_100_67(),
            instant_lock: InstantLockConfig::default_100_67(),
            execution: ExecutionConfig {
                //we disable document triggers because we are using dpns and dpns needs a preorder
                use_document_triggers: false,
                ..Default::default()
            },
            block_spacing_ms: 3000,
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let platform_version = PlatformVersion::latest();

        let mut rng = StdRng::seed_from_u64(567);

        let mut simple_signer = SimpleSigner::default();

        let (mut identity1, keys1) = Identity::random_identity_with_main_keys_with_private_key::<
            Vec<_>,
        >(2, &mut rng, platform_version)
        .unwrap();

        simple_signer.add_keys(keys1);

        let (mut identity2, keys2) = Identity::random_identity_with_main_keys_with_private_key::<
            Vec<_>,
        >(2, &mut rng, platform_version)
        .unwrap();

        simple_signer.add_keys(keys2);

        let start_identities: Vec<(Identity, Option<StateTransition>)> =
            create_state_transitions_for_identities(
                vec![&mut identity1, &mut identity2],
                &(dash_to_duffs!(1)..=dash_to_duffs!(1)),
                &simple_signer,
                &mut rng,
                platform_version,
            )
            .into_iter()
            .map(|(identity, transition)| (identity, Some(transition)))
            .collect();

        let dpns_contract = platform
            .drive
            .cache
            .system_data_contracts
            .load_dpns()
            .as_ref()
            .clone();

        let document_type = dpns_contract
            .document_type_for_name("domain")
            .expect("expected a profile document type")
            .to_owned_document_type();

        let identity1_id = start_identities.first().unwrap().0.id();
        let identity2_id = start_identities.last().unwrap().0.id();
        let document_op_1 = DocumentOp {
            contract: dpns_contract.clone(),
            action: DocumentAction::DocumentActionInsertSpecific(
                BTreeMap::from([
                    ("label".into(), "quantum".into()),
                    ("normalizedLabel".into(), "quantum".into()),
                    ("normalizedParentDomainName".into(), "dash".into()),
                    (
                        "records".into(),
                        BTreeMap::from([("identity", Value::from(identity1_id))]).into(),
                    ),
                ]),
                Some(start_identities.first().unwrap().0.id()),
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
            ),
            document_type: document_type.clone(),
        };

        let document_op_2 = DocumentOp {
            contract: dpns_contract.clone(),
            action: DocumentAction::DocumentActionInsertSpecific(
                BTreeMap::from([
                    ("label".into(), "quantum".into()),
                    ("normalizedLabel".into(), "quantum".into()),
                    ("normalizedParentDomainName".into(), "dash".into()),
                    (
                        "records".into(),
                        BTreeMap::from([(
                            "identity",
                            Value::from(start_identities.last().unwrap().0.id()),
                        )])
                        .into(),
                    ),
                ]),
                Some(start_identities.last().unwrap().0.id()),
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
            ),
            document_type: document_type.clone(),
        };

        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![
                    Operation {
                        op_type: OperationType::Document(document_op_1),
                        frequency: Frequency {
                            times_per_block_range: 1..2,
                            chance_per_block: None,
                        },
                    },
                    Operation {
                        op_type: OperationType::Document(document_op_2),
                        frequency: Frequency {
                            times_per_block_range: 1..2,
                            chance_per_block: None,
                        },
                    },
                ],
                start_identities: StartIdentities {
                    hard_coded: start_identities,
                    ..Default::default()
                },
                identity_inserts: Default::default(),

                identity_contract_nonce_gaps: None,
                signer: Some(simple_signer),
            },
            total_hpmns: 100,
            extra_normal_mns: 0,
            validator_quorum_count: 24,
            chain_lock_quorum_count: 24,
            upgrading_info: None,

            proposer_strategy: Default::default(),
            rotate_quorums: false,
            failure_testing: None,
            query_testing: None,
            verify_state_transition_results: true,
            ..Default::default()
        };

        let mut voting_signer = Some(SimpleSigner::default());

        // On the first block we only have identities and contracts
        let ChainExecutionOutcome {
            abci_app,
            proposers,
            validator_quorums,
            current_validator_quorum_hash,
            instant_lock_quorums,
            current_proposer_versions,
            end_time_ms,
            identity_nonce_counter,
            identity_contract_nonce_counter,
            state_transition_results_per_block,
            ..
        } = run_chain_for_strategy(
            &mut platform,
            2,
            strategy.clone(),
            config.clone(),
            15,
            &mut voting_signer,
            &mut None,
        );

        let platform = abci_app.platform;

        let platform_state = platform.state.load();

        let state_transitions_block_2 = state_transition_results_per_block
            .get(&2)
            .expect("expected to get block 2");

        let first_document_insert_result = &state_transitions_block_2
            .first()
            .as_ref()
            .expect("expected a document insert")
            .1;
        assert_eq!(first_document_insert_result.code, 0);

        let second_document_insert_result = &state_transitions_block_2
            .get(1)
            .as_ref()
            .expect("expected a document insert")
            .1;

        assert_eq!(second_document_insert_result.code, 0); // we expect the second to also be insertable as they are both contested

        let block_start = platform_state
            .last_committed_block_info()
            .as_ref()
            .unwrap()
            .basic_info()
            .height
            + 1;
        let outcome = continue_chain_for_strategy(
            abci_app,
            ChainExecutionParameters {
                block_start,
                core_height_start: 1,
                block_count: 30,
                proposers,
                validator_quorums,
                current_validator_quorum_hash,
                instant_lock_quorums,
                current_proposer_versions: Some(current_proposer_versions.clone()),
                current_identity_nonce_counter: identity_nonce_counter,
                current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                current_votes: BTreeMap::default(),
                start_time_ms: 1681094380000,
                current_time_ms: end_time_ms,
                current_identities: Vec::new(),
            },
            NetworkStrategy {
                strategy: Strategy {
                    start_contracts: vec![],
                    operations: vec![Operation {
                        op_type: OperationType::ResourceVote(ResourceVoteOp {
                            resolved_vote_poll: ContestedDocumentResourceVotePollWithContractInfo {
                                contract: DataContractOwnedResolvedInfo::OwnedDataContract(
                                    dpns_contract.clone(),
                                ),
                                document_type_name: "domain".to_string(),
                                index_name: "parentNameAndLabel".to_string(),
                                index_values: vec!["dash".into(), "quantum".into()],
                            },
                            action: VoteAction {
                                vote_choices_with_weights: vec![
                                    (ResourceVoteChoice::Abstain, 1),
                                    (ResourceVoteChoice::Lock, 1),
                                    (ResourceVoteChoice::TowardsIdentity(identity1_id), 5),
                                    (ResourceVoteChoice::TowardsIdentity(identity2_id), 10),
                                ],
                            },
                        }),
                        frequency: Frequency {
                            times_per_block_range: 1..3,
                            chance_per_block: None,
                        },
                    }],
                    start_identities: StartIdentities::default(),
                    identity_inserts: Default::default(),

                    identity_contract_nonce_gaps: None,
                    signer: voting_signer,
                },
                total_hpmns: 100,
                extra_normal_mns: 0,
                validator_quorum_count: 24,
                chain_lock_quorum_count: 24,
                upgrading_info: None,

                proposer_strategy: Default::default(),
                rotate_quorums: false,
                failure_testing: None,
                query_testing: None,
                verify_state_transition_results: true,
                ..Default::default()
            },
            config.clone(),
            StrategyRandomness::SeedEntropy(7),
        );

        let platform = outcome.abci_app.platform;

        // Now let's run a query for the vote totals

        let config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();

        let dash_encoded = bincode::encode_to_vec(Value::Text("dash".to_string()), config)
            .expect("expected to encode the word dash");

        let quantum_encoded = bincode::encode_to_vec(Value::Text("quantum".to_string()), config)
            .expect("expected to encode the word quantum");

        let index_name = "parentNameAndLabel".to_string();

        let query_validation_result = platform
            .query_contested_resource_vote_state(
                GetContestedResourceVoteStateRequest {
                    version: Some(get_contested_resource_vote_state_request::Version::V0(
                        GetContestedResourceVoteStateRequestV0 {
                            contract_id: dpns_contract.id().to_vec(),
                            document_type_name: document_type.name().clone(),
                            index_name: index_name.clone(),
                            index_values: vec![dash_encoded.clone(), quantum_encoded.clone()],
                            result_type: ResultType::DocumentsAndVoteTally as i32,
                            allow_include_locked_and_abstaining_vote_tally: true,
                            start_at_identifier_info: None,
                            count: None,
                            prove: false,
                        },
                    )),
                },
                &platform_state,
                platform_version,
            )
            .expect("expected to execute query")
            .into_data()
            .expect("expected query to be valid");

        let get_contested_resource_vote_state_response::Version::V0(
            GetContestedResourceVoteStateResponseV0 {
                metadata: _,
                result,
            },
        ) = query_validation_result.version.expect("expected a version");

        let Some(
            get_contested_resource_vote_state_response_v0::Result::ContestedResourceContenders(
                get_contested_resource_vote_state_response_v0::ContestedResourceContenders {
                    contenders,
                    abstain_vote_tally,
                    lock_vote_tally,
                    finished_vote_info,
                },
            ),
        ) = result
        else {
            panic!("expected contenders")
        };

        assert_eq!(contenders.len(), 2);

        let first_contender = contenders.first().unwrap();

        let second_contender = contenders.last().unwrap();

        assert!(first_contender.document.is_some());

        assert!(second_contender.document.is_some());

        assert_eq!(first_contender.identifier, identity2_id.to_vec());

        assert_eq!(second_contender.identifier, identity1_id.to_vec());

        // All vote counts are weighted, so for evonodes, these are in multiples of 4

        assert_eq!(first_contender.vote_count, Some(52));

        assert_eq!(second_contender.vote_count, Some(56));

        assert_eq!(lock_vote_tally, Some(16));

        assert_eq!(abstain_vote_tally, Some(8));

        assert_eq!(finished_vote_info, None);
    }

    #[test]
    fn run_chain_with_voting_after_won_by_identity_no_specialized_funds_distribution() {
        // Define the desired stack size
        let stack_size = 4 * 1024 * 1024; //Let's set the stack size to be higher than the default 2MB

        let builder = std::thread::Builder::new()
            .stack_size(stack_size)
            .name("custom_stack_size_thread".into());

        let handler = builder
            .spawn(|| {
                // In this test we try to insert two state transitions with the same unique index
                // We use the DPNS contract, and we insert two documents both with the same "name"
                // This is a common scenario we should see quite often
                let config = PlatformConfig {
                    testing_configs: PlatformTestConfig::default_minimal_verifications(),
                    chain_lock: ChainLockConfig::default_100_67(),
                    instant_lock: InstantLockConfig::default_100_67(),
                    execution: ExecutionConfig {
                        //we disable document triggers because we are using dpns and dpns needs a preorder
                        use_document_triggers: false,

                        ..Default::default()
                    },
                    block_spacing_ms: 3000,
                    ..Default::default()
                };
                let mut platform = TestPlatformBuilder::new()
                    .with_config(config.clone())
                    .with_initial_protocol_version(7)
                    .build_with_mock_rpc();

                let platform_version = PlatformVersion::get(7).unwrap();

                let mut rng = StdRng::seed_from_u64(567);

                let mut simple_signer = SimpleSigner::default();

                let (mut identity1, keys1) =
                    Identity::random_identity_with_main_keys_with_private_key::<Vec<_>>(
                        2,
                        &mut rng,
                        platform_version,
                    )
                    .unwrap();

                simple_signer.add_keys(keys1);

                let (mut identity2, keys2) =
                    Identity::random_identity_with_main_keys_with_private_key::<Vec<_>>(
                        2,
                        &mut rng,
                        platform_version,
                    )
                    .unwrap();

                simple_signer.add_keys(keys2);

                let start_identities: Vec<(Identity, Option<StateTransition>)> =
                    create_state_transitions_for_identities(
                        vec![&mut identity1, &mut identity2],
                        &(dash_to_duffs!(1)..=dash_to_duffs!(1)),
                        &simple_signer,
                        &mut rng,
                        platform_version,
                    )
                    .into_iter()
                    .map(|(identity, transition)| (identity, Some(transition)))
                    .collect();

                let dpns_contract = platform
                    .drive
                    .cache
                    .system_data_contracts
                    .load_dpns()
                    .as_ref()
                    .clone();

                let document_type = dpns_contract
                    .document_type_for_name("domain")
                    .expect("expected a profile document type")
                    .to_owned_document_type();

                let identity1_id = start_identities.first().unwrap().0.id();
                let identity2_id = start_identities.last().unwrap().0.id();
                let document_op_1 = DocumentOp {
                    contract: dpns_contract.clone(),
                    action: DocumentAction::DocumentActionInsertSpecific(
                        BTreeMap::from([
                            ("label".into(), "quantum".into()),
                            ("normalizedLabel".into(), "quantum".into()),
                            ("normalizedParentDomainName".into(), "dash".into()),
                            (
                                "records".into(),
                                BTreeMap::from([("identity", Value::from(identity1_id))]).into(),
                            ),
                        ]),
                        Some(start_identities.first().unwrap().0.id()),
                        DocumentFieldFillType::FillIfNotRequired,
                        DocumentFieldFillSize::AnyDocumentFillSize,
                    ),
                    document_type: document_type.clone(),
                };

                let document_op_2 = DocumentOp {
                    contract: dpns_contract.clone(),
                    action: DocumentAction::DocumentActionInsertSpecific(
                        BTreeMap::from([
                            ("label".into(), "quantum".into()),
                            ("normalizedLabel".into(), "quantum".into()),
                            ("normalizedParentDomainName".into(), "dash".into()),
                            (
                                "records".into(),
                                BTreeMap::from([(
                                    "identity",
                                    Value::from(start_identities.last().unwrap().0.id()),
                                )])
                                .into(),
                            ),
                        ]),
                        Some(start_identities.last().unwrap().0.id()),
                        DocumentFieldFillType::FillIfNotRequired,
                        DocumentFieldFillSize::AnyDocumentFillSize,
                    ),
                    document_type: document_type.clone(),
                };

                let strategy = NetworkStrategy {
                    strategy: Strategy {
                        start_contracts: vec![],
                        operations: vec![
                            Operation {
                                op_type: OperationType::Document(document_op_1),
                                frequency: Frequency {
                                    times_per_block_range: 1..2,
                                    chance_per_block: None,
                                },
                            },
                            Operation {
                                op_type: OperationType::Document(document_op_2),
                                frequency: Frequency {
                                    times_per_block_range: 1..2,
                                    chance_per_block: None,
                                },
                            },
                        ],
                        start_identities: StartIdentities {
                            hard_coded: start_identities,
                            ..Default::default()
                        },
                        identity_inserts: Default::default(),

                        identity_contract_nonce_gaps: None,
                        signer: Some(simple_signer),
                    },
                    total_hpmns: 100,
                    extra_normal_mns: 0,
                    validator_quorum_count: 24,
                    chain_lock_quorum_count: 24,
                    upgrading_info: None,

                    proposer_strategy: Default::default(),
                    rotate_quorums: false,
                    failure_testing: None,
                    query_testing: None,
                    verify_state_transition_results: true,
                    ..Default::default()
                };

                let mut voting_signer = Some(SimpleSigner::default());

                // On the first block we only have identities and contracts
                let ChainExecutionOutcome {
                    abci_app,
                    proposers,
                    validator_quorums,
                    current_validator_quorum_hash,
                    instant_lock_quorums,
                    current_proposer_versions,
                    end_time_ms,
                    identity_nonce_counter,
                    identity_contract_nonce_counter,
                    state_transition_results_per_block,
                    ..
                } = run_chain_for_strategy(
                    &mut platform,
                    2,
                    strategy.clone(),
                    config.clone(),
                    15,
                    &mut voting_signer,
                    &mut None,
                );

                let platform = abci_app.platform;

                let platform_state = platform.state.load();

                let state_transitions_block_2 = state_transition_results_per_block
                    .get(&2)
                    .expect("expected to get block 2");

                let first_document_insert_result = &state_transitions_block_2
                    .first()
                    .as_ref()
                    .expect("expected a document insert")
                    .1;
                assert_eq!(first_document_insert_result.code, 0);

                let second_document_insert_result = &state_transitions_block_2
                    .get(1)
                    .as_ref()
                    .expect("expected a document insert")
                    .1;

                assert_eq!(second_document_insert_result.code, 0); // we expect the second to also be insertable as they are both contested

                let block_start = platform_state
                    .last_committed_block_info()
                    .as_ref()
                    .unwrap()
                    .basic_info()
                    .height
                    + 1;
                let day_in_ms = 1000 * 60 * 60 * 24;
                let config = PlatformConfig {
                    chain_lock: ChainLockConfig::default_100_67(),
                    instant_lock: InstantLockConfig::default_100_67(),
                    execution: ExecutionConfig {
                        //we disable document triggers because we are using dpns and dpns needs a preorder
                        use_document_triggers: false,

                        ..Default::default()
                    },
                    block_spacing_ms: day_in_ms,
                    ..Default::default()
                };

                let outcome = continue_chain_for_strategy(
                    abci_app,
                    ChainExecutionParameters {
                        block_start,
                        core_height_start: 1,
                        block_count: 16,
                        proposers,
                        validator_quorums,
                        current_validator_quorum_hash,
                        instant_lock_quorums,
                        current_proposer_versions: Some(current_proposer_versions.clone()),
                        current_identity_nonce_counter: identity_nonce_counter,
                        current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                        current_votes: BTreeMap::default(),
                        start_time_ms: 1681094380000,
                        current_time_ms: end_time_ms,
                        current_identities: Vec::new(),
                    },
                    NetworkStrategy {
                        strategy: Strategy {
                            start_contracts: vec![],
                            operations: vec![Operation {
                                op_type: OperationType::ResourceVote(ResourceVoteOp {
                                    resolved_vote_poll:
                                        ContestedDocumentResourceVotePollWithContractInfo {
                                            contract:
                                                DataContractOwnedResolvedInfo::OwnedDataContract(
                                                    dpns_contract.clone(),
                                                ),
                                            document_type_name: "domain".to_string(),
                                            index_name: "parentNameAndLabel".to_string(),
                                            index_values: vec!["dash".into(), "quantum".into()],
                                        },
                                    action: VoteAction {
                                        vote_choices_with_weights: vec![
                                            (ResourceVoteChoice::Abstain, 1),
                                            (ResourceVoteChoice::Lock, 1),
                                            (ResourceVoteChoice::TowardsIdentity(identity1_id), 2),
                                            (ResourceVoteChoice::TowardsIdentity(identity2_id), 10),
                                        ],
                                    },
                                }),
                                frequency: Frequency {
                                    times_per_block_range: 1..3,
                                    chance_per_block: None,
                                },
                            }],
                            start_identities: StartIdentities::default(),
                            identity_inserts: Default::default(),

                            identity_contract_nonce_gaps: None,
                            signer: voting_signer,
                        },
                        total_hpmns: 100,
                        extra_normal_mns: 0,
                        validator_quorum_count: 24,
                        chain_lock_quorum_count: 24,
                        upgrading_info: None,

                        proposer_strategy: Default::default(),
                        rotate_quorums: false,
                        failure_testing: None,
                        query_testing: None,
                        verify_state_transition_results: true,
                        ..Default::default()
                    },
                    config.clone(),
                    StrategyRandomness::SeedEntropy(9),
                );

                let platform = outcome.abci_app.platform;

                // Now let's run a query for the vote totals

                let config = bincode::config::standard()
                    .with_big_endian()
                    .with_no_limit();

                let dash_encoded = bincode::encode_to_vec(Value::Text("dash".to_string()), config)
                    .expect("expected to encode the word dash");

                let quantum_encoded =
                    bincode::encode_to_vec(Value::Text("quantum".to_string()), config)
                        .expect("expected to encode the word quantum");

                let index_name = "parentNameAndLabel".to_string();

                let query_validation_result = platform
                    .query_contested_resource_vote_state(
                        GetContestedResourceVoteStateRequest {
                            version: Some(get_contested_resource_vote_state_request::Version::V0(
                                GetContestedResourceVoteStateRequestV0 {
                                    contract_id: dpns_contract.id().to_vec(),
                                    document_type_name: document_type.name().clone(),
                                    index_name: index_name.clone(),
                                    index_values: vec![
                                        dash_encoded.clone(),
                                        quantum_encoded.clone(),
                                    ],
                                    result_type: ResultType::DocumentsAndVoteTally as i32,
                                    allow_include_locked_and_abstaining_vote_tally: true,
                                    start_at_identifier_info: None,
                                    count: None,
                                    prove: false,
                                },
                            )),
                        },
                        &platform_state,
                        platform_version,
                    )
                    .expect("expected to execute query")
                    .into_data()
                    .expect("expected query to be valid");

                let get_contested_resource_vote_state_response::Version::V0(
                    GetContestedResourceVoteStateResponseV0 {
                        metadata: _,
                        result,
                    },
                ) = query_validation_result.version.expect("expected a version");

                let Some(
            get_contested_resource_vote_state_response_v0::Result::ContestedResourceContenders(
                get_contested_resource_vote_state_response_v0::ContestedResourceContenders {
                    contenders,
                    abstain_vote_tally,
                    lock_vote_tally,
                    finished_vote_info,
                },
            ),
        ) = result
        else {
            panic!("expected contenders")
        };

                assert_eq!(contenders.len(), 2);

                let first_contender = contenders.first().unwrap();

                let second_contender = contenders.last().unwrap();

                assert_eq!(first_contender.identifier, identity2_id.to_vec());

                assert_eq!(second_contender.identifier, identity1_id.to_vec());

                // All vote counts are weighted, so for evonodes, these are in multiples of 4

                assert_eq!(first_contender.vote_count, Some(60));

                assert_eq!(second_contender.vote_count, Some(4));

                assert_eq!(lock_vote_tally, Some(4));

                assert_eq!(abstain_vote_tally, Some(8));

                assert_eq!(
                    finished_vote_info,
                    Some(FinishedVoteInfo {
                        finished_vote_outcome: FinishedVoteOutcome::TowardsIdentity.into(),
                        won_by_identity_id: Some(identity2_id.to_vec()),
                        finished_at_block_height: 17,
                        finished_at_core_block_height: 1,
                        finished_at_block_time_ms: 1682303986000,
                        finished_at_epoch: 1
                    })
                );

                // not let's see how much is in processing pools

                let processing_fees = platform
                    .drive
                    .get_epoch_processing_credits_for_distribution(
                        &Epoch::new(1).unwrap(),
                        None,
                        platform_version,
                    )
                    .expect("expected to get processing fees made in epoch");

                // A vote costs 10_000_000
                // Hence we did 5 votes in this epoch
                assert_eq!(processing_fees, 50_000_000);
            })
            .expect("Failed to create thread with custom stack size");

        // Wait for the thread to finish and assert that it didn't panic.
        handler.join().expect("Thread has panicked");
    }

    #[test]
    fn run_chain_with_voting_after_won_by_identity_with_specialized_funds_distribution() {
        // Define the desired stack size
        let stack_size = 4 * 1024 * 1024; //Let's set the stack size to be higher than the default 2MB

        let builder = std::thread::Builder::new()
            .stack_size(stack_size)
            .name("custom_stack_size_thread".into());

        let handler = builder
            .spawn(|| {
                // In this test we try to insert two state transitions with the same unique index
                // We use the DPNS contract, and we insert two documents both with the same "name"
                // This is a common scenario we should see quite often
                let config = PlatformConfig {
                    testing_configs: PlatformTestConfig::default_minimal_verifications(),
                    chain_lock: ChainLockConfig::default_100_67(),
                    instant_lock: InstantLockConfig::default_100_67(),
                    execution: ExecutionConfig {
                        //we disable document triggers because we are using dpns and dpns needs a preorder
                        use_document_triggers: false,

                        ..Default::default()
                    },
                    block_spacing_ms: 3000,
                    ..Default::default()
                };
                let mut platform = TestPlatformBuilder::new()
                    .with_config(config.clone())
                    .build_with_mock_rpc();

                let platform_version = PlatformVersion::latest();

                let mut rng = StdRng::seed_from_u64(567);

                let mut simple_signer = SimpleSigner::default();

                let (mut identity1, keys1) =
                    Identity::random_identity_with_main_keys_with_private_key::<Vec<_>>(
                        2,
                        &mut rng,
                        platform_version,
                    )
                    .unwrap();

                simple_signer.add_keys(keys1);

                let (mut identity2, keys2) =
                    Identity::random_identity_with_main_keys_with_private_key::<Vec<_>>(
                        2,
                        &mut rng,
                        platform_version,
                    )
                    .unwrap();

                simple_signer.add_keys(keys2);

                let start_identities: Vec<(Identity, Option<StateTransition>)> =
                    create_state_transitions_for_identities(
                        vec![&mut identity1, &mut identity2],
                        &(dash_to_duffs!(1)..=dash_to_duffs!(1)),
                        &simple_signer,
                        &mut rng,
                        platform_version,
                    )
                    .into_iter()
                    .map(|(identity, transition)| (identity, Some(transition)))
                    .collect();

                let dpns_contract = platform
                    .drive
                    .cache
                    .system_data_contracts
                    .load_dpns()
                    .as_ref()
                    .clone();

                let document_type = dpns_contract
                    .document_type_for_name("domain")
                    .expect("expected a profile document type")
                    .to_owned_document_type();

                let identity1_id = start_identities.first().unwrap().0.id();
                let identity2_id = start_identities.last().unwrap().0.id();
                let document_op_1 = DocumentOp {
                    contract: dpns_contract.clone(),
                    action: DocumentAction::DocumentActionInsertSpecific(
                        BTreeMap::from([
                            ("label".into(), "quantum".into()),
                            ("normalizedLabel".into(), "quantum".into()),
                            ("normalizedParentDomainName".into(), "dash".into()),
                            (
                                "records".into(),
                                BTreeMap::from([("identity", Value::from(identity1_id))]).into(),
                            ),
                        ]),
                        Some(start_identities.first().unwrap().0.id()),
                        DocumentFieldFillType::FillIfNotRequired,
                        DocumentFieldFillSize::AnyDocumentFillSize,
                    ),
                    document_type: document_type.clone(),
                };

                let document_op_2 = DocumentOp {
                    contract: dpns_contract.clone(),
                    action: DocumentAction::DocumentActionInsertSpecific(
                        BTreeMap::from([
                            ("label".into(), "quantum".into()),
                            ("normalizedLabel".into(), "quantum".into()),
                            ("normalizedParentDomainName".into(), "dash".into()),
                            (
                                "records".into(),
                                BTreeMap::from([(
                                    "identity",
                                    Value::from(start_identities.last().unwrap().0.id()),
                                )])
                                .into(),
                            ),
                        ]),
                        Some(start_identities.last().unwrap().0.id()),
                        DocumentFieldFillType::FillIfNotRequired,
                        DocumentFieldFillSize::AnyDocumentFillSize,
                    ),
                    document_type: document_type.clone(),
                };

                let strategy = NetworkStrategy {
                    strategy: Strategy {
                        start_contracts: vec![],
                        operations: vec![
                            Operation {
                                op_type: OperationType::Document(document_op_1),
                                frequency: Frequency {
                                    times_per_block_range: 1..2,
                                    chance_per_block: None,
                                },
                            },
                            Operation {
                                op_type: OperationType::Document(document_op_2),
                                frequency: Frequency {
                                    times_per_block_range: 1..2,
                                    chance_per_block: None,
                                },
                            },
                        ],
                        start_identities: StartIdentities {
                            hard_coded: start_identities,
                            ..Default::default()
                        },
                        identity_inserts: Default::default(),

                        identity_contract_nonce_gaps: None,
                        signer: Some(simple_signer),
                    },
                    total_hpmns: 100,
                    extra_normal_mns: 0,
                    validator_quorum_count: 24,
                    chain_lock_quorum_count: 24,
                    upgrading_info: None,

                    proposer_strategy: Default::default(),
                    rotate_quorums: false,
                    failure_testing: None,
                    query_testing: None,
                    verify_state_transition_results: true,
                    ..Default::default()
                };

                let mut voting_signer = Some(SimpleSigner::default());

                // On the first block we only have identities and contracts
                let ChainExecutionOutcome {
                    abci_app,
                    proposers,
                    validator_quorums,
                    current_validator_quorum_hash,
                    instant_lock_quorums,
                    current_proposer_versions,
                    end_time_ms,
                    identity_nonce_counter,
                    identity_contract_nonce_counter,
                    state_transition_results_per_block,
                    ..
                } = run_chain_for_strategy(
                    &mut platform,
                    2,
                    strategy.clone(),
                    config.clone(),
                    15,
                    &mut voting_signer,
                    &mut None,
                );

                let platform = abci_app.platform;

                let platform_state = platform.state.load();

                let state_transitions_block_2 = state_transition_results_per_block
                    .get(&2)
                    .expect("expected to get block 2");

                let first_document_insert_result = &state_transitions_block_2
                    .first()
                    .as_ref()
                    .expect("expected a document insert")
                    .1;
                assert_eq!(first_document_insert_result.code, 0);

                let second_document_insert_result = &state_transitions_block_2
                    .get(1)
                    .as_ref()
                    .expect("expected a document insert")
                    .1;

                assert_eq!(second_document_insert_result.code, 0); // we expect the second to also be insertable as they are both contested

                let block_start = platform_state
                    .last_committed_block_info()
                    .as_ref()
                    .unwrap()
                    .basic_info()
                    .height
                    + 1;
                let day_in_ms = 1000 * 60 * 60 * 24;
                let config = PlatformConfig {
                    chain_lock: ChainLockConfig::default_100_67(),
                    instant_lock: InstantLockConfig::default_100_67(),
                    execution: ExecutionConfig {
                        //we disable document triggers because we are using dpns and dpns needs a preorder
                        use_document_triggers: false,

                        ..Default::default()
                    },
                    block_spacing_ms: day_in_ms,
                    ..Default::default()
                };

                let outcome = continue_chain_for_strategy(
                    abci_app,
                    ChainExecutionParameters {
                        block_start,
                        core_height_start: 1,
                        block_count: 16,
                        proposers,
                        validator_quorums,
                        current_validator_quorum_hash,
                        instant_lock_quorums,
                        current_proposer_versions: Some(current_proposer_versions.clone()),
                        current_identity_nonce_counter: identity_nonce_counter,
                        current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                        current_votes: BTreeMap::default(),
                        start_time_ms: 1681094380000,
                        current_time_ms: end_time_ms,
                        current_identities: Vec::new(),
                    },
                    NetworkStrategy {
                        strategy: Strategy {
                            start_contracts: vec![],
                            operations: vec![Operation {
                                op_type: OperationType::ResourceVote(ResourceVoteOp {
                                    resolved_vote_poll:
                                        ContestedDocumentResourceVotePollWithContractInfo {
                                            contract:
                                                DataContractOwnedResolvedInfo::OwnedDataContract(
                                                    dpns_contract.clone(),
                                                ),
                                            document_type_name: "domain".to_string(),
                                            index_name: "parentNameAndLabel".to_string(),
                                            index_values: vec!["dash".into(), "quantum".into()],
                                        },
                                    action: VoteAction {
                                        vote_choices_with_weights: vec![
                                            (ResourceVoteChoice::Abstain, 1),
                                            (ResourceVoteChoice::Lock, 1),
                                            (ResourceVoteChoice::TowardsIdentity(identity1_id), 2),
                                            (ResourceVoteChoice::TowardsIdentity(identity2_id), 10),
                                        ],
                                    },
                                }),
                                frequency: Frequency {
                                    times_per_block_range: 1..3,
                                    chance_per_block: None,
                                },
                            }],
                            start_identities: StartIdentities::default(),
                            identity_inserts: Default::default(),

                            identity_contract_nonce_gaps: None,
                            signer: voting_signer,
                        },
                        total_hpmns: 100,
                        extra_normal_mns: 0,
                        validator_quorum_count: 24,
                        chain_lock_quorum_count: 24,
                        upgrading_info: None,

                        proposer_strategy: Default::default(),
                        rotate_quorums: false,
                        failure_testing: None,
                        query_testing: None,
                        verify_state_transition_results: true,
                        ..Default::default()
                    },
                    config.clone(),
                    StrategyRandomness::SeedEntropy(9),
                );

                let platform = outcome.abci_app.platform;

                // Now let's run a query for the vote totals

                let config = bincode::config::standard()
                    .with_big_endian()
                    .with_no_limit();

                let dash_encoded = bincode::encode_to_vec(Value::Text("dash".to_string()), config)
                    .expect("expected to encode the word dash");

                let quantum_encoded =
                    bincode::encode_to_vec(Value::Text("quantum".to_string()), config)
                        .expect("expected to encode the word quantum");

                let index_name = "parentNameAndLabel".to_string();

                let query_validation_result = platform
                    .query_contested_resource_vote_state(
                        GetContestedResourceVoteStateRequest {
                            version: Some(get_contested_resource_vote_state_request::Version::V0(
                                GetContestedResourceVoteStateRequestV0 {
                                    contract_id: dpns_contract.id().to_vec(),
                                    document_type_name: document_type.name().clone(),
                                    index_name: index_name.clone(),
                                    index_values: vec![
                                        dash_encoded.clone(),
                                        quantum_encoded.clone(),
                                    ],
                                    result_type: ResultType::DocumentsAndVoteTally as i32,
                                    allow_include_locked_and_abstaining_vote_tally: true,
                                    start_at_identifier_info: None,
                                    count: None,
                                    prove: false,
                                },
                            )),
                        },
                        &platform_state,
                        platform_version,
                    )
                    .expect("expected to execute query")
                    .into_data()
                    .expect("expected query to be valid");

                let get_contested_resource_vote_state_response::Version::V0(
                    GetContestedResourceVoteStateResponseV0 {
                        metadata: _,
                        result,
                    },
                ) = query_validation_result.version.expect("expected a version");

                let Some(
            get_contested_resource_vote_state_response_v0::Result::ContestedResourceContenders(
                get_contested_resource_vote_state_response_v0::ContestedResourceContenders {
                    contenders,
                    abstain_vote_tally,
                    lock_vote_tally,
                    finished_vote_info,
                },
            ),
        ) = result
        else {
            panic!("expected contenders")
        };

                assert_eq!(contenders.len(), 2);

                let first_contender = contenders.first().unwrap();

                let second_contender = contenders.last().unwrap();

                assert_eq!(first_contender.identifier, identity2_id.to_vec());

                assert_eq!(second_contender.identifier, identity1_id.to_vec());

                // All vote counts are weighted, so for evonodes, these are in multiples of 4

                // 19 votes were cast

                assert_eq!(first_contender.vote_count, Some(60));

                assert_eq!(second_contender.vote_count, Some(4));

                assert_eq!(lock_vote_tally, Some(4));

                assert_eq!(abstain_vote_tally, Some(8));

                assert_eq!(
                    finished_vote_info,
                    Some(FinishedVoteInfo {
                        finished_vote_outcome: FinishedVoteOutcome::TowardsIdentity.into(),
                        won_by_identity_id: Some(identity2_id.to_vec()),
                        finished_at_block_height: 17,
                        finished_at_core_block_height: 1,
                        finished_at_block_time_ms: 1682303986000,
                        finished_at_epoch: 1
                    })
                );

                // not let's see how much is in processing pools

                let processing_fees = platform
                    .drive
                    .get_epoch_processing_credits_for_distribution(
                        &Epoch::new(1).unwrap(),
                        None,
                        platform_version,
                    )
                    .expect("expected to get processing fees made in epoch");

                // A vote costs 10_000_000
                // We did 5 votes in this epoch,
                // We had 39_810_000_000 left over, which is only the cost of 19 votes
                // So we basically have 39_810_000_000 + 50_000_000
                assert_eq!(processing_fees, 39_860_000_000);
            })
            .expect("Failed to create thread with custom stack size");

        // Wait for the thread to finish and assert that it didn't panic.
        handler.join().expect("Thread has panicked");
    }

    #[test]
    fn run_chain_with_voting_after_won_by_identity_no_specialized_funds_distribution_until_version_8(
    ) {
        // Define the desired stack size
        let stack_size = 4 * 1024 * 1024; //Let's set the stack size to be higher than the default 2MB

        let builder = std::thread::Builder::new()
            .stack_size(stack_size)
            .name("custom_stack_size_thread".into());

        let handler = builder
            .spawn(|| {
                // In this test the goal is to verify that when we hit version 8 that the specialized balances
                // that hadn't been properly distributed are distributed.
                let config = PlatformConfig {
                    validator_set: ValidatorSetConfig {
                        quorum_size: 10,
                        ..Default::default()
                    },
                    testing_configs: PlatformTestConfig::default_minimal_verifications(),
                    chain_lock: ChainLockConfig::default_100_67(),
                    instant_lock: InstantLockConfig::default_100_67(),
                    execution: ExecutionConfig {
                        //we disable document triggers because we are using dpns and dpns needs a preorder
                        use_document_triggers: false,

                        ..Default::default()
                    },
                    block_spacing_ms: 3000,
                    ..Default::default()
                };
                let mut platform = TestPlatformBuilder::new()
                    .with_config(config.clone())
                    .with_initial_protocol_version(7)
                    .build_with_mock_rpc();

                let platform_version = PlatformVersion::get(7).unwrap();

                let mut rng = StdRng::seed_from_u64(567);

                let mut simple_signer = SimpleSigner::default();

                let (mut identity1, keys1) =
                    Identity::random_identity_with_main_keys_with_private_key::<Vec<_>>(
                        2,
                        &mut rng,
                        platform_version,
                    )
                    .unwrap();

                simple_signer.add_keys(keys1);

                let (mut identity2, keys2) =
                    Identity::random_identity_with_main_keys_with_private_key::<Vec<_>>(
                        2,
                        &mut rng,
                        platform_version,
                    )
                    .unwrap();

                simple_signer.add_keys(keys2);

                let start_identities: Vec<(Identity, Option<StateTransition>)> =
                    create_state_transitions_for_identities(
                        vec![&mut identity1, &mut identity2],
                        &(dash_to_duffs!(1)..=dash_to_duffs!(1)),
                        &simple_signer,
                        &mut rng,
                        platform_version,
                    )
                    .into_iter()
                    .map(|(identity, transition)| (identity, Some(transition)))
                    .collect();

                let dpns_contract = platform
                    .drive
                    .cache
                    .system_data_contracts
                    .load_dpns()
                    .as_ref()
                    .clone();

                let document_type = dpns_contract
                    .document_type_for_name("domain")
                    .expect("expected a profile document type")
                    .to_owned_document_type();

                let identity1_id = start_identities.first().unwrap().0.id();
                let identity2_id = start_identities.last().unwrap().0.id();
                let document_op_1 = DocumentOp {
                    contract: dpns_contract.clone(),
                    action: DocumentAction::DocumentActionInsertSpecific(
                        BTreeMap::from([
                            ("label".into(), "quantum".into()),
                            ("normalizedLabel".into(), "quantum".into()),
                            ("normalizedParentDomainName".into(), "dash".into()),
                            (
                                "records".into(),
                                BTreeMap::from([("identity", Value::from(identity1_id))]).into(),
                            ),
                        ]),
                        Some(identity1_id),
                        DocumentFieldFillType::FillIfNotRequired,
                        DocumentFieldFillSize::AnyDocumentFillSize,
                    ),
                    document_type: document_type.clone(),
                };

                let document_op_2 = DocumentOp {
                    contract: dpns_contract.clone(),
                    action: DocumentAction::DocumentActionInsertSpecific(
                        BTreeMap::from([
                            ("label".into(), "quantum".into()),
                            ("normalizedLabel".into(), "quantum".into()),
                            ("normalizedParentDomainName".into(), "dash".into()),
                            (
                                "records".into(),
                                BTreeMap::from([(
                                    "identity",
                                    Value::from(start_identities.last().unwrap().0.id()),
                                )])
                                .into(),
                            ),
                        ]),
                        Some(identity2_id),
                        DocumentFieldFillType::FillIfNotRequired,
                        DocumentFieldFillSize::AnyDocumentFillSize,
                    ),
                    document_type: document_type.clone(),
                };

                let strategy = NetworkStrategy {
                    strategy: Strategy {
                        start_contracts: vec![],
                        operations: vec![
                            Operation {
                                op_type: OperationType::Document(document_op_1),
                                frequency: Frequency {
                                    times_per_block_range: 1..2,
                                    chance_per_block: None,
                                },
                            },
                            Operation {
                                op_type: OperationType::Document(document_op_2),
                                frequency: Frequency {
                                    times_per_block_range: 1..2,
                                    chance_per_block: None,
                                },
                            },
                        ],
                        start_identities: StartIdentities {
                            hard_coded: start_identities,
                            ..Default::default()
                        },
                        identity_inserts: Default::default(),

                        identity_contract_nonce_gaps: None,
                        signer: Some(simple_signer.clone()),
                    },
                    total_hpmns: 20,
                    extra_normal_mns: 0,
                    validator_quorum_count: 24,
                    chain_lock_quorum_count: 24,
                    upgrading_info: Some(UpgradingInfo {
                        current_protocol_version: 7,
                        proposed_protocol_versions_with_weight: vec![(7, 1)],
                        upgrade_three_quarters_life: 0.2,
                    }),

                    proposer_strategy: Default::default(),
                    rotate_quorums: false,
                    failure_testing: None,
                    query_testing: None,
                    verify_state_transition_results: true,
                    ..Default::default()
                };

                let mut voting_signer = Some(SimpleSigner::default());

                // On the first block we only have identities and contracts
                let ChainExecutionOutcome {
                    abci_app,
                    identities,
                    proposers,
                    validator_quorums,
                    current_validator_quorum_hash,
                    instant_lock_quorums,
                    current_proposer_versions,
                    end_time_ms,
                    identity_nonce_counter,
                    identity_contract_nonce_counter,
                    state_transition_results_per_block,
                    ..
                } = run_chain_for_strategy(
                    &mut platform,
                    2,
                    strategy.clone(),
                    config.clone(),
                    15,
                    &mut voting_signer,
                    &mut None,
                );

                let platform = abci_app.platform;

                let platform_state = platform.state.load();

                let state_transitions_block_2 = state_transition_results_per_block
                    .get(&2)
                    .expect("expected to get block 2");

                let first_document_insert_result = &state_transitions_block_2
                    .first()
                    .as_ref()
                    .expect("expected a document insert")
                    .1;
                assert_eq!(first_document_insert_result.code, 0);

                let second_document_insert_result = &state_transitions_block_2
                    .get(1)
                    .as_ref()
                    .expect("expected a document insert")
                    .1;

                assert_eq!(second_document_insert_result.code, 0); // we expect the second to also be insertable as they are both contested

                let block_start = platform_state
                    .last_committed_block_info()
                    .as_ref()
                    .unwrap()
                    .basic_info()
                    .height
                    + 1;
                let day_in_ms = 1000 * 60 * 60 * 24;
                let config = PlatformConfig {
                    chain_lock: ChainLockConfig::default_100_67(),
                    instant_lock: InstantLockConfig::default_100_67(),
                    execution: ExecutionConfig {
                        //we disable document triggers because we are using dpns and dpns needs a preorder
                        use_document_triggers: false,

                        ..Default::default()
                    },
                    block_spacing_ms: day_in_ms,
                    ..Default::default()
                };

                // On the first block we only have identities and contracts
                let ChainExecutionOutcome {
                    abci_app,
                    proposers,
                    validator_quorums,
                    current_validator_quorum_hash,
                    instant_lock_quorums,
                    end_time_ms,
                    identity_nonce_counter,
                    identity_contract_nonce_counter,
                    ..
                } = continue_chain_for_strategy(
                    abci_app,
                    ChainExecutionParameters {
                        block_start,
                        core_height_start: 1,
                        block_count: 16,
                        proposers,
                        validator_quorums,
                        current_validator_quorum_hash,
                        instant_lock_quorums,
                        current_proposer_versions: Some(current_proposer_versions.clone()),
                        current_identity_nonce_counter: identity_nonce_counter,
                        current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                        current_votes: BTreeMap::default(),
                        start_time_ms: 1681094380000,
                        current_time_ms: end_time_ms,
                        current_identities: Vec::new(),
                    },
                    NetworkStrategy {
                        strategy: Strategy {
                            start_contracts: vec![],
                            operations: vec![Operation {
                                op_type: OperationType::ResourceVote(ResourceVoteOp {
                                    resolved_vote_poll:
                                        ContestedDocumentResourceVotePollWithContractInfo {
                                            contract:
                                                DataContractOwnedResolvedInfo::OwnedDataContract(
                                                    dpns_contract.clone(),
                                                ),
                                            document_type_name: "domain".to_string(),
                                            index_name: "parentNameAndLabel".to_string(),
                                            index_values: vec!["dash".into(), "quantum".into()],
                                        },
                                    action: VoteAction {
                                        vote_choices_with_weights: vec![
                                            (ResourceVoteChoice::Abstain, 1),
                                            (ResourceVoteChoice::Lock, 1),
                                            (ResourceVoteChoice::TowardsIdentity(identity1_id), 2),
                                            (ResourceVoteChoice::TowardsIdentity(identity2_id), 10),
                                        ],
                                    },
                                }),
                                frequency: Frequency {
                                    times_per_block_range: 1..3,
                                    chance_per_block: None,
                                },
                            }],
                            start_identities: StartIdentities::default(),
                            identity_inserts: Default::default(),

                            identity_contract_nonce_gaps: None,
                            signer: voting_signer,
                        },
                        total_hpmns: 20,
                        extra_normal_mns: 0,
                        validator_quorum_count: 24,
                        chain_lock_quorum_count: 24,
                        upgrading_info: Some(UpgradingInfo {
                            current_protocol_version: 7,
                            proposed_protocol_versions_with_weight: vec![(7, 1)],
                            upgrade_three_quarters_life: 0.2,
                        }),

                        proposer_strategy: Default::default(),
                        rotate_quorums: false,
                        failure_testing: None,
                        query_testing: None,
                        verify_state_transition_results: true,
                        ..Default::default()
                    },
                    config.clone(),
                    StrategyRandomness::SeedEntropy(9),
                );

                let platform = abci_app.platform;

                // Now let's run a query for the vote totals

                let bincode_config = bincode::config::standard()
                    .with_big_endian()
                    .with_no_limit();

                let dash_encoded =
                    bincode::encode_to_vec(Value::Text("dash".to_string()), bincode_config)
                        .expect("expected to encode the word dash");

                let quantum_encoded =
                    bincode::encode_to_vec(Value::Text("quantum".to_string()), bincode_config)
                        .expect("expected to encode the word quantum");

                let index_name = "parentNameAndLabel".to_string();

                let query_validation_result = platform
                    .query_contested_resource_vote_state(
                        GetContestedResourceVoteStateRequest {
                            version: Some(get_contested_resource_vote_state_request::Version::V0(
                                GetContestedResourceVoteStateRequestV0 {
                                    contract_id: dpns_contract.id().to_vec(),
                                    document_type_name: document_type.name().clone(),
                                    index_name: index_name.clone(),
                                    index_values: vec![
                                        dash_encoded.clone(),
                                        quantum_encoded.clone(),
                                    ],
                                    result_type: ResultType::DocumentsAndVoteTally as i32,
                                    allow_include_locked_and_abstaining_vote_tally: true,
                                    start_at_identifier_info: None,
                                    count: None,
                                    prove: false,
                                },
                            )),
                        },
                        &platform_state,
                        platform_version,
                    )
                    .expect("expected to execute query")
                    .into_data()
                    .expect("expected query to be valid");

                let get_contested_resource_vote_state_response::Version::V0(
                    GetContestedResourceVoteStateResponseV0 {
                        metadata: _,
                        result,
                    },
                ) = query_validation_result.version.expect("expected a version");

                let Some(
            get_contested_resource_vote_state_response_v0::Result::ContestedResourceContenders(
                get_contested_resource_vote_state_response_v0::ContestedResourceContenders {
                    contenders,
                    abstain_vote_tally,
                    lock_vote_tally,
                    finished_vote_info,
                },
            ),
        ) = result
        else {
            panic!("expected contenders")
        };

                assert_eq!(contenders.len(), 2);

                let first_contender = contenders.first().unwrap();

                let second_contender = contenders.last().unwrap();

                assert_eq!(first_contender.identifier, identity2_id.to_vec());

                assert_eq!(second_contender.identifier, identity1_id.to_vec());

                // All vote counts are weighted, so for evonodes, these are in multiples of 4

                assert_eq!(
                    (
                        first_contender.vote_count,
                        second_contender.vote_count,
                        lock_vote_tally,
                        abstain_vote_tally
                    ),
                    (Some(64), Some(8), Some(0), Some(0))
                );

                assert_eq!(
                    finished_vote_info,
                    Some(FinishedVoteInfo {
                        finished_vote_outcome: FinishedVoteOutcome::TowardsIdentity.into(),
                        won_by_identity_id: Some(identity2_id.to_vec()),
                        finished_at_block_height: 17,
                        finished_at_core_block_height: 1,
                        finished_at_block_time_ms: 1682303986000,
                        finished_at_epoch: 1
                    })
                );

                // not let's see how much is in processing pools

                let processing_fees = platform
                    .drive
                    .get_epoch_processing_credits_for_distribution(
                        &Epoch::new(1).unwrap(),
                        None,
                        platform_version,
                    )
                    .expect("expected to get processing fees made in epoch");

                // A vote costs 10_000_000
                // Hence we did 4 votes in this epoch
                assert_eq!(processing_fees, 40_000_000);

                // Now let's upgrade to version 8

                let platform = abci_app.platform;

                let platform_state = platform.state.load();

                let block_start = platform_state
                    .last_committed_block_info()
                    .as_ref()
                    .unwrap()
                    .basic_info()
                    .height
                    + 1;

                let ten_hours_in_ms = 1000 * 60 * 60 * 10;
                let config = PlatformConfig {
                    chain_lock: ChainLockConfig::default_100_67(),
                    instant_lock: InstantLockConfig::default_100_67(),
                    execution: ExecutionConfig {
                        //we disable document triggers because we are using dpns and dpns needs a preorder
                        use_document_triggers: false,

                        ..Default::default()
                    },
                    block_spacing_ms: ten_hours_in_ms,
                    ..Default::default()
                };

                // We go 45 blocks later
                let ChainExecutionOutcome {
                    abci_app,
                    proposers,
                    validator_quorums,
                    current_validator_quorum_hash,
                    instant_lock_quorums,
                    end_time_ms,
                    identity_nonce_counter,
                    identity_contract_nonce_counter,
                    ..
                } = continue_chain_for_strategy(
                    abci_app,
                    ChainExecutionParameters {
                        block_start,
                        core_height_start: 1,
                        block_count: 45,
                        proposers,
                        validator_quorums,
                        current_validator_quorum_hash,
                        instant_lock_quorums,
                        current_proposer_versions: None,
                        current_identity_nonce_counter: identity_nonce_counter,
                        current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                        current_votes: BTreeMap::default(),
                        start_time_ms: 1681094380000,
                        current_time_ms: end_time_ms,
                        current_identities: Vec::new(),
                    },
                    NetworkStrategy {
                        strategy: Strategy {
                            start_contracts: vec![],
                            operations: vec![],
                            start_identities: StartIdentities::default(),
                            identity_inserts: Default::default(),

                            identity_contract_nonce_gaps: None,
                            signer: None,
                        },
                        total_hpmns: 20,
                        extra_normal_mns: 0,
                        validator_quorum_count: 24,
                        chain_lock_quorum_count: 24,
                        upgrading_info: Some(UpgradingInfo {
                            current_protocol_version: 8,
                            proposed_protocol_versions_with_weight: vec![(8, 1)],
                            upgrade_three_quarters_life: 0.1,
                        }),

                        proposer_strategy: Default::default(),
                        rotate_quorums: false,
                        failure_testing: None,
                        query_testing: None,
                        verify_state_transition_results: false,
                        ..Default::default()
                    },
                    config.clone(),
                    StrategyRandomness::SeedEntropy(9203),
                );

                let platform = abci_app.platform;

                let platform_state = platform.state.load();

                let mut block_start = platform_state
                    .last_committed_block_info()
                    .as_ref()
                    .unwrap()
                    .basic_info()
                    .height
                    + 1;

                // We need to create a few more contests

                let document_op_1 = DocumentOp {
                    contract: dpns_contract.clone(),
                    action: DocumentAction::DocumentActionInsertSpecific(
                        BTreeMap::from([
                            ("label".into(), "sam".into()),
                            ("normalizedLabel".into(), "sam".into()),
                            ("normalizedParentDomainName".into(), "dash".into()),
                            ("parentDomainName".into(), "dash".into()),
                            (
                                "records".into(),
                                BTreeMap::from([("identity", Value::from(identity1_id))]).into(),
                            ),
                        ]),
                        Some(identity1_id),
                        DocumentFieldFillType::FillIfNotRequired,
                        DocumentFieldFillSize::AnyDocumentFillSize,
                    ),
                    document_type: document_type.clone(),
                };

                let document_op_2 = DocumentOp {
                    contract: dpns_contract.clone(),
                    action: DocumentAction::DocumentActionInsertSpecific(
                        BTreeMap::from([
                            ("label".into(), "sam".into()),
                            ("normalizedLabel".into(), "sam".into()),
                            ("normalizedParentDomainName".into(), "dash".into()),
                            ("parentDomainName".into(), "dash".into()),
                            (
                                "records".into(),
                                BTreeMap::from([("identity", Value::from(identity2_id))]).into(),
                            ),
                        ]),
                        Some(identity2_id),
                        DocumentFieldFillType::FillIfNotRequired,
                        DocumentFieldFillSize::AnyDocumentFillSize,
                    ),
                    document_type: document_type.clone(),
                };

                let ChainExecutionOutcome {
                    abci_app,
                    proposers,
                    validator_quorums,
                    current_validator_quorum_hash,
                    instant_lock_quorums,
                    end_time_ms,
                    identity_nonce_counter,
                    identity_contract_nonce_counter,
                    ..
                } = continue_chain_for_strategy(
                    abci_app,
                    ChainExecutionParameters {
                        block_start,
                        core_height_start: 1,
                        block_count: 1,
                        proposers,
                        validator_quorums,
                        current_validator_quorum_hash,
                        instant_lock_quorums,
                        current_proposer_versions: None,
                        current_identity_nonce_counter: identity_nonce_counter,
                        current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                        current_votes: BTreeMap::default(),
                        start_time_ms: 1681094380000,
                        current_time_ms: end_time_ms,
                        current_identities: identities,
                    },
                    NetworkStrategy {
                        strategy: Strategy {
                            start_contracts: vec![],
                            operations: vec![
                                Operation {
                                    op_type: OperationType::Document(document_op_1),
                                    frequency: Frequency {
                                        times_per_block_range: 1..2,
                                        chance_per_block: None,
                                    },
                                },
                                Operation {
                                    op_type: OperationType::Document(document_op_2),
                                    frequency: Frequency {
                                        times_per_block_range: 1..2,
                                        chance_per_block: None,
                                    },
                                },
                            ],
                            start_identities: StartIdentities::default(),
                            identity_inserts: Default::default(),

                            identity_contract_nonce_gaps: None,
                            signer: Some(simple_signer),
                        },
                        total_hpmns: 20,
                        extra_normal_mns: 0,
                        validator_quorum_count: 24,
                        chain_lock_quorum_count: 24,
                        upgrading_info: Some(UpgradingInfo {
                            current_protocol_version: 8,
                            proposed_protocol_versions_with_weight: vec![(8, 1)],
                            upgrade_three_quarters_life: 0.1,
                        }),

                        proposer_strategy: Default::default(),
                        rotate_quorums: false,
                        failure_testing: None,
                        query_testing: None,
                        verify_state_transition_results: false,
                        ..Default::default()
                    },
                    config.clone(),
                    StrategyRandomness::SeedEntropy(9203),
                );

                block_start += 1;

                // We go 14 blocks later till version 8 is active
                let outcome = continue_chain_for_strategy(
                    abci_app,
                    ChainExecutionParameters {
                        block_start,
                        core_height_start: 1,
                        block_count: 14,
                        proposers,
                        validator_quorums,
                        current_validator_quorum_hash,
                        instant_lock_quorums,
                        current_proposer_versions: None,
                        current_identity_nonce_counter: identity_nonce_counter,
                        current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                        current_votes: BTreeMap::default(),
                        start_time_ms: 1681094380000,
                        current_time_ms: end_time_ms,
                        current_identities: Vec::new(),
                    },
                    NetworkStrategy {
                        strategy: Strategy {
                            start_contracts: vec![],
                            operations: vec![],
                            start_identities: StartIdentities::default(),
                            identity_inserts: Default::default(),

                            identity_contract_nonce_gaps: None,
                            signer: None,
                        },
                        total_hpmns: 20,
                        extra_normal_mns: 0,
                        validator_quorum_count: 24,
                        chain_lock_quorum_count: 24,
                        upgrading_info: Some(UpgradingInfo {
                            current_protocol_version: 8,
                            proposed_protocol_versions_with_weight: vec![(8, 1)],
                            upgrade_three_quarters_life: 0.1,
                        }),

                        proposer_strategy: Default::default(),
                        rotate_quorums: false,
                        failure_testing: None,
                        query_testing: None,
                        verify_state_transition_results: false,
                        ..Default::default()
                    },
                    config.clone(),
                    StrategyRandomness::SeedEntropy(9203),
                );

                let platform = outcome.abci_app.platform;
                platform
                    .drive
                    .fetch_versions_with_counter(None, &platform_version.drive)
                    .expect("expected to get versions");

                let state = platform.state.load();
                assert_eq!(
                    state
                        .last_committed_block_info()
                        .as_ref()
                        .unwrap()
                        .basic_info()
                        .epoch
                        .index,
                    4
                );
                assert_eq!(state.current_protocol_version_in_consensus(), 8);

                let processing_fees = platform
                    .drive
                    .get_epoch_processing_credits_for_distribution(
                        &Epoch::new(4).unwrap(),
                        None,
                        platform_version,
                    )
                    .expect("expected to get processing fees made in epoch");

                // A vote costs 10_000_000
                // There were 23 votes total so that means that there would have been 39_780_000_000 left over
                // We see that there is 39_780_000_000 to distribute
                assert_eq!(processing_fees, 39_780_000_000);
            })
            .expect("Failed to create thread with custom stack size");

        // Wait for the thread to finish and assert that it didn't panic.
        handler.join().expect("Thread has panicked");
    }
}
