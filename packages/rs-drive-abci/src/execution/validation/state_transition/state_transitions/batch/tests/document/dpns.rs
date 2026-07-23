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

mod dpns_username_transfer_tests {
    use super::*;
    use dpp::consensus::basic::BasicError;
    use dpp::consensus::state::data_trigger::DataTriggerError;
    use dpp::data_contract::DataContract;
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

        let dpns = platform.drive.cache.system_data_contracts.load_dpns();
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
                in_clause: None,
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
                .load_document_history(),
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
                .load_document_history(),
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
                .load_document_history(),
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
    ) -> Vec<Document> {
        let history_contract = Arc::clone(
            &platform
                .drive
                .cache
                .system_data_contracts
                .load_document_history(),
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
                in_clause: None,
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
        let pricing_history =
            query_history_documents(&platform, "priceUpdate", dpns_contract.id(), document.id());

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
        let history_documents =
            query_history_documents(&platform, "transfer", contract.id(), document.id());

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
                .load_document_history(),
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
