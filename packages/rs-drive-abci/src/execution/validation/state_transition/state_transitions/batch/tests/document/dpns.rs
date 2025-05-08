use super::*;

mod dpns_tests {
    use super::*;
    use crate::execution::validation::state_transition::tests::setup_identity;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::dash_to_credits;
    use dpp::data_contract::document_type::random_document::{
        DocumentFieldFillSize, DocumentFieldFillType,
    };
    use dpp::data_contract::DataContract;
    use dpp::platform_value::Bytes32;
    use dpp::state_transition::batch_transition::BatchTransition;
    use dpp::util::hash::hash_double;
    use drive::query::{InternalClauses, OrderClause, WhereClause, WhereOperator};
    use drive::util::test_helpers::setup_contract;
    use indexmap::IndexMap;
    use platform_version::version::PlatformVersion;
    use rand::prelude::StdRng;
    use std::collections::BTreeMap;

    #[test]
    fn test_dpns_contract_references_with_no_contested_unique_index() {
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

        let (identity_3, signer_3, key_3) =
            setup_identity(&mut platform, 98, dash_to_credits!(0.5));

        let dashpay_contract = setup_contract(
            &platform.drive,
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
            None,
            None,
            None::<fn(&mut DataContract)>,
            None,
            None,
        );

        let card_game = setup_contract(
            &platform.drive,
            "tests/supporting_files/contract/crypto-card-game/crypto-card-game-direct-purchase.json",
            None,
            None,
            None::<fn(&mut DataContract)>,
            None,
            None,
        );

        let dpns_contract = setup_contract(
            &platform.drive,
            "tests/supporting_files/contract/dpns/dpns-contract-contested-unique-index-with-contract-id.json",
            None,
            None,
            None::<fn(&mut DataContract)>,
            None,
            None,
        );

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

        let mut preorder_document_3 = preorder
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity_3.id(),
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

        let mut document_3 = domain
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity_3.id(),
                entropy,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        document_1.set("parentDomainName", "dash".into());
        document_1.set("normalizedParentDomainName", "dash".into());
        document_1.set("label", "quantum123".into());
        document_1.set("normalizedLabel", "quantum123".into());
        document_1.set("records.contract", dashpay_contract.id().into());
        document_1.set("subdomainRules.allowSubdomains", false.into());

        document_2.set("parentDomainName", "dash".into());
        document_2.set("normalizedParentDomainName", "dash".into());
        document_2.set("label", "van89".into());
        document_2.set("normalizedLabel", "van89".into());
        document_2.set("records.contract", card_game.id().into());
        document_2.set("subdomainRules.allowSubdomains", false.into());

        document_3.set("parentDomainName", "dash".into());
        document_3.set("normalizedParentDomainName", "dash".into());
        document_3.set("label", "jazz65".into());
        document_3.set("normalizedLabel", "jazz65".into());
        document_3.set("records.identity", document_3.owner_id().into());
        document_3.set("subdomainRules.allowSubdomains", false.into());

        let salt_1: [u8; 32] = rng.gen();
        let salt_2: [u8; 32] = rng.gen();
        let salt_3: [u8; 32] = rng.gen();

        let mut salted_domain_buffer_1: Vec<u8> = vec![];
        salted_domain_buffer_1.extend(salt_1);
        salted_domain_buffer_1.extend("quantum123.dash".as_bytes());

        let salted_domain_hash_1 = hash_double(salted_domain_buffer_1);

        let mut salted_domain_buffer_2: Vec<u8> = vec![];
        salted_domain_buffer_2.extend(salt_2);
        salted_domain_buffer_2.extend("van89.dash".as_bytes());

        let salted_domain_hash_2 = hash_double(salted_domain_buffer_2);

        let mut salted_domain_buffer_3: Vec<u8> = vec![];
        salted_domain_buffer_3.extend(salt_3);
        salted_domain_buffer_3.extend("jazz65.dash".as_bytes());

        let salted_domain_hash_3 = hash_double(salted_domain_buffer_3);

        preorder_document_1.set("saltedDomainHash", salted_domain_hash_1.into());
        preorder_document_2.set("saltedDomainHash", salted_domain_hash_2.into());
        preorder_document_3.set("saltedDomainHash", salted_domain_hash_3.into());

        document_1.set("preorderSalt", salt_1.into());
        document_2.set("preorderSalt", salt_2.into());
        document_3.set("preorderSalt", salt_3.into());

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
                preorder_document_3,
                preorder,
                entropy.0,
                &key_3,
                2,
                0,
                None,
                &signer_3,
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

        let documents_batch_create_transition_3 =
            BatchTransition::new_document_creation_transition_from_document(
                document_3.clone(),
                domain,
                entropy.0,
                &key_3,
                3,
                0,
                None,
                &signer_3,
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
                    documents_batch_create_serialized_transition_3.clone(),
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

        let mut order_by = IndexMap::new();

        order_by.insert(
            "records.identity".to_string(),
            OrderClause {
                field: "records.identity".to_string(),
                ascending: true,
            },
        );

        let drive_query = DriveDocumentQuery {
            contract: &dpns_contract,
            document_type: domain,
            internal_clauses: InternalClauses {
                primary_key_in_clause: None,
                primary_key_equal_clause: None,
                in_clause: None,
                range_clause: Some(WhereClause {
                    field: "records.identity".to_string(),
                    operator: WhereOperator::LessThanOrEquals,
                    value: Value::Bytes32([255; 32]),
                }),
                equal_clauses: Default::default(),
            },
            offset: None,
            limit: None,
            order_by,
            start_at: None,
            start_at_included: false,
            block_time_ms: None,
        };

        let documents = platform
            .drive
            .query_documents(drive_query, None, false, None, None)
            .expect("expected to get back documents")
            .documents_owned();

        let transient_fields = domain
            .transient_fields()
            .iter()
            .map(|a| a.as_str())
            .collect();

        assert!(documents
            .first()
            .expect("expected a document")
            .is_equal_ignoring_time_based_fields(
                &document_3,
                Some(transient_fields),
                platform_version
            )
            .expect("expected to run is equal"));

        let drive_query = DriveDocumentQuery {
            contract: &dpns_contract,
            document_type: domain,
            internal_clauses: InternalClauses {
                primary_key_in_clause: None,
                primary_key_equal_clause: None,
                in_clause: None,
                range_clause: None,
                equal_clauses: BTreeMap::from([(
                    "records.identity".to_string(),
                    WhereClause {
                        field: "records.identity".to_string(),
                        operator: WhereOperator::Equal,
                        value: Value::Null,
                    },
                )]),
            },
            offset: None,
            limit: None,
            order_by: Default::default(),
            start_at: None,
            start_at_included: false,
            block_time_ms: None,
        };

        let documents = platform
            .drive
            .query_documents(drive_query, None, false, None, None)
            .expect("expected to get back documents")
            .documents_owned();

        // This is normal because we set that we could not query on null
        assert_eq!(documents.len(), 0);
    }

    #[test]
    fn test_dpns_contract_references_with_no_contested_unique_index_null_searchable_true() {
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

        let (identity_3, signer_3, key_3) =
            setup_identity(&mut platform, 98, dash_to_credits!(0.5));

        let dashpay_contract = setup_contract(
            &platform.drive,
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
            None,
            None,
            None::<fn(&mut DataContract)>,
            None,
            None,
        );

        let card_game = setup_contract(
            &platform.drive,
            "tests/supporting_files/contract/crypto-card-game/crypto-card-game-direct-purchase.json",
            None,
            None,
            None::<fn(&mut DataContract)>,
            None,
            None,
        );

        let dpns_contract = setup_contract(
            &platform.drive,
            "tests/supporting_files/contract/dpns/dpns-contract-contested-unique-index-with-contract-id-null-searchable-true.json",
            None,
            None,
            None::<fn(&mut DataContract)>,
            None,
            None,
        );

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

        let mut preorder_document_3 = preorder
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity_3.id(),
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

        let mut document_3 = domain
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity_3.id(),
                entropy,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        document_1.set("parentDomainName", "dash".into());
        document_1.set("normalizedParentDomainName", "dash".into());
        document_1.set("label", "quantum123".into());
        document_1.set("normalizedLabel", "quantum123".into());
        document_1.set("records.contract", dashpay_contract.id().into());
        document_1.set("subdomainRules.allowSubdomains", false.into());

        document_2.set("parentDomainName", "dash".into());
        document_2.set("normalizedParentDomainName", "dash".into());
        document_2.set("label", "van89".into());
        document_2.set("normalizedLabel", "van89".into());
        document_2.set("records.contract", card_game.id().into());
        document_2.set("subdomainRules.allowSubdomains", false.into());

        document_3.set("parentDomainName", "dash".into());
        document_3.set("normalizedParentDomainName", "dash".into());
        document_3.set("label", "jazz65".into());
        document_3.set("normalizedLabel", "jazz65".into());
        document_3.set("records.identity", document_3.owner_id().into());
        document_3.set("subdomainRules.allowSubdomains", false.into());

        let salt_1: [u8; 32] = rng.gen();
        let salt_2: [u8; 32] = rng.gen();
        let salt_3: [u8; 32] = rng.gen();

        let mut salted_domain_buffer_1: Vec<u8> = vec![];
        salted_domain_buffer_1.extend(salt_1);
        salted_domain_buffer_1.extend("quantum123.dash".as_bytes());

        let salted_domain_hash_1 = hash_double(salted_domain_buffer_1);

        let mut salted_domain_buffer_2: Vec<u8> = vec![];
        salted_domain_buffer_2.extend(salt_2);
        salted_domain_buffer_2.extend("van89.dash".as_bytes());

        let salted_domain_hash_2 = hash_double(salted_domain_buffer_2);

        let mut salted_domain_buffer_3: Vec<u8> = vec![];
        salted_domain_buffer_3.extend(salt_3);
        salted_domain_buffer_3.extend("jazz65.dash".as_bytes());

        let salted_domain_hash_3 = hash_double(salted_domain_buffer_3);

        preorder_document_1.set("saltedDomainHash", salted_domain_hash_1.into());
        preorder_document_2.set("saltedDomainHash", salted_domain_hash_2.into());
        preorder_document_3.set("saltedDomainHash", salted_domain_hash_3.into());

        document_1.set("preorderSalt", salt_1.into());
        document_2.set("preorderSalt", salt_2.into());
        document_3.set("preorderSalt", salt_3.into());

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
                preorder_document_3,
                preorder,
                entropy.0,
                &key_3,
                2,
                0,
                None,
                &signer_3,
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

        let documents_batch_create_transition_3 =
            BatchTransition::new_document_creation_transition_from_document(
                document_3.clone(),
                domain,
                entropy.0,
                &key_3,
                3,
                0,
                None,
                &signer_3,
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
                    documents_batch_create_serialized_transition_3.clone(),
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

        let mut order_by = IndexMap::new();

        order_by.insert(
            "records.identity".to_string(),
            OrderClause {
                field: "records.identity".to_string(),
                ascending: true,
            },
        );

        let drive_query = DriveDocumentQuery {
            contract: &dpns_contract,
            document_type: domain,
            internal_clauses: InternalClauses {
                primary_key_in_clause: None,
                primary_key_equal_clause: None,
                in_clause: None,
                range_clause: Some(WhereClause {
                    field: "records.identity".to_string(),
                    operator: WhereOperator::LessThanOrEquals,
                    value: Value::Bytes32([255; 32]),
                }),
                equal_clauses: Default::default(),
            },
            offset: None,
            limit: None,
            order_by,
            start_at: None,
            start_at_included: false,
            block_time_ms: None,
        };

        let documents = platform
            .drive
            .query_documents(drive_query, None, false, None, None)
            .expect("expected to get back documents")
            .documents_owned();

        // here we will get all 3 documents
        assert_eq!(documents.len(), 3);

        let drive_query = DriveDocumentQuery {
            contract: &dpns_contract,
            document_type: domain,
            internal_clauses: InternalClauses {
                primary_key_in_clause: None,
                primary_key_equal_clause: None,
                in_clause: None,
                range_clause: None,
                equal_clauses: BTreeMap::from([(
                    "records.identity".to_string(),
                    WhereClause {
                        field: "records.identity".to_string(),
                        operator: WhereOperator::Equal,
                        value: Value::Null,
                    },
                )]),
            },
            offset: None,
            limit: None,
            order_by: Default::default(),
            start_at: None,
            start_at_included: false,
            block_time_ms: None,
        };

        let documents = platform
            .drive
            .query_documents(drive_query, None, false, None, None)
            .expect("expected to get back documents")
            .documents_owned();

        assert_eq!(documents.len(), 2);
    }
}
