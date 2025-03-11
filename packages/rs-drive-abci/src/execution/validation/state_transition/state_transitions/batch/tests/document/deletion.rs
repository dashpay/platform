use super::*;

mod deletion_tests {
    use super::*;

    #[test]
    fn test_document_delete_on_document_type_that_is_mutable_and_can_be_deleted() {
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
                &signer,
                platform_version,
                None,
                None,
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
                &signer,
                platform_version,
                None,
                None,
                None,
            )
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

        assert_eq!(processing_result.aggregated_fees().processing_fee, 1711420);

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

    #[test]
    fn test_document_delete_on_document_type_that_is_mutable_and_can_not_be_deleted() {
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
                &signer,
                platform_version,
                None,
                None,
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
                &signer,
                platform_version,
                None,
                None,
                None,
            )
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

    #[test]
    fn test_document_delete_on_document_type_that_is_not_mutable_and_can_be_deleted() {
        let mut platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let contract_path = "tests/supporting_files/contract/dashpay/dashpay-contract-contact-request-not-mutable-and-can-be-deleted.json";

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
                &signer,
                platform_version,
                None,
                None,
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
                &signer,
                platform_version,
                None,
                None,
                None,
            )
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

        assert_eq!(processing_result.aggregated_fees().processing_fee, 2762400);

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

    #[test]
    fn test_document_delete_on_document_type_that_is_not_mutable_and_can_not_be_deleted() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(437);

        let platform_state = platform.state.load();

        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

        let (other_identity, ..) = setup_identity(&mut platform, 495, dash_to_credits!(0.1));

        let dashpay = platform.drive.cache.system_data_contracts.load_dashpay();
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
                &signer,
                platform_version,
                None,
                None,
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
                &signer,
                platform_version,
                None,
                None,
                None,
            )
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

    #[test]
    fn test_document_delete_that_does_not_yet_exist() {
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
                &signer,
                platform_version,
                None,
                None,
                None,
            )
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

        assert_eq!(processing_result.aggregated_fees().processing_fee, 516040);
    }
}
