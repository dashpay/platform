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

    #[tokio::test]
    async fn test_dpns_contract_references_with_no_contested_unique_index() {
        run_dpns_contract_references_with_no_contested_unique_index_at_protocol_version(
            PlatformVersion::latest().protocol_version,
            6_010_380,
        )
        .await;
    }

    /// PROTOCOL_VERSION_11: pre-T1/T2 fee — `create_domain_data_trigger_v0`
    /// runs the same parent-domain + preorder queries but discards their
    /// cost (epoch=None → `query_documents` cost short-circuits to 0,
    /// trigger context's add_operation never called on v0). Pinned so
    /// v11 chain history stays bit-for-bit reproducible.
    ///
    /// Delta vs PV12: 6_010_380 - 5_978_080 = 32_300 credits = T1 + T2
    /// query costs across 3 subdomain creates (~10,767 per transition,
    /// or ~5,383 per query).
    #[tokio::test]
    async fn test_dpns_contract_references_with_no_contested_unique_index_protocol_version_11() {
        run_dpns_contract_references_with_no_contested_unique_index_at_protocol_version(
            11, 5_978_080,
        )
        .await;
    }

    async fn run_dpns_contract_references_with_no_contested_unique_index_at_protocol_version(
        protocol_version: dpp::version::ProtocolVersion,
        expected_processing_fee: dpp::fee::Credits,
    ) {
        let platform_version = PlatformVersion::get(protocol_version)
            .expect("expected platform version for the requested protocol_version");
        let mut platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(protocol_version)
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
            .await
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
            .await
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
            .await
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
            .await
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
            .await
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
            .await
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

        // T1/T2 regression pin: the DPNS `create_domain_data_trigger`
        // runs two `query_documents` calls per transition (parent-domain
        // + preorder). On PV12+ (`transform_into_action: 1`) the
        // accumulated cost is billed via the trigger's returned
        // `FeeResult`. On PV11 the cost is discarded.
        assert_eq!(
            processing_result.aggregated_fees().processing_fee,
            expected_processing_fee,
            "PROTOCOL_VERSION_{}: DPNS domain create fee must match the version-specific baseline (T1 parent-domain + T2 preorder query costs billed only at PV12+)",
            protocol_version,
        );

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
                in_clauses: Vec::new(),
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
            resolved_time_ranges: vec![],
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
                in_clauses: Vec::new(),
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
            resolved_time_ranges: vec![],
        };

        let documents = platform
            .drive
            .query_documents(drive_query, None, false, None, None)
            .expect("expected to get back documents")
            .documents_owned();

        // This is normal because we set that we could not query on null
        assert_eq!(documents.len(), 0);
    }

    #[tokio::test]
    async fn test_dpns_contract_references_with_no_contested_unique_index_null_searchable_true() {
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
            .await
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
            .await
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
            .await
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
            .await
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
            .await
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
            .await
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
                in_clauses: Vec::new(),
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
            resolved_time_ranges: vec![],
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
                in_clauses: Vec::new(),
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
            resolved_time_ranges: vec![],
        };

        let documents = platform
            .drive
            .query_documents(drive_query, None, false, None, None)
            .expect("expected to get back documents")
            .documents_owned();

        assert_eq!(documents.len(), 2);
    }
}

mod dpns_username_transfer_tests {
    use super::*;
    use dpp::consensus::basic::BasicError;
    use dpp::consensus::state::data_trigger::DataTriggerError;
    use dpp::data_contract::DataContract;
    use dpp::data_contracts::SystemDataContract;
    use dpp::document::Document;
    use dpp::identity::{Identity, IdentityPublicKey};
    use dpp::util::hash::hash_double;
    use dpp::util::strings::convert_to_homograph_safe_chars;
    use drive::query::drive_document_average_query::{
        AverageMode, DocumentAverageRequest, DocumentAverageResponse,
    };
    use drive::query::drive_document_count_query::{
        CountMode, DocumentCountRequest, DocumentCountResponse,
    };
    use drive::query::drive_document_sum_query::{
        DocumentSumRequest, DocumentSumResponse, SumMode,
    };
    use drive::query::{InternalClauses, WhereClause, WhereOperator};
    use simple_signer::signer::SimpleSigner;
    use std::collections::BTreeMap;
    use std::sync::Arc;

    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TempPlatform;
    use dpp::prelude::Identifier;
    use dpp::version::ProtocolVersion;

    /// Registers `<label>.dash` (preorder + domain) for the given identity and
    /// returns the domain document as submitted along with the DPNS contract.
    ///
    /// The label must not match the contested-name regex (it should contain at
    /// least one digit in 2-9) so the domain document is inserted immediately
    /// instead of starting a masternode vote contest.
    async fn register_dpns_username(
        platform: &mut TempPlatform<MockCoreRPCLike>,
        identity: &(Identity, SimpleSigner, IdentityPublicKey),
        rng: &mut StdRng,
        label: &str,
        platform_version: &PlatformVersion,
    ) -> (Document, Arc<DataContract>) {
        let (identity, signer, key) = identity;

        let platform_state = platform.state.load();

        let dpns = platform
            .drive
            .cache
            .system_data_contracts
            .load_dpns(platform_version)
            .expect("expected the dpns system contract");
        let dpns_contract = dpns.clone();

        let preorder = dpns_contract
            .document_type_for_name("preorder")
            .expect("expected the preorder document type");

        let domain = dpns_contract
            .document_type_for_name("domain")
            .expect("expected the domain document type");

        let entropy = Bytes32::random_with_rng(rng);

        let mut preorder_document = preorder
            .random_document_with_identifier_and_entropy(
                rng,
                identity.id(),
                entropy,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random preorder document");

        let mut document = domain
            .random_document_with_identifier_and_entropy(
                rng,
                identity.id(),
                entropy,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random domain document");

        let normalized_label = convert_to_homograph_safe_chars(label);

        document.set("parentDomainName", "dash".into());
        document.set("normalizedParentDomainName", "dash".into());
        document.set("label", label.into());
        document.set("normalizedLabel", normalized_label.clone().into());
        document.set("records.identity", document.owner_id().into());
        document.set("subdomainRules.allowSubdomains", false.into());

        let salt: [u8; 32] = rng.gen();

        let mut salted_domain_buffer: Vec<u8> = vec![];
        salted_domain_buffer.extend(salt);
        salted_domain_buffer.extend((normalized_label + ".dash").as_bytes());

        let salted_domain_hash = hash_double(salted_domain_buffer);

        preorder_document.set("saltedDomainHash", salted_domain_hash.into());
        document.set("preorderSalt", salt.into());

        let documents_batch_create_preorder_transition =
            BatchTransition::new_document_creation_transition_from_document(
                preorder_document,
                preorder,
                entropy.0,
                key,
                2,
                0,
                None,
                signer,
                platform_version,
                None,
            )
            .await
            .expect("expect to create preorder batch transition");

        let documents_batch_create_serialized_preorder_transition =
            documents_batch_create_preorder_transition
                .serialize_to_bytes()
                .expect("expected preorder batch serialized state transition");

        let documents_batch_create_transition =
            BatchTransition::new_document_creation_transition_from_document(
                document.clone(),
                domain,
                entropy.0,
                key,
                3,
                0,
                None,
                signer,
                platform_version,
                None,
            )
            .await
            .expect("expect to create domain batch transition");

        let documents_batch_create_serialized_transition = documents_batch_create_transition
            .serialize_to_bytes()
            .expect("expected domain batch serialized state transition");

        for serialized_transition in [
            documents_batch_create_serialized_preorder_transition,
            documents_batch_create_serialized_transition,
        ] {
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[serialized_transition],
                    &platform_state,
                    &BlockInfo::default_with_time(
                        platform_state
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

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution { .. }]
            );

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");
        }

        (document, dpns_contract)
    }

    /// Queries domain documents whose `records.identity` name record points to
    /// the given identity.
    fn query_domain_documents_by_record_identity(
        platform: &TempPlatform<MockCoreRPCLike>,
        dpns_contract: &DataContract,
        identity_id: Identifier,
    ) -> Vec<Document> {
        let domain = dpns_contract
            .document_type_for_name("domain")
            .expect("expected the domain document type");

        let drive_query = DriveDocumentQuery {
            contract: dpns_contract,
            document_type: domain,
            internal_clauses: InternalClauses {
                primary_key_in_clause: None,
                primary_key_equal_clause: None,
                in_clauses: Vec::new(),
                range_clause: None,
                equal_clauses: BTreeMap::from([(
                    "records.identity".to_string(),
                    WhereClause {
                        field: "records.identity".to_string(),
                        operator: WhereOperator::Equal,
                        value: Value::Identifier(identity_id.to_buffer()),
                    },
                )]),
            },
            offset: None,
            limit: None,
            order_by: Default::default(),
            start_at: None,
            start_at_included: false,
            block_time_ms: None,
            resolved_time_ranges: vec![],
        };

        platform
            .drive
            .query_documents(drive_query, None, false, None, None)
            .expect("expected to query domain documents")
            .documents_owned()
    }

    fn assert_identity_record(document: &Document, expected_identity_id: Identifier) {
        let records = document
            .properties()
            .get("records")
            .expect("expected the domain document to have records");

        let record_identity = records
            .get_optional_identifier("identity")
            .expect("expected a valid identity record")
            .expect("expected an identity record to be set");

        assert_eq!(record_identity, expected_identity_id);
    }

    /// Asserts the document history system contract does not exist in the
    /// state: before protocol version 13 it must be entirely absent, exactly
    /// as on a non-upgraded node.
    fn assert_document_history_contract_absent(
        platform: &TempPlatform<MockCoreRPCLike>,
        platform_version: &PlatformVersion,
    ) {
        let (_fee_result, maybe_contract) = platform
            .drive
            .get_contract_with_fetch_info_and_fee(
                dpp::system_data_contracts::SystemDataContract::DocumentHistory
                    .id()
                    .to_buffer(),
                None,
                false,
                None,
                platform_version,
            )
            .expect("expected to fetch contract");

        assert!(
            maybe_contract.is_none(),
            "the document history contract must not exist before protocol version 13"
        );
    }

    fn history_where_by_contract(source_data_contract_id: Identifier) -> WhereClause {
        WhereClause {
            field: "dataContractId".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Identifier(source_data_contract_id.to_buffer()),
        }
    }

    fn history_where_created_between(from_ms: u64, to_ms: u64) -> WhereClause {
        WhereClause {
            field: "$createdAt".to_string(),
            operator: WhereOperator::Between,
            value: Value::Array(vec![Value::U64(from_ms), Value::U64(to_ms)]),
        }
    }

    /// Provable aggregate count of history documents matching the where
    /// clauses (whole doctype when empty).
    fn history_aggregate_count(
        platform: &TempPlatform<MockCoreRPCLike>,
        history_document_type_name: &str,
        where_clauses: Vec<WhereClause>,
        platform_version: &PlatformVersion,
    ) -> u64 {
        let history_contract = Arc::clone(
            &platform
                .drive
                .cache
                .system_data_contracts
                .load_document_history(platform_version)
                .expect("expected the document_history system contract"),
        );

        let document_type = history_contract
            .document_type_for_name(history_document_type_name)
            .expect("expected the history document type");

        let request = DocumentCountRequest {
            contract: &history_contract,
            document_type,
            where_clauses,
            order_clauses: vec![],
            mode: CountMode::Aggregate,
            limit: None,
            prove: false,
            drive_config: &platform.config.drive,
            resolved_time_ranges: vec![],
        };

        match platform
            .drive
            .execute_document_count_request(request, None, platform_version)
            .expect("expected the count query to execute")
        {
            DocumentCountResponse::Aggregate(count) => count,
            other => panic!("expected an aggregate count, got {:?}", other),
        }
    }

    /// Provable aggregate sum of the `price` property over history documents
    /// matching the where clauses (whole doctype when empty).
    fn history_aggregate_price_sum(
        platform: &TempPlatform<MockCoreRPCLike>,
        history_document_type_name: &str,
        where_clauses: Vec<WhereClause>,
        platform_version: &PlatformVersion,
    ) -> i64 {
        let history_contract = Arc::clone(
            &platform
                .drive
                .cache
                .system_data_contracts
                .load_document_history(platform_version)
                .expect("expected the document_history system contract"),
        );

        let document_type = history_contract
            .document_type_for_name(history_document_type_name)
            .expect("expected the history document type");

        let request = DocumentSumRequest {
            contract: &history_contract,
            document_type,
            sum_property: "price".to_string(),
            where_clauses,
            order_clauses: vec![],
            mode: SumMode::Aggregate,
            limit: None,
            prove: false,
            drive_config: &platform.config.drive,
            resolved_time_ranges: vec![],
        };

        match platform
            .drive
            .execute_document_sum_request(request, None, platform_version)
            .expect("expected the sum query to execute")
        {
            DocumentSumResponse::Aggregate(sum) => sum,
            other => panic!("expected an aggregate sum, got {:?}", other),
        }
    }

    /// Provable `(count, sum)` average pair of the `price` property over
    /// history documents matching the where clauses.
    fn history_aggregate_price_average(
        platform: &TempPlatform<MockCoreRPCLike>,
        history_document_type_name: &str,
        where_clauses: Vec<WhereClause>,
        platform_version: &PlatformVersion,
    ) -> (u64, i64) {
        let history_contract = Arc::clone(
            &platform
                .drive
                .cache
                .system_data_contracts
                .load_document_history(platform_version)
                .expect("expected the document_history system contract"),
        );

        let document_type = history_contract
            .document_type_for_name(history_document_type_name)
            .expect("expected the history document type");

        let request = DocumentAverageRequest {
            contract: &history_contract,
            document_type,
            sum_property: "price".to_string(),
            where_clauses,
            order_clauses: vec![],
            mode: AverageMode::Aggregate,
            limit: None,
            prove: false,
            drive_config: &platform.config.drive,
            resolved_time_ranges: vec![],
        };

        match platform
            .drive
            .execute_document_average_request(request, None, platform_version)
            .expect("expected the average query to execute")
        {
            DocumentAverageResponse::Aggregate { count, sum } => (count, sum),
            other => panic!("expected an aggregate average, got {:?}", other),
        }
    }

    /// Queries the document history system contract for history documents of
    /// the given history document type recorded for a source document.
    fn query_history_documents(
        platform: &TempPlatform<MockCoreRPCLike>,
        history_document_type_name: &str,
        source_data_contract_id: Identifier,
        source_document_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Vec<Document> {
        let history_contract = Arc::clone(
            &platform
                .drive
                .cache
                .system_data_contracts
                .load_document_history(platform_version)
                .expect("expected the document_history system contract"),
        );

        let history_document_type = history_contract
            .document_type_for_name(history_document_type_name)
            .expect("expected the history document type");

        let drive_query = DriveDocumentQuery {
            contract: &history_contract,
            document_type: history_document_type,
            internal_clauses: InternalClauses {
                primary_key_in_clause: None,
                primary_key_equal_clause: None,
                in_clauses: Vec::new(),
                range_clause: None,
                equal_clauses: BTreeMap::from([
                    (
                        "dataContractId".to_string(),
                        WhereClause {
                            field: "dataContractId".to_string(),
                            operator: WhereOperator::Equal,
                            value: Value::Identifier(source_data_contract_id.to_buffer()),
                        },
                    ),
                    (
                        "documentId".to_string(),
                        WhereClause {
                            field: "documentId".to_string(),
                            operator: WhereOperator::Equal,
                            value: Value::Identifier(source_document_id.to_buffer()),
                        },
                    ),
                ]),
            },
            offset: None,
            limit: None,
            order_by: Default::default(),
            start_at: None,
            start_at_included: false,
            block_time_ms: None,
            resolved_time_ranges: vec![],
        };

        platform
            .drive
            .query_documents(drive_query, None, false, None, None)
            .expect("expected to query history documents")
            .documents_owned()
    }

    #[tokio::test]
    async fn test_dpns_username_transfer() {
        run_dpns_username_transfer_at_protocol_version(
            PlatformVersion::latest().protocol_version,
            true,
        )
        .await;
    }

    /// PROTOCOL_VERSION_12: domain transfers are still rejected by the
    /// `reject_data_trigger` binding. Pinned so v12 chain history stays
    /// bit-for-bit reproducible.
    #[tokio::test]
    async fn test_dpns_username_transfer_protocol_version_12_rejected() {
        run_dpns_username_transfer_at_protocol_version(12, false).await;
    }

    #[tokio::test]
    async fn test_v12_username_survives_retried_v13_transition_and_records_transfer() {
        let platform_version_12 = PlatformVersion::get(12).expect("expected platform version 12");
        let platform_version_13 = PlatformVersion::get(13).expect("expected platform version 13");
        let mut platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(12)
            .build_with_mock_rpc()
            .set_genesis_state();
        let mut rng = StdRng::seed_from_u64(438);

        let alice = setup_identity(&mut platform, 958, dash_to_credits!(0.5));
        let (bob, _, _) = setup_identity(&mut platform, 450, dash_to_credits!(0.5));
        let (document, _) = register_dpns_username(
            &mut platform,
            &alice,
            &mut rng,
            "upgrade7",
            platform_version_12,
        )
        .await;

        let committed_root_before = platform
            .drive
            .grove
            .root_hash(None, &platform_version_13.drive.grove_version)
            .unwrap()
            .expect("expected a committed root hash");
        let block_info = BlockInfo {
            time_ms: 1_000_000,
            height: 100,
            core_height: 100,
            epoch: dpp::block::epoch::Epoch::new(2).expect("expected epoch"),
        };

        let platform_state = platform.state.load();
        let rejected_transaction = platform.drive.grove.start_transaction();
        platform
            .perform_events_on_first_block_of_protocol_change(
                &platform_state,
                &block_info,
                &rejected_transaction,
                12,
                platform_version_13,
            )
            .expect("expected the speculative transition to succeed");
        drop(rejected_transaction);
        drop(platform_state);

        let committed_root_after_rejection = platform
            .drive
            .grove
            .root_hash(None, &platform_version_13.drive.grove_version)
            .unwrap()
            .expect("expected a committed root hash");
        assert_eq!(
            committed_root_before, committed_root_after_rejection,
            "dropping a rejected transition transaction must preserve committed state"
        );
        let cached_dpns_v12 = platform
            .drive
            .cache
            .system_data_contracts
            .find_by_id(SystemDataContract::DPNS.id(), platform_version_12)
            .expect("expected the DPNS lookup to succeed")
            .expect("explicit v12 lookup must survive a speculative v13 cache reload");
        let cached_v12_domain = cached_dpns_v12
            .document_type_for_name("domain")
            .expect("DPNS must contain its domain document type");
        assert!(!cached_v12_domain.documents_keep_transfer_history());
        assert!(!cached_v12_domain.documents_keep_purchase_history());
        assert!(!cached_v12_domain.documents_keep_pricing_history());
        assert_eq!(
            query_domain_documents_by_record_identity(&platform, &cached_dpns_v12, alice.0.id())
                .len(),
            1,
            "the v12 username must remain after a rejected transition"
        );

        let platform_state = platform.state.load();
        let retry_transaction = platform.drive.grove.start_transaction();
        platform
            .perform_events_on_first_block_of_protocol_change(
                &platform_state,
                &block_info,
                &retry_transaction,
                12,
                platform_version_13,
            )
            .expect("expected the retried transition to succeed");
        platform
            .drive
            .grove
            .commit_transaction(retry_transaction)
            .unwrap()
            .expect("expected to commit the retried transition");
        drop(platform_state);

        let committed_root_after_transition = platform
            .drive
            .grove
            .root_hash(None, &platform_version_13.drive.grove_version)
            .unwrap()
            .expect("expected a committed root hash");
        assert_ne!(
            committed_root_before, committed_root_after_transition,
            "the committed transition must install the v13 contracts"
        );

        let mut clean_control = TestPlatformBuilder::new()
            .with_initial_protocol_version(12)
            .build_with_mock_rpc()
            .set_genesis_state();
        let mut clean_control_rng = StdRng::seed_from_u64(438);
        let clean_control_alice = setup_identity(&mut clean_control, 958, dash_to_credits!(0.5));
        setup_identity(&mut clean_control, 450, dash_to_credits!(0.5));
        register_dpns_username(
            &mut clean_control,
            &clean_control_alice,
            &mut clean_control_rng,
            "upgrade7",
            platform_version_12,
        )
        .await;
        let clean_control_root_before = clean_control
            .drive
            .grove
            .root_hash(None, &platform_version_13.drive.grove_version)
            .unwrap()
            .expect("expected a clean-control committed root hash");
        assert_eq!(
            committed_root_before, clean_control_root_before,
            "the clean control must begin from the same populated v12 state"
        );
        let clean_control_state = clean_control.state.load();
        let clean_control_transaction = clean_control.drive.grove.start_transaction();
        clean_control
            .perform_events_on_first_block_of_protocol_change(
                &clean_control_state,
                &block_info,
                &clean_control_transaction,
                12,
                platform_version_13,
            )
            .expect("expected the clean-control transition to succeed");
        clean_control
            .drive
            .grove
            .commit_transaction(clean_control_transaction)
            .unwrap()
            .expect("expected to commit the clean-control transition");
        drop(clean_control_state);
        let clean_control_root_after_transition = clean_control
            .drive
            .grove
            .root_hash(None, &platform_version_13.drive.grove_version)
            .unwrap()
            .expect("expected the clean-control transition root hash");
        assert_eq!(
            committed_root_after_transition, clean_control_root_after_transition,
            "retry after a rejected candidate must match a clean transition from the same v12 state"
        );

        let dpns_contract_v13 = platform
            .drive
            .cache
            .system_data_contracts
            .load_dpns(platform_version_13)
            .expect("expected the DPNS system contract");
        let mut alice_documents =
            query_domain_documents_by_record_identity(&platform, &dpns_contract_v13, alice.0.id());
        assert_eq!(
            alice_documents.len(),
            1,
            "the v12 username must survive the committed transition"
        );
        let mut document_after_upgrade = alice_documents
            .pop()
            .expect("expected the pre-upgrade username");
        assert_eq!(document_after_upgrade.id(), document.id());

        let mut upgraded_state = platform.state.load().as_ref().clone();
        upgraded_state.set_current_protocol_version_in_consensus(13);
        upgraded_state.set_next_epoch_protocol_version(13);
        platform.state.store(Arc::new(upgraded_state));

        let domain = dpns_contract_v13
            .document_type_for_name("domain")
            .expect("expected the domain document type");
        document_after_upgrade.set_revision(Some(2));
        let (alice_identity, signer, key) = &alice;
        let transfer = BatchTransition::new_document_transfer_transition_from_document(
            document_after_upgrade,
            domain,
            bob.id(),
            key,
            4,
            0,
            None,
            signer,
            platform_version_13,
            None,
        )
        .await
        .expect("expected to create the post-upgrade transfer");
        let serialized_transfer = transfer
            .serialize_to_bytes()
            .expect("expected to serialize the post-upgrade transfer");

        let platform_state = platform.state.load();
        let transfer_transaction = platform.drive.grove.start_transaction();
        let processing_result = platform
            .process_raw_state_transitions(
                &[serialized_transfer],
                &platform_state,
                &BlockInfo {
                    time_ms: block_info.time_ms + 3_000,
                    height: block_info.height + 1,
                    core_height: block_info.core_height,
                    epoch: block_info.epoch,
                },
                &transfer_transaction,
                platform_version_13,
                false,
                None,
            )
            .expect("expected to process the post-upgrade transfer");
        platform
            .drive
            .grove
            .commit_transaction(transfer_transaction)
            .unwrap()
            .expect("expected to commit the post-upgrade transfer");

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );

        let bob_documents =
            query_domain_documents_by_record_identity(&platform, &dpns_contract_v13, bob.id());
        assert_eq!(bob_documents.len(), 1);
        let transferred_document = bob_documents
            .first()
            .expect("expected transferred username");
        assert_eq!(transferred_document.id(), document.id());
        assert_eq!(transferred_document.owner_id(), bob.id());
        assert_identity_record(transferred_document, bob.id());

        let transfer_history = query_history_documents(
            &platform,
            "transfer",
            dpns_contract_v13.id(),
            transferred_document.id(),
            platform_version_13,
        );
        assert_eq!(transfer_history.len(), 1);
        assert_eq!(transfer_history[0].owner_id(), alice_identity.id());
        assert_eq!(
            transfer_history[0]
                .properties()
                .get_identifier("toIdentityId")
                .expect("expected transfer recipient"),
            bob.id()
        );
    }

    /// The block info the protocol version 13 activation is run at in the cache-coherence
    /// tests below.
    fn v13_activation_block_info() -> BlockInfo {
        BlockInfo {
            time_ms: 1_000_000,
            height: 100,
            core_height: 100,
            epoch: dpp::block::epoch::Epoch::new(2).expect("expected epoch"),
        }
    }

    /// Registers one DPNS username at protocol version 12, reproducibly, so that two
    /// instances reach the same committed state.
    async fn populated_v12_platform(
        platform_version_12: &PlatformVersion,
    ) -> (
        TempPlatform<MockCoreRPCLike>,
        (Identity, SimpleSigner, IdentityPublicKey),
        Identity,
        Document,
    ) {
        let mut platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(12)
            .build_with_mock_rpc()
            .set_genesis_state();
        let mut rng = StdRng::seed_from_u64(781);

        let alice = setup_identity(&mut platform, 958, dash_to_credits!(0.5));
        let (bob, _, _) = setup_identity(&mut platform, 450, dash_to_credits!(0.5));
        let (document, _) = register_dpns_username(
            &mut platform,
            &alice,
            &mut rng,
            "stalecache7",
            platform_version_12,
        )
        .await;

        (platform, alice, bob, document)
    }

    /// The transfer/purchase/pricing history flags the DPNS `domain` document type gains
    /// at protocol version 13.
    fn domain_history_flags(dpns: &DataContract) -> (bool, bool, bool) {
        let domain = dpns
            .document_type_for_name("domain")
            .expect("DPNS must contain its domain document type");

        (
            domain.documents_keep_transfer_history(),
            domain.documents_keep_purchase_history(),
            domain.documents_keep_pricing_history(),
        )
    }

    fn committed_root(
        platform: &TempPlatform<MockCoreRPCLike>,
        platform_version: &PlatformVersion,
    ) -> [u8; 32] {
        platform
            .drive
            .grove
            .root_hash(None, &platform_version.drive.grove_version)
            .unwrap()
            .expect("expected a committed root hash")
    }

    /// Puts the pre-activation DPNS definition in the global contract cache, which is what a
    /// validator that has served any DPNS read before the upgrade is carrying.
    fn warm_the_dpns_contract_cache(
        platform: &TempPlatform<MockCoreRPCLike>,
        platform_version: &PlatformVersion,
    ) {
        platform
            .drive
            .get_contract_with_fetch_info(
                SystemDataContract::DPNS.id().to_buffer(),
                true,
                None,
                platform_version,
            )
            .expect("expected to warm the DPNS contract cache")
            .expect("DPNS must be present in the state");
    }

    /// Resolves DPNS exactly as the batch document transformer does: through the ordinary
    /// data contract cache, falling back to the state.
    fn dpns_as_the_document_transformer_sees_it(
        platform: &TempPlatform<MockCoreRPCLike>,
        platform_version: &PlatformVersion,
    ) -> Arc<DataContract> {
        let (_fee, fetch_info) = platform
            .drive
            .get_contract_with_fetch_info_and_fee(
                SystemDataContract::DPNS.id().to_buffer(),
                None,
                false,
                None,
                platform_version,
            )
            .expect("expected to resolve the DPNS contract");

        Arc::new(
            fetch_info
                .expect("DPNS must be present in the state")
                .contract
                .clone(),
        )
    }

    /// Runs the protocol version 13 activation and either commits it or drops the transaction,
    /// standing in for a candidate block that is accepted or rejected.
    fn run_v13_activation(
        platform: &TempPlatform<MockCoreRPCLike>,
        block_info: &BlockInfo,
        platform_version_13: &PlatformVersion,
        commit: bool,
    ) {
        let platform_state = platform.state.load();
        let transaction = platform.drive.grove.start_transaction();
        platform
            .perform_events_on_first_block_of_protocol_change(
                &platform_state,
                block_info,
                &transaction,
                12,
                platform_version_13,
            )
            .expect("expected the transition to protocol version 13 to succeed");

        if !commit {
            drop(transaction);
            drop(platform_state);
            return;
        }

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit the transition to protocol version 13");
        drop(platform_state);

        let mut upgraded_state = platform.state.load().as_ref().clone();
        upgraded_state.set_current_protocol_version_in_consensus(13);
        upgraded_state.set_next_epoch_protocol_version(13);
        platform.state.store(Arc::new(upgraded_state));
    }

    fn activate_protocol_version_13(
        platform: &TempPlatform<MockCoreRPCLike>,
        block_info: &BlockInfo,
        platform_version_13: &PlatformVersion,
    ) {
        run_v13_activation(platform, block_info, platform_version_13, true);
    }

    /// A read-only query served from committed state while the activation block is still
    /// executing must not divert that block onto the pre-activation DPNS.
    ///
    /// The ordinary contract cache is keyed by contract id alone, and a transactional lookup
    /// falls back to the global cache when the block cache misses. Query threads read committed
    /// state with no transaction and populate that global cache. So merely evicting the stale
    /// entry leaves a window in which a query puts it straight back and the rest of the block
    /// serializes documents against it — a divergence decided by query timing rather than by
    /// state. The activation seeds the block cache precisely so transactional reads have an
    /// authority they reach first.
    #[tokio::test]
    async fn concurrent_committed_query_must_not_divert_the_activation_block_from_migrated_dpns() {
        let platform_version_12 = PlatformVersion::get(12).expect("expected platform version 12");
        let platform_version_13 = PlatformVersion::get(13).expect("expected platform version 13");
        let dpns_id = SystemDataContract::DPNS.id().to_buffer();

        let (platform, _alice, _bob, _document) = populated_v12_platform(platform_version_12).await;

        // A long-lived node: DPNS has been read at least once, so the pre-activation
        // definition sits in the global cache.
        warm_the_dpns_contract_cache(&platform, platform_version_12);

        let platform_state = platform.state.load();
        let transaction = platform.drive.grove.start_transaction();

        // The order the real block flow uses: the block cache is cleared for the proposal,
        // then the protocol change events run and seed it.
        platform
            .clear_drive_block_cache(platform_version_13)
            .expect("expected to clear the block cache");
        platform
            .perform_events_on_first_block_of_protocol_change(
                &platform_state,
                &v13_activation_block_info(),
                &transaction,
                12,
                platform_version_13,
            )
            .expect("expected the activation to succeed");

        // The interleaving: a DAPI query resolves DPNS from committed state and publishes the
        // pre-activation definition into the global cache.
        platform
            .drive
            .get_contract_with_fetch_info(dpns_id, true, None, platform_version_12)
            .expect("expected the query to resolve DPNS")
            .expect("DPNS must be present in committed state");

        // The batch document transformer's read, with the block transaction.
        let (_fee, fetch_info) = platform
            .drive
            .get_contract_with_fetch_info_and_fee(
                dpns_id,
                None,
                false,
                Some(&transaction),
                platform_version_13,
            )
            .expect("expected the transformer read to succeed");
        let resolved = fetch_info.expect("DPNS must be present");

        assert_eq!(
            (
                resolved.contract.version(),
                domain_history_flags(&resolved.contract)
            ),
            (2, (true, true, true)),
            "the activation block must serialize against the definition it installed, whatever \
             a concurrent query cached"
        );
    }

    /// A candidate activation that is never committed must leave a warm node resolving the
    /// still-committed protocol version 12 definition, and the retry that does commit must
    /// still retire it. Pins that the fix drops the stale entry rather than publishing the
    /// candidate definition into the cache, which a rejected block would then be unable to
    /// take back.
    #[tokio::test]
    async fn warm_dpns_contract_cache_survives_a_rejected_v13_activation_and_its_retry() {
        let platform_version_12 = PlatformVersion::get(12).expect("expected platform version 12");
        let platform_version_13 = PlatformVersion::get(13).expect("expected platform version 13");
        let activation_block_info = v13_activation_block_info();

        let (platform, _alice, _bob, _document) = populated_v12_platform(platform_version_12).await;
        let (control, _control_alice, _control_bob, _control_document) =
            populated_v12_platform(platform_version_12).await;

        warm_the_dpns_contract_cache(&platform, platform_version_12);
        let root_before = committed_root(&platform, platform_version_13);

        // Rejected candidate: the activation runs, then its transaction is dropped.
        run_v13_activation(
            &platform,
            &activation_block_info,
            platform_version_13,
            false,
        );

        assert_eq!(
            committed_root(&platform, platform_version_13),
            root_before,
            "a rejected candidate must not change committed state"
        );
        assert_eq!(
            domain_history_flags(&dpns_as_the_document_transformer_sees_it(
                &platform,
                platform_version_12,
            )),
            (false, false, false),
            "a rejected candidate must leave the node resolving the committed v12 definition"
        );

        // Retry, this time committed, and compare against a node that never saw the rejection.
        activate_protocol_version_13(&platform, &activation_block_info, platform_version_13);
        activate_protocol_version_13(&control, &activation_block_info, platform_version_13);

        let retried = dpns_as_the_document_transformer_sees_it(&platform, platform_version_13);
        let clean = dpns_as_the_document_transformer_sees_it(&control, platform_version_13);
        assert_eq!(
            (retried.version(), domain_history_flags(&retried)),
            (2, (true, true, true)),
            "the committed retry must retire the pre-upgrade DPNS definition"
        );
        assert_eq!(
            (retried.version(), domain_history_flags(&retried)),
            (clean.version(), domain_history_flags(&clean)),
            "a retried activation must resolve what a clean one resolves"
        );
        assert_eq!(
            committed_root(&platform, platform_version_13),
            committed_root(&control, platform_version_13),
            "a retried activation must commit the same state as a clean one"
        );
    }

    /// The v13 activation replaces the persisted DPNS definition with schema v2. The batch
    /// document transformer resolves DPNS through the ordinary data contract cache, which the
    /// direct `apply_contract` used by the migration does not invalidate — so a validator that
    /// had warmed that cache before the activation would keep serializing domain documents
    /// against the pre-upgrade definition while a validator that restarted reads the new one
    /// from the state. The two would write different bytes for the same transition and diverge
    /// on the app hash.
    ///
    /// Two platforms are driven into byte-identical populated v12 state, the activation is
    /// committed on both, and the only difference between them is that one had DPNS warmed into
    /// its contract cache beforehand.
    #[tokio::test]
    async fn warm_dpns_contract_cache_must_not_survive_the_committed_v13_activation() {
        let platform_version_12 = PlatformVersion::get(12).expect("expected platform version 12");
        let platform_version_13 = PlatformVersion::get(13).expect("expected platform version 13");
        let activation_block_info = v13_activation_block_info();

        let (warm_platform, warm_alice, warm_bob, warm_document) =
            populated_v12_platform(platform_version_12).await;
        let (cold_platform, cold_alice, cold_bob, cold_document) =
            populated_v12_platform(platform_version_12).await;
        assert_eq!(
            committed_root(&warm_platform, platform_version_13),
            committed_root(&cold_platform, platform_version_13),
            "both platforms must start from the same populated v12 state"
        );

        // The only difference between the two nodes: this one served a DPNS read before the
        // activation, which is what puts the pre-upgrade definition in the global cache.
        warm_the_dpns_contract_cache(&warm_platform, platform_version_12);
        assert_eq!(
            domain_history_flags(&dpns_as_the_document_transformer_sees_it(
                &warm_platform,
                platform_version_12,
            )),
            (false, false, false),
            "the warmed cache must hold the pre-activation DPNS definition"
        );

        activate_protocol_version_13(&warm_platform, &activation_block_info, platform_version_13);
        activate_protocol_version_13(&cold_platform, &activation_block_info, platform_version_13);
        assert_eq!(
            committed_root(&warm_platform, platform_version_13),
            committed_root(&cold_platform, platform_version_13),
            "the activation must commit the same state on both platforms"
        );

        let warm_dpns =
            dpns_as_the_document_transformer_sees_it(&warm_platform, platform_version_13);
        let cold_dpns =
            dpns_as_the_document_transformer_sees_it(&cold_platform, platform_version_13);
        assert_eq!(
            (warm_dpns.version(), domain_history_flags(&warm_dpns)),
            (cold_dpns.version(), domain_history_flags(&cold_dpns)),
            "a warm and a cold node must resolve the same DPNS definition after the activation"
        );
        assert_eq!(
            (warm_dpns.version(), domain_history_flags(&warm_dpns)),
            (2, (true, true, true)),
            "the committed activation must retire the pre-upgrade DPNS definition"
        );
        assert_eq!(
            warm_dpns, cold_dpns,
            "the resolved definitions must be identical in every field"
        );

        // Same transition on both nodes: it may only be accepted, and its committed effect must
        // be identical, because the document is serialized against the contract resolved above.
        for (platform, alice, bob, document, dpns) in [
            (
                &warm_platform,
                &warm_alice,
                &warm_bob,
                warm_document,
                &warm_dpns,
            ),
            (
                &cold_platform,
                &cold_alice,
                &cold_bob,
                cold_document,
                &cold_dpns,
            ),
        ] {
            let domain = dpns
                .document_type_for_name("domain")
                .expect("DPNS must contain its domain document type");
            let (_alice_identity, signer, key) = alice;
            let mut document = document;
            document.set_revision(Some(2));

            let transfer = BatchTransition::new_document_transfer_transition_from_document(
                document,
                domain,
                bob.id(),
                key,
                4,
                0,
                None,
                signer,
                platform_version_13,
                None,
            )
            .await
            .expect("expected to create the post-upgrade transfer");
            let serialized_transfer = transfer
                .serialize_to_bytes()
                .expect("expected to serialize the post-upgrade transfer");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();
            let processing_result = platform
                .process_raw_state_transitions(
                    &[serialized_transfer],
                    &platform_state,
                    &BlockInfo {
                        time_ms: activation_block_info.time_ms + 3_000,
                        height: activation_block_info.height + 1,
                        core_height: activation_block_info.core_height,
                        epoch: activation_block_info.epoch,
                    },
                    &transaction,
                    platform_version_13,
                    false,
                    None,
                )
                .expect("expected to process the post-upgrade transfer");
            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit the post-upgrade transfer");
            drop(platform_state);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution { .. }]
            );
        }

        assert_eq!(
            committed_root(&warm_platform, platform_version_13),
            committed_root(&cold_platform, platform_version_13),
            "a warm and a cold node must commit the same root for the same post-upgrade transition"
        );
    }

    async fn run_dpns_username_transfer_at_protocol_version(
        protocol_version: ProtocolVersion,
        expect_allowed: bool,
    ) {
        let mut platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(protocol_version)
            .build_with_mock_rpc()
            .set_genesis_state();

        let platform_state = platform.state.load();
        let platform_version = platform_state
            .current_platform_version()
            .expect("expected to get current platform version");

        let mut rng = StdRng::seed_from_u64(437);

        let alice = setup_identity(&mut platform, 958, dash_to_credits!(0.5));

        let (bob, _, _) = setup_identity(&mut platform, 450, dash_to_credits!(0.5));

        // "quantum7" contains a digit outside 0/1 so the name is not contested
        let (mut document, dpns_contract) = register_dpns_username(
            &mut platform,
            &alice,
            &mut rng,
            "quantum7",
            platform_version,
        )
        .await;

        let (alice, signer, key) = &alice;

        let domain = dpns_contract
            .document_type_for_name("domain")
            .expect("expected the domain document type");

        document.set_revision(Some(2));

        let documents_batch_transfer_transition =
            BatchTransition::new_document_transfer_transition_from_document(
                document,
                domain,
                bob.id(),
                key,
                4,
                0,
                None,
                signer,
                platform_version,
                None,
            )
            .await
            .expect("expect to create documents batch transition for transfer");

        let documents_batch_transfer_serialized_transition = documents_batch_transfer_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[documents_batch_transfer_serialized_transition],
                &platform_state,
                &BlockInfo::default_with_time(
                    platform_state
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

        if expect_allowed {
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution { .. }]
            );

            // The username must now belong to and resolve to Bob
            let bob_documents =
                query_domain_documents_by_record_identity(&platform, &dpns_contract, bob.id());

            assert_eq!(bob_documents.len(), 1);

            let transferred_document = bob_documents.first().expect("expected a document");

            assert_eq!(transferred_document.owner_id(), bob.id());

            assert_identity_record(transferred_document, bob.id());

            // Alice must no longer have a name record pointing to her
            let alice_documents =
                query_domain_documents_by_record_identity(&platform, &dpns_contract, alice.id());

            assert_eq!(alice_documents.len(), 0);

            // The transfer must be recorded in the document history contract,
            // owned by the sender and pointing at the recipient
            let history_documents = query_history_documents(
                &platform,
                "transfer",
                dpns_contract.id(),
                transferred_document.id(),
                platform_version,
            );

            assert_eq!(history_documents.len(), 1);

            let history_document = history_documents
                .first()
                .expect("expected a history document");

            assert_eq!(history_document.owner_id(), alice.id());

            assert_eq!(
                history_document
                    .properties()
                    .get_identifier("toIdentityId")
                    .expect("expected the recipient on the transfer record"),
                bob.id()
            );

            assert!(history_document.created_at().is_some());

            // Provable transfer counts: doctype-wide and per-contract in a
            // time window
            assert_eq!(
                history_aggregate_count(&platform, "transfer", vec![], platform_version),
                1
            );

            assert_eq!(
                history_aggregate_count(
                    &platform,
                    "transfer",
                    vec![
                        history_where_by_contract(dpns_contract.id()),
                        history_where_created_between(0, 9_999_999_999_999),
                    ],
                    platform_version,
                ),
                1
            );

            assert_eq!(
                history_aggregate_count(
                    &platform,
                    "transfer",
                    vec![
                        history_where_by_contract(dpns_contract.id()),
                        history_where_created_between(1, 2),
                    ],
                    platform_version,
                ),
                0
            );
        } else {
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::PaidConsensusError {
                    error: ConsensusError::StateError(StateError::DataTriggerError(
                        DataTriggerError::DataTriggerConditionError(_)
                    )),
                    ..
                }]
            );

            // The username must still belong to and resolve to Alice
            let alice_documents =
                query_domain_documents_by_record_identity(&platform, &dpns_contract, alice.id());

            assert_eq!(alice_documents.len(), 1);

            let kept_document = alice_documents.first().expect("expected a document");

            assert_eq!(kept_document.owner_id(), alice.id());

            // The document history contract itself must not even exist yet
            assert_document_history_contract_absent(&platform, platform_version);
        }

        let issues = platform
            .drive
            .grove
            .visualize_verify_grovedb(None, true, false, &platform_version.drive.grove_version)
            .expect("expected to have no issues");

        assert_eq!(
            issues.len(),
            0,
            "issues are {}",
            issues
                .iter()
                .map(|(hash, (a, b, c))| format!("{}: {} {} {}", hash, a, b, c))
                .collect::<Vec<_>>()
                .join(" | ")
        );
    }

    #[tokio::test]
    async fn test_dpns_username_sale() {
        run_dpns_username_sale_at_protocol_version(
            PlatformVersion::latest().protocol_version,
            true,
        )
        .await;
    }

    /// PROTOCOL_VERSION_12: setting a price on a domain document is still
    /// rejected by the `reject_data_trigger` binding, so a sale can not even
    /// be started. Pinned so v12 chain history stays bit-for-bit reproducible.
    #[tokio::test]
    async fn test_dpns_username_sale_protocol_version_12_rejected() {
        run_dpns_username_sale_at_protocol_version(12, false).await;
    }

    async fn run_dpns_username_sale_at_protocol_version(
        protocol_version: ProtocolVersion,
        expect_allowed: bool,
    ) {
        let mut platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(protocol_version)
            .build_with_mock_rpc()
            .set_genesis_state();

        let platform_state = platform.state.load();
        let platform_version = platform_state
            .current_platform_version()
            .expect("expected to get current platform version");

        let mut rng = StdRng::seed_from_u64(438);

        let alice = setup_identity(&mut platform, 959, dash_to_credits!(0.5));

        let (bob, bob_signer, bob_key) = setup_identity(&mut platform, 451, dash_to_credits!(0.5));

        let (mut document, dpns_contract) = register_dpns_username(
            &mut platform,
            &alice,
            &mut rng,
            "quantum9",
            platform_version,
        )
        .await;

        let (alice, signer, key) = &alice;

        let domain = dpns_contract
            .document_type_for_name("domain")
            .expect("expected the domain document type");

        document.set_revision(Some(2));

        let documents_batch_update_price_transition =
            BatchTransition::new_document_update_price_transition_from_document(
                document,
                domain,
                dash_to_credits!(0.1),
                key,
                4,
                0,
                None,
                signer,
                platform_version,
                None,
            )
            .await
            .expect("expect to create documents batch transition for the price update");

        let documents_batch_update_price_serialized_transition =
            documents_batch_update_price_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[documents_batch_update_price_serialized_transition],
                &platform_state,
                &BlockInfo::default_with_time(
                    platform_state
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

        if !expect_allowed {
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::PaidConsensusError {
                    error: ConsensusError::StateError(StateError::DataTriggerError(
                        DataTriggerError::DataTriggerConditionError(_)
                    )),
                    ..
                }]
            );

            // The domain must still belong to Alice, keep its records.identity
            // pointing at her, and have no $price set
            let alice_documents =
                query_domain_documents_by_record_identity(&platform, &dpns_contract, alice.id());

            assert_eq!(alice_documents.len(), 1);

            let kept_document = alice_documents.first().expect("expected a document");

            assert_eq!(kept_document.owner_id(), alice.id());

            assert_identity_record(kept_document, alice.id());

            assert!(kept_document.properties().get("$price").is_none());

            // The document history contract itself must not even exist yet
            assert_document_history_contract_absent(&platform, platform_version);

            return;
        }

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );

        // Read the listed document back so the purchase is built on the
        // priced revision
        let mut listed_documents =
            query_domain_documents_by_record_identity(&platform, &dpns_contract, alice.id());

        assert_eq!(listed_documents.len(), 1);

        let mut document = listed_documents.remove(0);

        let price: Credits = document
            .properties()
            .get_integer("$price")
            .expect("expected to get back the price");

        assert_eq!(price, dash_to_credits!(0.1));

        // The listing must be recorded in the document history contract
        let pricing_history = query_history_documents(
            &platform,
            "priceUpdate",
            dpns_contract.id(),
            document.id(),
            platform_version,
        );

        assert_eq!(pricing_history.len(), 1);

        let pricing_document = pricing_history.first().expect("expected a pricing record");

        assert_eq!(pricing_document.owner_id(), alice.id());

        assert_eq!(
            pricing_document
                .properties()
                .get_integer::<Credits>("price")
                .expect("expected the price on the pricing record"),
            dash_to_credits!(0.1)
        );

        let alice_balance_before_sale = platform
            .drive
            .fetch_identity_balance(alice.id().to_buffer(), None, platform_version)
            .expect("expected to get alice's balance")
            .expect("expected alice's identity to exist");

        let bob_balance_before_purchase = platform
            .drive
            .fetch_identity_balance(bob.id().to_buffer(), None, platform_version)
            .expect("expected to get bob's balance")
            .expect("expected bob's identity to exist");

        document.set_revision(Some(3));

        let documents_batch_purchase_transition =
            BatchTransition::new_document_purchase_transition_from_document(
                document,
                domain,
                bob.id(),
                dash_to_credits!(0.1),
                &bob_key,
                1,
                0,
                None,
                &bob_signer,
                platform_version,
                None,
            )
            .await
            .expect("expect to create documents batch transition for the purchase");

        let documents_batch_purchase_serialized_transition = documents_batch_purchase_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[documents_batch_purchase_serialized_transition],
                &platform_state,
                &BlockInfo::default_with_time(
                    platform_state
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

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );

        // The username must now belong to and resolve to Bob, and it must no
        // longer be for sale
        let bob_documents =
            query_domain_documents_by_record_identity(&platform, &dpns_contract, bob.id());

        assert_eq!(bob_documents.len(), 1);

        let purchased_document = bob_documents.first().expect("expected a document");

        assert_eq!(purchased_document.owner_id(), bob.id());

        assert_identity_record(purchased_document, bob.id());

        assert!(purchased_document.properties().get("$price").is_none());

        let alice_documents =
            query_domain_documents_by_record_identity(&platform, &dpns_contract, alice.id());

        assert_eq!(alice_documents.len(), 0);

        // Alice pays no fees on Bob's purchase transition, so her balance must
        // grow by exactly the sale price plus the refund of the document
        // storage she originally paid for: the purchased revision is stored
        // under Bob's storage flags, releasing the bytes Alice had prepaid
        let alice_storage_refund: Credits = 7711268;

        let alice_balance_after_sale = platform
            .drive
            .fetch_identity_balance(alice.id().to_buffer(), None, platform_version)
            .expect("expected to get alice's balance")
            .expect("expected alice's identity to exist");

        assert_eq!(
            alice_balance_after_sale,
            alice_balance_before_sale + dash_to_credits!(0.1) + alice_storage_refund
        );

        // Bob pays the sale price plus processing and storage fees
        let bob_balance_after_purchase = platform
            .drive
            .fetch_identity_balance(bob.id().to_buffer(), None, platform_version)
            .expect("expected to get bob's balance")
            .expect("expected bob's identity to exist");

        assert!(bob_balance_after_purchase < bob_balance_before_purchase - dash_to_credits!(0.1));

        // The sale must be recorded in the document history contract, owned
        // by the buyer, naming the seller and the price paid
        let purchase_history = query_history_documents(
            &platform,
            "purchase",
            dpns_contract.id(),
            purchased_document.id(),
            platform_version,
        );

        assert_eq!(purchase_history.len(), 1);

        let purchase_document = purchase_history
            .first()
            .expect("expected a purchase record");

        assert_eq!(purchase_document.owner_id(), bob.id());

        assert_eq!(
            purchase_document
                .properties()
                .get_identifier("sellerId")
                .expect("expected the seller on the purchase record"),
            alice.id()
        );

        assert_eq!(
            purchase_document
                .properties()
                .get_integer::<Credits>("price")
                .expect("expected the price on the purchase record"),
            dash_to_credits!(0.1)
        );

        assert!(purchase_document.created_at().is_some());

        // A sale is not a plain transfer: no transfer record may exist
        let transfer_history = query_history_documents(
            &platform,
            "transfer",
            dpns_contract.id(),
            purchased_document.id(),
            platform_version,
        );

        assert_eq!(transfer_history.len(), 0);

        // Provable marketplace aggregates: exactly one sale for 0.1 dash was
        // recorded, visible as O(1) doctype-wide count and volume
        assert_eq!(
            history_aggregate_count(&platform, "purchase", vec![], platform_version),
            1
        );

        assert_eq!(
            history_aggregate_price_sum(&platform, "purchase", vec![], platform_version),
            dash_to_credits!(0.1) as i64
        );

        // Time-range aggregates on the byContract index: the sale falls in a
        // wide window and average sale price = sum / count
        let (window_count, window_sum) = history_aggregate_price_average(
            &platform,
            "purchase",
            vec![
                history_where_by_contract(dpns_contract.id()),
                history_where_created_between(0, 9_999_999_999_999),
            ],
            platform_version,
        );

        assert_eq!(
            (window_count, window_sum),
            (1, dash_to_credits!(0.1) as i64)
        );

        // ... and an empty window contains no sales
        let (empty_count, empty_sum) = history_aggregate_price_average(
            &platform,
            "purchase",
            vec![
                history_where_by_contract(dpns_contract.id()),
                history_where_created_between(1, 2),
            ],
            platform_version,
        );

        assert_eq!((empty_count, empty_sum), (0, 0));

        // The listing is aggregated the same way: average asking price
        // between dates over priceUpdate records
        let (listing_count, listing_sum) = history_aggregate_price_average(
            &platform,
            "priceUpdate",
            vec![
                history_where_by_contract(dpns_contract.id()),
                history_where_created_between(0, 9_999_999_999_999),
            ],
            platform_version,
        );

        assert_eq!(
            (listing_count, listing_sum),
            (1, dash_to_credits!(0.1) as i64)
        );

        let issues = platform
            .drive
            .grove
            .visualize_verify_grovedb(None, true, false, &platform_version.drive.grove_version)
            .expect("expected to have no issues");

        assert_eq!(
            issues.len(),
            0,
            "issues are {}",
            issues
                .iter()
                .map(|(hash, (a, b, c))| format!("{}: {} {} {}", hash, a, b, c))
                .collect::<Vec<_>>()
                .join(" | ")
        );
    }

    /// The v1 data-trigger bindings list drops the rejects for `Transfer`,
    /// `Purchase` and `UpdatePrice`, but `Replace` and `Delete` must stay
    /// rejected: name records are immutable and permanent even though they can
    /// now change hands.
    ///
    /// `Replace` is stopped before data triggers run because the domain
    /// document type is not mutable, but `Delete` is stopped only by the
    /// reject data trigger (the domain document type sets `canBeDeleted:
    /// true`), so this pins the single guard protecting names from deletion.
    #[tokio::test]
    async fn test_dpns_username_replace_and_delete_still_rejected() {
        let mut platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(PlatformVersion::latest().protocol_version)
            .build_with_mock_rpc()
            .set_genesis_state();

        let platform_state = platform.state.load();
        let platform_version = platform_state
            .current_platform_version()
            .expect("expected to get current platform version");

        let mut rng = StdRng::seed_from_u64(439);

        let alice = setup_identity(&mut platform, 960, dash_to_credits!(0.5));

        let (mut document, dpns_contract) = register_dpns_username(
            &mut platform,
            &alice,
            &mut rng,
            "quantum8",
            platform_version,
        )
        .await;

        let (alice, signer, key) = &alice;

        let domain = dpns_contract
            .document_type_for_name("domain")
            .expect("expected the domain document type");

        document.set_revision(Some(2));

        let documents_batch_replace_transition =
            BatchTransition::new_document_replacement_transition_from_document(
                document.clone(),
                domain,
                key,
                4,
                0,
                None,
                signer,
                platform_version,
                None,
            )
            .await
            .expect("expect to create documents batch transition for the replacement");

        let documents_batch_delete_transition =
            BatchTransition::new_document_deletion_transition_from_document(
                document,
                domain,
                key,
                5,
                0,
                None,
                signer,
                platform_version,
                None,
            )
            .await
            .expect("expect to create documents batch transition for the deletion");

        for (transition, expect_data_trigger_reject) in [
            (documents_batch_replace_transition, false),
            (documents_batch_delete_transition, true),
        ] {
            let serialized_transition = transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[serialized_transition],
                    &platform_state,
                    &BlockInfo::default_with_time(
                        platform_state
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

            if expect_data_trigger_reject {
                assert_matches!(
                    processing_result.execution_results().as_slice(),
                    [StateTransitionExecutionResult::PaidConsensusError {
                        error: ConsensusError::StateError(StateError::DataTriggerError(
                            DataTriggerError::DataTriggerConditionError(_)
                        )),
                        ..
                    }]
                );
            } else {
                assert_matches!(
                    processing_result.execution_results().as_slice(),
                    [StateTransitionExecutionResult::PaidConsensusError {
                        error: ConsensusError::BasicError(
                            BasicError::InvalidDocumentTransitionActionError(_)
                        ),
                        ..
                    }]
                );
            }

            // The username must still belong to and resolve to Alice
            let alice_documents =
                query_domain_documents_by_record_identity(&platform, &dpns_contract, alice.id());

            assert_eq!(alice_documents.len(), 1);

            let kept_document = alice_documents.first().expect("expected a document");

            assert_eq!(kept_document.owner_id(), alice.id());

            assert_identity_record(kept_document, alice.id());
        }

        let issues = platform
            .drive
            .grove
            .visualize_verify_grovedb(None, true, false, &platform_version.drive.grove_version)
            .expect("expected to have no issues");

        assert_eq!(
            issues.len(),
            0,
            "issues are {}",
            issues
                .iter()
                .map(|(hash, (a, b, c))| format!("{}: {} {} {}", hash, a, b, c))
                .collect::<Vec<_>>()
                .join(" | ")
        );
    }

    /// Transfers of a document type that did NOT subscribe to history must
    /// not write anything into the document history contract, even at
    /// PROTOCOL_VERSION_13.
    #[tokio::test]
    async fn test_document_transfer_without_history_subscription_records_nothing() {
        let platform_version = PlatformVersion::latest();
        let (mut platform, contract) = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure()
            .with_crypto_card_game_transfer_only(Transferable::Always);

        let mut rng = StdRng::seed_from_u64(441);

        let platform_state = platform.state.load();

        let (identity, signer, key) = setup_identity(&mut platform, 962, dash_to_credits!(0.5));

        let (receiver, _, _) = setup_identity(&mut platform, 452, dash_to_credits!(0.5));

        let card_document_type = contract
            .document_type_for_name("card")
            .expect("expected a card document type");

        assert!(!card_document_type.documents_keep_transfer_history());

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
            .await
            .expect("expect to create documents batch transition");

        let documents_batch_create_serialized_transition = documents_batch_create_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[documents_batch_create_serialized_transition],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        assert_eq!(processing_result.valid_count(), 1);

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        document.set_revision(Some(2));

        let documents_batch_transfer_transition =
            BatchTransition::new_document_transfer_transition_from_document(
                document.clone(),
                card_document_type,
                receiver.id(),
                &key,
                3,
                0,
                None,
                &signer,
                platform_version,
                None,
            )
            .await
            .expect("expect to create documents batch transition for transfer");

        let documents_batch_transfer_serialized_transition = documents_batch_transfer_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[documents_batch_transfer_serialized_transition],
                &platform_state,
                &BlockInfo::default_with_time(50000000),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        // The card document type did not subscribe to history, so nothing may
        // have been recorded
        let history_documents = query_history_documents(
            &platform,
            "transfer",
            contract.id(),
            document.id(),
            platform_version,
        );

        assert_eq!(history_documents.len(), 0);
    }

    /// Documents in the document history contract can only ever be created by
    /// the protocol itself: user creation is blocked by
    /// `creationRestrictionMode: 2`.
    #[tokio::test]
    async fn test_document_history_contract_direct_writes_rejected() {
        let mut platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let platform_state = platform.state.load();
        let platform_version = platform_state
            .current_platform_version()
            .expect("expected to get current platform version");

        let mut rng = StdRng::seed_from_u64(442);

        let (identity, signer, key) = setup_identity(&mut platform, 963, dash_to_credits!(0.5));

        let history_contract = Arc::clone(
            &platform
                .drive
                .cache
                .system_data_contracts
                .load_document_history(platform_version)
                .expect("expected the document_history system contract"),
        );

        let transfer_document_type = history_contract
            .document_type_for_name("transfer")
            .expect("expected the transfer document type");

        let entropy = Bytes32::random_with_rng(&mut rng);

        let document = transfer_document_type
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity.id(),
                entropy,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        let documents_batch_create_transition =
            BatchTransition::new_document_creation_transition_from_document(
                document,
                transfer_document_type,
                entropy.0,
                &key,
                2,
                0,
                None,
                &signer,
                platform_version,
                None,
            )
            .await
            .expect("expect to create documents batch transition");

        let documents_batch_create_serialized_transition = documents_batch_create_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[documents_batch_create_serialized_transition],
                &platform_state,
                &BlockInfo::default_with_time(
                    platform_state
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

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::BasicError(BasicError::DocumentCreationNotAllowedError(_)),
                ..
            }]
        );
    }
}
