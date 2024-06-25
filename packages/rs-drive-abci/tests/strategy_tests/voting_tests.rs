#[cfg(test)]
mod tests {
    use crate::execution::{continue_chain_for_strategy, run_chain_for_strategy};
    use crate::strategy::{ChainExecutionOutcome, ChainExecutionParameters, NetworkStrategy, StrategyRandomness};
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::document_type::random_document::{
        DocumentFieldFillSize, DocumentFieldFillType,
    };
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::Identity;
    use dpp::platform_value::Value;
    use drive_abci::config::{ExecutionConfig, PlatformConfig};
    use drive_abci::test::helpers::setup::TestPlatformBuilder;
    use platform_version::version::PlatformVersion;
    use rand::prelude::StdRng;
    use rand::SeedableRng;
    use simple_signer::signer::SimpleSigner;
    use std::collections::BTreeMap;
    use dapi_grpc::platform::v0::{get_contested_resource_vote_state_request, get_contested_resource_vote_state_response, GetContestedResourceVoteStateRequest};
    use dapi_grpc::platform::v0::get_contested_resource_vote_state_request::get_contested_resource_vote_state_request_v0::ResultType;
    use dapi_grpc::platform::v0::get_contested_resource_vote_state_request::GetContestedResourceVoteStateRequestV0;
    use dapi_grpc::platform::v0::get_contested_resource_vote_state_response::{get_contested_resource_vote_state_response_v0, GetContestedResourceVoteStateResponseV0};
    use dpp::block::extended_block_info::v0::ExtendedBlockInfoV0Getters;
    use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
    use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
    use drive::drive::object_size_info::DataContractOwnedResolvedInfo;
    use drive::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
    use drive_abci::platform_types::platform_state::v0::PlatformStateV0Methods;
    use strategy_tests::frequency::Frequency;
    use strategy_tests::operations::{DocumentAction, DocumentOp, Operation, OperationType, ResourceVoteOp, VoteAction};
    use strategy_tests::transitions::create_state_transitions_for_identities;
    use strategy_tests::{StartIdentities, Strategy};

    #[test]
    fn run_chain_block_two_state_transitions_conflicting_unique_index_inserted_same_block() {
        // In this test we try to insert two state transitions with the same unique index
        // We use the DPNS contract, and we insert two documents both with the same "name"
        // This is a common scenario we should see quite often
        let config = PlatformConfig {
            validator_set_quorum_size: 100,
            validator_set_quorum_type: "llmq_100_67".to_string(),
            chain_lock_quorum_type: "llmq_100_67".to_string(),
            execution: ExecutionConfig {
                //we disable document triggers because we are using dpns and dpns needs a preorder
                use_document_triggers: false,
                validator_set_rotation_block_count: 25,
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

        let (identity1, keys1) =
            Identity::random_identity_with_main_keys_with_private_key::<Vec<_>>(
                2,
                &mut rng,
                platform_version,
            )
            .unwrap();

        simple_signer.add_keys(keys1);

        let (identity2, keys2) =
            Identity::random_identity_with_main_keys_with_private_key::<Vec<_>>(
                2,
                &mut rng,
                platform_version,
            )
            .unwrap();

        simple_signer.add_keys(keys2);

        let start_identities = create_state_transitions_for_identities(
            vec![identity1, identity2],
            &simple_signer,
            &mut rng,
            platform_version,
        );

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
                        BTreeMap::from([("dashUniqueIdentityId", Value::from(identity1_id))])
                            .into(),
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
                            "dashUniqueIdentityId",
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

        // On the first block we only have identities and contracts
        let outcome = run_chain_for_strategy(
            &mut platform,
            2,
            strategy.clone(),
            config.clone(),
            15,
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
            first_contender.document,
            Some(vec![
                0, 24, 85, 248, 135, 55, 81, 210, 5, 93, 112, 104, 77, 97, 177, 49, 255, 108, 242,
                0, 83, 232, 168, 214, 145, 55, 49, 246, 246, 126, 99, 17, 108, 41, 18, 75, 231,
                232, 111, 151, 233, 89, 137, 74, 103, 169, 204, 7, 140, 62, 1, 6, 212, 191, 207,
                191, 52, 188, 64, 58, 79, 9, 153, 37, 180, 0, 0, 7, 113, 117, 97, 110, 116, 117,
                109, 7, 113, 117, 97, 110, 116, 117, 109, 1, 9, 112, 48, 81, 101, 48, 107, 49, 65,
                122, 4, 100, 97, 115, 104, 48, 165, 41, 91, 32, 215, 12, 4, 215, 10, 9, 207, 71,
                187, 248, 211, 105, 252, 147, 22, 127, 31, 203, 145, 6, 255, 132, 220, 231, 96, 76,
                195, 34, 1, 41, 18, 75, 231, 232, 111, 151, 233, 89, 137, 74, 103, 169, 204, 7,
                140, 62, 1, 6, 212, 191, 207, 191, 52, 188, 64, 58, 79, 9, 153, 37, 180, 0, 1, 0
            ])
        );

        assert_eq!(
            second_contender.document,
            Some(vec![
                0, 23, 193, 35, 24, 227, 101, 215, 103, 217, 98, 152, 114, 80, 94, 3, 27, 65, 246,
                202, 212, 59, 205, 101, 140, 243, 61, 26, 152, 167, 199, 96, 133, 139, 137, 72,
                166, 128, 21, 1, 187, 224, 67, 30, 61, 153, 77, 207, 113, 207, 90, 42, 9, 57, 254,
                81, 176, 230, 0, 7, 97, 153, 171, 164, 251, 0, 0, 7, 113, 117, 97, 110, 116, 117,
                109, 7, 113, 117, 97, 110, 116, 117, 109, 1, 36, 65, 50, 104, 52, 88, 69, 66, 112,
                116, 74, 101, 99, 48, 101, 98, 87, 53, 67, 52, 89, 106, 72, 119, 82, 81, 48, 51,
                88, 54, 83, 99, 75, 103, 89, 111, 97, 4, 100, 97, 115, 104, 110, 35, 254, 120, 68,
                194, 240, 23, 122, 207, 220, 40, 135, 147, 185, 9, 126, 239, 26, 0, 22, 196, 197,
                243, 182, 218, 58, 240, 230, 102, 185, 157, 34, 1, 139, 137, 72, 166, 128, 21, 1,
                187, 224, 67, 30, 61, 153, 77, 207, 113, 207, 90, 42, 9, 57, 254, 81, 176, 230, 0,
                7, 97, 153, 171, 164, 251, 0, 1, 0
            ])
        );

        assert_eq!(first_contender.identifier, identity2_id.to_vec());

        assert_eq!(second_contender.identifier, identity1_id.to_vec());

        assert_eq!(first_contender.vote_count, Some(0));

        assert_eq!(second_contender.vote_count, Some(0));
    }

    #[test]
    fn run_chain_with_voting_on_conflicting_index() {
        // In this test we try to insert two state transitions with the same unique index
        // We use the DPNS contract, and we insert two documents both with the same "name"
        // This is a common scenario we should see quite often
        let config = PlatformConfig {
            validator_set_quorum_size: 100,
            validator_set_quorum_type: "llmq_100_67".to_string(),
            chain_lock_quorum_type: "llmq_100_67".to_string(),
            execution: ExecutionConfig {
                //we disable document triggers because we are using dpns and dpns needs a preorder
                use_document_triggers: false,
                validator_set_rotation_block_count: 25,
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

        let (identity1, keys1) =
            Identity::random_identity_with_main_keys_with_private_key::<Vec<_>>(
                2,
                &mut rng,
                platform_version,
            )
            .unwrap();

        simple_signer.add_keys(keys1);

        let (identity2, keys2) =
            Identity::random_identity_with_main_keys_with_private_key::<Vec<_>>(
                2,
                &mut rng,
                platform_version,
            )
            .unwrap();

        simple_signer.add_keys(keys2);

        let start_identities = create_state_transitions_for_identities(
            vec![identity1, identity2],
            &simple_signer,
            &mut rng,
            platform_version,
        );

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
                        BTreeMap::from([("dashUniqueIdentityId", Value::from(identity1_id))])
                            .into(),
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
                            "dashUniqueIdentityId",
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
            quorums,
            current_quorum_hash,
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
                quorums,
                current_quorum_hash,
                current_proposer_versions: Some(current_proposer_versions.clone()),
                current_identity_nonce_counter: identity_nonce_counter,
                current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                start_time_ms: 1681094380000,
                current_time_ms: end_time_ms,
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
                                vote_choices_with_weights: vec![(
                                    ResourceVoteChoice::TowardsIdentity(identity1_id),
                                    1,
                                )],
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
            first_contender.document,
            Some(vec![
                0, 24, 85, 248, 135, 55, 81, 210, 5, 93, 112, 104, 77, 97, 177, 49, 255, 108, 242,
                0, 83, 232, 168, 214, 145, 55, 49, 246, 246, 126, 99, 17, 108, 41, 18, 75, 231,
                232, 111, 151, 233, 89, 137, 74, 103, 169, 204, 7, 140, 62, 1, 6, 212, 191, 207,
                191, 52, 188, 64, 58, 79, 9, 153, 37, 180, 0, 0, 7, 113, 117, 97, 110, 116, 117,
                109, 7, 113, 117, 97, 110, 116, 117, 109, 1, 9, 112, 48, 81, 101, 48, 107, 49, 65,
                122, 4, 100, 97, 115, 104, 48, 165, 41, 91, 32, 215, 12, 4, 215, 10, 9, 207, 71,
                187, 248, 211, 105, 252, 147, 22, 127, 31, 203, 145, 6, 255, 132, 220, 231, 96, 76,
                195, 34, 1, 41, 18, 75, 231, 232, 111, 151, 233, 89, 137, 74, 103, 169, 204, 7,
                140, 62, 1, 6, 212, 191, 207, 191, 52, 188, 64, 58, 79, 9, 153, 37, 180, 0, 1, 0
            ])
        );

        assert_eq!(
            second_contender.document,
            Some(vec![
                0, 23, 193, 35, 24, 227, 101, 215, 103, 217, 98, 152, 114, 80, 94, 3, 27, 65, 246,
                202, 212, 59, 205, 101, 140, 243, 61, 26, 152, 167, 199, 96, 133, 139, 137, 72,
                166, 128, 21, 1, 187, 224, 67, 30, 61, 153, 77, 207, 113, 207, 90, 42, 9, 57, 254,
                81, 176, 230, 0, 7, 97, 153, 171, 164, 251, 0, 0, 7, 113, 117, 97, 110, 116, 117,
                109, 7, 113, 117, 97, 110, 116, 117, 109, 1, 36, 65, 50, 104, 52, 88, 69, 66, 112,
                116, 74, 101, 99, 48, 101, 98, 87, 53, 67, 52, 89, 106, 72, 119, 82, 81, 48, 51,
                88, 54, 83, 99, 75, 103, 89, 111, 97, 4, 100, 97, 115, 104, 110, 35, 254, 120, 68,
                194, 240, 23, 122, 207, 220, 40, 135, 147, 185, 9, 126, 239, 26, 0, 22, 196, 197,
                243, 182, 218, 58, 240, 230, 102, 185, 157, 34, 1, 139, 137, 72, 166, 128, 21, 1,
                187, 224, 67, 30, 61, 153, 77, 207, 113, 207, 90, 42, 9, 57, 254, 81, 176, 230, 0,
                7, 97, 153, 171, 164, 251, 0, 1, 0
            ])
        );

        assert_eq!(first_contender.identifier, identity2_id.to_vec());

        assert_eq!(second_contender.identifier, identity1_id.to_vec());

        assert_eq!(first_contender.vote_count, Some(0));

        assert_eq!(second_contender.vote_count, Some(0));
    }
}
