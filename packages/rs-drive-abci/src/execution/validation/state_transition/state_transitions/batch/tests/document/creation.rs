use super::*;

mod creation_tests {
    use super::*;
    use dapi_grpc::platform::v0::{get_contested_resource_vote_state_request, get_contested_resource_vote_state_response, GetContestedResourceVoteStateRequest, GetContestedResourceVoteStateResponse};
    use dapi_grpc::platform::v0::get_contested_resource_vote_state_request::get_contested_resource_vote_state_request_v0::ResultType;
    use dapi_grpc::platform::v0::get_contested_resource_vote_state_request::GetContestedResourceVoteStateRequestV0;
    use dapi_grpc::platform::v0::get_contested_resource_vote_state_response::{get_contested_resource_vote_state_response_v0, GetContestedResourceVoteStateResponseV0};
    use assert_matches::assert_matches;
    use dpp::platform_value::string_encoding::Encoding;
    use dpp::prelude::Identifier;
    use rand::distributions::Standard;
    use dpp::consensus::basic::document::DocumentFieldMaxSizeExceededError;
    use dpp::consensus::ConsensusError;
    use dpp::consensus::basic::BasicError;
    use dpp::fee::fee_result::refunds::FeeRefunds;
    use dpp::fee::fee_result::FeeResult;
    use dpp::data_contract::accessors::v0::DataContractV0Setters;
    use dpp::data_contract::document_type::restricted_creation::CreationRestrictionMode;
    use dpp::document::Document;
    use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
    use dpp::util::hash::hash_double;
    use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
    use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice::TowardsIdentity;
    use drive::util::object_size_info::DataContractResolvedInfo;
    use drive::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfoAllowBorrowed;
    use drive::query::vote_poll_vote_state_query::ContestedDocumentVotePollDriveQueryResultType::DocumentsAndVoteTally;
    use drive::query::vote_poll_vote_state_query::ResolvedContestedDocumentVotePollDriveQuery;
    use drive::util::test_helpers::setup_contract;
    use crate::execution::validation::state_transition::state_transitions::tests::{add_contender_to_dpns_name_contest, create_dpns_identity_name_contest, create_dpns_name_contest_give_key_info, perform_votes_multi};
    use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
    use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult::PaidConsensusError;
    use crate::test::helpers::fast_forward_to_block::fast_forward_to_block;
    use dpp::consensus::state::state_error::StateError;
    use dpp::dashcore::Network;
    use dpp::dashcore::Network::Testnet;
    use dpp::data_contract::{DataContract, TokenConfiguration};
    use dpp::document::transfer::Transferable;
    use dpp::identity::SecurityLevel;
    use dpp::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
    use dpp::state_transition::batch_transition::document_create_transition::DocumentCreateTransitionV0;
    use dpp::state_transition::batch_transition::{DocumentCreateTransition, BatchTransitionV0};
    use dpp::state_transition::StateTransition;
    use dpp::data_contract::accessors::v1::DataContractV1Getters;
    use dpp::data_contract::document_type::accessors::DocumentTypeV1Getters;
    use dpp::tokens::gas_fees_paid_by::GasFeesPaidBy;
    use dpp::tokens::token_amount_on_contract_token::DocumentActionTokenCost;
    use dpp::tokens::token_payment_info::TokenPaymentInfo;
    use dpp::tokens::token_payment_info::v0::TokenPaymentInfoV0;
    use crate::config::PlatformConfig;
    use crate::execution::validation::state_transition::tests::{create_card_game_external_token_contract_with_owner_identity, create_card_game_internal_token_contract_with_owner_identity_transfer_tokens, create_token_contract_with_owner_identity};

    #[test]
    fn test_document_creation() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(433);

        let platform_state = platform.state.load();

        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

        let dashpay = platform.drive.cache.system_data_contracts.load_dashpay();
        let dashpay_contract = dashpay.clone();

        let profile = dashpay_contract
            .document_type_for_name("profile")
            .expect("expected a profile document type");

        assert!(profile.documents_mutable());

        let entropy = Bytes32::random_with_rng(&mut rng);

        let mut document = profile
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity.id(),
                entropy,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        document.set("avatarUrl", "http://test.com/bob.jpg".into());

        let documents_batch_create_transition =
            BatchTransition::new_document_creation_transition_from_document(
                document,
                profile,
                entropy.0,
                &key,
                2,
                0,
                None,
                &signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

        let documents_batch_create_serialized_transition = documents_batch_create_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_create_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
        );

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");
    }

    #[test]
    fn test_document_creation_should_fail_when_creator_id_is_provided() {
        let platform_version = PlatformVersion::latest();
        let (mut platform, contract) = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure()
            .with_crypto_card_game_transfer_only(Transferable::Always);

        let mut rng = StdRng::seed_from_u64(433);

        let platform_state = platform.state.load();

        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

        let card_document_type = contract
            .document_type_for_name("card")
            .expect("expected a card document type");

        let entropy = Bytes32::random_with_rng(&mut rng);

        let mut document = card_document_type
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity.id(),
                entropy,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        document.set("attack", 4.into());
        document.set("defense", 7.into());
        document.set("imageUrl", "https://example.com/card.png".into());

        let forged_creator_bytes = Bytes32::random_with_rng(&mut rng);
        let forged_creator = Identifier::from(forged_creator_bytes.0);
        document.set("$creatorId", forged_creator.into());

        let documents_batch_create_transition =
            BatchTransition::new_document_creation_transition_from_document(
                document,
                card_document_type,
                entropy.0,
                &key,
                2,
                0,
                None,
                &signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

        let documents_batch_create_serialized_transition = documents_batch_create_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_create_serialized_transition],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        assert_eq!(processing_result.invalid_paid_count(), 1);

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let result = processing_result.into_execution_results().remove(0);
        let PaidConsensusError(consensus_error, _) = result else {
            panic!("expected a paid consensus error");
        };

        assert!(
            consensus_error.to_string().contains("$creatorId"),
            "expected the error to mention $creatorId but got: {}",
            consensus_error
        );
    }

    #[test]
    fn test_document_creation_should_fail_if_reusing_entropy() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(433);

        let platform_state = platform.state.load();

        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

        let dashpay = platform.drive.cache.system_data_contracts.load_dashpay();
        let dashpay_contract = dashpay.clone();

        let profile = dashpay_contract
            .document_type_for_name("profile")
            .expect("expected a profile document type");

        assert!(profile.documents_mutable());

        let entropy = Bytes32::random_with_rng(&mut rng);

        let mut document = profile
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity.id(),
                entropy,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        document.set("avatarUrl", "http://test.com/bob.jpg".into());

        let documents_batch_create_transition =
            BatchTransition::new_document_creation_transition_from_document(
                document,
                profile,
                entropy.0,
                &key,
                2,
                0,
                None,
                &signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

        let documents_batch_create_serialized_transition = documents_batch_create_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_create_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
        );

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        // Now let's create a second document with the same entropy

        let mut document = profile
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity.id(),
                entropy,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        document.set("avatarUrl", "http://test.com/coy.jpg".into());

        let documents_batch_create_transition =
            BatchTransition::new_document_creation_transition_from_document(
                document,
                profile,
                entropy.0,
                &key,
                3,
                0,
                None,
                &signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

        let documents_batch_create_serialized_transition = documents_batch_create_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_create_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [PaidConsensusError(
                ConsensusError::StateError(StateError::DocumentAlreadyPresentError { .. }),
                _
            )]
        );

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");
    }

    #[test]
    fn test_document_creation_with_very_big_field() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(433);

        let platform_state = platform.state.load();

        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

        let dashpay_contract_no_max_length = setup_contract(
            &platform.drive,
            "tests/supporting_files/contract/dashpay/dashpay-contract-no-max-length.json",
            None,
            None,
            None::<fn(&mut DataContract)>,
            None,
            None,
        );

        let dashpay_contract = dashpay_contract_no_max_length.clone();

        let profile = dashpay_contract
            .document_type_for_name("profile")
            .expect("expected a profile document type");

        assert!(profile.documents_mutable());

        let entropy = Bytes32::random_with_rng(&mut rng);

        let mut document = profile
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity.id(),
                entropy,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        let max_field_size = platform_version.system_limits.max_field_value_size;
        let avatar_size = max_field_size + 1000;

        document.set(
            "avatar",
            Value::Bytes(
                rng.sample_iter(Standard)
                    .take(avatar_size as usize)
                    .collect(),
            ),
        );

        let documents_batch_create_transition =
            BatchTransition::new_document_creation_transition_from_document(
                document,
                profile,
                entropy.0,
                &key,
                2,
                0,
                None,
                &signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

        let documents_batch_create_serialized_transition = documents_batch_create_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_create_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");
        assert_eq!(
            processing_result.execution_results().first().unwrap(),
            &PaidConsensusError(
                ConsensusError::BasicError(BasicError::DocumentFieldMaxSizeExceededError(
                    DocumentFieldMaxSizeExceededError::new(
                        "avatar".to_string(),
                        avatar_size as u64,
                        max_field_size as u64
                    )
                )),
                FeeResult {
                    storage_fee: 11556000,
                    processing_fee: 526140,
                    fee_refunds: FeeRefunds::default(),
                    removed_bytes_from_system: 0
                }
            )
        );

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");
    }

    #[test]
    fn test_document_creation_on_contested_unique_index() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(433);

        let platform_state = platform.state.load();

        let (identity_1, signer_1, key_1) =
            setup_identity(&mut platform, 958, dash_to_credits!(0.5));

        let (identity_2, signer_2, key_2) =
            setup_identity(&mut platform, 93, dash_to_credits!(0.5));

        let dpns = platform.drive.cache.system_data_contracts.load_dpns();
        let dpns_contract = dpns.clone();

        let preorder = dpns_contract
            .document_type_for_name("preorder")
            .expect("expected a profile document type");

        assert!(!preorder.documents_mutable());
        assert!(preorder.documents_can_be_deleted());
        assert!(!preorder.documents_transferable().is_transferable());

        let domain = dpns_contract
            .document_type_for_name("domain")
            .expect("expected a profile document type");

        assert!(!domain.documents_mutable());
        // Deletion is disabled with data trigger
        assert!(domain.documents_can_be_deleted());
        assert!(domain.documents_transferable().is_transferable());

        let entropy = Bytes32::random_with_rng(&mut rng);

        let mut preorder_document_1 = preorder
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity_1.id(),
                entropy,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        let mut preorder_document_2 = preorder
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity_2.id(),
                entropy,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        let mut document_1 = domain
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity_1.id(),
                entropy,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        let mut document_2 = domain
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity_2.id(),
                entropy,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        document_1.set("parentDomainName", "dash".into());
        document_1.set("normalizedParentDomainName", "dash".into());
        document_1.set("label", "quantum".into());
        document_1.set("normalizedLabel", "quantum".into());
        document_1.set("records.identity", document_1.owner_id().into());
        document_1.set("subdomainRules.allowSubdomains", false.into());

        document_2.set("parentDomainName", "dash".into());
        document_2.set("normalizedParentDomainName", "dash".into());
        document_2.set("label", "quantum".into());
        document_2.set("normalizedLabel", "quantum".into());
        document_2.set("records.identity", document_2.owner_id().into());
        document_2.set("subdomainRules.allowSubdomains", false.into());

        let salt_1: [u8; 32] = rng.gen();
        let salt_2: [u8; 32] = rng.gen();

        let mut salted_domain_buffer_1: Vec<u8> = vec![];
        salted_domain_buffer_1.extend(salt_1);
        salted_domain_buffer_1.extend("quantum.dash".as_bytes());

        let salted_domain_hash_1 = hash_double(salted_domain_buffer_1);

        let mut salted_domain_buffer_2: Vec<u8> = vec![];
        salted_domain_buffer_2.extend(salt_2);
        salted_domain_buffer_2.extend("quantum.dash".as_bytes());

        let salted_domain_hash_2 = hash_double(salted_domain_buffer_2);

        preorder_document_1.set("saltedDomainHash", salted_domain_hash_1.into());
        preorder_document_2.set("saltedDomainHash", salted_domain_hash_2.into());

        document_1.set("preorderSalt", salt_1.into());
        document_2.set("preorderSalt", salt_2.into());

        let documents_batch_create_preorder_transition_1 =
            BatchTransition::new_document_creation_transition_from_document(
                preorder_document_1,
                preorder,
                entropy.0,
                &key_1,
                2,
                0,
                None,
                &signer_1,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

        let documents_batch_create_serialized_preorder_transition_1 =
            documents_batch_create_preorder_transition_1
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

        let documents_batch_create_preorder_transition_2 =
            BatchTransition::new_document_creation_transition_from_document(
                preorder_document_2,
                preorder,
                entropy.0,
                &key_2,
                2,
                0,
                None,
                &signer_2,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

        let documents_batch_create_serialized_preorder_transition_2 =
            documents_batch_create_preorder_transition_2
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

        let documents_batch_create_transition_1 =
            BatchTransition::new_document_creation_transition_from_document(
                document_1,
                domain,
                entropy.0,
                &key_1,
                3,
                0,
                None,
                &signer_1,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

        let documents_batch_create_serialized_transition_1 = documents_batch_create_transition_1
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let documents_batch_create_transition_2 =
            BatchTransition::new_document_creation_transition_from_document(
                document_2,
                domain,
                entropy.0,
                &key_2,
                3,
                0,
                None,
                &signer_2,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

        let documents_batch_create_serialized_transition_2 = documents_batch_create_transition_2
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![
                    documents_batch_create_serialized_preorder_transition_1.clone(),
                    documents_batch_create_serialized_preorder_transition_2.clone(),
                ],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        assert_eq!(processing_result.valid_count(), 2);

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![
                    documents_batch_create_serialized_transition_1.clone(),
                    documents_batch_create_serialized_transition_2.clone(),
                ],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        assert_eq!(processing_result.valid_count(), 2);

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
                            document_type_name: domain.name().clone(),
                            index_name: index_name.clone(),
                            index_values: vec![dash_encoded.clone(), quantum_encoded.clone()],
                            result_type: ResultType::DocumentsAndVoteTally as i32,
                            allow_include_locked_and_abstaining_vote_tally: false,
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

        assert_eq!(abstain_vote_tally, None);

        assert_eq!(lock_vote_tally, None);

        assert_eq!(finished_vote_info, None);

        assert_eq!(contenders.len(), 2);

        let first_contender = contenders.first().unwrap();

        let second_contender = contenders.last().unwrap();

        let first_contender_document = Document::from_bytes(
            first_contender
                .document
                .as_ref()
                .expect("expected a document")
                .as_slice(),
            domain,
            platform_version,
        )
        .expect("expected to get document");

        let second_contender_document = Document::from_bytes(
            second_contender
                .document
                .as_ref()
                .expect("expected a document")
                .as_slice(),
            domain,
            platform_version,
        )
        .expect("expected to get document");

        assert_ne!(first_contender_document, second_contender_document);

        assert_eq!(first_contender.identifier, identity_1.id().to_vec());

        assert_eq!(second_contender.identifier, identity_2.id().to_vec());

        assert_eq!(first_contender.vote_count, Some(0));

        assert_eq!(second_contender.vote_count, Some(0));

        let GetContestedResourceVoteStateResponse { version } = platform
            .query_contested_resource_vote_state(
                GetContestedResourceVoteStateRequest {
                    version: Some(get_contested_resource_vote_state_request::Version::V0(
                        GetContestedResourceVoteStateRequestV0 {
                            contract_id: dpns_contract.id().to_vec(),
                            document_type_name: domain.name().clone(),
                            index_name: "parentNameAndLabel".to_string(),
                            index_values: vec![dash_encoded, quantum_encoded],
                            result_type: ResultType::DocumentsAndVoteTally as i32,
                            allow_include_locked_and_abstaining_vote_tally: true,
                            start_at_identifier_info: None,
                            count: None,
                            prove: true,
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
        ) = version.expect("expected a version");

        let Some(get_contested_resource_vote_state_response_v0::Result::Proof(proof)) = result
        else {
            panic!("expected contenders")
        };

        let resolved_contested_document_vote_poll_drive_query =
            ResolvedContestedDocumentVotePollDriveQuery {
                vote_poll: ContestedDocumentResourceVotePollWithContractInfoAllowBorrowed {
                    contract: DataContractResolvedInfo::BorrowedDataContract(&dpns_contract),
                    document_type_name: domain.name().clone(),
                    index_name: index_name.clone(),
                    index_values: vec![
                        Value::Text("dash".to_string()),
                        Value::Text("quantum".to_string()),
                    ],
                },
                result_type: DocumentsAndVoteTally,
                offset: None,
                limit: None,
                start_at: None,
                allow_include_locked_and_abstaining_vote_tally: true,
            };

        let (_root_hash, result) = resolved_contested_document_vote_poll_drive_query
            .verify_vote_poll_vote_state_proof(proof.grovedb_proof.as_ref(), platform_version)
            .expect("expected to verify proof");

        let contenders = result.contenders;
        assert_eq!(contenders.len(), 2);

        let first_contender = contenders.first().unwrap();

        let second_contender = contenders.last().unwrap();

        let first_contender_document = Document::from_bytes(
            first_contender
                .serialized_document()
                .as_ref()
                .expect("expected a document")
                .as_slice(),
            domain,
            platform_version,
        )
        .expect("expected to get document");

        let second_contender_document = Document::from_bytes(
            second_contender
                .serialized_document()
                .as_ref()
                .expect("expected a document")
                .as_slice(),
            domain,
            platform_version,
        )
        .expect("expected to get document");

        assert_ne!(first_contender_document, second_contender_document);

        assert_eq!(first_contender.identity_id(), identity_1.id());

        assert_eq!(second_contender.identity_id(), identity_2.id());

        assert_eq!(first_contender.vote_tally(), Some(0));

        assert_eq!(second_contender.vote_tally(), Some(0));
    }

    #[test]
    fn test_document_creation_on_contested_unique_index_should_fail_if_not_paying_for_it() {
        let platform_version = PlatformVersion::latest();
        let platform_config = PlatformConfig {
            network: Network::Dash,
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .with_config(platform_config)
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(433);

        let platform_state = platform.state.load();

        let (identity_1, signer_1, key_1) =
            setup_identity(&mut platform, 958, dash_to_credits!(0.5));

        let dpns = platform.drive.cache.system_data_contracts.load_dpns();
        let dpns_contract = dpns.clone();

        let preorder = dpns_contract
            .document_type_for_name("preorder")
            .expect("expected a profile document type");

        assert!(!preorder.documents_mutable());
        assert!(preorder.documents_can_be_deleted());
        assert!(!preorder.documents_transferable().is_transferable());

        let domain = dpns_contract
            .document_type_for_name("domain")
            .expect("expected a profile document type");

        assert!(!domain.documents_mutable());
        // Deletion is disabled with data trigger
        assert!(domain.documents_can_be_deleted());
        assert!(domain.documents_transferable().is_transferable());

        let entropy = Bytes32::random_with_rng(&mut rng);

        let mut preorder_document_1 = preorder
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity_1.id(),
                entropy,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        let mut document_1 = domain
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity_1.id(),
                entropy,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        document_1.set("parentDomainName", "dash".into());
        document_1.set("normalizedParentDomainName", "dash".into());
        document_1.set("label", "quantum".into());
        document_1.set("normalizedLabel", "quantum".into());
        document_1.set("records.identity", document_1.owner_id().into());
        document_1.set("subdomainRules.allowSubdomains", false.into());

        let salt_1: [u8; 32] = rng.gen();

        let mut salted_domain_buffer_1: Vec<u8> = vec![];
        salted_domain_buffer_1.extend(salt_1);
        salted_domain_buffer_1.extend("quantum.dash".as_bytes());

        let salted_domain_hash_1 = hash_double(salted_domain_buffer_1);

        preorder_document_1.set("saltedDomainHash", salted_domain_hash_1.into());

        document_1.set("preorderSalt", salt_1.into());

        let documents_batch_create_preorder_transition_1 =
            BatchTransition::new_document_creation_transition_from_document(
                preorder_document_1,
                preorder,
                entropy.0,
                &key_1,
                2,
                0,
                None,
                &signer_1,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

        let documents_batch_create_serialized_preorder_transition_1 =
            documents_batch_create_preorder_transition_1
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

        let owner_id = document_1.owner_id();
        let create_transition: DocumentCreateTransition = DocumentCreateTransitionV0 {
            base: DocumentBaseTransition::from_document(
                &document_1,
                domain,
                None,
                3,
                platform_version,
                None,
            )
            .expect("expected a base transition"),
            entropy: entropy.0,
            data: document_1.clone().properties_consumed(),
            // Sending 0 balance that should not be valid
            prefunded_voting_balance: None,
        }
        .into();
        let documents_batch_inner_create_transition_1: BatchTransition = BatchTransitionV0 {
            owner_id,
            transitions: vec![create_transition.into()],
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: Default::default(),
        }
        .into();
        let mut documents_batch_create_transition_1: StateTransition =
            documents_batch_inner_create_transition_1.into();
        documents_batch_create_transition_1
            .sign_external(&key_1, &signer_1, Some(|_, _| Ok(SecurityLevel::HIGH)))
            .expect("expected to sign");

        let documents_batch_create_serialized_transition_1 = documents_batch_create_transition_1
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_create_serialized_preorder_transition_1.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        assert_eq!(processing_result.valid_count(), 1);

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_create_serialized_transition_1.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [PaidConsensusError(
                ConsensusError::StateError(StateError::DocumentContestNotPaidForError(_)),
                _
            )]
        );

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
                            document_type_name: domain.name().clone(),
                            index_name: index_name.clone(),
                            index_values: vec![dash_encoded.clone(), quantum_encoded.clone()],
                            result_type: ResultType::DocumentsAndVoteTally as i32,
                            allow_include_locked_and_abstaining_vote_tally: false,
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

        assert_eq!(abstain_vote_tally, None);

        assert_eq!(lock_vote_tally, None);

        assert_eq!(finished_vote_info, None);

        assert_eq!(contenders.len(), 0);

        let drive_query =
            DriveDocumentQuery::new_primary_key_single_item_query(&dpns, domain, document_1.id());

        let documents = platform
            .drive
            .query_documents(drive_query, None, false, None, None)
            .expect("expected to get back documents")
            .documents_owned();

        assert!(documents.is_empty());
    }

    #[test]
    fn test_document_creation_on_contested_unique_index_should_not_fail_if_not_paying_for_it_on_testnet_before_epoch_2080(
    ) {
        let platform_version = PlatformVersion::latest();
        let platform_config = PlatformConfig {
            network: Testnet,
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .with_config(platform_config)
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(433);

        let platform_state = platform.state.load();

        let (identity_1, signer_1, key_1) =
            setup_identity(&mut platform, 958, dash_to_credits!(0.5));

        let dpns = platform.drive.cache.system_data_contracts.load_dpns();
        let dpns_contract = dpns.clone();

        let preorder = dpns_contract
            .document_type_for_name("preorder")
            .expect("expected a profile document type");

        assert!(!preorder.documents_mutable());
        assert!(preorder.documents_can_be_deleted());
        assert!(!preorder.documents_transferable().is_transferable());

        let domain = dpns_contract
            .document_type_for_name("domain")
            .expect("expected a profile document type");

        assert!(!domain.documents_mutable());
        // Deletion is disabled with data trigger
        assert!(domain.documents_can_be_deleted());
        assert!(domain.documents_transferable().is_transferable());

        let entropy = Bytes32::random_with_rng(&mut rng);

        let mut preorder_document_1 = preorder
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity_1.id(),
                entropy,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        let mut document_1 = domain
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity_1.id(),
                entropy,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        document_1.set("parentDomainName", "dash".into());
        document_1.set("normalizedParentDomainName", "dash".into());
        document_1.set("label", "quantum".into());
        document_1.set("normalizedLabel", "quantum".into());
        document_1.set("records.identity", document_1.owner_id().into());
        document_1.set("subdomainRules.allowSubdomains", false.into());

        let salt_1: [u8; 32] = rng.gen();

        let mut salted_domain_buffer_1: Vec<u8> = vec![];
        salted_domain_buffer_1.extend(salt_1);
        salted_domain_buffer_1.extend("quantum.dash".as_bytes());

        let salted_domain_hash_1 = hash_double(salted_domain_buffer_1);

        preorder_document_1.set("saltedDomainHash", salted_domain_hash_1.into());

        document_1.set("preorderSalt", salt_1.into());

        let documents_batch_create_preorder_transition_1 =
            BatchTransition::new_document_creation_transition_from_document(
                preorder_document_1,
                preorder,
                entropy.0,
                &key_1,
                2,
                0,
                None,
                &signer_1,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

        let documents_batch_create_serialized_preorder_transition_1 =
            documents_batch_create_preorder_transition_1
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

        let owner_id = document_1.owner_id();
        let create_transition: DocumentCreateTransition = DocumentCreateTransitionV0 {
            base: DocumentBaseTransition::from_document(
                &document_1,
                domain,
                None,
                3,
                platform_version,
                None,
            )
            .expect("expected a base transition"),
            entropy: entropy.0,
            data: document_1.clone().properties_consumed(),
            prefunded_voting_balance: None,
        }
        .into();
        let documents_batch_inner_create_transition_1: BatchTransition = BatchTransitionV0 {
            owner_id,
            transitions: vec![create_transition.into()],
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: Default::default(),
        }
        .into();
        let mut documents_batch_create_transition_1: StateTransition =
            documents_batch_inner_create_transition_1.into();
        documents_batch_create_transition_1
            .sign_external(&key_1, &signer_1, Some(|_, _| Ok(SecurityLevel::HIGH)))
            .expect("expected to sign");

        let documents_batch_create_serialized_transition_1 = documents_batch_create_transition_1
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_create_serialized_preorder_transition_1.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        assert_eq!(processing_result.valid_count(), 1);

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_create_serialized_transition_1.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution(..)]
        );

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
                            document_type_name: domain.name().clone(),
                            index_name: index_name.clone(),
                            index_values: vec![dash_encoded.clone(), quantum_encoded.clone()],
                            result_type: ResultType::DocumentsAndVoteTally as i32,
                            allow_include_locked_and_abstaining_vote_tally: false,
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

        assert_eq!(abstain_vote_tally, None);

        assert_eq!(lock_vote_tally, None);

        assert_eq!(finished_vote_info, None);

        assert_eq!(contenders.len(), 0); // no contenders should have been created the document should just exist

        let drive_query =
            DriveDocumentQuery::new_primary_key_single_item_query(&dpns, domain, document_1.id());

        let documents = platform
            .drive
            .query_documents(drive_query, None, false, None, None)
            .expect("expected to get back documents")
            .documents_owned();

        assert!(!documents.is_empty());
    }

    #[test]
    fn test_document_creation_on_contested_unique_index_should_fail_if_reusing_entropy() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(433);

        let platform_state = platform.state.load();

        let (identity_1, signer_1, key_1) =
            setup_identity(&mut platform, 958, dash_to_credits!(0.5));

        let (identity_2, signer_2, key_2) =
            setup_identity(&mut platform, 93, dash_to_credits!(0.5));

        let dpns = platform.drive.cache.system_data_contracts.load_dpns();
        let dpns_contract = dpns.clone();

        let preorder = dpns_contract
            .document_type_for_name("preorder")
            .expect("expected a profile document type");

        assert!(!preorder.documents_mutable());
        assert!(preorder.documents_can_be_deleted());
        assert!(!preorder.documents_transferable().is_transferable());

        let domain = dpns_contract
            .document_type_for_name("domain")
            .expect("expected a profile document type");

        assert!(!domain.documents_mutable());
        // Deletion is disabled with data trigger
        assert!(domain.documents_can_be_deleted());
        assert!(domain.documents_transferable().is_transferable());

        let entropy = Bytes32::random_with_rng(&mut rng);

        let mut preorder_document_1 = preorder
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity_1.id(),
                entropy,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        let mut preorder_document_2 = preorder
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity_2.id(),
                entropy,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        let new_entropy = Bytes32::random_with_rng(&mut rng);

        let mut preorder_document_3_on_identity_1 = preorder
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity_1.id(),
                new_entropy, //change entropy here
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        let mut document_1 = domain
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity_1.id(),
                entropy,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        let mut document_2 = domain
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity_2.id(),
                entropy,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        let mut document_3_on_identity_1 = domain
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity_1.id(),
                entropy, //same entropy
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        document_1.set("parentDomainName", "dash".into());
        document_1.set("normalizedParentDomainName", "dash".into());
        document_1.set("label", "quantum".into());
        document_1.set("normalizedLabel", "quantum".into());
        document_1.set("records.identity", document_1.owner_id().into());
        document_1.set("subdomainRules.allowSubdomains", false.into());

        document_2.set("parentDomainName", "dash".into());
        document_2.set("normalizedParentDomainName", "dash".into());
        document_2.set("label", "quantum".into());
        document_2.set("normalizedLabel", "quantum".into());
        document_2.set("records.identity", document_2.owner_id().into());
        document_2.set("subdomainRules.allowSubdomains", false.into());

        document_3_on_identity_1.set("parentDomainName", "dash".into());
        document_3_on_identity_1.set("normalizedParentDomainName", "dash".into());
        document_3_on_identity_1.set("label", "cry".into());
        document_3_on_identity_1.set("normalizedLabel", "cry".into());
        document_3_on_identity_1.set(
            "records.identity",
            document_3_on_identity_1.owner_id().into(),
        );
        document_3_on_identity_1.set("subdomainRules.allowSubdomains", false.into());

        let salt_1: [u8; 32] = rng.gen();
        let salt_2: [u8; 32] = rng.gen();
        let salt_3: [u8; 32] = rng.gen();

        let mut salted_domain_buffer_1: Vec<u8> = vec![];
        salted_domain_buffer_1.extend(salt_1);
        salted_domain_buffer_1.extend("quantum.dash".as_bytes());

        let salted_domain_hash_1 = hash_double(salted_domain_buffer_1);

        let mut salted_domain_buffer_2: Vec<u8> = vec![];
        salted_domain_buffer_2.extend(salt_2);
        salted_domain_buffer_2.extend("quantum.dash".as_bytes());

        let salted_domain_hash_2 = hash_double(salted_domain_buffer_2);

        let mut salted_domain_buffer_3: Vec<u8> = vec![];
        salted_domain_buffer_3.extend(salt_3);
        salted_domain_buffer_3.extend("cry.dash".as_bytes());

        let salted_domain_hash_3 = hash_double(salted_domain_buffer_3);

        preorder_document_1.set("saltedDomainHash", salted_domain_hash_1.into());
        preorder_document_2.set("saltedDomainHash", salted_domain_hash_2.into());
        preorder_document_3_on_identity_1.set("saltedDomainHash", salted_domain_hash_3.into());

        document_1.set("preorderSalt", salt_1.into());
        document_2.set("preorderSalt", salt_2.into());
        document_3_on_identity_1.set("preorderSalt", salt_3.into());

        let documents_batch_create_preorder_transition_1 =
            BatchTransition::new_document_creation_transition_from_document(
                preorder_document_1,
                preorder,
                entropy.0,
                &key_1,
                2,
                0,
                None,
                &signer_1,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

        let documents_batch_create_serialized_preorder_transition_1 =
            documents_batch_create_preorder_transition_1
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

        let documents_batch_create_preorder_transition_2 =
            BatchTransition::new_document_creation_transition_from_document(
                preorder_document_2,
                preorder,
                entropy.0,
                &key_2,
                2,
                0,
                None,
                &signer_2,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

        let documents_batch_create_serialized_preorder_transition_2 =
            documents_batch_create_preorder_transition_2
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

        let documents_batch_create_preorder_transition_3 =
            BatchTransition::new_document_creation_transition_from_document(
                preorder_document_3_on_identity_1,
                preorder,
                new_entropy.0,
                &key_1,
                3,
                0,
                None,
                &signer_1,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

        let documents_batch_create_serialized_preorder_transition_3 =
            documents_batch_create_preorder_transition_3
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

        let documents_batch_create_transition_1 =
            BatchTransition::new_document_creation_transition_from_document(
                document_1,
                domain,
                entropy.0,
                &key_1,
                4,
                0,
                None,
                &signer_1,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

        let documents_batch_create_serialized_transition_1 = documents_batch_create_transition_1
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let documents_batch_create_transition_2 =
            BatchTransition::new_document_creation_transition_from_document(
                document_2,
                domain,
                entropy.0,
                &key_2,
                3,
                0,
                None,
                &signer_2,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

        let documents_batch_create_serialized_transition_2 = documents_batch_create_transition_2
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let documents_batch_create_transition_3 =
            BatchTransition::new_document_creation_transition_from_document(
                document_3_on_identity_1,
                domain,
                entropy.0,
                &key_1,
                5,
                0,
                None,
                &signer_1,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

        let documents_batch_create_serialized_transition_3 = documents_batch_create_transition_3
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![
                    documents_batch_create_serialized_preorder_transition_1.clone(),
                    documents_batch_create_serialized_preorder_transition_2.clone(),
                    documents_batch_create_serialized_preorder_transition_3.clone(),
                ],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        assert_eq!(processing_result.valid_count(), 3);

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![
                    documents_batch_create_serialized_transition_1.clone(),
                    documents_batch_create_serialized_transition_2.clone(),
                ],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        assert_eq!(processing_result.valid_count(), 2);

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_create_serialized_transition_3.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [PaidConsensusError(
                ConsensusError::StateError(
                    StateError::DocumentContestDocumentWithSameIdAlreadyPresentError { .. }
                ),
                _
            )]
        );

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
                            document_type_name: domain.name().clone(),
                            index_name: index_name.clone(),
                            index_values: vec![dash_encoded.clone(), quantum_encoded.clone()],
                            result_type: ResultType::DocumentsAndVoteTally as i32,
                            allow_include_locked_and_abstaining_vote_tally: false,
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

        assert_eq!(abstain_vote_tally, None);

        assert_eq!(lock_vote_tally, None);

        assert_eq!(finished_vote_info, None);

        assert_eq!(contenders.len(), 2);

        let first_contender = contenders.first().unwrap();

        let second_contender = contenders.last().unwrap();

        let first_contender_document = Document::from_bytes(
            first_contender
                .document
                .as_ref()
                .expect("expected a document")
                .as_slice(),
            domain,
            platform_version,
        )
        .expect("expected to get document");

        let second_contender_document = Document::from_bytes(
            second_contender
                .document
                .as_ref()
                .expect("expected a document")
                .as_slice(),
            domain,
            platform_version,
        )
        .expect("expected to get document");

        assert_ne!(first_contender_document, second_contender_document);

        assert_eq!(first_contender.identifier, identity_1.id().to_vec());

        assert_eq!(second_contender.identifier, identity_2.id().to_vec());

        assert_eq!(first_contender.vote_count, Some(0));

        assert_eq!(second_contender.vote_count, Some(0));

        let GetContestedResourceVoteStateResponse { version } = platform
            .query_contested_resource_vote_state(
                GetContestedResourceVoteStateRequest {
                    version: Some(get_contested_resource_vote_state_request::Version::V0(
                        GetContestedResourceVoteStateRequestV0 {
                            contract_id: dpns_contract.id().to_vec(),
                            document_type_name: domain.name().clone(),
                            index_name: "parentNameAndLabel".to_string(),
                            index_values: vec![dash_encoded, quantum_encoded],
                            result_type: ResultType::DocumentsAndVoteTally as i32,
                            allow_include_locked_and_abstaining_vote_tally: true,
                            start_at_identifier_info: None,
                            count: None,
                            prove: true,
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
        ) = version.expect("expected a version");

        let Some(get_contested_resource_vote_state_response_v0::Result::Proof(proof)) = result
        else {
            panic!("expected contenders")
        };

        let resolved_contested_document_vote_poll_drive_query =
            ResolvedContestedDocumentVotePollDriveQuery {
                vote_poll: ContestedDocumentResourceVotePollWithContractInfoAllowBorrowed {
                    contract: DataContractResolvedInfo::BorrowedDataContract(&dpns_contract),
                    document_type_name: domain.name().clone(),
                    index_name: index_name.clone(),
                    index_values: vec![
                        Value::Text("dash".to_string()),
                        Value::Text("quantum".to_string()),
                    ],
                },
                result_type: DocumentsAndVoteTally,
                offset: None,
                limit: None,
                start_at: None,
                allow_include_locked_and_abstaining_vote_tally: true,
            };

        let (_root_hash, result) = resolved_contested_document_vote_poll_drive_query
            .verify_vote_poll_vote_state_proof(proof.grovedb_proof.as_ref(), platform_version)
            .expect("expected to verify proof");

        let contenders = result.contenders;
        assert_eq!(contenders.len(), 2);

        let first_contender = contenders.first().unwrap();

        let second_contender = contenders.last().unwrap();

        let first_contender_document = Document::from_bytes(
            first_contender
                .serialized_document()
                .as_ref()
                .expect("expected a document")
                .as_slice(),
            domain,
            platform_version,
        )
        .expect("expected to get document");

        let second_contender_document = Document::from_bytes(
            second_contender
                .serialized_document()
                .as_ref()
                .expect("expected a document")
                .as_slice(),
            domain,
            platform_version,
        )
        .expect("expected to get document");

        assert_ne!(first_contender_document, second_contender_document);

        assert_eq!(first_contender.identity_id(), identity_1.id());

        assert_eq!(second_contender.identity_id(), identity_2.id());

        assert_eq!(first_contender.vote_tally(), Some(0));

        assert_eq!(second_contender.vote_tally(), Some(0));
    }

    #[test]
    fn test_that_a_contested_document_can_not_be_added_to_after_a_week() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let platform_state = platform.state.load();

        let (contender_1, contender_2, dpns_contract) = create_dpns_identity_name_contest(
            &mut platform,
            &platform_state,
            7,
            "quantum",
            platform_version,
        );

        perform_votes_multi(
            &mut platform,
            dpns_contract.as_ref(),
            vec![
                (TowardsIdentity(contender_1.id()), 50),
                (TowardsIdentity(contender_2.id()), 5),
                (ResourceVoteChoice::Abstain, 10),
                (ResourceVoteChoice::Lock, 3),
            ],
            "quantum",
            10,
            None,
            platform_version,
        );

        let max_join_time = platform_version
            .dpp
            .validation
            .voting
            .allow_other_contenders_time_testing_ms;

        fast_forward_to_block(&platform, max_join_time / 2, 900, 42, 0, false);

        let platform_state = platform.state.load();

        let _contender_3 = add_contender_to_dpns_name_contest(
            &mut platform,
            &platform_state,
            4,
            "quantum",
            None, // this should succeed, as we are under a week
            platform_version,
        );

        let time_now = platform_version
            .dpp
            .validation
            .voting
            .allow_other_contenders_time_testing_ms
            + 100;

        fast_forward_to_block(&platform, time_now, 900, 42, 0, false); //more than a week, less than 2 weeks

        let platform_state = platform.state.load();

        // We expect this to fail

        let time_started = 0;

        let extra_time_used = 3000; // add_contender_to_dpns_name_contest uses this extra time

        let expected_error_message = format!(
            "Document Contest for vote_poll ContestedDocumentResourceVotePoll {{ contract_id: GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec, document_type_name: domain, index_name: parentNameAndLabel, index_values: [string dash, string quantum] }} is not joinable V0(ContestedDocumentVotePollStoredInfoV0 {{ finalized_events: [], vote_poll_status: Started(BlockInfo {{ time_ms: {}, height: 0, core_height: 0, epoch: 0 }}), locked_count: 0 }}), it started {} and it is now {}, and you can only join for {}",
            time_started + extra_time_used,
            time_started + extra_time_used,
            time_now + extra_time_used,
            max_join_time
        );

        let _contender_4 = add_contender_to_dpns_name_contest(
            &mut platform,
            &platform_state,
            9,
            "quantum",
            Some(expected_error_message.as_str()), // this should fail, as we are over a week
            platform_version,
        );
    }

    #[test]
    fn test_that_a_contest_can_not_be_joined_twice_by_the_same_identity() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let platform_state = platform.state.load();

        let (
            (
                _contender_1,
                contender_1_signer,
                contender_1_key,
                _preorder_document_1,
                (mut document_1, _entropy),
            ),
            (_contender_2, _, _, _, _),
            dpns_contract,
        ) = create_dpns_name_contest_give_key_info(
            &mut platform,
            &platform_state,
            7,
            "quantum",
            platform_version,
        );

        let domain = dpns_contract
            .document_type_for_name("domain")
            .expect("expected a profile document type");

        let mut rng = StdRng::seed_from_u64(89);

        let different_entropy = Bytes32::random_with_rng(&mut rng);

        document_1.set_id(Document::generate_document_id_v0(
            dpns_contract.id_ref(),
            document_1.owner_id_ref(),
            domain.name(),
            different_entropy.as_slice(),
        ));

        let documents_batch_create_transition_1 =
            BatchTransition::new_document_creation_transition_from_document(
                document_1,
                domain,
                different_entropy.0,
                &contender_1_key,
                4,
                0,
                None,
                &contender_1_signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

        let documents_batch_create_serialized_transition_1 = documents_batch_create_transition_1
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_create_serialized_transition_1.clone()],
                &platform_state,
                &BlockInfo::default_with_time(
                    &platform_state
                        .last_committed_block_time_ms()
                        .unwrap_or_default()
                        + 3000,
                ),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let result = processing_result.into_execution_results().remove(0);

        let PaidConsensusError(consensus_error, _) = result else {
            panic!("expected a paid consensus error");
        };
        assert_eq!(consensus_error.to_string(), "An Identity with the id BjNejy4r9QAvLHpQ9Yq6yRMgNymeGZ46d48fJxJbMrfW is already a contestant for the vote_poll ContestedDocumentResourceVotePoll { contract_id: GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec, document_type_name: domain, index_name: parentNameAndLabel, index_values: [string dash, string quantum] }");
    }

    #[test]
    fn test_that_a_contested_document_can_not_be_added_if_we_are_locked() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let platform_state = platform.state.load();

        let (contender_1, contender_2, dpns_contract) = create_dpns_identity_name_contest(
            &mut platform,
            &platform_state,
            7,
            "quantum",
            platform_version,
        );

        perform_votes_multi(
            &mut platform,
            dpns_contract.as_ref(),
            vec![
                (TowardsIdentity(contender_1.id()), 3),
                (TowardsIdentity(contender_2.id()), 5),
                (ResourceVoteChoice::Abstain, 8),
                (ResourceVoteChoice::Lock, 10),
            ],
            "quantum",
            10,
            None,
            platform_version,
        );

        fast_forward_to_block(
            &platform,
            platform_version
                .dpp
                .validation
                .voting
                .allow_other_contenders_time_testing_ms
                / 2,
            900,
            42,
            0,
            false,
        ); // a time when others can join

        let platform_state = platform.state.load();

        let _contender_3 = add_contender_to_dpns_name_contest(
            &mut platform,
            &platform_state,
            4,
            "quantum",
            None, // this should succeed, as we are under the `platform_version.dpp.validation.voting.allow_other_contenders_time_testing_ms`
            platform_version,
        );

        let time_after_distribution_limit = platform_version
            .dpp
            .voting_versions
            .default_vote_poll_time_duration_test_network_ms
            + 10_000; // add 10s (3 seconds is used by create_dpns_identity_name_contest)

        fast_forward_to_block(&platform, time_after_distribution_limit, 900, 42, 0, false); // after distribution

        let platform_state = platform.state.load();

        let transaction = platform.drive.grove.start_transaction();

        platform
            .check_for_ended_vote_polls(
                &platform_state,
                &platform_state,
                &BlockInfo {
                    time_ms: time_after_distribution_limit,
                    height: 900,
                    core_height: 42,
                    epoch: Default::default(),
                },
                Some(&transaction),
                platform_version,
            )
            .expect("expected to check for ended vote polls");

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let platform_state = platform.state.load();

        // We expect this to fail

        let expected_error_message = format!(
            "Document Contest for vote_poll ContestedDocumentResourceVotePoll {{ contract_id: GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec, document_type_name: domain, index_name: parentNameAndLabel, index_values: [string dash, string quantum] }} is currently already locked V0(ContestedDocumentVotePollStoredInfoV0 {{ finalized_events: [ContestedDocumentVotePollStoredInfoVoteEventV0 {{ resource_vote_choices: [FinalizedResourceVoteChoicesWithVoterInfo {{ resource_vote_choice: TowardsIdentity(BjNejy4r9QAvLHpQ9Yq6yRMgNymeGZ46d48fJxJbMrfW), voters: [2oGomAQc47V9h3mkpyHUPbF74gT2AmoYKg1oSb94Rbwm:1, 4iroeiNBeBYZetCt21kW7FGyczE8WqoqzZ48YAHwyV7R:1, Cdf8V4KGHHd395x5xPJPPrzTKwmp5MqbuszSE2iMzzeP:1] }}, FinalizedResourceVoteChoicesWithVoterInfo {{ resource_vote_choice: TowardsIdentity(FiLk5pGtspYtF65PKsQq3YFr1DEiXPHTZeKjusT6DuqN), voters: [] }}, FinalizedResourceVoteChoicesWithVoterInfo {{ resource_vote_choice: TowardsIdentity(Fv8S6kTbNrRqKC7PR7XcRUoPR59bxNhhggg5mRaNN6ow), voters: [4MK8GWEWX1PturUqjZJefdE4WGrUqz1UQZnbK17ENkeA:1, 5gRudU7b4n8LYkNvhZomv6FtMrP7gvaTvRrHKfaTS22K:1, AfzQBrdwzDuTVdXrMWqQyVvXRWqPMDVjA76hViuGLh6W:1, E75wdFZB22P1uW1wJBJGPgXZuZKLotK7YmbH5wUk5msH:1, G3ZfS2v39x6FuLGnnJ1RNQyy4zn4Wb64KiGAjqj39wUu:1] }}, FinalizedResourceVoteChoicesWithVoterInfo {{ resource_vote_choice: Abstain, voters: [5Ur8tDxJnatfUd9gcVFDde7ptHydujZzJLNTxa6aMYYy:1, 93Gsg14oT9K4FLYmC7N26uS4g5b7JcM1GwGEDeJCCBPJ:1, 96eX4PTjbXRuGHuMzwXdptWFtHcboXbtevk51Jd73pP7:1, AE9xm2mbemDeMxPUzyt35Agq1axRxggVfV4DRLAZp7Qt:1, FbLyu5d7JxEsvSsujj7Wopg57Wrvz9HH3UULCusKpBnF:1, GsubMWb3LH1skUJrcxTmZ7wus1habJcbpb8su8yBVqFY:1, H9UrL7aWaxDmXhqeGMJy7LrGdT2wWb45mc7kQYsoqwuf:1, Hv88mzPZVKq2fnjoUqK56vjzkcmqRHpWE1ME4z1MXDrw:1] }}, FinalizedResourceVoteChoicesWithVoterInfo {{ resource_vote_choice: Lock, voters: [F1oA8iAoyJ8dgCAi2GSPqcNhp9xEuAqhP47yXBDw5QR:1, 2YSjsJUp74MJpm12rdn8wyPR5MY3c322pV8E8siw989u:1, 3fQrmN4PWhthUFnCFTaJqbT2PPGf7MytAyik4eY1DP8V:1, 7r7gnAiZunVLjtSd5ky4yvPpnWTFYbJuQAapg8kDCeNK:1, 86TUE89xNkBDcmshXRD198xjAvMmKecvHbwo6i83AmqA:1, 97iYr4cirPdG176kqa5nvJWT9tsnqxHmENfRnZUgM6SC:1, 99nKfYZL4spsTe9p9pPNhc1JWv9yq4CbPPMPm87a5sgn:1, BYAqFxCVwMKrw5YAQMCFQGiAF2v3YhKRm2EdGfgkYN9G:1, CGKeK3AfdZUxXF3qH9zxp5MR7Z4WvDVqMrU5wjMKqT5C:1, HRPPEX4mdoZAMkg6NLJUgDzN4pSTpiDXEAGcR5JBdiXX:1] }}], start_block: BlockInfo {{ time_ms: 3000, height: 0, core_height: 0, epoch: 0 }}, finalization_block: BlockInfo {{ time_ms: {}, height: 900, core_height: 42, epoch: 0 }}, winner: Locked }}], vote_poll_status: Locked, locked_count: 1 }}), unlocking is possible by paying 400000000000 credits",
            time_after_distribution_limit
        );

        let _contender_4 = add_contender_to_dpns_name_contest(
            &mut platform,
            &platform_state,
            9,
            "quantum",
            Some(expected_error_message.as_str()), // this should fail, as it is locked
            platform_version,
        );
    }

    #[test]
    fn test_document_creation_on_restricted_document_type_that_only_allows_contract_owner_to_create(
    ) {
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

        let (another_identity, another_identity_signer, another_identity_key) =
            setup_identity(&mut platform, 450, dash_to_credits!(0.1));

        let card_game_path = "tests/supporting_files/contract/crypto-card-game/crypto-card-game-direct-purchase-creation-restricted-to-owner.json";

        let platform_state = platform.state.load();
        let platform_version = platform_state
            .current_platform_version()
            .expect("expected to get current platform version");

        // let's construct the grovedb structure for the card game data contract
        let mut contract = json_document_to_contract(card_game_path, true, platform_version)
            .expect("expected to get data contract");

        contract.set_owner_id(identity.id());

        platform
            .drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("expected to apply contract successfully");

        let card_document_type = contract
            .document_type_for_name("card")
            .expect("expected a profile document type");

        assert_eq!(
            card_document_type.creation_restriction_mode(),
            CreationRestrictionMode::OwnerOnly
        );

        let mut rng = StdRng::seed_from_u64(433);

        let entropy = Bytes32::random_with_rng(&mut rng);

        let mut document = card_document_type
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity.id(),
                entropy,
                DocumentFieldFillType::DoNotFillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        document.set("attack", 4.into());
        document.set("defense", 7.into());

        let documents_batch_create_transition =
            BatchTransition::new_document_creation_transition_from_document(
                document.clone(),
                card_document_type,
                entropy.0,
                &key,
                2,
                0,
                None,
                &signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

        let documents_batch_create_serialized_transition = documents_batch_create_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_create_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        // There is no issue because the creator of the contract made the document

        assert_eq!(processing_result.valid_count(), 1);

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        // Now let's try for another identity

        let mut document = card_document_type
            .random_document_with_identifier_and_entropy(
                &mut rng,
                another_identity.id(),
                entropy,
                DocumentFieldFillType::DoNotFillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        document.set("attack", 8.into());
        document.set("defense", 2.into());

        let documents_batch_create_transition =
            BatchTransition::new_document_creation_transition_from_document(
                document.clone(),
                card_document_type,
                entropy.0,
                &another_identity_key,
                2,
                0,
                None,
                &another_identity_signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

        let documents_batch_create_serialized_transition = documents_batch_create_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_create_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        // There is no issue because the creator of the contract made the document

        assert_eq!(processing_result.invalid_paid_count(), 1);

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let result = processing_result.into_execution_results().remove(0);

        let PaidConsensusError(consensus_error, _) = result else {
            panic!("expected a paid consensus error");
        };
        assert_eq!(consensus_error.to_string(), "Document Creation on 86LHvdC1Tqx5P97LQUSibGFqf2vnKFpB6VkqQ7oso86e:card is not allowed because of the document type's creation restriction mode Owner Only");
    }

    #[test]
    fn test_document_creation_on_search_system_contract_fails_due_to_restriction() {
        // Build test platform
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        // Create an identity that will attempt to create the document
        let (identity, signer, key) = setup_identity(&mut platform, 999, dash_to_credits!(0.1));

        // Obtain the current platform state and version
        let platform_state = platform.state.load();
        let platform_version = platform_state
            .current_platform_version()
            .expect("expected to get current platform version");

        // Load the system Search contract
        let search_contract = platform
            .drive
            .cache
            .system_data_contracts
            .load_keyword_search();

        platform
            .drive
            .apply_contract(
                &search_contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("expected to apply contract successfully");

        // Get the document type from the search contract.
        let doc_type = search_contract
            .document_type_for_name("contractKeywords")
            .expect("expected to find 'contractKeywords' in Search contract");

        // Verify that the document type has the restrictive creation mode
        assert_eq!(
            doc_type.creation_restriction_mode(),
            CreationRestrictionMode::NoCreationAllowed,
            "Expected creation restriction mode to be NoCreationAllowed (2)."
        );

        // Create a random document
        let mut rng = StdRng::seed_from_u64(123456);
        let entropy = Bytes32::random_with_rng(&mut rng);

        let mut document = doc_type
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity.id(),
                entropy,
                DocumentFieldFillType::DoNotFillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected to create a random document");

        // Set fields in the document
        document.set("keyword", "meme".into());
        document.set("contractId", Identifier::random().into());

        // Create the transition
        let documents_batch_create_transition =
            BatchTransition::new_document_creation_transition_from_document(
                document.clone(),
                doc_type,
                entropy.0,
                &key,
                1,
                0,
                None,
                &signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

        let documents_batch_create_serialized_transition = documents_batch_create_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        // Start transaction and submit the transition
        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_create_serialized_transition],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        // Since the creationRestrictionMode is 2 (NoCreationAllowed), this should fail
        assert_eq!(
            processing_result.invalid_paid_count(),
            1,
            "Expected exactly 1 invalid paid transition"
        );

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        // Check the returned consensus error
        let result = processing_result.into_execution_results().remove(0);
        let PaidConsensusError(consensus_error, _) = result else {
            panic!("expected a paid consensus error");
        };

        // Compare message to whatever your code sets for this error
        assert_eq!(
            consensus_error.to_string(),
            format!(
                "Document Creation on {}:{} is not allowed because of the document type's creation restriction mode No Creation Allowed",
                search_contract.id().to_string(Encoding::Base58),
                "contractKeywords"
            ),
            "Mismatch in error message"
        );
    }

    #[test]
    fn test_document_creation_paid_with_a_token_burn() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(433);

        let platform_state = platform.state.load();

        let (contract_owner_id, _, _) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

        let (buyer, signer, key) = setup_identity(&mut platform, 234, dash_to_credits!(0.1));

        let (contract, gold_token_id, _) =
            create_card_game_internal_token_contract_with_owner_identity_burn_tokens(
                &mut platform,
                contract_owner_id.id(),
                platform_version,
            );

        let token_supply = platform
            .drive
            .fetch_token_total_supply(gold_token_id.to_buffer(), None, platform_version)
            .expect("expected to fetch total supply");

        assert_eq!(token_supply, Some(0));

        assert_eq!(contract.tokens().len(), 2);

        add_tokens_to_identity(&mut platform, gold_token_id, buyer.id(), 15);

        let token_supply = platform
            .drive
            .fetch_token_total_supply(gold_token_id.to_buffer(), None, platform_version)
            .expect("expected to fetch total supply");

        assert_eq!(token_supply, Some(15));

        let card_document_type = contract
            .document_type_for_name("card")
            .expect("expected a profile document type");

        let entropy = Bytes32::random_with_rng(&mut rng);

        let mut document = card_document_type
            .random_document_with_identifier_and_entropy(
                &mut rng,
                buyer.id(),
                entropy,
                DocumentFieldFillType::DoNotFillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        document.set("attack", 4.into());
        document.set("defense", 7.into());

        let documents_batch_create_transition =
            BatchTransition::new_document_creation_transition_from_document(
                document.clone(),
                card_document_type,
                entropy.0,
                &key,
                2,
                0,
                Some(TokenPaymentInfo::V0(TokenPaymentInfoV0 {
                    payment_token_contract_id: None,
                    token_contract_position: 0,
                    minimum_token_cost: None,
                    maximum_token_cost: Some(10),
                    gas_fees_paid_by: GasFeesPaidBy::DocumentOwner,
                })),
                &signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

        let documents_batch_create_serialized_transition = documents_batch_create_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_create_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
        );

        platform
            .validate_token_aggregated_balance(&transaction, platform_version)
            .expect("expected to validate token aggregated balances");

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let token_balance = platform
            .drive
            .fetch_identity_token_balance(
                gold_token_id.to_buffer(),
                buyer.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");

        // He had 15, but spent 10
        assert_eq!(token_balance, Some(5));

        // There was a burn so the contract owner shouldn't have gotten more tokens
        let contract_owner_token_balance = platform
            .drive
            .fetch_identity_token_balance(
                gold_token_id.to_buffer(),
                contract_owner_id.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");

        // He started with None, so should have None
        assert_eq!(contract_owner_token_balance, None);
    }

    #[test]
    fn test_document_creation_paid_with_a_token_transfer() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(433);

        let platform_state = platform.state.load();

        let (contract_owner_id, _, _) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

        let (buyer, signer, key) = setup_identity(&mut platform, 234, dash_to_credits!(0.1));

        let (contract, gold_token_id, _) =
            create_card_game_internal_token_contract_with_owner_identity_transfer_tokens(
                &mut platform,
                contract_owner_id.id(),
                platform_version,
            );

        let token_supply = platform
            .drive
            .fetch_token_total_supply(gold_token_id.to_buffer(), None, platform_version)
            .expect("expected to fetch total supply");

        assert_eq!(token_supply, Some(0));

        assert_eq!(contract.tokens().len(), 2);

        add_tokens_to_identity(&mut platform, gold_token_id, buyer.id(), 15);

        let token_supply = platform
            .drive
            .fetch_token_total_supply(gold_token_id.to_buffer(), None, platform_version)
            .expect("expected to fetch total supply");

        assert_eq!(token_supply, Some(15));

        let card_document_type = contract
            .document_type_for_name("card")
            .expect("expected a profile document type");

        let entropy = Bytes32::random_with_rng(&mut rng);

        let mut document = card_document_type
            .random_document_with_identifier_and_entropy(
                &mut rng,
                buyer.id(),
                entropy,
                DocumentFieldFillType::DoNotFillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        document.set("attack", 4.into());
        document.set("defense", 7.into());

        let documents_batch_create_transition =
            BatchTransition::new_document_creation_transition_from_document(
                document.clone(),
                card_document_type,
                entropy.0,
                &key,
                2,
                0,
                Some(TokenPaymentInfo::V0(TokenPaymentInfoV0 {
                    payment_token_contract_id: None,
                    token_contract_position: 0,
                    minimum_token_cost: None,
                    maximum_token_cost: Some(10),
                    gas_fees_paid_by: GasFeesPaidBy::DocumentOwner,
                })),
                &signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

        let documents_batch_create_serialized_transition = documents_batch_create_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_create_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
        );

        platform
            .validate_token_aggregated_balance(&transaction, platform_version)
            .expect("expected to validate token aggregated balances");

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let token_balance = platform
            .drive
            .fetch_identity_token_balance(
                gold_token_id.to_buffer(),
                buyer.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");

        // The buyer had 15, but spent 10
        assert_eq!(token_balance, Some(5));

        let contract_owner_token_balance = platform
            .drive
            .fetch_identity_token_balance(
                gold_token_id.to_buffer(),
                contract_owner_id.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");

        // He was paid 10
        assert_eq!(contract_owner_token_balance, Some(10));
    }

    #[test]
    fn test_document_creation_paid_with_a_token_transfer_to_ones_self() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(433);

        let platform_state = platform.state.load();

        let (contract_owner_id, signer, key) =
            setup_identity(&mut platform, 958, dash_to_credits!(0.1));

        let (contract, gold_token_id, _) =
            create_card_game_internal_token_contract_with_owner_identity_transfer_tokens(
                &mut platform,
                contract_owner_id.id(),
                platform_version,
            );

        let token_supply = platform
            .drive
            .fetch_token_total_supply(gold_token_id.to_buffer(), None, platform_version)
            .expect("expected to fetch total supply");

        assert_eq!(token_supply, Some(0));

        assert_eq!(contract.tokens().len(), 2);

        add_tokens_to_identity(&mut platform, gold_token_id, contract_owner_id.id(), 15);

        let token_supply = platform
            .drive
            .fetch_token_total_supply(gold_token_id.to_buffer(), None, platform_version)
            .expect("expected to fetch total supply");

        assert_eq!(token_supply, Some(15));

        let card_document_type = contract
            .document_type_for_name("card")
            .expect("expected a profile document type");

        let entropy = Bytes32::random_with_rng(&mut rng);

        let mut document = card_document_type
            .random_document_with_identifier_and_entropy(
                &mut rng,
                contract_owner_id.id(),
                entropy,
                DocumentFieldFillType::DoNotFillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        document.set("attack", 4.into());
        document.set("defense", 7.into());

        let documents_batch_create_transition =
            BatchTransition::new_document_creation_transition_from_document(
                document.clone(),
                card_document_type,
                entropy.0,
                &key,
                2,
                0,
                Some(TokenPaymentInfo::V0(TokenPaymentInfoV0 {
                    payment_token_contract_id: None,
                    token_contract_position: 0,
                    minimum_token_cost: None,
                    maximum_token_cost: Some(10),
                    gas_fees_paid_by: GasFeesPaidBy::DocumentOwner,
                })),
                &signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

        let documents_batch_create_serialized_transition = documents_batch_create_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_create_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
        );

        platform
            .validate_token_aggregated_balance(&transaction, platform_version)
            .expect("expected to validate token aggregated balances");

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let token_balance = platform
            .drive
            .fetch_identity_token_balance(
                gold_token_id.to_buffer(),
                contract_owner_id.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");

        // He still has 15
        assert_eq!(token_balance, Some(15));
    }

    #[test]
    fn test_document_creation_paid_with_a_token_not_spending_enough() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(433);

        let platform_state = platform.state.load();

        let (contract_owner_id, _, _) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

        let (buyer, signer, key) = setup_identity(&mut platform, 234, dash_to_credits!(0.1));

        let (contract, gold_token_id, _) =
            create_card_game_internal_token_contract_with_owner_identity_burn_tokens(
                &mut platform,
                contract_owner_id.id(),
                platform_version,
            );

        let token_supply = platform
            .drive
            .fetch_token_total_supply(gold_token_id.to_buffer(), None, platform_version)
            .expect("expected to fetch total supply");

        assert_eq!(token_supply, Some(0));

        assert_eq!(contract.tokens().len(), 2);

        add_tokens_to_identity(&mut platform, gold_token_id.into(), buyer.id(), 15);

        let token_supply = platform
            .drive
            .fetch_token_total_supply(gold_token_id.to_buffer(), None, platform_version)
            .expect("expected to fetch total supply");

        assert_eq!(token_supply, Some(15));

        let card_document_type = contract
            .document_type_for_name("card")
            .expect("expected a profile document type");

        let entropy = Bytes32::random_with_rng(&mut rng);

        let mut document = card_document_type
            .random_document_with_identifier_and_entropy(
                &mut rng,
                buyer.id(),
                entropy,
                DocumentFieldFillType::DoNotFillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        document.set("attack", 4.into());
        document.set("defense", 7.into());

        let documents_batch_create_transition =
            BatchTransition::new_document_creation_transition_from_document(
                document.clone(),
                card_document_type,
                entropy.0,
                &key,
                2,
                0,
                Some(TokenPaymentInfo::V0(TokenPaymentInfoV0 {
                    payment_token_contract_id: None,
                    token_contract_position: 0,
                    minimum_token_cost: None,
                    maximum_token_cost: Some(9),
                    gas_fees_paid_by: GasFeesPaidBy::DocumentOwner,
                })),
                &signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

        let documents_batch_create_serialized_transition = documents_batch_create_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_create_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [PaidConsensusError(
                ConsensusError::StateError(
                    StateError::IdentityHasNotAgreedToPayRequiredTokenAmountError(_)
                ),
                _
            )]
        );

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let token_balance = platform
            .drive
            .fetch_identity_token_balance(
                gold_token_id.to_buffer(),
                buyer.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");

        // He should still have 15
        assert_eq!(token_balance, Some(15));
    }

    #[test]
    fn test_document_creation_paid_with_a_token_minimum_cost_set_rare_scenario() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(433);

        let platform_state = platform.state.load();

        let (contract_owner_id, _, _) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

        let (buyer, signer, key) = setup_identity(&mut platform, 234, dash_to_credits!(0.1));

        let (contract, gold_token_id, _) =
            create_card_game_internal_token_contract_with_owner_identity_burn_tokens(
                &mut platform,
                contract_owner_id.id(),
                platform_version,
            );

        let token_supply = platform
            .drive
            .fetch_token_total_supply(gold_token_id.to_buffer(), None, platform_version)
            .expect("expected to fetch total supply");

        assert_eq!(token_supply, Some(0));

        assert_eq!(contract.tokens().len(), 2);

        add_tokens_to_identity(&mut platform, gold_token_id.into(), buyer.id(), 15);

        let token_supply = platform
            .drive
            .fetch_token_total_supply(gold_token_id.to_buffer(), None, platform_version)
            .expect("expected to fetch total supply");

        assert_eq!(token_supply, Some(15));

        let card_document_type = contract
            .document_type_for_name("card")
            .expect("expected a profile document type");

        let entropy = Bytes32::random_with_rng(&mut rng);

        let mut document = card_document_type
            .random_document_with_identifier_and_entropy(
                &mut rng,
                buyer.id(),
                entropy,
                DocumentFieldFillType::DoNotFillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        document.set("attack", 4.into());
        document.set("defense", 7.into());

        let documents_batch_create_transition =
            BatchTransition::new_document_creation_transition_from_document(
                document.clone(),
                card_document_type,
                entropy.0,
                &key,
                2,
                0,
                Some(TokenPaymentInfo::V0(TokenPaymentInfoV0 {
                    payment_token_contract_id: None,
                    token_contract_position: 0,
                    minimum_token_cost: Some(12),
                    maximum_token_cost: None,
                    gas_fees_paid_by: GasFeesPaidBy::DocumentOwner,
                })),
                &signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

        let documents_batch_create_serialized_transition = documents_batch_create_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_create_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [PaidConsensusError(
                ConsensusError::StateError(
                    StateError::IdentityHasNotAgreedToPayRequiredTokenAmountError(_)
                ),
                _
            )]
        );

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let token_balance = platform
            .drive
            .fetch_identity_token_balance(
                gold_token_id.to_buffer(),
                buyer.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");

        // He should still have 15
        assert_eq!(token_balance, Some(15));
    }

    #[test]
    fn test_document_creation_paid_with_a_token_agreeing_to_too_much() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(433);

        let platform_state = platform.state.load();

        let (contract_owner_id, _, _) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

        let (buyer, signer, key) = setup_identity(&mut platform, 234, dash_to_credits!(0.1));

        let (contract, gold_token_id, _) =
            create_card_game_internal_token_contract_with_owner_identity_burn_tokens(
                &mut platform,
                contract_owner_id.id(),
                platform_version,
            );

        let token_supply = platform
            .drive
            .fetch_token_total_supply(gold_token_id.to_buffer(), None, platform_version)
            .expect("expected to fetch total supply");

        assert_eq!(token_supply, Some(0));

        assert_eq!(contract.tokens().len(), 2);

        add_tokens_to_identity(&mut platform, gold_token_id.into(), buyer.id(), 15);

        let token_supply = platform
            .drive
            .fetch_token_total_supply(gold_token_id.to_buffer(), None, platform_version)
            .expect("expected to fetch total supply");

        assert_eq!(token_supply, Some(15));

        let card_document_type = contract
            .document_type_for_name("card")
            .expect("expected a profile document type");

        let entropy = Bytes32::random_with_rng(&mut rng);

        let mut document = card_document_type
            .random_document_with_identifier_and_entropy(
                &mut rng,
                buyer.id(),
                entropy,
                DocumentFieldFillType::DoNotFillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        document.set("attack", 4.into());
        document.set("defense", 7.into());

        let documents_batch_create_transition =
            BatchTransition::new_document_creation_transition_from_document(
                document.clone(),
                card_document_type,
                entropy.0,
                &key,
                2,
                0,
                Some(TokenPaymentInfo::V0(TokenPaymentInfoV0 {
                    payment_token_contract_id: None,
                    token_contract_position: 0,
                    minimum_token_cost: None,
                    maximum_token_cost: Some(12),
                    gas_fees_paid_by: GasFeesPaidBy::DocumentOwner,
                })),
                &signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

        let documents_batch_create_serialized_transition = documents_batch_create_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_create_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
        );

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let token_balance = platform
            .drive
            .fetch_identity_token_balance(
                gold_token_id.to_buffer(),
                buyer.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");

        // He had 15, but only spent 10
        assert_eq!(token_balance, Some(5));
    }

    #[test]
    fn test_document_creation_paid_with_a_token_token_info_not_set() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(433);

        let platform_state = platform.state.load();

        let (contract_owner_id, _, _) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

        let (buyer, signer, key) = setup_identity(&mut platform, 234, dash_to_credits!(0.1));

        let (contract, gold_token_id, _) =
            create_card_game_internal_token_contract_with_owner_identity_burn_tokens(
                &mut platform,
                contract_owner_id.id(),
                platform_version,
            );

        let token_supply = platform
            .drive
            .fetch_token_total_supply(gold_token_id.to_buffer(), None, platform_version)
            .expect("expected to fetch total supply");

        assert_eq!(token_supply, Some(0));

        assert_eq!(contract.tokens().len(), 2);

        add_tokens_to_identity(&mut platform, gold_token_id.into(), buyer.id(), 15);

        let token_supply = platform
            .drive
            .fetch_token_total_supply(gold_token_id.to_buffer(), None, platform_version)
            .expect("expected to fetch total supply");

        assert_eq!(token_supply, Some(15));

        let card_document_type = contract
            .document_type_for_name("card")
            .expect("expected a profile document type");

        let entropy = Bytes32::random_with_rng(&mut rng);

        let mut document = card_document_type
            .random_document_with_identifier_and_entropy(
                &mut rng,
                buyer.id(),
                entropy,
                DocumentFieldFillType::DoNotFillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        document.set("attack", 4.into());
        document.set("defense", 7.into());

        let documents_batch_create_transition =
            BatchTransition::new_document_creation_transition_from_document(
                document.clone(),
                card_document_type,
                entropy.0,
                &key,
                2,
                0,
                None,
                &signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

        let documents_batch_create_serialized_transition = documents_batch_create_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_create_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [PaidConsensusError(
                ConsensusError::StateError(StateError::RequiredTokenPaymentInfoNotSetError(_)),
                _
            )]
        );

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let token_balance = platform
            .drive
            .fetch_identity_token_balance(
                gold_token_id.to_buffer(),
                buyer.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");

        // Nothing should have happened
        assert_eq!(token_balance, Some(15));
    }

    #[test]
    fn test_document_creation_not_enough_token_balance_to_create_document() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(433);

        let platform_state = platform.state.load();

        let (contract_owner_id, _, _) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

        let (buyer, signer, key) = setup_identity(&mut platform, 234, dash_to_credits!(0.1));

        let (contract, gold_token_id, _) =
            create_card_game_internal_token_contract_with_owner_identity_burn_tokens(
                &mut platform,
                contract_owner_id.id(),
                platform_version,
            );

        let token_supply = platform
            .drive
            .fetch_token_total_supply(gold_token_id.to_buffer(), None, platform_version)
            .expect("expected to fetch total supply");

        assert_eq!(token_supply, Some(0));

        assert_eq!(contract.tokens().len(), 2);

        // We need 10 tokens, we have 8.
        add_tokens_to_identity(&mut platform, gold_token_id, buyer.id(), 8);

        let token_supply = platform
            .drive
            .fetch_token_total_supply(gold_token_id.to_buffer(), None, platform_version)
            .expect("expected to fetch total supply");

        assert_eq!(token_supply, Some(8));

        let card_document_type = contract
            .document_type_for_name("card")
            .expect("expected a profile document type");

        let entropy = Bytes32::random_with_rng(&mut rng);

        let mut document = card_document_type
            .random_document_with_identifier_and_entropy(
                &mut rng,
                buyer.id(),
                entropy,
                DocumentFieldFillType::DoNotFillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        document.set("attack", 4.into());
        document.set("defense", 7.into());

        let documents_batch_create_transition =
            BatchTransition::new_document_creation_transition_from_document(
                document.clone(),
                card_document_type,
                entropy.0,
                &key,
                2,
                0,
                Some(TokenPaymentInfo::V0(TokenPaymentInfoV0 {
                    payment_token_contract_id: None, // This contract
                    token_contract_position: 0,
                    minimum_token_cost: None,
                    maximum_token_cost: None, // We'll pay whatever
                    gas_fees_paid_by: GasFeesPaidBy::DocumentOwner,
                })),
                &signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

        let documents_batch_create_serialized_transition = documents_batch_create_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_create_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [PaidConsensusError(
                ConsensusError::StateError(StateError::IdentityDoesNotHaveEnoughTokenBalanceError(
                    _
                )),
                _
            )]
        );

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let token_balance = platform
            .drive
            .fetch_identity_token_balance(
                gold_token_id.to_buffer(),
                buyer.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");

        // We should still have 8
        assert_eq!(token_balance, Some(8));
    }

    #[test]
    fn test_document_creation_paid_with_an_external_token() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(433);

        let platform_state = platform.state.load();

        let (document_contract_owner_id, _, _) =
            setup_identity(&mut platform, 958, dash_to_credits!(0.1));

        let (token_contract_owner_id, _, _) =
            setup_identity(&mut platform, 11, dash_to_credits!(0.1));

        let (buyer, signer, key) = setup_identity(&mut platform, 234, dash_to_credits!(0.1));

        let (token_contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            token_contract_owner_id.id(),
            None::<fn(&mut TokenConfiguration)>,
            None,
            None,
            None,
            platform_version,
        );

        let document_contract = create_card_game_external_token_contract_with_owner_identity(
            &mut platform,
            token_contract.id(),
            0,
            5,
            GasFeesPaidBy::DocumentOwner,
            document_contract_owner_id.id(),
            platform_version,
        );

        let token_supply = platform
            .drive
            .fetch_token_total_supply(token_id.to_buffer(), None, platform_version)
            .expect("expected to fetch total supply");

        assert_eq!(token_supply, Some(100000));

        assert_eq!(token_contract.tokens().len(), 1);

        add_tokens_to_identity(&mut platform, token_id.into(), buyer.id(), 15);

        let token_supply = platform
            .drive
            .fetch_token_total_supply(token_id.to_buffer(), None, platform_version)
            .expect("expected to fetch total supply");

        assert_eq!(token_supply, Some(100015));

        let card_document_type = document_contract
            .document_type_for_name("card")
            .expect("expected a profile document type");

        assert_eq!(
            card_document_type.document_creation_token_cost(),
            Some(DocumentActionTokenCost {
                contract_id: Some(token_contract.id()),
                token_contract_position: 0,
                token_amount: 5,
                effect: Default::default(),
                gas_fees_paid_by: GasFeesPaidBy::DocumentOwner,
            })
        );

        let entropy = Bytes32::random_with_rng(&mut rng);

        let mut document = card_document_type
            .random_document_with_identifier_and_entropy(
                &mut rng,
                buyer.id(),
                entropy,
                DocumentFieldFillType::DoNotFillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        document.set("attack", 4.into());
        document.set("defense", 7.into());

        let documents_batch_create_transition =
            BatchTransition::new_document_creation_transition_from_document(
                document.clone(),
                card_document_type,
                entropy.0,
                &key,
                2,
                0,
                Some(TokenPaymentInfo::V0(TokenPaymentInfoV0 {
                    payment_token_contract_id: Some(token_contract.id()),
                    token_contract_position: 0,
                    minimum_token_cost: None,
                    maximum_token_cost: Some(5),
                    gas_fees_paid_by: GasFeesPaidBy::DocumentOwner,
                })),
                &signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

        assert_matches!(
            documents_batch_create_transition,
            StateTransition::Batch(BatchTransition::V1(_))
        );

        let documents_batch_create_serialized_transition = documents_batch_create_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_create_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
        );

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let token_balance = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                buyer.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");

        // He had 15, but spent 5
        assert_eq!(token_balance, Some(10));

        let token_supply = platform
            .drive
            .fetch_token_total_supply(token_id.to_buffer(), None, platform_version)
            .expect("expected to fetch total supply");

        assert_eq!(token_supply, Some(100015));

        let token_balance = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                document_contract_owner_id.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");

        // He was paid 5
        assert_eq!(token_balance, Some(5));
    }
}
