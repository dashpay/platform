use super::*;

mod deletion_tests {
    use super::*;
    use crate::execution::validation::state_transition::tests::create_card_game_internal_token_contract_with_owner_identity_burn_tokens;
    use dpp::tokens::token_payment_info::v0::TokenPaymentInfoV0;
    use dpp::tokens::token_payment_info::TokenPaymentInfo;

    #[tokio::test]
    async fn test_document_delete_on_document_type_that_is_mutable_and_can_be_deleted() {
        run_document_delete_on_document_type_that_is_mutable_and_can_be_deleted_at_protocol_version(
            PlatformVersion::latest().protocol_version,
            1678920,
        )
        .await;
    }

    /// PROTOCOL_VERSION_11: pre-B7 fee — the transformer's local execution
    /// context was dropped, so the user wasn't charged for the per-transition
    /// grovedb reads `try_from_borrowed_*_with_contract_lookup` performs.
    /// Pinned so v11 chain history stays bit-for-bit reproducible.
    #[tokio::test]
    async fn test_document_delete_on_document_type_that_is_mutable_and_can_be_deleted_protocol_version_11(
    ) {
        run_document_delete_on_document_type_that_is_mutable_and_can_be_deleted_at_protocol_version(
            11, 1666860,
        )
        .await;
    }

    /// Helper for the paired happy-path delete fee test. Same scenario is
    /// exercised at PROTOCOL_VERSION_11 (transformer reads dropped) and at
    /// PROTOCOL_VERSION_12+ (transformer reads billed via outer execution
    /// context).
    async fn run_document_delete_on_document_type_that_is_mutable_and_can_be_deleted_at_protocol_version(
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

        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

        let dashpay = platform
            .drive
            .cache
            .system_data_contracts
            .load_dashpay(platform_version)
            .expect("expected the dashpay system contract");
        let dashpay_contract = dashpay.clone();

        let profile = dashpay_contract
            .document_type_for_name("profile")
            .expect("expected a profile document type");

        assert!(profile.documents_mutable());

        assert!(profile.documents_can_be_deleted());

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

        let mut altered_document = document.clone();

        altered_document.increment_revision().unwrap();
        altered_document.set("displayName", "Samuel".into());
        altered_document.set("avatarUrl", "http://test.com/cat.jpg".into());

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
            .await
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

        assert_eq!(processing_result.valid_count(), 1);

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let documents_batch_deletion_transition =
            BatchTransition::new_document_deletion_transition_from_document(
                altered_document,
                profile,
                &key,
                3,
                0,
                None,
                &signer,
                platform_version,
                None,
            )
            .await
            .expect("expect to create documents batch transition");

        let documents_batch_update_serialized_transition = documents_batch_deletion_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_update_serialized_transition.clone()],
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

        assert_eq!(processing_result.invalid_paid_count(), 0);

        assert_eq!(processing_result.invalid_unpaid_count(), 0);

        assert_eq!(processing_result.valid_count(), 1);

        assert_eq!(
            processing_result.aggregated_fees().processing_fee,
            expected_processing_fee,
            "PROTOCOL_VERSION_{}: happy-path delete processing fee must match the version-specific baseline",
            protocol_version,
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

    #[tokio::test]
    async fn test_document_delete_on_document_type_that_is_mutable_and_can_not_be_deleted() {
        let mut platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let contract_path = "tests/supporting_files/contract/dashpay/dashpay-contract-contact-request-mutable-and-can-not-be-deleted.json";

        let platform_state = platform.state.load();
        let platform_version = platform_state
            .current_platform_version()
            .expect("expected to get current platform version");

        // let's construct the grovedb structure for the card game data contract
        let dashpay_contract = json_document_to_contract(contract_path, true, platform_version)
            .expect("expected to get data contract");
        platform
            .drive
            .apply_contract(
                &dashpay_contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("expected to apply contract successfully");

        let mut rng = StdRng::seed_from_u64(437);

        let platform_state = platform.state.load();

        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

        let (other_identity, ..) = setup_identity(&mut platform, 495, dash_to_credits!(0.1));

        let contact_request_document_type = dashpay_contract
            .document_type_for_name("contactRequest")
            .expect("expected a profile document type");

        assert!(contact_request_document_type.documents_mutable());

        assert!(!contact_request_document_type.documents_can_be_deleted());

        let entropy = Bytes32::random_with_rng(&mut rng);

        let mut document = contact_request_document_type
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity.id(),
                entropy,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        document.set(
            "toUserId",
            Value::Identifier(other_identity.id().to_buffer()),
        );
        document.set("recipientKeyIndex", Value::U32(1));
        document.set("senderKeyIndex", Value::U32(1));
        document.set("accountReference", Value::U32(0));

        let mut altered_document = document.clone();

        altered_document.set_revision(Some(1));
        altered_document.set("senderKeyIndex", Value::U32(2));

        let documents_batch_create_transition =
            BatchTransition::new_document_creation_transition_from_document(
                document,
                contact_request_document_type,
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
                &vec![documents_batch_create_serialized_transition.clone()],
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

        let documents_batch_deletion_transition =
            BatchTransition::new_document_deletion_transition_from_document(
                altered_document,
                contact_request_document_type,
                &key,
                3,
                0,
                None,
                &signer,
                platform_version,
                None,
            )
            .await
            .expect("expect to create documents batch transition");

        let documents_batch_deletion_serialized_transition = documents_batch_deletion_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_deletion_serialized_transition.clone()],
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

        assert_eq!(processing_result.invalid_paid_count(), 1);

        assert_eq!(processing_result.invalid_unpaid_count(), 0);

        assert_eq!(processing_result.valid_count(), 0);

        assert_eq!(processing_result.aggregated_fees().processing_fee, 445700);
    }

    #[tokio::test]
    async fn test_document_delete_on_document_type_that_is_not_mutable_and_can_be_deleted() {
        run_document_delete_on_document_type_that_is_not_mutable_and_can_be_deleted_at_protocol_version(
            PlatformVersion::latest().protocol_version,
            2778700,
        )
        .await;
    }

    /// PROTOCOL_VERSION_11: pre-B7 fee — see sibling docs above. Pinned so
    /// v11 chain history stays bit-for-bit reproducible.
    #[tokio::test]
    async fn test_document_delete_on_document_type_that_is_not_mutable_and_can_be_deleted_protocol_version_11(
    ) {
        run_document_delete_on_document_type_that_is_not_mutable_and_can_be_deleted_at_protocol_version(
            11, 2762400,
        )
        .await;
    }

    async fn run_document_delete_on_document_type_that_is_not_mutable_and_can_be_deleted_at_protocol_version(
        protocol_version: dpp::version::ProtocolVersion,
        expected_processing_fee: dpp::fee::Credits,
    ) {
        let mut platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(protocol_version)
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let contract_path = "tests/supporting_files/contract/dashpay/dashpay-contract-contact-request-not-mutable-and-can-be-deleted.json";

        let platform_state = platform.state.load();
        let platform_version = PlatformVersion::get(protocol_version)
            .expect("expected platform version for the requested protocol_version");

        // let's construct the grovedb structure for the card game data contract
        let dashpay_contract = json_document_to_contract(contract_path, true, platform_version)
            .expect("expected to get data contract");
        platform
            .drive
            .apply_contract(
                &dashpay_contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("expected to apply contract successfully");

        let mut rng = StdRng::seed_from_u64(437);

        let platform_state = platform.state.load();

        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

        let (other_identity, ..) = setup_identity(&mut platform, 495, dash_to_credits!(0.1));

        let contact_request_document_type = dashpay_contract
            .document_type_for_name("contactRequest")
            .expect("expected a profile document type");

        assert!(!contact_request_document_type.documents_mutable());

        assert!(contact_request_document_type.documents_can_be_deleted());

        let entropy = Bytes32::random_with_rng(&mut rng);

        let mut document = contact_request_document_type
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity.id(),
                entropy,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        document.set(
            "toUserId",
            Value::Identifier(other_identity.id().to_buffer()),
        );
        document.set("recipientKeyIndex", Value::U32(1));
        document.set("senderKeyIndex", Value::U32(1));
        document.set("accountReference", Value::U32(0));

        let mut altered_document = document.clone();

        altered_document.set_revision(Some(1));
        altered_document.set("senderKeyIndex", Value::U32(2));

        let documents_batch_create_transition =
            BatchTransition::new_document_creation_transition_from_document(
                document,
                contact_request_document_type,
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
                &vec![documents_batch_create_serialized_transition.clone()],
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

        let documents_batch_deletion_transition =
            BatchTransition::new_document_deletion_transition_from_document(
                altered_document,
                contact_request_document_type,
                &key,
                3,
                0,
                None,
                &signer,
                platform_version,
                None,
            )
            .await
            .expect("expect to create documents batch transition");

        let documents_batch_deletion_serialized_transition = documents_batch_deletion_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_deletion_serialized_transition.clone()],
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

        assert_eq!(processing_result.invalid_paid_count(), 0);

        assert_eq!(processing_result.invalid_unpaid_count(), 0);

        assert_eq!(processing_result.valid_count(), 1);

        assert_eq!(
            processing_result.aggregated_fees().processing_fee,
            expected_processing_fee,
            "PROTOCOL_VERSION_{}: happy-path delete (non-mutable doc) processing fee must match the version-specific baseline",
            protocol_version,
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

    #[tokio::test]
    async fn test_document_delete_on_document_type_that_is_not_mutable_and_can_not_be_deleted() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(437);

        let platform_state = platform.state.load();

        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

        let (other_identity, ..) = setup_identity(&mut platform, 495, dash_to_credits!(0.1));

        let dashpay = platform
            .drive
            .cache
            .system_data_contracts
            .load_dashpay(platform_version)
            .expect("expected the dashpay system contract");
        let dashpay_contract = dashpay.clone();

        let contact_request_document_type = dashpay_contract
            .document_type_for_name("contactRequest")
            .expect("expected a profile document type");

        assert!(!contact_request_document_type.documents_mutable());

        assert!(!contact_request_document_type.documents_can_be_deleted());

        let entropy = Bytes32::random_with_rng(&mut rng);

        let mut document = contact_request_document_type
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity.id(),
                entropy,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        document.set(
            "toUserId",
            Value::Identifier(other_identity.id().to_buffer()),
        );
        document.set("recipientKeyIndex", Value::U32(1));
        document.set("senderKeyIndex", Value::U32(1));
        document.set("accountReference", Value::U32(0));

        let mut altered_document = document.clone();

        altered_document.set_revision(Some(1));
        altered_document.set("senderKeyIndex", Value::U32(2));

        let documents_batch_create_transition =
            BatchTransition::new_document_creation_transition_from_document(
                document,
                contact_request_document_type,
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
                &vec![documents_batch_create_serialized_transition.clone()],
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

        let documents_batch_deletion_transition =
            BatchTransition::new_document_deletion_transition_from_document(
                altered_document,
                contact_request_document_type,
                &key,
                3,
                0,
                None,
                &signer,
                platform_version,
                None,
            )
            .await
            .expect("expect to create documents batch transition");

        let documents_batch_deletion_serialized_transition = documents_batch_deletion_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_deletion_serialized_transition.clone()],
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

        assert_eq!(processing_result.invalid_paid_count(), 1);

        assert_eq!(processing_result.invalid_unpaid_count(), 0);

        assert_eq!(processing_result.valid_count(), 0);

        assert_eq!(processing_result.aggregated_fees().processing_fee, 445700);
    }

    #[tokio::test]
    async fn test_document_delete_that_does_not_yet_exist() {
        run_document_delete_that_does_not_yet_exist_at_protocol_version(
            PlatformVersion::latest().protocol_version,
            520340,
        )
        .await;
    }

    /// PROTOCOL_VERSION_11: pre-B7 fee — the transformer's local execution
    /// context was dropped, so the per-transition grovedb reads
    /// `try_from_borrowed_*_with_contract_lookup` performs were not billed.
    /// Pinned so v11 chain history stays bit-for-bit reproducible.
    #[tokio::test]
    async fn test_document_delete_that_does_not_yet_exist_protocol_version_11() {
        run_document_delete_that_does_not_yet_exist_at_protocol_version(11, 516040).await;
    }

    /// Helper for the paired delete-that-does-not-yet-exist fee test.
    /// PROTOCOL_VERSION_11 yields the pre-fix bump-only fee (transformer
    /// reads dropped); PROTOCOL_VERSION_12+ adds the reads.
    async fn run_document_delete_that_does_not_yet_exist_at_protocol_version(
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

        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

        let dashpay = platform
            .drive
            .cache
            .system_data_contracts
            .load_dashpay(platform_version)
            .expect("expected the dashpay system contract");
        let dashpay_contract = dashpay.clone();

        let profile = dashpay_contract
            .document_type_for_name("profile")
            .expect("expected a profile document type");

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

        let mut altered_document = document.clone();

        altered_document.increment_revision().unwrap();
        altered_document.set("displayName", "Samuel".into());
        altered_document.set("avatarUrl", "http://test.com/cat.jpg".into());

        let documents_batch_delete_transition =
            BatchTransition::new_document_deletion_transition_from_document(
                altered_document,
                profile,
                &key,
                3,
                0,
                None,
                &signer,
                platform_version,
                None,
            )
            .await
            .expect("expect to create documents batch transition");

        let documents_batch_delete_serialized_transition = documents_batch_delete_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_delete_serialized_transition.clone()],
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

        assert_eq!(processing_result.invalid_paid_count(), 1);

        assert_eq!(processing_result.invalid_unpaid_count(), 0);

        assert_eq!(processing_result.valid_count(), 0);

        assert_eq!(
            processing_result.aggregated_fees().processing_fee,
            expected_processing_fee,
            "PROTOCOL_VERSION_{}: delete-that-does-not-yet-exist processing fee must match the version-specific baseline",
            protocol_version,
        );
    }
    #[tokio::test]
    async fn test_document_deletion_that_needs_a_token() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(433);

        let platform_state = platform.state.load();

        let (contract_owner_id, _, _) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

        let (buyer, signer, key) = setup_identity(&mut platform, 234, dash_to_credits!(0.1));

        let (contract, gold_token_id, gas_token_id) =
            create_card_game_internal_token_contract_with_owner_identity_burn_tokens(
                &mut platform,
                contract_owner_id.id(),
                platform_version,
            );

        assert_eq!(contract.tokens().len(), 2);

        add_tokens_to_identity(&mut platform, gold_token_id, buyer.id(), 15);
        add_tokens_to_identity(&mut platform, gas_token_id, buyer.id(), 5);

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
                    gas_fees_paid_by: Default::default(),
                })),
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
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
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

        // He had 15, but spent 10
        assert_eq!(token_balance, Some(5));

        let documents_batch_deletion_transition =
            BatchTransition::new_document_deletion_transition_from_document(
                document,
                card_document_type,
                &key,
                3,
                0,
                Some(TokenPaymentInfo::V0(TokenPaymentInfoV0 {
                    payment_token_contract_id: None,
                    token_contract_position: 1,
                    minimum_token_cost: None,
                    maximum_token_cost: Some(1),
                    gas_fees_paid_by: Default::default(),
                })),
                &signer,
                platform_version,
                None,
            )
            .await
            .expect("expect to create documents batch transition");

        let documents_batch_update_serialized_transition = documents_batch_deletion_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_update_serialized_transition.clone()],
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
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
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
                gas_token_id.to_buffer(),
                buyer.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");

        // He had 5, but spent 1
        assert_eq!(token_balance, Some(4));
    }

    #[tokio::test]
    async fn test_document_deletion_that_needs_a_token_not_enough_balance_to_delete() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(433);

        let platform_state = platform.state.load();

        let (contract_owner_id, _, _) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

        let (buyer, signer, key) = setup_identity(&mut platform, 234, dash_to_credits!(0.1));

        let (contract, gold_token_id, gas_token_id) =
            create_card_game_internal_token_contract_with_owner_identity_burn_tokens(
                &mut platform,
                contract_owner_id.id(),
                platform_version,
            );

        assert_eq!(contract.tokens().len(), 2);

        add_tokens_to_identity(&mut platform, gold_token_id, buyer.id(), 15);

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
                    gas_fees_paid_by: Default::default(),
                })),
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
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
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

        // He had 15, but spent 10
        assert_eq!(token_balance, Some(5));

        let documents_batch_deletion_transition =
            BatchTransition::new_document_deletion_transition_from_document(
                document,
                card_document_type,
                &key,
                3,
                0,
                Some(TokenPaymentInfo::V0(TokenPaymentInfoV0 {
                    payment_token_contract_id: None,
                    token_contract_position: 1,
                    minimum_token_cost: None,
                    maximum_token_cost: Some(10),
                    gas_fees_paid_by: Default::default(),
                })),
                &signer,
                platform_version,
                None,
            )
            .await
            .expect("expect to create documents batch transition");

        let documents_batch_update_serialized_transition = documents_batch_deletion_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_update_serialized_transition.clone()],
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
            [PaidConsensusError {
                error: ConsensusError::StateError(
                    StateError::IdentityDoesNotHaveEnoughTokenBalanceError(_)
                ),
                ..
            }]
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
                gas_token_id.to_buffer(),
                buyer.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");

        // He still has no token balance of the gas token
        assert_eq!(token_balance, None);
    }

    #[tokio::test]
    async fn test_document_deletion_where_we_are_not_the_owner() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(433);

        let platform_state = platform.state.load();

        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

        let (other_identity, other_signer, other_key) =
            setup_identity(&mut platform, 495, dash_to_credits!(0.1));

        let dpns = platform
            .drive
            .cache
            .system_data_contracts
            .load_dpns(platform_version)
            .expect("expected the dpns system contract");
        let dpns_contract = dpns.clone();

        let preorder_document_type = dpns_contract
            .document_type_for_name("preorder")
            .expect("expected a profile document type");

        assert!(preorder_document_type.documents_can_be_deleted());

        let entropy = Bytes32::random_with_rng(&mut rng);

        let document = preorder_document_type
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity.id(),
                entropy,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        let mut altered_document = document.clone();

        altered_document.set_revision(Some(1));

        altered_document.set_owner_id(other_identity.id());

        let documents_batch_create_transition =
            BatchTransition::new_document_creation_transition_from_document(
                document,
                preorder_document_type,
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
                &vec![documents_batch_create_serialized_transition.clone()],
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

        let documents_batch_deletion_transition =
            BatchTransition::new_document_deletion_transition_from_document(
                altered_document,
                preorder_document_type,
                &other_key,
                3,
                0,
                None,
                &other_signer,
                platform_version,
                None,
            )
            .await
            .expect("expect to create documents batch transition");

        let documents_batch_deletion_serialized_transition = documents_batch_deletion_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_deletion_serialized_transition.clone()],
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
            [PaidConsensusError {
                error: ConsensusError::StateError(StateError::DocumentOwnerIdMismatchError(_)),
                ..
            }]
        );
    }
}
