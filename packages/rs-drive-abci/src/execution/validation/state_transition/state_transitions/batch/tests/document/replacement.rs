use super::*;

mod replacement_tests {
    use super::*;
    use crate::test::helpers::fast_forward_to_block::fast_forward_to_block;
    use dpp::identifier::Identifier;
    use dpp::prelude::IdentityNonce;
    use dpp::tokens::token_payment_info::v0::TokenPaymentInfoV0;
    use dpp::tokens::token_payment_info::TokenPaymentInfo;
    use std::collections::BTreeMap;

    #[test]
    fn test_document_replace_on_document_type_that_is_mutable() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(433);

        fast_forward_to_block(&platform, 1_200_000_000, 900, 42, 1, false); //next epoch

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

        let documents_batch_update_transition =
            BatchTransition::new_document_replacement_transition_from_document(
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
            .expect("expect to create documents batch transition");

        let documents_batch_update_serialized_transition = documents_batch_update_transition
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

        assert_eq!(processing_result.aggregated_fees().processing_fee, 1443820);

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

    fn perform_document_replace_on_profile_after_epoch_change(
        original_name: &str,
        new_names: Vec<(&str, StorageFlags)>,
    ) {
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

        document.set("displayName", original_name.into());
        document.set("avatarUrl", "http://test.com/bob.jpg".into());

        let documents_batch_create_transition =
            BatchTransition::new_document_creation_transition_from_document(
                document.clone(),
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

        assert_eq!(processing_result.valid_count(), 1);

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        for (i, (new_name, mut expected_flags)) in new_names.into_iter().enumerate() {
            document.increment_revision().unwrap();
            document.set("displayName", new_name.into());

            fast_forward_to_block(
                &platform,
                500_000_000 + i as u64 * 1000,
                900 + i as u64,
                42,
                1 + i as u16,
                true,
            ); //less than a week

            let documents_batch_update_transition =
                BatchTransition::new_document_replacement_transition_from_document(
                    document.clone(),
                    profile,
                    &key,
                    3 + i as IdentityNonce,
                    0,
                    None,
                    &signer,
                    platform_version,
                    None,
                )
                .expect("expect to create documents batch transition");

            let documents_batch_update_serialized_transition = documents_batch_update_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let platform_state = platform.state.load();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![documents_batch_update_serialized_transition.clone()],
                    &platform_state,
                    platform_state.last_block_info(),
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

            assert_eq!(
                processing_result.valid_count(),
                1,
                "{:?}",
                processing_result.execution_results()
            );

            let drive_query = DriveDocumentQuery::new_primary_key_single_item_query(
                &dashpay,
                profile,
                document.id(),
            );

            let mut documents = platform
                .drive
                .query_documents_with_flags(drive_query, None, false, None, None)
                .expect("expected to get back documents")
                .documents_owned();

            let (_first_document, storage_flags) = documents.remove(0);

            let storage_flags = storage_flags.expect("expected storage flags");

            expected_flags.set_owner_id(identity.id().to_buffer());

            assert_eq!(storage_flags, expected_flags);
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

    #[test]
    fn test_document_replace_on_document_type_that_is_mutable_different_epoch_bigger_size() {
        perform_document_replace_on_profile_after_epoch_change(
            "Sam",
            vec![(
                "Samuel",
                StorageFlags::MultiEpochOwned(
                    0,
                    BTreeMap::from([(1, 6)]),
                    Identifier::default().to_buffer(),
                ),
            )],
        );
    }

    #[test]
    fn test_document_replace_on_document_type_that_is_mutable_different_epoch_smaller_size() {
        perform_document_replace_on_profile_after_epoch_change(
            "Sam",
            vec![(
                "S",
                StorageFlags::SingleEpochOwned(0, Identifier::default().to_buffer()),
            )],
        );
    }

    #[test]
    fn test_document_replace_on_document_type_that_is_mutable_different_epoch_same_size() {
        perform_document_replace_on_profile_after_epoch_change(
            "Sam",
            vec![(
                "Max",
                StorageFlags::SingleEpochOwned(0, Identifier::default().to_buffer()),
            )],
        );
    }

    #[test]
    fn test_document_replace_on_document_type_that_is_mutable_different_epoch_bigger_size_then_bigger_size(
    ) {
        perform_document_replace_on_profile_after_epoch_change(
            "Sam",
            vec![
                (
                    "Samuel",
                    StorageFlags::MultiEpochOwned(
                        0,
                        BTreeMap::from([(1, 6)]),
                        Identifier::default().to_buffer(),
                    ),
                ),
                (
                    "SamuelW",
                    StorageFlags::MultiEpochOwned(
                        0,
                        BTreeMap::from([(1, 6), (2, 4)]),
                        Identifier::default().to_buffer(),
                    ),
                ),
            ],
        );
    }

    #[test]
    fn test_document_replace_on_document_type_that_is_mutable_different_epoch_bigger_size_then_bigger_size_by_3_bytes(
    ) {
        perform_document_replace_on_profile_after_epoch_change(
            "Sam",
            vec![
                (
                    "Samuel",
                    StorageFlags::MultiEpochOwned(
                        0,
                        BTreeMap::from([(1, 6)]),
                        Identifier::default().to_buffer(),
                    ),
                ),
                (
                    "SamuelWes",
                    StorageFlags::MultiEpochOwned(
                        0,
                        BTreeMap::from([(1, 6), (2, 6)]),
                        Identifier::default().to_buffer(),
                    ),
                ),
            ],
        );
    }

    #[test]
    fn test_document_replace_on_document_type_that_is_mutable_different_epoch_bigger_size_then_smaller_size(
    ) {
        // In this case we start with the size Samuell Base epoch 0 epoch 1 added 7 bytes
        // Then we try to update it to         Sami    Base epoch 2
        // Epoch 1 added 7 bytes is itself 3 bytes
        // Sami is 3 bytes less than Samuell
        // First iteration will say we should remove 6 bytes
        // We need to start by calculating the cost of the original storage flags, in this case 5 bytes
        // Then we need to calculate the cost of the new storage flags, in this case 2 bytes
        // We should do the difference, then apply that difference in the combination function
        perform_document_replace_on_profile_after_epoch_change(
            "Sam",
            vec![
                (
                    "Samuell",
                    StorageFlags::MultiEpochOwned(
                        0,
                        BTreeMap::from([(1, 7)]),
                        Identifier::default().to_buffer(),
                    ),
                ),
                (
                    "Sami",
                    StorageFlags::MultiEpochOwned(
                        0,
                        BTreeMap::from([(1, 4)]),
                        Identifier::default().to_buffer(),
                    ),
                ),
            ],
        );
    }

    #[test]
    fn test_document_replace_on_document_type_that_is_mutable_different_epoch_bigger_size_then_back_to_original(
    ) {
        perform_document_replace_on_profile_after_epoch_change(
            "Sam",
            vec![
                (
                    "Samuel",
                    StorageFlags::MultiEpochOwned(
                        0,
                        BTreeMap::from([(1, 6)]),
                        Identifier::default().to_buffer(),
                    ),
                ),
                (
                    "Sam",
                    StorageFlags::SingleEpochOwned(0, Identifier::default().to_buffer()),
                ),
            ],
        );
    }

    #[test]
    fn test_document_replace_on_document_type_that_is_not_mutable() {
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

        let documents_batch_update_transition =
            BatchTransition::new_document_replacement_transition_from_document(
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
            .expect("expect to create documents batch transition");

        let documents_batch_update_serialized_transition = documents_batch_update_transition
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

        assert_eq!(processing_result.invalid_paid_count(), 1);

        assert_eq!(processing_result.invalid_unpaid_count(), 0);

        assert_eq!(processing_result.valid_count(), 0);

        assert_eq!(processing_result.aggregated_fees().processing_fee, 41880);
    }

    #[test]
    fn test_document_replace_on_document_type_that_is_not_mutable_but_is_transferable() {
        let platform_version = PlatformVersion::latest();
        let (mut platform, contract) = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure()
            .with_crypto_card_game_transfer_only(Transferable::Always);

        let mut rng = StdRng::seed_from_u64(435);

        let platform_state = platform.state.load();

        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

        let (receiver, _, _) = setup_identity(&mut platform, 452, dash_to_credits!(0.1));

        let card_document_type = contract
            .document_type_for_name("card")
            .expect("expected a profile document type");

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

        assert_eq!(processing_result.valid_count(), 1);

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let sender_documents_sql_string =
            format!("select * from card where $ownerId == '{}'", identity.id());

        let query_sender_identity_documents = DriveDocumentQuery::from_sql_expr(
            sender_documents_sql_string.as_str(),
            &contract,
            Some(&platform.config.drive),
        )
        .expect("expected document query");

        let receiver_documents_sql_string =
            format!("select * from card where $ownerId == '{}'", receiver.id());

        let query_receiver_identity_documents = DriveDocumentQuery::from_sql_expr(
            receiver_documents_sql_string.as_str(),
            &contract,
            Some(&platform.config.drive),
        )
        .expect("expected document query");

        let query_sender_results = platform
            .drive
            .query_documents(
                query_sender_identity_documents.clone(),
                None,
                false,
                None,
                None,
            )
            .expect("expected query result");

        let query_receiver_results = platform
            .drive
            .query_documents(
                query_receiver_identity_documents.clone(),
                None,
                false,
                None,
                None,
            )
            .expect("expected query result");

        // We expect the sender to have 1 document, and the receiver to have none
        assert_eq!(query_sender_results.documents().len(), 1);

        assert_eq!(query_receiver_results.documents().len(), 0);

        document.set_revision(Some(2));

        document.set("attack", 6.into());
        document.set("defense", 0.into());

        let documents_batch_transfer_transition =
            BatchTransition::new_document_replacement_transition_from_document(
                document,
                card_document_type,
                &key,
                3,
                0,
                None,
                &signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition for transfer");

        let documents_batch_transfer_serialized_transition = documents_batch_transfer_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_transfer_serialized_transition.clone()],
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

        let query_sender_results = platform
            .drive
            .query_documents(query_sender_identity_documents, None, false, None, None)
            .expect("expected query result");

        let query_receiver_results = platform
            .drive
            .query_documents(query_receiver_identity_documents, None, false, None, None)
            .expect("expected query result");

        // We expect the sender to still have their document, and the receiver to have none
        assert_eq!(query_sender_results.documents().len(), 1);

        assert_eq!(query_receiver_results.documents().len(), 0);
    }

    #[test]
    fn test_document_replace_that_does_not_yet_exist() {
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

        let documents_batch_update_transition =
            BatchTransition::new_document_replacement_transition_from_document(
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
            .expect("expect to create documents batch transition");

        let documents_batch_update_serialized_transition = documents_batch_update_transition
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

        assert_eq!(processing_result.invalid_paid_count(), 1);

        assert_eq!(processing_result.invalid_unpaid_count(), 0);

        assert_eq!(processing_result.valid_count(), 0);

        assert_eq!(processing_result.aggregated_fees().processing_fee, 516040);
    }

    #[test]
    fn test_double_document_replace() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(433);

        fast_forward_to_block(&platform, 1_200_000_000, 900, 42, 1, false); //next epoch

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

        let mut altered_document = document.clone();

        altered_document.increment_revision().unwrap();
        altered_document.set("displayName", "Samuel".into());
        altered_document.set("avatarUrl", "http://test.com/cat.jpg".into());

        let mut altered_document_2 = altered_document.clone();

        altered_document_2.increment_revision().unwrap();
        altered_document_2.set("displayName", "Ody".into());
        altered_document_2.set("avatarUrl", "http://test.com/drapes.jpg".into());

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
                platform_state.last_block_info(),
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

        let receiver_documents_sql_string = "select * from profile".to_string();

        let query_documents = DriveDocumentQuery::from_sql_expr(
            receiver_documents_sql_string.as_str(),
            &dashpay,
            Some(&platform.config.drive),
        )
        .expect("expected document query");

        let query_sender_results = platform
            .drive
            .query_documents(query_documents.clone(), None, false, None, None)
            .expect("expected query result");

        let document = query_sender_results
            .documents()
            .first()
            .expect("expected a document");

        assert_eq!(document.to_string(), "v0 : id:GcviwUsEr9Ji4rCrnnsgmVAghNaVPDumsfcagvBbBy45 owner_id:CisQdz2ej7EwWv8JbetSXBNsV4xsf8QsSS8tqp4tEf7V created_at:1970-01-14 21:20:00 updated_at:1970-01-14 21:20:00 avatarFingerprint:bytes d7b0e2b357c10312 avatarHash:bytes32 YonaRoE0hMgat53AYt5LTlQlIkKLReGpB7xNAqJ5HM8= avatarUrl:string http://test.com/bob.[...(23)] displayName:string QBwBNNXXYCngB0er publicMessage:string 8XG7KBGNvm2  ");

        let documents_batch_update_transition_1 =
            BatchTransition::new_document_replacement_transition_from_document(
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
            .expect("expect to create documents batch transition");

        let documents_batch_update_serialized_transition_1 = documents_batch_update_transition_1
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let documents_batch_update_transition_2 =
            BatchTransition::new_document_replacement_transition_from_document(
                altered_document_2,
                profile,
                &key,
                4,
                0,
                None,
                &signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

        let documents_batch_update_serialized_transition_2 = documents_batch_update_transition_2
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![
                    documents_batch_update_serialized_transition_1.clone(),
                    documents_batch_update_serialized_transition_2.clone(),
                ],
                &platform_state,
                platform_state.last_block_info(),
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

        assert_eq!(processing_result.valid_count(), 2);

        let query_sender_results = platform
            .drive
            .query_documents(query_documents.clone(), None, false, None, None)
            .expect("expected query result");

        let document = query_sender_results
            .documents()
            .first()
            .expect("expected a document");

        assert_eq!(document.to_string(), "v0 : id:GcviwUsEr9Ji4rCrnnsgmVAghNaVPDumsfcagvBbBy45 owner_id:CisQdz2ej7EwWv8JbetSXBNsV4xsf8QsSS8tqp4tEf7V created_at:1970-01-14 21:20:00 updated_at:1970-01-14 21:20:00 avatarFingerprint:bytes d7b0e2b357c10312 avatarHash:bytes32 YonaRoE0hMgat53AYt5LTlQlIkKLReGpB7xNAqJ5HM8= avatarUrl:string http://test.com/drap[...(26)] displayName:string Ody publicMessage:string 8XG7KBGNvm2  ");

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
    fn test_double_document_replace_different_height_same_epoch() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(433);

        fast_forward_to_block(&platform, 1_200_000_000, 900, 42, 1, false); //next epoch

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

        let mut altered_document = document.clone();

        altered_document.increment_revision().unwrap();
        altered_document.set("displayName", "Samuel".into());
        altered_document.set("avatarUrl", "http://test.com/cat.jpg".into());

        let mut altered_document_2 = altered_document.clone();

        altered_document_2.increment_revision().unwrap();
        altered_document_2.set("displayName", "Ody".into());
        altered_document_2.set("avatarUrl", "http://test.com/drapes.jpg".into());

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
                platform_state.last_block_info(),
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

        let receiver_documents_sql_string = "select * from profile".to_string();

        let query_documents = DriveDocumentQuery::from_sql_expr(
            receiver_documents_sql_string.as_str(),
            &dashpay,
            Some(&platform.config.drive),
        )
        .expect("expected document query");

        let query_sender_results = platform
            .drive
            .query_documents(query_documents.clone(), None, false, None, None)
            .expect("expected query result");

        let document = query_sender_results
            .documents()
            .first()
            .expect("expected a document");

        assert_eq!(document.to_string(), "v0 : id:GcviwUsEr9Ji4rCrnnsgmVAghNaVPDumsfcagvBbBy45 owner_id:CisQdz2ej7EwWv8JbetSXBNsV4xsf8QsSS8tqp4tEf7V created_at:1970-01-14 21:20:00 updated_at:1970-01-14 21:20:00 avatarFingerprint:bytes d7b0e2b357c10312 avatarHash:bytes32 YonaRoE0hMgat53AYt5LTlQlIkKLReGpB7xNAqJ5HM8= avatarUrl:string http://test.com/bob.[...(23)] displayName:string QBwBNNXXYCngB0er publicMessage:string 8XG7KBGNvm2  ");

        fast_forward_to_block(&platform, 1_400_000_000, 901, 43, 1, false); //next epoch

        let platform_state = platform.state.load();

        let documents_batch_update_transition_1 =
            BatchTransition::new_document_replacement_transition_from_document(
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
            .expect("expect to create documents batch transition");

        let documents_batch_update_serialized_transition_1 = documents_batch_update_transition_1
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let documents_batch_update_transition_2 =
            BatchTransition::new_document_replacement_transition_from_document(
                altered_document_2,
                profile,
                &key,
                4,
                0,
                None,
                &signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

        let documents_batch_update_serialized_transition_2 = documents_batch_update_transition_2
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_update_serialized_transition_1.clone()],
                &platform_state,
                platform_state.last_block_info(),
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

        let query_sender_results = platform
            .drive
            .query_documents(query_documents.clone(), None, false, None, None)
            .expect("expected query result");

        let document = query_sender_results
            .documents()
            .first()
            .expect("expected a document");

        assert_eq!(document.to_string(), "v0 : id:GcviwUsEr9Ji4rCrnnsgmVAghNaVPDumsfcagvBbBy45 owner_id:CisQdz2ej7EwWv8JbetSXBNsV4xsf8QsSS8tqp4tEf7V created_at:1970-01-14 21:20:00 updated_at:1970-01-17 04:53:20 avatarFingerprint:bytes d7b0e2b357c10312 avatarHash:bytes32 YonaRoE0hMgat53AYt5LTlQlIkKLReGpB7xNAqJ5HM8= avatarUrl:string http://test.com/cat.[...(23)] displayName:string Samuel publicMessage:string 8XG7KBGNvm2  ");

        fast_forward_to_block(&platform, 1_600_000_000, 902, 44, 1, false); //next epoch

        let platform_state = platform.state.load();

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_update_serialized_transition_2.clone()],
                &platform_state,
                platform_state.last_block_info(),
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

        let query_sender_results = platform
            .drive
            .query_documents(query_documents.clone(), None, false, None, None)
            .expect("expected query result");

        let document = query_sender_results
            .documents()
            .first()
            .expect("expected a document");

        assert_eq!(document.to_string(), "v0 : id:GcviwUsEr9Ji4rCrnnsgmVAghNaVPDumsfcagvBbBy45 owner_id:CisQdz2ej7EwWv8JbetSXBNsV4xsf8QsSS8tqp4tEf7V created_at:1970-01-14 21:20:00 updated_at:1970-01-19 12:26:40 avatarFingerprint:bytes d7b0e2b357c10312 avatarHash:bytes32 YonaRoE0hMgat53AYt5LTlQlIkKLReGpB7xNAqJ5HM8= avatarUrl:string http://test.com/drap[...(26)] displayName:string Ody publicMessage:string 8XG7KBGNvm2  ");

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
    fn test_double_document_replace_no_change_different_height_same_epoch() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(433);

        fast_forward_to_block(&platform, 1_200_000_000, 900, 42, 1, false); //next epoch

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

        let mut altered_document = document.clone();

        altered_document.increment_revision().unwrap();

        let mut altered_document_2 = altered_document.clone();

        altered_document_2.increment_revision().unwrap();

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
                platform_state.last_block_info(),
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

        let receiver_documents_sql_string = "select * from profile".to_string();

        let query_documents = DriveDocumentQuery::from_sql_expr(
            receiver_documents_sql_string.as_str(),
            &dashpay,
            Some(&platform.config.drive),
        )
        .expect("expected document query");

        let query_sender_results = platform
            .drive
            .query_documents(query_documents.clone(), None, false, None, None)
            .expect("expected query result");

        let document = query_sender_results
            .documents()
            .first()
            .expect("expected a document");

        assert_eq!(document.to_string(), "v0 : id:GcviwUsEr9Ji4rCrnnsgmVAghNaVPDumsfcagvBbBy45 owner_id:CisQdz2ej7EwWv8JbetSXBNsV4xsf8QsSS8tqp4tEf7V created_at:1970-01-14 21:20:00 updated_at:1970-01-14 21:20:00 avatarFingerprint:bytes d7b0e2b357c10312 avatarHash:bytes32 YonaRoE0hMgat53AYt5LTlQlIkKLReGpB7xNAqJ5HM8= avatarUrl:string http://test.com/bob.[...(23)] displayName:string QBwBNNXXYCngB0er publicMessage:string 8XG7KBGNvm2  ");

        fast_forward_to_block(&platform, 1_400_000_000, 901, 43, 1, false); //next epoch

        let platform_state = platform.state.load();

        let documents_batch_update_transition_1 =
            BatchTransition::new_document_replacement_transition_from_document(
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
            .expect("expect to create documents batch transition");

        let documents_batch_update_serialized_transition_1 = documents_batch_update_transition_1
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let documents_batch_update_transition_2 =
            BatchTransition::new_document_replacement_transition_from_document(
                altered_document_2,
                profile,
                &key,
                4,
                0,
                None,
                &signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

        let documents_batch_update_serialized_transition_2 = documents_batch_update_transition_2
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_update_serialized_transition_1.clone()],
                &platform_state,
                platform_state.last_block_info(),
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

        let query_sender_results = platform
            .drive
            .query_documents(query_documents.clone(), None, false, None, None)
            .expect("expected query result");

        let document = query_sender_results
            .documents()
            .first()
            .expect("expected a document");

        assert_eq!(document.to_string(), "v0 : id:GcviwUsEr9Ji4rCrnnsgmVAghNaVPDumsfcagvBbBy45 owner_id:CisQdz2ej7EwWv8JbetSXBNsV4xsf8QsSS8tqp4tEf7V created_at:1970-01-14 21:20:00 updated_at:1970-01-17 04:53:20 avatarFingerprint:bytes d7b0e2b357c10312 avatarHash:bytes32 YonaRoE0hMgat53AYt5LTlQlIkKLReGpB7xNAqJ5HM8= avatarUrl:string http://test.com/bob.[...(23)] displayName:string QBwBNNXXYCngB0er publicMessage:string 8XG7KBGNvm2  ");

        fast_forward_to_block(&platform, 1_600_000_000, 902, 44, 1, false); //next epoch

        let platform_state = platform.state.load();

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_update_serialized_transition_2.clone()],
                &platform_state,
                platform_state.last_block_info(),
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

        let query_sender_results = platform
            .drive
            .query_documents(query_documents.clone(), None, false, None, None)
            .expect("expected query result");

        let document = query_sender_results
            .documents()
            .first()
            .expect("expected a document");

        assert_eq!(document.to_string(), "v0 : id:GcviwUsEr9Ji4rCrnnsgmVAghNaVPDumsfcagvBbBy45 owner_id:CisQdz2ej7EwWv8JbetSXBNsV4xsf8QsSS8tqp4tEf7V created_at:1970-01-14 21:20:00 updated_at:1970-01-19 12:26:40 avatarFingerprint:bytes d7b0e2b357c10312 avatarHash:bytes32 YonaRoE0hMgat53AYt5LTlQlIkKLReGpB7xNAqJ5HM8= avatarUrl:string http://test.com/bob.[...(23)] displayName:string QBwBNNXXYCngB0er publicMessage:string 8XG7KBGNvm2  ");

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
    fn test_double_document_replace_different_height_different_epoch() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(433);

        fast_forward_to_block(&platform, 1_200_000_000, 900, 42, 1, false); //next epoch

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

        let mut altered_document = document.clone();

        altered_document.increment_revision().unwrap();
        altered_document.set("displayName", "Samuel".into());
        altered_document.set("avatarUrl", "http://test.com/cat.jpg".into());

        let mut altered_document_2 = altered_document.clone();

        altered_document_2.increment_revision().unwrap();
        altered_document_2.set("displayName", "Ody".into());
        altered_document_2.set("avatarUrl", "http://test.com/drapes.jpg".into());

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
                platform_state.last_block_info(),
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

        let receiver_documents_sql_string = "select * from profile".to_string();

        let query_documents = DriveDocumentQuery::from_sql_expr(
            receiver_documents_sql_string.as_str(),
            &dashpay,
            Some(&platform.config.drive),
        )
        .expect("expected document query");

        let query_sender_results = platform
            .drive
            .query_documents(query_documents.clone(), None, false, None, None)
            .expect("expected query result");

        let document = query_sender_results
            .documents()
            .first()
            .expect("expected a document");

        assert_eq!(document.to_string(), "v0 : id:GcviwUsEr9Ji4rCrnnsgmVAghNaVPDumsfcagvBbBy45 owner_id:CisQdz2ej7EwWv8JbetSXBNsV4xsf8QsSS8tqp4tEf7V created_at:1970-01-14 21:20:00 updated_at:1970-01-14 21:20:00 avatarFingerprint:bytes d7b0e2b357c10312 avatarHash:bytes32 YonaRoE0hMgat53AYt5LTlQlIkKLReGpB7xNAqJ5HM8= avatarUrl:string http://test.com/bob.[...(23)] displayName:string QBwBNNXXYCngB0er publicMessage:string 8XG7KBGNvm2  ");

        fast_forward_to_block(&platform, 1_400_000_000, 901, 43, 1, false); //next epoch

        let platform_state = platform.state.load();

        let documents_batch_update_transition_1 =
            BatchTransition::new_document_replacement_transition_from_document(
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
            .expect("expect to create documents batch transition");

        let documents_batch_update_serialized_transition_1 = documents_batch_update_transition_1
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let documents_batch_update_transition_2 =
            BatchTransition::new_document_replacement_transition_from_document(
                altered_document_2,
                profile,
                &key,
                4,
                0,
                None,
                &signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

        let documents_batch_update_serialized_transition_2 = documents_batch_update_transition_2
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_update_serialized_transition_1.clone()],
                &platform_state,
                platform_state.last_block_info(),
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

        let query_sender_results = platform
            .drive
            .query_documents(query_documents.clone(), None, false, None, None)
            .expect("expected query result");

        let document = query_sender_results
            .documents()
            .first()
            .expect("expected a document");

        assert_eq!(document.to_string(), "v0 : id:GcviwUsEr9Ji4rCrnnsgmVAghNaVPDumsfcagvBbBy45 owner_id:CisQdz2ej7EwWv8JbetSXBNsV4xsf8QsSS8tqp4tEf7V created_at:1970-01-14 21:20:00 updated_at:1970-01-17 04:53:20 avatarFingerprint:bytes d7b0e2b357c10312 avatarHash:bytes32 YonaRoE0hMgat53AYt5LTlQlIkKLReGpB7xNAqJ5HM8= avatarUrl:string http://test.com/cat.[...(23)] displayName:string Samuel publicMessage:string 8XG7KBGNvm2  ");

        fast_forward_to_block(&platform, 1_600_000_000, 905, 44, 2, true); //next epoch

        let platform_state = platform.state.load();

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_update_serialized_transition_2.clone()],
                &platform_state,
                platform_state.last_block_info(),
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

        let query_sender_results = platform
            .drive
            .query_documents(query_documents.clone(), None, false, None, None)
            .expect("expected query result");

        let document = query_sender_results
            .documents()
            .first()
            .expect("expected a document");

        assert_eq!(document.to_string(), "v0 : id:GcviwUsEr9Ji4rCrnnsgmVAghNaVPDumsfcagvBbBy45 owner_id:CisQdz2ej7EwWv8JbetSXBNsV4xsf8QsSS8tqp4tEf7V created_at:1970-01-14 21:20:00 updated_at:1970-01-19 12:26:40 avatarFingerprint:bytes d7b0e2b357c10312 avatarHash:bytes32 YonaRoE0hMgat53AYt5LTlQlIkKLReGpB7xNAqJ5HM8= avatarUrl:string http://test.com/drap[...(26)] displayName:string Ody publicMessage:string 8XG7KBGNvm2  ");

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
    fn test_document_replace_on_document_type_that_requires_a_token() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(433);

        let platform_state = platform.state.load();

        let (contract_owner_id, _, _) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

        let (creator, signer, key) = setup_identity(&mut platform, 234, dash_to_credits!(0.1));

        let (contract, gold_token_id, gas_token_id) =
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

        add_tokens_to_identity(&mut platform, gold_token_id, creator.id(), 15);
        add_tokens_to_identity(&mut platform, gas_token_id, creator.id(), 5);

        let card_document_type = contract
            .document_type_for_name("card")
            .expect("expected a profile document type");

        let entropy = Bytes32::random_with_rng(&mut rng);

        let mut document = card_document_type
            .random_document_with_identifier_and_entropy(
                &mut rng,
                creator.id(),
                entropy,
                DocumentFieldFillType::DoNotFillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        document.set("attack", 4.into());
        document.set("defense", 7.into());

        let mut altered_document = document.clone();

        altered_document.increment_revision().unwrap();
        altered_document.set("attack", 5.into());

        let documents_batch_create_transition =
            BatchTransition::new_document_creation_transition_from_document(
                document,
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

        let documents_batch_update_transition =
            BatchTransition::new_document_replacement_transition_from_document(
                altered_document,
                card_document_type,
                &key,
                3,
                0,
                Some(TokenPaymentInfo::V0(TokenPaymentInfoV0 {
                    payment_token_contract_id: None,
                    token_contract_position: 1,
                    minimum_token_cost: None,
                    maximum_token_cost: Some(2),
                    gas_fees_paid_by: Default::default(),
                })),
                &signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

        let documents_batch_update_serialized_transition = documents_batch_update_transition
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

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
        );

        let token_balance = platform
            .drive
            .fetch_identity_token_balance(
                gas_token_id.to_buffer(),
                creator.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");

        // He had 5, but spent 2
        assert_eq!(token_balance, Some(3));
    }
}
