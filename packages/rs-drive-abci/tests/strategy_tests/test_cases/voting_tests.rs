#[cfg(test)]
mod tests {
    use crate::execution::{continue_chain_for_strategy, run_chain_for_strategy};
    use crate::strategy::{ChainExecutionOutcome, ChainExecutionParameters, NetworkStrategy, StrategyRandomness, UpgradingInfo};
    use dash_platform_macros::stack_size;
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
    use std::collections::{BTreeMap, BTreeSet};
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
    use dpp::data_contract::DataContract;
    use dpp::platform_value::Identifier;
    use dpp::state_transition::StateTransition;
    use drive_abci::test::helpers::dpns::dpns_preorder_properties;
    use dpp::util::strings::convert_to_homograph_safe_chars;
    use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
    use drive::util::object_size_info::DataContractOwnedResolvedInfo;
    use drive::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
    use drive_abci::platform_types::platform_state::PlatformStateV0Methods;
    use strategy_tests::frequency::Frequency;
    use strategy_tests::operations::{DocumentAction, DocumentOp, Operation, OperationType, ResourceVoteOp, VoteAction, YesNoVoteOp};
    use dpp::dashcore_rpc::dashcore_rpc_json::MasternodeType;
    use dpp::decision_proposal::{decision_proposal_required_fund, DecisionProposalVotePoll, PROPOSAL_DOCUMENT_TYPE_NAME};
    use dpp::document::DocumentV0Getters;
    use dpp::voting::vote_choices::yes_no_abstain_vote_choice::YesNoAbstainVoteChoice;
    use dpp::voting::vote_info_storage::contested_document_vote_poll_stored_info::ContestedDocumentVotePollStoredInfoV0Getters;
    use dpp::voting::vote_info_storage::contested_document_vote_poll_winner_info::ContestedDocumentVotePollWinnerInfo;
    use dpp::voting::vote_info_storage::yes_no_vote_poll_stored_info::YesNoVotePollStatus;
    use dpp::voting::vote_polls::yes_no_vote_poll::VotingPower;
    use dpp::voting::vote_polls::VotePoll;
    use drive::drive::document::query::QueryDocumentsOutcomeV0Methods;
    use drive::query::vote_polls_by_end_date_query::VotePollsByEndDateDriveQuery;
    use drive::query::DriveDocumentQuery;
    use drive_abci::platform_types::platform::Platform;
    use drive_abci::rpc::core::MockCoreRPCLike;
    use dpp::block::block_info::BlockInfo;
    use strategy_tests::transitions::create_state_transitions_for_identities;
    use strategy_tests::{StartIdentities, StartAddresses, Strategy};
    use crate::addresses_with_balance::AddressesWithBalance;
    use dpp::platform_serialization::PLATFORM_BINCODE_CONFIG;

    const STACK_SIZE: usize = 4 * 1024 * 1024; // 4 MB

    /// The `preorder` op of `owner_id` for `label` under `dash` salted with `salt`, hashed as
    /// the DPNS contract of `platform_version` declares, and the `domain` op revealing it. From protocol version 15 a domain must reveal its owner's
    /// preorder made in an earlier block, so the preorder op runs in a block before the domain op.
    fn dpns_preorder_and_domain_ops(
        dpns_contract: &DataContract,
        owner_id: Identifier,
        label: &str,
        salt: [u8; 32],
        platform_version: &PlatformVersion,
    ) -> (DocumentOp, DocumentOp) {
        let normalized_label = convert_to_homograph_safe_chars(label);
        let domain_values: BTreeMap<String, Value> = BTreeMap::from([
            ("label".into(), label.into()),
            ("normalizedLabel".into(), normalized_label.clone().into()),
            ("parentDomainName".into(), "dash".into()),
            ("normalizedParentDomainName".into(), "dash".into()),
            ("preorderSalt".into(), salt.into()),
            (
                "records".into(),
                BTreeMap::from([("identity", Value::from(owner_id))]).into(),
            ),
        ]);
        let preorder_values: BTreeMap<String, Value> =
            dpns_preorder_properties(owner_id, &salt, &normalized_label, "dash", platform_version)
                .into_iter()
                .collect();

        let preorder_op = DocumentOp {
            contract: dpns_contract.clone(),
            action: DocumentAction::DocumentActionInsertSpecific(
                preorder_values,
                Some(owner_id),
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
            ),
            document_type: dpns_contract
                .document_type_for_name("preorder")
                .expect("expected the preorder document type")
                .to_owned_document_type(),
        };

        let domain_op = DocumentOp {
            contract: dpns_contract.clone(),
            action: DocumentAction::DocumentActionInsertSpecific(
                domain_values,
                Some(owner_id),
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
            ),
            document_type: dpns_contract
                .document_type_for_name("domain")
                .expect("expected the domain document type")
                .to_owned_document_type(),
        };

        (preorder_op, domain_op)
    }

    /// `document_op` run once in every block of its stage
    fn once_per_block(document_op: DocumentOp) -> Operation {
        Operation {
            op_type: OperationType::Document(document_op),
            frequency: Frequency {
                times_per_block_range: 1..2,
                chance_per_block: None,
            },
        }
    }

    /// Runs the block after `outcome` with `operations`, carrying on the identities, nonces and
    /// proposer versions it ended with
    async fn continue_chain_for_one_block<'a>(
        outcome: ChainExecutionOutcome<'a>,
        operations: Vec<Operation>,
        config: PlatformConfig,
    ) -> ChainExecutionOutcome<'a> {
        let block_start = outcome
            .abci_app
            .platform
            .state
            .load()
            .last_committed_block_info()
            .as_ref()
            .expect("expected a committed block")
            .basic_info()
            .height
            + 1;

        let ChainExecutionOutcome {
            abci_app,
            identities,
            addresses_with_balance,
            proposers,
            validator_quorums,
            current_validator_quorum_hash,
            current_proposer_versions,
            instant_lock_quorums,
            identity_nonce_counter,
            identity_contract_nonce_counter,
            end_time_ms,
            strategy,
            ..
        } = outcome;

        continue_chain_for_strategy(
            abci_app,
            ChainExecutionParameters {
                block_start,
                core_height_start: 1,
                block_count: 1,
                proposers,
                validator_quorums,
                current_validator_quorum_hash,
                instant_lock_quorums,
                current_proposer_versions: Some(current_proposer_versions),
                current_identity_nonce_counter: identity_nonce_counter,
                current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                current_votes: BTreeMap::default(),
                start_time_ms: strategy.start_time_ms,
                current_time_ms: end_time_ms,
                current_identities: identities,
                current_addresses_with_balance: addresses_with_balance,
            },
            NetworkStrategy {
                strategy: Strategy {
                    operations,
                    ..strategy.strategy
                },
                ..strategy
            },
            config,
            StrategyRandomness::SeedEntropy(7),
        )
        .await
    }

    #[stack_size(STACK_SIZE)]
    #[test]
    async fn run_chain_accepts_contested_documents_before_epoch_4() {
        let epoch_time_length_s = 60;

        let config = PlatformConfig {
            testing_configs: PlatformTestConfig {
                block_signing: false,
                store_platform_state: false,
                block_commit_signature_verification: false,
                disable_instant_lock_signature_verification: true,
                disable_checkpoints: true,
                ..Default::default()
            },
            chain_lock: ChainLockConfig::default_100_67(),
            instant_lock: InstantLockConfig::default_100_67(),
            execution: ExecutionConfig {
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

        simple_signer.add_identity_public_keys(keys1);

        let start_identities: Vec<(Identity, Option<StateTransition>)> =
            create_state_transitions_for_identities(
                vec![&mut identity1],
                &(dash_to_duffs!(1)..=dash_to_duffs!(1)),
                &simple_signer,
                &mut rng,
                platform_version,
            )
            .await
            .into_iter()
            .map(|(identity, transition)| (identity, Some(transition)))
            .collect();

        let dpns_contract = platform
            .drive
            .cache
            .system_data_contracts
            .load_dpns(platform_version)
            .expect("expected the dpns system contract")
            .as_ref()
            .clone();

        let identity1_id = start_identities.first().unwrap().0.id();
        let (preorder_op_1, domain_op_1) = dpns_preorder_and_domain_ops(
            &dpns_contract,
            identity1_id,
            "quantum",
            [1u8; 32],
            platform_version,
        );

        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![once_per_block(preorder_op_1)],
                start_identities: StartIdentities {
                    hard_coded: start_identities.clone(),
                    ..Default::default()
                },
                start_addresses: StartAddresses::default(),
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

        let outcome = run_chain_for_strategy(
            &mut platform,
            2,
            strategy,
            config.clone(),
            15,
            &mut voting_signer,
            &mut None,
        )
        .await;

        let ChainExecutionOutcome {
            abci_app,
            state_transition_results_per_block,
            ..
        } = continue_chain_for_one_block(outcome, vec![once_per_block(domain_op_1)], config).await;

        let platform_state = abci_app.platform.state.load();

        // The identity is created in the first block, its preorder in the second and the
        // contested DPNS name in the third, long before epoch 4
        assert_eq!(platform_state.last_committed_block_epoch().index, 2);

        let state_transitions_block_3 = state_transition_results_per_block
            .get(&3)
            .expect("expected to get block 3");

        assert_eq!(state_transitions_block_3.len(), 1);

        let (state_transition, execution_result) = state_transitions_block_3
            .first()
            .expect("expected a document insert");

        assert_matches!(state_transition, StateTransition::Batch(_));

        assert_eq!(execution_result.code, 0);
    }

    #[stack_size(STACK_SIZE)]
    #[test]
    async fn run_chain_block_two_state_transitions_conflicting_unique_index_inserted_same_block_version_8(
    ) {
        // In this test we try to insert two state transitions with the same unique index
        // We use the DPNS contract, and we insert two documents both with the same "name"
        // This is a common scenario we should see quite often
        let config = PlatformConfig {
            testing_configs: PlatformTestConfig::default_minimal_verifications(),
            chain_lock: ChainLockConfig::default_100_67(),
            instant_lock: InstantLockConfig::default_100_67(),
            execution: ExecutionConfig {
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

        simple_signer.add_identity_public_keys(keys1);

        let (mut identity2, keys2) = Identity::random_identity_with_main_keys_with_private_key::<
            Vec<_>,
        >(2, &mut rng, platform_version)
        .unwrap();

        simple_signer.add_identity_public_keys(keys2);

        let start_identities: Vec<(Identity, Option<StateTransition>)> =
            create_state_transitions_for_identities(
                vec![&mut identity1, &mut identity2],
                &(dash_to_duffs!(1)..=dash_to_duffs!(1)),
                &simple_signer,
                &mut rng,
                platform_version,
            )
            .await
            .into_iter()
            .map(|(identity, transition)| (identity, Some(transition)))
            .collect();

        let dpns_contract = platform
            .drive
            .cache
            .system_data_contracts
            .load_dpns(platform_version)
            .expect("expected the dpns system contract")
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
                start_addresses: StartAddresses::default(),
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
        )
        .await;

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

        let config = PLATFORM_BINCODE_CONFIG;

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

        // Look up by identity id rather than vec position — see explanatory
        // comment in the abstain-only test below.
        let identity1_contender = contenders
            .iter()
            .find(|c| c.identifier == identity1_id.to_vec())
            .expect("identity1 in contenders");
        let identity2_contender = contenders
            .iter()
            .find(|c| c.identifier == identity2_id.to_vec())
            .expect("identity2 in contenders");

        assert_eq!(
            identity1_contender.document.as_ref().map(hex::encode),
            Some("0021278016512ff707d45a0aa8893a4b1ba67b08158b31bc2bda11a0addb8ab1f4341e6a8e6c01488a75104a8dbc73501ded5cd23abaf688349a8ab34b1157513201000700000187690895980000018769089598000001876908959800077175616e74756d077175616e74756d000464617368002101341e6a8e6c01488a75104a8dbc73501ded5cd23abaf688349a8ab34b115751320100".to_string())
        );

        assert_eq!(
            identity2_contender.document.as_ref().map(hex::encode),
            Some("0010b1d465b94520e76daf018ee6b2740c871d51b7ce9690f60fc7a4a70f1bfd6f3a3c745d6c4d3b88ea52976eaab80dbaa77258885b7b6d8e1edfd00fed72414a01000700000187690895980000018769089598000001876908959800077175616e74756d077175616e74756d0004646173680021013a3c745d6c4d3b88ea52976eaab80dbaa77258885b7b6d8e1edfd00fed72414a0101".to_string())
        );

        assert_eq!(identity1_contender.vote_count, Some(0));
        assert_eq!(identity2_contender.vote_count, Some(0));
    }

    #[stack_size(STACK_SIZE)]
    #[test]
    async fn run_chain_with_voting_on_conflicting_index_just_abstain_votes() {
        // In this test we try to insert two state transitions with the same unique index
        // We use the DPNS contract, and we insert two documents both with the same "name"
        // This is a common scenario we should see quite often
        let config = PlatformConfig {
            testing_configs: PlatformTestConfig::default_minimal_verifications(),
            chain_lock: ChainLockConfig::default_100_67(),
            instant_lock: InstantLockConfig::default_100_67(),
            execution: ExecutionConfig {
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

        simple_signer.add_identity_public_keys(keys1);

        let (mut identity2, keys2) = Identity::random_identity_with_main_keys_with_private_key::<
            Vec<_>,
        >(2, &mut rng, platform_version)
        .unwrap();

        simple_signer.add_identity_public_keys(keys2);

        let start_identities: Vec<(Identity, Option<StateTransition>)> =
            create_state_transitions_for_identities(
                vec![&mut identity1, &mut identity2],
                &(dash_to_duffs!(1)..=dash_to_duffs!(1)),
                &simple_signer,
                &mut rng,
                platform_version,
            )
            .await
            .into_iter()
            .map(|(identity, transition)| (identity, Some(transition)))
            .collect();

        let dpns_contract = platform
            .drive
            .cache
            .system_data_contracts
            .load_dpns(platform_version)
            .expect("expected the dpns system contract")
            .as_ref()
            .clone();

        let document_type = dpns_contract
            .document_type_for_name("domain")
            .expect("expected a profile document type")
            .to_owned_document_type();

        let identity1_id = start_identities.first().unwrap().0.id();
        let identity2_id = start_identities.last().unwrap().0.id();
        let (preorder_op_1, domain_op_1) = dpns_preorder_and_domain_ops(
            &dpns_contract,
            identity1_id,
            "quantum",
            [1u8; 32],
            platform_version,
        );
        let (preorder_op_2, domain_op_2) = dpns_preorder_and_domain_ops(
            &dpns_contract,
            identity2_id,
            "quantum",
            [2u8; 32],
            platform_version,
        );

        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![once_per_block(preorder_op_1), once_per_block(preorder_op_2)],
                start_identities: StartIdentities {
                    hard_coded: start_identities,
                    ..Default::default()
                },
                start_addresses: StartAddresses::default(),
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

        // The first block creates the identities and the second their preorders
        let outcome = run_chain_for_strategy(
            &mut platform,
            2,
            strategy.clone(),
            config.clone(),
            15,
            &mut voting_signer,
            &mut None,
        )
        .await;

        // The contested names reveal the preorders in the third block
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
        } = continue_chain_for_one_block(
            outcome,
            vec![once_per_block(domain_op_1), once_per_block(domain_op_2)],
            config.clone(),
        )
        .await;

        let platform = abci_app.platform;

        let platform_state = platform.state.load();

        let state_transitions_block_3 = state_transition_results_per_block
            .get(&3)
            .expect("expected to get block 3");

        let first_document_insert_result = &state_transitions_block_3
            .first()
            .as_ref()
            .expect("expected a document insert")
            .1;
        assert_eq!(first_document_insert_result.code, 0);

        let second_document_insert_result = &state_transitions_block_3
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
                current_addresses_with_balance: AddressesWithBalance::default(),
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
                    start_addresses: StartAddresses::default(),
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
        )
        .await;

        let platform = outcome.abci_app.platform;

        // Now let's run a query for the vote totals

        let config = PLATFORM_BINCODE_CONFIG;

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

        // Look up by identity id rather than vec position — contender ordering
        // follows identifier sort, which derives from the asset-lock txid and
        // shifts whenever the asset-lock fixture's wire format changes.
        let identity1_contender = contenders
            .iter()
            .find(|c| c.identifier == identity1_id.to_vec())
            .expect("identity1 in contenders");
        let identity2_contender = contenders
            .iter()
            .find(|c| c.identifier == identity2_id.to_vec())
            .expect("identity2 in contenders");

        assert!(identity1_contender.document.is_some());
        assert!(identity2_contender.document.is_some());

        assert_eq!(identity1_contender.vote_count, Some(0));
        assert_eq!(identity2_contender.vote_count, Some(0));

        assert_eq!(abstain_vote_tally, Some(124));
    }

    #[stack_size(STACK_SIZE)]
    #[test]
    async fn run_chain_with_voting_on_conflicting_index_various_votes() {
        // In this test we try to insert two state transitions with the same unique index
        // We use the DPNS contract, and we insert two documents both with the same "name"
        // This is a common scenario we should see quite often
        let config = PlatformConfig {
            testing_configs: PlatformTestConfig::default_minimal_verifications(),
            chain_lock: ChainLockConfig::default_100_67(),
            instant_lock: InstantLockConfig::default_100_67(),
            execution: ExecutionConfig {
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

        simple_signer.add_identity_public_keys(keys1);

        let (mut identity2, keys2) = Identity::random_identity_with_main_keys_with_private_key::<
            Vec<_>,
        >(2, &mut rng, platform_version)
        .unwrap();

        simple_signer.add_identity_public_keys(keys2);

        let start_identities: Vec<(Identity, Option<StateTransition>)> =
            create_state_transitions_for_identities(
                vec![&mut identity1, &mut identity2],
                &(dash_to_duffs!(1)..=dash_to_duffs!(1)),
                &simple_signer,
                &mut rng,
                platform_version,
            )
            .await
            .into_iter()
            .map(|(identity, transition)| (identity, Some(transition)))
            .collect();

        let dpns_contract = platform
            .drive
            .cache
            .system_data_contracts
            .load_dpns(platform_version)
            .expect("expected the dpns system contract")
            .as_ref()
            .clone();

        let document_type = dpns_contract
            .document_type_for_name("domain")
            .expect("expected a profile document type")
            .to_owned_document_type();

        let identity1_id = start_identities.first().unwrap().0.id();
        let identity2_id = start_identities.last().unwrap().0.id();
        let (preorder_op_1, domain_op_1) = dpns_preorder_and_domain_ops(
            &dpns_contract,
            identity1_id,
            "quantum",
            [1u8; 32],
            platform_version,
        );
        let (preorder_op_2, domain_op_2) = dpns_preorder_and_domain_ops(
            &dpns_contract,
            identity2_id,
            "quantum",
            [2u8; 32],
            platform_version,
        );

        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![once_per_block(preorder_op_1), once_per_block(preorder_op_2)],
                start_identities: StartIdentities {
                    hard_coded: start_identities,
                    ..Default::default()
                },
                start_addresses: StartAddresses::default(),
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

        // The first block creates the identities and the second their preorders
        let outcome = run_chain_for_strategy(
            &mut platform,
            2,
            strategy.clone(),
            config.clone(),
            15,
            &mut voting_signer,
            &mut None,
        )
        .await;

        // The contested names reveal the preorders in the third block
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
        } = continue_chain_for_one_block(
            outcome,
            vec![once_per_block(domain_op_1), once_per_block(domain_op_2)],
            config.clone(),
        )
        .await;

        let platform = abci_app.platform;

        let platform_state = platform.state.load();

        let state_transitions_block_3 = state_transition_results_per_block
            .get(&3)
            .expect("expected to get block 3");

        let first_document_insert_result = &state_transitions_block_3
            .first()
            .as_ref()
            .expect("expected a document insert")
            .1;
        assert_eq!(first_document_insert_result.code, 0);

        let second_document_insert_result = &state_transitions_block_3
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
                current_addresses_with_balance: AddressesWithBalance::default(),
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
                    start_addresses: StartAddresses::default(),
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
        )
        .await;

        let platform = outcome.abci_app.platform;

        // Now let's run a query for the vote totals

        let config = PLATFORM_BINCODE_CONFIG;

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

        // Look up by identity id rather than vec position — see explanatory
        // comment in the abstain-only test above.
        let identity1_contender = contenders
            .iter()
            .find(|c| c.identifier == identity1_id.to_vec())
            .expect("identity1 in contenders");
        let identity2_contender = contenders
            .iter()
            .find(|c| c.identifier == identity2_id.to_vec())
            .expect("identity2 in contenders");

        assert!(identity1_contender.document.is_some());
        assert!(identity2_contender.document.is_some());

        // All vote counts are weighted, so for evonodes, these are in multiples of 4

        assert_eq!(identity1_contender.vote_count, Some(56));
        assert_eq!(identity2_contender.vote_count, Some(52));

        assert_eq!(lock_vote_tally, Some(16));

        assert_eq!(abstain_vote_tally, Some(8));

        assert_eq!(finished_vote_info, None);
    }

    #[stack_size(STACK_SIZE)]
    #[test]
    async fn run_chain_with_voting_after_won_by_identity_no_specialized_funds_distribution() {
        // In this test we try to insert two state transitions with the same unique index
        // We use the DPNS contract, and we insert two documents both with the same "name"
        // This is a common scenario we should see quite often
        let config = PlatformConfig {
            testing_configs: PlatformTestConfig::default_minimal_verifications(),
            chain_lock: ChainLockConfig::default_100_67(),
            instant_lock: InstantLockConfig::default_100_67(),
            execution: ExecutionConfig {
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

        let (mut identity1, keys1) = Identity::random_identity_with_main_keys_with_private_key::<
            Vec<_>,
        >(2, &mut rng, platform_version)
        .unwrap();

        simple_signer.add_identity_public_keys(keys1);

        let (mut identity2, keys2) = Identity::random_identity_with_main_keys_with_private_key::<
            Vec<_>,
        >(2, &mut rng, platform_version)
        .unwrap();

        simple_signer.add_identity_public_keys(keys2);

        let start_identities: Vec<(Identity, Option<StateTransition>)> =
            create_state_transitions_for_identities(
                vec![&mut identity1, &mut identity2],
                &(dash_to_duffs!(1)..=dash_to_duffs!(1)),
                &simple_signer,
                &mut rng,
                platform_version,
            )
            .await
            .into_iter()
            .map(|(identity, transition)| (identity, Some(transition)))
            .collect();

        let dpns_contract = platform
            .drive
            .cache
            .system_data_contracts
            .load_dpns(platform_version)
            .expect("expected the dpns system contract")
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
                start_addresses: StartAddresses::default(),
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
        )
        .await;

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
                current_addresses_with_balance: AddressesWithBalance::default(),
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
                    start_addresses: StartAddresses::default(),
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
        )
        .await;

        let platform = outcome.abci_app.platform;

        // Now let's run a query for the vote totals

        let config = PLATFORM_BINCODE_CONFIG;

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

        // Look up by identity id rather than vec position — see explanatory
        // comment in the abstain-only test above.
        let identity1_contender = contenders
            .iter()
            .find(|c| c.identifier == identity1_id.to_vec())
            .expect("identity1 in contenders");
        let identity2_contender = contenders
            .iter()
            .find(|c| c.identifier == identity2_id.to_vec())
            .expect("identity2 in contenders");

        // All vote counts are weighted, so for evonodes, these are in multiples of 4

        assert_eq!(identity1_contender.vote_count, Some(4));
        assert_eq!(identity2_contender.vote_count, Some(60));

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
    }

    #[stack_size(STACK_SIZE)]
    #[test]
    async fn run_chain_with_voting_after_won_by_identity_with_specialized_funds_distribution() {
        // A vote costs 2_000_000 from protocol version 14.
        // We did 5 votes in this epoch, and from protocol version 14 each contested document
        // contributes 0.1 DASH (10_000_000_000) to the vote resolution fund instead of 0.2
        // DASH, so the two contenders funded 20_000_000_000, of which 19 votes cost 38_000_000
        // and 19_962_000_000 was left over when the vote finished.
        // So we basically have 19_962_000_000 + 10_000_000
        run_chain_with_voting_after_won_by_identity_with_specialized_funds_distribution_at_protocol_version(
            PlatformVersion::latest().protocol_version,
            19_972_000_000,
        )
        .await;
    }

    /// PROTOCOL_VERSION_13: a vote costs 10_000_000 and each contested document contributes
    /// 0.2 DASH, so the two contenders funded 40_000_000_000, of which 19 votes cost
    /// 190_000_000 and 39_810_000_000 was left over; with the 5 votes of the epoch that is
    /// 39_810_000_000 + 50_000_000.
    #[stack_size(STACK_SIZE)]
    #[test]
    async fn run_chain_with_voting_after_won_by_identity_with_specialized_funds_distribution_protocol_version_13(
    ) {
        run_chain_with_voting_after_won_by_identity_with_specialized_funds_distribution_at_protocol_version(
            13,
            39_860_000_000,
        )
        .await;
    }

    async fn run_chain_with_voting_after_won_by_identity_with_specialized_funds_distribution_at_protocol_version(
        protocol_version: dpp::version::ProtocolVersion,
        expected_processing_fees: u64,
    ) {
        // In this test we try to insert two state transitions with the same unique index
        // We use the DPNS contract, and we insert two documents both with the same "name"
        // This is a common scenario we should see quite often
        let config = PlatformConfig {
            testing_configs: PlatformTestConfig::default_minimal_verifications(),
            chain_lock: ChainLockConfig::default_100_67(),
            instant_lock: InstantLockConfig::default_100_67(),
            execution: ExecutionConfig {
                use_document_triggers: false,
                ..Default::default()
            },
            block_spacing_ms: 3000,
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .with_initial_protocol_version(protocol_version)
            .build_with_mock_rpc();

        let platform_version = PlatformVersion::get(protocol_version)
            .expect("expected platform version for the requested protocol_version");
        // An older protocol version is held by the proposers the first run sets up, whose
        // versions the continued run carries on; the latest needs no holding
        let upgrading_info =
            (protocol_version != PlatformVersion::latest().protocol_version).then(|| {
                UpgradingInfo {
                    current_protocol_version: protocol_version,
                    proposed_protocol_versions_with_weight: vec![(protocol_version, 1)],
                    upgrade_three_quarters_life: 0.2,
                }
            });

        let mut rng = StdRng::seed_from_u64(567);

        let mut simple_signer = SimpleSigner::default();

        let (mut identity1, keys1) = Identity::random_identity_with_main_keys_with_private_key::<
            Vec<_>,
        >(2, &mut rng, platform_version)
        .unwrap();

        simple_signer.add_identity_public_keys(keys1);

        let (mut identity2, keys2) = Identity::random_identity_with_main_keys_with_private_key::<
            Vec<_>,
        >(2, &mut rng, platform_version)
        .unwrap();

        simple_signer.add_identity_public_keys(keys2);

        let start_identities: Vec<(Identity, Option<StateTransition>)> =
            create_state_transitions_for_identities(
                vec![&mut identity1, &mut identity2],
                &(dash_to_duffs!(1)..=dash_to_duffs!(1)),
                &simple_signer,
                &mut rng,
                platform_version,
            )
            .await
            .into_iter()
            .map(|(identity, transition)| (identity, Some(transition)))
            .collect();

        let dpns_contract = platform
            .drive
            .cache
            .system_data_contracts
            .load_dpns(platform_version)
            .expect("expected the dpns system contract")
            .as_ref()
            .clone();

        let document_type = dpns_contract
            .document_type_for_name("domain")
            .expect("expected a profile document type")
            .to_owned_document_type();

        let identity1_id = start_identities.first().unwrap().0.id();
        let identity2_id = start_identities.last().unwrap().0.id();
        let (preorder_op_1, domain_op_1) = dpns_preorder_and_domain_ops(
            &dpns_contract,
            identity1_id,
            "quantum",
            [1u8; 32],
            platform_version,
        );
        let (preorder_op_2, domain_op_2) = dpns_preorder_and_domain_ops(
            &dpns_contract,
            identity2_id,
            "quantum",
            [2u8; 32],
            platform_version,
        );

        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![once_per_block(preorder_op_1), once_per_block(preorder_op_2)],
                start_identities: StartIdentities {
                    hard_coded: start_identities,
                    ..Default::default()
                },
                start_addresses: StartAddresses::default(),
                identity_inserts: Default::default(),
                identity_contract_nonce_gaps: None,
                signer: Some(simple_signer),
            },
            total_hpmns: 100,
            extra_normal_mns: 0,
            validator_quorum_count: 24,
            chain_lock_quorum_count: 24,
            upgrading_info,
            proposer_strategy: Default::default(),
            rotate_quorums: false,
            failure_testing: None,
            query_testing: None,
            verify_state_transition_results: true,
            ..Default::default()
        };

        let mut voting_signer = Some(SimpleSigner::default());

        // The first block creates the identities and the second their preorders
        let outcome = run_chain_for_strategy(
            &mut platform,
            2,
            strategy.clone(),
            config.clone(),
            15,
            &mut voting_signer,
            &mut None,
        )
        .await;

        // The contested names reveal the preorders in the third block
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
        } = continue_chain_for_one_block(
            outcome,
            vec![once_per_block(domain_op_1), once_per_block(domain_op_2)],
            config.clone(),
        )
        .await;

        let platform = abci_app.platform;

        let platform_state = platform.state.load();

        let state_transitions_block_3 = state_transition_results_per_block
            .get(&3)
            .expect("expected to get block 3");

        let first_document_insert_result = &state_transitions_block_3
            .first()
            .as_ref()
            .expect("expected a document insert")
            .1;
        assert_eq!(first_document_insert_result.code, 0);

        let second_document_insert_result = &state_transitions_block_3
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
                current_addresses_with_balance: AddressesWithBalance::default(),
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
                    start_addresses: StartAddresses::default(),
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
        )
        .await;

        let platform = outcome.abci_app.platform;

        // Now let's run a query for the vote totals

        let config = PLATFORM_BINCODE_CONFIG;

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

        // Look up by identity id rather than vec position — see explanatory
        // comment in the abstain-only test above.
        let identity1_contender = contenders
            .iter()
            .find(|c| c.identifier == identity1_id.to_vec())
            .expect("identity1 in contenders");
        let identity2_contender = contenders
            .iter()
            .find(|c| c.identifier == identity2_id.to_vec())
            .expect("identity2 in contenders");

        // All vote counts are weighted, so for evonodes, these are in multiples of 4

        // 19 votes were cast

        assert_eq!(identity1_contender.vote_count, Some(4));
        assert_eq!(identity2_contender.vote_count, Some(60));

        assert_eq!(lock_vote_tally, Some(4));

        assert_eq!(abstain_vote_tally, Some(8));

        // The names were registered in block 3, behind their preorders, so the contest ends
        // in block 18
        assert_eq!(
            finished_vote_info,
            Some(FinishedVoteInfo {
                finished_vote_outcome: FinishedVoteOutcome::TowardsIdentity.into(),
                won_by_identity_id: Some(identity2_id.to_vec()),
                finished_at_block_height: 18,
                finished_at_core_block_height: 1,
                finished_at_block_time_ms: 1682303989000,
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

        assert_eq!(processing_fees, expected_processing_fees);
    }

    #[stack_size(STACK_SIZE)]
    #[test]
    async fn run_chain_with_voting_after_won_by_identity_no_specialized_funds_distribution_until_version_8(
    ) {
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

        let (mut identity1, keys1) = Identity::random_identity_with_main_keys_with_private_key::<
            Vec<_>,
        >(2, &mut rng, platform_version)
        .unwrap();

        simple_signer.add_identity_public_keys(keys1);

        let (mut identity2, keys2) = Identity::random_identity_with_main_keys_with_private_key::<
            Vec<_>,
        >(2, &mut rng, platform_version)
        .unwrap();

        simple_signer.add_identity_public_keys(keys2);

        let start_identities: Vec<(Identity, Option<StateTransition>)> =
            create_state_transitions_for_identities(
                vec![&mut identity1, &mut identity2],
                &(dash_to_duffs!(1)..=dash_to_duffs!(1)),
                &simple_signer,
                &mut rng,
                platform_version,
            )
            .await
            .into_iter()
            .map(|(identity, transition)| (identity, Some(transition)))
            .collect();

        let dpns_contract = platform
            .drive
            .cache
            .system_data_contracts
            .load_dpns(platform_version)
            .expect("expected the dpns system contract")
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
                start_addresses: StartAddresses::default(),
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
            addresses_with_balance,
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
        )
        .await;

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
                current_addresses_with_balance: AddressesWithBalance::default(),
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
                    start_addresses: StartAddresses::default(),
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
        )
        .await;

        let platform = abci_app.platform;

        // Now let's run a query for the vote totals

        let bincode_config = PLATFORM_BINCODE_CONFIG;

        let dash_encoded = bincode::encode_to_vec(Value::Text("dash".to_string()), bincode_config)
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

        // Look up by identity id rather than vec position — see explanatory
        // comment in the abstain-only test above.
        let identity1_contender = contenders
            .iter()
            .find(|c| c.identifier == identity1_id.to_vec())
            .expect("identity1 in contenders");
        let identity2_contender = contenders
            .iter()
            .find(|c| c.identifier == identity2_id.to_vec())
            .expect("identity2 in contenders");

        // All vote counts are weighted, so for evonodes, these are in multiples of 4

        assert_eq!(
            (
                identity1_contender.vote_count,
                identity2_contender.vote_count,
                lock_vote_tally,
                abstain_vote_tally
            ),
            (Some(8), Some(64), Some(0), Some(0))
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
                current_addresses_with_balance: AddressesWithBalance::default(),
            },
            NetworkStrategy {
                strategy: Strategy {
                    start_contracts: vec![],
                    operations: vec![],
                    start_identities: StartIdentities::default(),
                    start_addresses: StartAddresses::default(),
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
        )
        .await;

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
                current_addresses_with_balance: addresses_with_balance,
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
                    start_addresses: StartAddresses::default(),
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
        )
        .await;

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
                current_addresses_with_balance: AddressesWithBalance::default(),
            },
            NetworkStrategy {
                strategy: Strategy {
                    start_contracts: vec![],
                    operations: vec![],
                    start_identities: StartIdentities::default(),
                    start_addresses: StartAddresses::default(),
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
        )
        .await;

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
    }

    /// A decision proposal the chain test below opens: its rule, its end date and how the
    /// masternodes answer it
    struct ProposalPlan {
        title: &'static str,
        pass_percent: u8,
        minimum_turnout_percent: u8,
        end_date: u64,
        answers: Vec<(YesNoAbstainVoteChoice, u8)>,
    }

    /// Every poll of the end date index from `start` to `end`, both included (the whole index
    /// when `None`), by end date
    fn vote_polls_by_end_date(
        platform: &Platform<MockCoreRPCLike>,
        range: Option<(u64, u64)>,
        platform_version: &PlatformVersion,
    ) -> BTreeMap<u64, Vec<VotePoll>> {
        VotePollsByEndDateDriveQuery {
            start_time: range.map(|(start, _)| (start, true)),
            end_time: range.map(|(_, end)| (end, true)),
            limit: Some(100),
            offset: None,
            order_ascending: true,
        }
        .execute_no_proof(&platform.drive, None, &mut vec![], platform_version)
        .expect("expected to read the end date index")
    }

    /// Decision proposals (protocol version 15) in a running chain. Four identities open eight
    /// proposals in one block, three of them ending on the very end date of a DPNS contest
    /// opened the block before, the others one to four weeks out. Masternodes then answer every
    /// proposal and vote on the contest for four weeks of daily blocks.
    ///
    /// Each proposal opens its poll funded with exactly the decision proposal fund, filed under
    /// its end date beside the contest. Due polls close in end date order as soon as a block has
    /// room, a block closing at most `maximum_vote_polls_to_process` of them, the contest's
    /// included: the contest and the three proposals sharing its end date take two daily blocks,
    /// and the proposal due half a day later waits a block behind them. Each result follows from
    /// the poll's rule and its tallies: some pass, some fail on the answers and some fail short
    /// of their turnout. The rest of each fund is spent, the end date index is emptied, and the
    /// proposals stay. The strategy verifies the proof of every vote and create as it runs.
    #[stack_size(STACK_SIZE)]
    #[test]
    async fn run_chain_with_decision_proposals_closing_beside_a_dpns_contest() {
        const WEEK_MS: u64 = 604_800_000;
        const DAY_MS: u64 = 86_400_000;

        let config = PlatformConfig {
            testing_configs: PlatformTestConfig::default_minimal_verifications(),
            chain_lock: ChainLockConfig::default_100_67(),
            instant_lock: InstantLockConfig::default_100_67(),
            execution: ExecutionConfig {
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

        let mut rng = StdRng::seed_from_u64(5679);

        let mut simple_signer = SimpleSigner::default();

        // Two identities contest a DPNS name and four make two proposals each
        let mut contenders = Vec::new();
        let mut proposers = Vec::new();
        for index in 0..6 {
            let (identity, keys) = Identity::random_identity_with_main_keys_with_private_key::<
                Vec<_>,
            >(2, &mut rng, platform_version)
            .unwrap();
            simple_signer.add_identity_public_keys(keys);
            if index < 2 {
                contenders.push(identity);
            } else {
                proposers.push(identity);
            }
        }

        let mut start_identities: Vec<(Identity, Option<StateTransition>)> =
            create_state_transitions_for_identities(
                contenders.iter_mut(),
                &(dash_to_duffs!(1)..=dash_to_duffs!(1)),
                &simple_signer,
                &mut rng,
                platform_version,
            )
            .await
            .into_iter()
            .map(|(identity, transition)| (identity, Some(transition)))
            .collect();
        // Two proposals cost two funds of 0.5 Dash and their fees
        start_identities.extend(
            create_state_transitions_for_identities(
                proposers.iter_mut(),
                &(dash_to_duffs!(2)..=dash_to_duffs!(2)),
                &simple_signer,
                &mut rng,
                platform_version,
            )
            .await
            .into_iter()
            .map(|(identity, transition)| (identity, Some(transition))),
        );

        let contender_ids: Vec<Identifier> = start_identities[..2]
            .iter()
            .map(|(identity, _)| identity.id())
            .collect();
        let proposer_ids: Vec<Identifier> = start_identities[2..]
            .iter()
            .map(|(identity, _)| identity.id())
            .collect();

        let dpns_contract = platform
            .drive
            .cache
            .system_data_contracts
            .load_dpns(platform_version)
            .expect("expected the dpns system contract")
            .as_ref()
            .clone();

        let decision_proposals_contract = platform
            .drive
            .cache
            .system_data_contracts
            .load_decision_proposals(platform_version)
            .expect("expected the decision proposals system contract")
            .as_ref()
            .clone();
        let proposal_type = decision_proposals_contract
            .document_type_for_name(PROPOSAL_DOCUMENT_TYPE_NAME)
            .expect("expected the proposal document type")
            .to_owned_document_type();

        let (preorder_op_1, domain_op_1) = dpns_preorder_and_domain_ops(
            &dpns_contract,
            contender_ids[0],
            "quantum",
            [1u8; 32],
            platform_version,
        );
        let (preorder_op_2, domain_op_2) = dpns_preorder_and_domain_ops(
            &dpns_contract,
            contender_ids[1],
            "quantum",
            [2u8; 32],
            platform_version,
        );

        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![once_per_block(preorder_op_1), once_per_block(preorder_op_2)],
                start_identities: StartIdentities {
                    hard_coded: start_identities,
                    ..Default::default()
                },
                start_addresses: StartAddresses::default(),
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

        // The first block creates the identities and the second the preorders
        let outcome = run_chain_for_strategy(
            &mut platform,
            2,
            strategy,
            config.clone(),
            15,
            &mut voting_signer,
            &mut None,
        )
        .await;

        // The contested names reveal the preorders in the third block, opening the contest
        let outcome = continue_chain_for_one_block(
            outcome,
            vec![once_per_block(domain_op_1), once_per_block(domain_op_2)],
            config.clone(),
        )
        .await;

        let opened_polls =
            vote_polls_by_end_date(outcome.abci_app.platform, None, platform_version);
        assert_eq!(
            opened_polls.len(),
            1,
            "expected only the contest to be open"
        );
        let contest_end_date = *opened_polls
            .keys()
            .next()
            .expect("expected the contest's end date");

        // The proposals are created in the fourth block, at the time the run ended on
        let proposals_block_time = outcome.end_time_ms;

        let passing = vec![
            (YesNoAbstainVoteChoice::Yes, 6),
            (YesNoAbstainVoteChoice::No, 2),
            (YesNoAbstainVoteChoice::Abstain, 1),
        ];
        let failing = vec![
            (YesNoAbstainVoteChoice::Yes, 1),
            (YesNoAbstainVoteChoice::No, 3),
            (YesNoAbstainVoteChoice::Abstain, 1),
        ];
        let even = vec![
            (YesNoAbstainVoteChoice::Yes, 1),
            (YesNoAbstainVoteChoice::No, 1),
            (YesNoAbstainVoteChoice::Abstain, 1),
        ];
        let plans = vec![
            ProposalPlan {
                title: "Contest day, simple majority",
                pass_percent: 50,
                minimum_turnout_percent: 0,
                end_date: contest_end_date,
                answers: passing.clone(),
            },
            ProposalPlan {
                title: "Contest day, four fifths",
                pass_percent: 80,
                minimum_turnout_percent: 0,
                end_date: contest_end_date,
                answers: passing.clone(),
            },
            ProposalPlan {
                title: "Contest day, half the network",
                pass_percent: 50,
                minimum_turnout_percent: 50,
                end_date: contest_end_date,
                answers: passing.clone(),
            },
            ProposalPlan {
                title: "Shortest run",
                pass_percent: 50,
                minimum_turnout_percent: 10,
                end_date: proposals_block_time + WEEK_MS,
                answers: failing.clone(),
            },
            ProposalPlan {
                title: "Three weeks, simple majority",
                pass_percent: 50,
                minimum_turnout_percent: 0,
                end_date: proposals_block_time + 3 * WEEK_MS,
                answers: failing,
            },
            ProposalPlan {
                title: "Three weeks, seven tenths",
                pass_percent: 70,
                minimum_turnout_percent: 10,
                end_date: proposals_block_time + 3 * WEEK_MS,
                answers: passing.clone(),
            },
            ProposalPlan {
                title: "Two weeks and a half day",
                pass_percent: 60,
                minimum_turnout_percent: 0,
                end_date: proposals_block_time + 2 * WEEK_MS + DAY_MS / 2,
                answers: even,
            },
            ProposalPlan {
                title: "Longest run",
                pass_percent: 66,
                minimum_turnout_percent: 25,
                end_date: proposals_block_time + 4 * WEEK_MS,
                answers: passing,
            },
        ];

        // Each proposer makes two of the proposals, in separate batches of the same block
        let proposal_ops = plans
            .iter()
            .enumerate()
            .map(|(index, plan)| {
                once_per_block(DocumentOp {
                    contract: decision_proposals_contract.clone(),
                    action: DocumentAction::DocumentActionInsertSpecific(
                        BTreeMap::from([
                            ("title".to_string(), Value::Text(plan.title.to_string())),
                            (
                                "description".to_string(),
                                Value::Text(format!("Proposal {} of the chain test", index + 1)),
                            ),
                            ("passPercent".to_string(), Value::U8(plan.pass_percent)),
                            (
                                "minimumTurnoutPercent".to_string(),
                                Value::U8(plan.minimum_turnout_percent),
                            ),
                            ("endDate".to_string(), Value::U64(plan.end_date)),
                        ]),
                        Some(proposer_ids[index / 2]),
                        DocumentFieldFillType::DoNotFillIfNotRequired,
                        DocumentFieldFillSize::AnyDocumentFillSize,
                    ),
                    document_type: proposal_type.clone(),
                })
            })
            .collect();

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
        } = continue_chain_for_one_block(outcome, proposal_ops, config.clone()).await;

        let platform = abci_app.platform;

        let block_4_results = state_transition_results_per_block
            .get(&4)
            .expect("expected the results of block 4");
        assert_eq!(block_4_results.len(), plans.len());
        for (_, result) in block_4_results {
            assert_eq!(result.code, 0, "a proposal was refused: {}", result.info);
        }

        let all_proposals_query = DriveDocumentQuery::all_items_query(
            &decision_proposals_contract,
            proposal_type.as_ref(),
            None,
        );
        let proposal_documents = platform
            .drive
            .query_documents(
                all_proposals_query.clone(),
                None,
                false,
                None,
                Some(platform_version.protocol_version),
            )
            .expect("expected to query the proposals")
            .documents_owned();
        assert_eq!(proposal_documents.len(), plans.len());

        // Each proposal with its plan, its poll as consensus reads it from the proposal, and
        // the poll's id
        let proposals: Vec<(&ProposalPlan, DecisionProposalVotePoll, Identifier)> =
            proposal_documents
                .iter()
                .map(|document| {
                    let title = document
                        .get("title")
                        .and_then(|title| title.as_text())
                        .expect("expected a proposal title");
                    let plan = plans
                        .iter()
                        .find(|plan| plan.title == title)
                        .expect("expected a planned proposal");
                    let poll = DecisionProposalVotePoll::from_proposal(
                        document.id(),
                        document.properties(),
                        platform_version,
                    )
                    .expect("expected to read the proposal")
                    .into_data()
                    .expect("expected a valid proposal");
                    let poll_id = poll
                        .vote_poll
                        .unique_id(platform_version)
                        .expect("expected the poll id");
                    (plan, poll, poll_id)
                })
                .collect();

        // Every poll is open, with its rule, and holds the whole fund
        let required_fund = decision_proposal_required_fund(platform_version);
        for (plan, poll, poll_id) in &proposals {
            assert_eq!(poll.end_date, plan.end_date, "{}", plan.title);
            assert_eq!(
                platform
                    .drive
                    .fetch_prefunded_specialized_balance(
                        poll_id.to_buffer(),
                        None,
                        platform_version
                    )
                    .expect("expected to read the poll's fund"),
                Some(required_fund),
                "{}",
                plan.title
            );
            let stored_info = platform
                .drive
                .fetch_yes_no_vote_poll_stored_info(*poll_id, None, platform_version)
                .expect("expected to read the poll")
                .unwrap_or_else(|| panic!("expected {} to have opened its poll", plan.title));
            assert_eq!(stored_info.vote_poll(), &poll.vote_poll);
            assert_matches!(
                stored_info.status(),
                YesNoVotePollStatus::Started(block_info) if block_info.height == 4
            );
        }

        // The contest's end date lists the contest and the three proposals ending with it
        let at_contest_end_date = vote_polls_by_end_date(
            platform,
            Some((contest_end_date, contest_end_date)),
            platform_version,
        );
        let polls_at_contest_end_date = at_contest_end_date
            .get(&contest_end_date)
            .expect("expected polls at the contest's end date");
        assert_eq!(
            polls_at_contest_end_date
                .iter()
                .filter(|vote_poll| matches!(
                    vote_poll,
                    VotePoll::ContestedDocumentResourceVotePoll(_)
                ))
                .count(),
            1
        );
        let yes_no_polls_at_contest_end_date: BTreeSet<Identifier> = polls_at_contest_end_date
            .iter()
            .filter_map(|vote_poll| match vote_poll {
                VotePoll::YesNoVotePoll(yes_no_vote_poll) => Some(
                    yes_no_vote_poll
                        .unique_id(platform_version)
                        .expect("expected the poll id"),
                ),
                _ => None,
            })
            .collect();
        let proposals_ending_with_the_contest: BTreeSet<Identifier> = proposals
            .iter()
            .filter(|(plan, ..)| plan.end_date == contest_end_date)
            .map(|(_, _, poll_id)| *poll_id)
            .collect();
        assert_eq!(proposals_ending_with_the_contest.len(), 3);
        assert_eq!(
            yes_no_polls_at_contest_end_date,
            proposals_ending_with_the_contest
        );

        // Four weeks of daily blocks: masternodes answer every open proposal and vote on the
        // contest
        let contest = ContestedDocumentResourceVotePollWithContractInfo {
            contract: DataContractOwnedResolvedInfo::OwnedDataContract(dpns_contract.clone()),
            document_type_name: "domain".to_string(),
            index_name: "parentNameAndLabel".to_string(),
            index_values: vec!["dash".into(), "quantum".into()],
        };
        let mut operations = vec![Operation {
            op_type: OperationType::ResourceVote(ResourceVoteOp {
                resolved_vote_poll: contest.clone(),
                action: VoteAction {
                    vote_choices_with_weights: vec![
                        (ResourceVoteChoice::Abstain, 1),
                        (ResourceVoteChoice::Lock, 1),
                        (ResourceVoteChoice::TowardsIdentity(contender_ids[0]), 2),
                        (ResourceVoteChoice::TowardsIdentity(contender_ids[1]), 10),
                    ],
                },
            }),
            frequency: Frequency {
                times_per_block_range: 1..3,
                chance_per_block: None,
            },
        }];
        operations.extend(proposals.iter().map(|(plan, poll, poll_id)| Operation {
            op_type: OperationType::YesNoVote(YesNoVoteOp {
                vote_poll_id: *poll_id,
                end_date: poll.end_date,
                vote_choices_with_weights: plan.answers.clone(),
            }),
            frequency: Frequency {
                times_per_block_range: 1..4,
                chance_per_block: None,
            },
        }));

        let daily_block_start = platform
            .state
            .load()
            .last_committed_block_info()
            .as_ref()
            .expect("expected a committed block")
            .basic_info()
            .height
            + 1;
        let daily_block_start_time = end_time_ms;
        let daily_config = PlatformConfig {
            chain_lock: ChainLockConfig::default_100_67(),
            instant_lock: InstantLockConfig::default_100_67(),
            execution: ExecutionConfig {
                use_document_triggers: false,
                ..Default::default()
            },
            block_spacing_ms: DAY_MS,
            ..Default::default()
        };

        let outcome = continue_chain_for_strategy(
            abci_app,
            ChainExecutionParameters {
                block_start: daily_block_start,
                core_height_start: 1,
                block_count: 30,
                proposers,
                validator_quorums,
                current_validator_quorum_hash,
                instant_lock_quorums,
                current_proposer_versions: Some(current_proposer_versions),
                current_identity_nonce_counter: identity_nonce_counter,
                current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                current_votes: BTreeMap::default(),
                start_time_ms: 1681094380000,
                current_time_ms: end_time_ms,
                current_identities: Vec::new(),
                current_addresses_with_balance: AddressesWithBalance::default(),
            },
            NetworkStrategy {
                strategy: Strategy {
                    start_contracts: vec![],
                    operations,
                    start_identities: StartIdentities::default(),
                    start_addresses: StartAddresses::default(),
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
            daily_config,
            StrategyRandomness::SeedEntropy(9),
        )
        .await;

        let platform = outcome.abci_app.platform;
        let platform_state = platform.state.load();

        // Every vote was accepted
        for (height, results) in &outcome.state_transition_results_per_block {
            for (_, result) in results {
                assert_eq!(
                    result.code, 0,
                    "a transition was refused in block {}: {}",
                    height, result.info
                );
            }
        }

        // The contest closed
        let (_, contest_stored_info) = platform
            .drive
            .fetch_contested_document_vote_poll_stored_info(&contest, None, None, platform_version)
            .expect("expected to read the contest");
        let contest_stored_info = contest_stored_info.expect("expected the contest's record");
        let contest_finalization_block = contest_stored_info
            .last_finalization_block()
            .expect("expected the contest to have closed");
        assert_eq!(
            contest_stored_info.winner(),
            ContestedDocumentVotePollWinnerInfo::WonByIdentity(contender_ids[1])
        );

        // Every poll's name, end date and closing block, the contest's included
        let mut closings: Vec<(&str, u64, BlockInfo)> =
            vec![("DPNS contest", contest_end_date, contest_finalization_block)];

        // A share of the network is measured against the closing block's masternode list,
        // each masternode weighed as its vote is
        let limits = &platform_version.system_limits;
        let total_voting_power: VotingPower = platform_state
            .full_masternode_list()
            .values()
            .filter(|masternode| masternode.state.pose_ban_height.is_none())
            .map(|masternode| {
                VotingPower::from(match masternode.node_type {
                    MasternodeType::Regular => limits.yes_no_vote_poll_masternode_vote_strength,
                    MasternodeType::Evo => limits.yes_no_vote_poll_evonode_vote_strength,
                })
            })
            .sum();

        let mut passed = Vec::new();
        let mut failed_on_the_answers = Vec::new();
        let mut failed_short_of_turnout = Vec::new();
        for (plan, poll, poll_id) in &proposals {
            let stored_info = platform
                .drive
                .fetch_yes_no_vote_poll_stored_info(*poll_id, None, platform_version)
                .expect("expected to read the poll")
                .unwrap_or_else(|| panic!("expected {} to keep its poll's record", plan.title));
            let result = stored_info
                .result()
                .unwrap_or_else(|| panic!("expected {} to have closed", plan.title));

            assert_eq!(result.start_block.height, 4, "{}", plan.title);
            closings.push((plan.title, poll.end_date, result.finalization_block.clone()));

            // The minimum is the turnout share of the network, rounded up, and the result
            // follows from the rule: yes beyond the pass percent of yes plus no, and yes plus
            // no reaching the minimum
            assert_eq!(
                result.required_voting_power,
                (VotingPower::from(plan.minimum_turnout_percent) * total_voting_power)
                    .div_ceil(100),
                "{}",
                plan.title
            );
            let cast = result.yes_voting_power + result.no_voting_power;
            let reaches_turnout = cast >= result.required_voting_power;
            let enough_yes = u64::from(result.yes_voting_power) * 100
                > u64::from(plan.pass_percent) * u64::from(cast);
            assert_eq!(
                result.passed,
                reaches_turnout && enough_yes,
                "{}",
                plan.title
            );
            assert_eq!(
                result.passed,
                poll.vote_poll
                    .passes(
                        result.yes_voting_power,
                        result.no_voting_power,
                        total_voting_power,
                        platform_version
                    )
                    .expect("expected to decide the poll"),
                "{}",
                plan.title
            );
            match (result.passed, reaches_turnout) {
                (true, _) => passed.push(plan.title),
                (false, true) => failed_on_the_answers.push(plan.title),
                (false, false) => failed_short_of_turnout.push(plan.title),
            }

            // What was left of the fund went to processing
            assert_eq!(
                platform
                    .drive
                    .fetch_prefunded_specialized_balance(
                        poll_id.to_buffer(),
                        None,
                        platform_version
                    )
                    .expect("expected to read the poll's fund"),
                None,
                "{}",
                plan.title
            );
        }
        assert!(
            !passed.is_empty()
                && !failed_on_the_answers.is_empty()
                && !failed_short_of_turnout.is_empty(),
            "passed {:?}, failed on the answers {:?}, failed short of turnout {:?}",
            passed,
            failed_on_the_answers,
            failed_short_of_turnout
        );

        // Due polls close in end date order, at most `maximum_vote_polls_to_process` a block: a
        // poll closes in the first daily block at or past its end date that has room, and every
        // block it waits through closes as many polls as it may, none due later than it
        let maximum_vote_polls_to_process = usize::from(
            platform_version
                .drive_abci
                .validation_and_processing
                .event_constants
                .maximum_vote_polls_to_process,
        );
        let mut end_dates_closed_by_height: BTreeMap<u64, Vec<u64>> = BTreeMap::new();
        for (_, end_date, block) in &closings {
            end_dates_closed_by_height
                .entry(block.height)
                .or_default()
                .push(*end_date);
        }
        // The daily blocks start at the height and time the proposals' block ended on
        let first_due_height = |end_date: u64| {
            daily_block_start + (end_date - daily_block_start_time).div_ceil(DAY_MS)
        };
        for (title, end_date, block) in &closings {
            assert_eq!(
                block.time_ms,
                daily_block_start_time + (block.height - daily_block_start) * DAY_MS
            );
            assert!(
                block.time_ms >= *end_date,
                "{} closed before its end date",
                title
            );
            for height in first_due_height(*end_date)..block.height {
                let closed = end_dates_closed_by_height
                    .get(&height)
                    .map(Vec::as_slice)
                    .unwrap_or_default();
                assert!(
                    closed.len() == maximum_vote_polls_to_process
                        && closed
                            .iter()
                            .all(|closed_end_date| closed_end_date <= end_date),
                    "{} waited in block {}, which closed polls due at {:?}",
                    title,
                    height,
                    closed
                );
            }
        }
        for (height, end_dates) in &end_dates_closed_by_height {
            assert!(
                end_dates.len() <= maximum_vote_polls_to_process,
                "block {} closed {} polls",
                height,
                end_dates.len()
            );
        }
        // Four polls due on the contest's end date take two blocks, so the proposal due half a
        // day later waits a block behind them
        assert_eq!(maximum_vote_polls_to_process, 2);
        let (_, waiting_end_date, waiting_block) = closings
            .iter()
            .find(|(title, ..)| *title == "Two weeks and a half day")
            .expect("expected the proposal due after the contest");
        assert_eq!(
            waiting_block.height,
            first_due_height(*waiting_end_date) + 1
        );

        // Nothing is left in the end date index, and the proposals stay
        assert!(vote_polls_by_end_date(platform, None, platform_version).is_empty());
        assert_eq!(
            platform
                .drive
                .query_documents(
                    all_proposals_query,
                    None,
                    false,
                    None,
                    Some(platform_version.protocol_version),
                )
                .expect("expected to query the proposals")
                .documents_owned()
                .len(),
            plans.len()
        );
    }
}
