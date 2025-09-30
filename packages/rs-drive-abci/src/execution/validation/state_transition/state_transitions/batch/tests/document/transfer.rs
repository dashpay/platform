use super::*;

mod transfer_tests {
    use super::*;
    use dpp::tokens::token_payment_info::v0::TokenPaymentInfoV0;
    use dpp::tokens::token_payment_info::TokenPaymentInfo;

    #[test]
    fn test_document_transfer_on_document_type_that_is_transferable_that_has_no_owner_indices() {
        let mut platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let card_game_path = "tests/supporting_files/contract/crypto-card-game/crypto-card-game-all-transferable-no-owner-indexes.json";

        let platform_state = platform.state.load();
        let platform_version = platform_state
            .current_platform_version()
            .expect("expected to get current platform version");

        // let's construct the grovedb structure for the card game data contract
        let contract = json_document_to_contract(card_game_path, true, platform_version)
            .expect("expected to get data contract");
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

        let mut rng = StdRng::seed_from_u64(433);

        let platform_state = platform.state.load();

        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

        let (receiver, _, _) = setup_identity(&mut platform, 450, dash_to_credits!(0.1));

        let card_document_type = contract
            .document_type_for_name("card")
            .expect("expected a profile document type");

        assert!(!card_document_type.documents_mutable());

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

        document.set_revision(Some(2));

        let documents_batch_transfer_transition =
            BatchTransition::new_document_transfer_transition_from_document(
                document,
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

        assert_eq!(processing_result.invalid_paid_count(), 0);

        assert_eq!(processing_result.invalid_unpaid_count(), 0);

        assert_eq!(processing_result.valid_count(), 1);

        assert_eq!(processing_result.aggregated_fees().storage_fee, 0); // There is no storage fee, as there are no indexes that will change

        assert_eq!(processing_result.aggregated_fees().processing_fee, 1985420);

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
    fn test_document_transfer_on_document_type_that_is_transferable_before_creator_id() {
        let platform_version = PlatformVersion::get(9).unwrap();

        let mut platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let card_game_path = "tests/supporting_files/contract/crypto-card-game/crypto-card-game-all-transferable-format-version-0.json";

        // let's construct the grovedb structure for the card game data contract
        let contract = json_document_to_contract(card_game_path, true, platform_version)
            .expect("expected to get data contract");

        assert!(contract.system_version_type() > 0);
        assert_eq!(contract.config().version(), 0); // this will cause us to serialize v0
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

        let mut rng = StdRng::seed_from_u64(433);

        let platform_state = platform.state.load();

        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

        let (receiver, _, _) = setup_identity(&mut platform, 450, dash_to_credits!(0.1));

        let card_document_type = contract
            .document_type_for_name("card")
            .expect("expected a profile document type");

        assert!(!card_document_type.documents_mutable());

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

        let documents_batch_transfer_transition =
            BatchTransition::new_document_transfer_transition_from_document(
                document,
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

        assert_eq!(processing_result.invalid_paid_count(), 0);

        assert_eq!(processing_result.invalid_unpaid_count(), 0);

        assert_eq!(processing_result.valid_count(), 1);

        assert_eq!(processing_result.aggregated_fees().storage_fee, 37341000); // 1383 bytes added

        // todo: we should expect these numbers to be closer

        assert_eq!(
            processing_result
                .aggregated_fees()
                .fee_refunds
                .calculate_refunds_amount_for_identity(identity.id()),
            Some(14992395)
        );

        assert_eq!(processing_result.aggregated_fees().processing_fee, 3369260);

        let query_sender_results = platform
            .drive
            .query_documents(query_sender_identity_documents, None, false, None, None)
            .expect("expected query result");

        let query_receiver_results = platform
            .drive
            .query_documents(query_receiver_identity_documents, None, false, None, None)
            .expect("expected query result");

        // We expect the sender to have no documents, and the receiver to have 1
        assert_eq!(query_sender_results.documents().len(), 0);

        assert_eq!(query_receiver_results.documents().len(), 1);

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
    fn test_document_transfer_on_document_type_that_is_transferable() {
        let platform_version = PlatformVersion::latest();
        let (mut platform, contract) = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure()
            .with_crypto_card_game_transfer_only(Transferable::Always);

        let mut rng = StdRng::seed_from_u64(433);

        let platform_state = platform.state.load();

        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

        let (receiver, _, _) = setup_identity(&mut platform, 450, dash_to_credits!(0.1));

        let card_document_type = contract
            .document_type_for_name("card")
            .expect("expected a profile document type");

        assert!(!card_document_type.documents_mutable());

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

        let creator_documents_sql_string =
            format!("select * from card where $creatorId == '{}'", identity.id());

        let query_creator_identity_documents = DriveDocumentQuery::from_sql_expr(
            creator_documents_sql_string.as_str(),
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

        let query_creator_results = platform
            .drive
            .query_documents(
                query_creator_identity_documents.clone(),
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

        // We expect 1 document by the creator id (sender)
        assert_eq!(query_creator_results.documents().len(), 1);

        document.set_revision(Some(2));

        let documents_batch_transfer_transition =
            BatchTransition::new_document_transfer_transition_from_document(
                document,
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

        assert_eq!(processing_result.invalid_paid_count(), 0);

        assert_eq!(processing_result.invalid_unpaid_count(), 0);

        assert_eq!(processing_result.valid_count(), 1);

        assert_eq!(processing_result.aggregated_fees().storage_fee, 37341000); // 1383 bytes added

        assert_eq!(
            processing_result
                .aggregated_fees()
                .fee_refunds
                .calculate_refunds_amount_for_identity(identity.id()),
            Some(14992395)
        );

        assert_eq!(processing_result.aggregated_fees().processing_fee, 3631040);

        let query_sender_results = platform
            .drive
            .query_documents(query_sender_identity_documents, None, false, None, None)
            .expect("expected query result");

        let query_creator_results = platform
            .drive
            .query_documents(query_creator_identity_documents, None, false, None, None)
            .expect("expected query result");

        let query_receiver_results = platform
            .drive
            .query_documents(query_receiver_identity_documents, None, false, None, None)
            .expect("expected query result");

        // We expect the sender to have no documents, and the receiver to have 1
        assert_eq!(query_sender_results.documents().len(), 0);

        assert_eq!(query_receiver_results.documents().len(), 1);

        assert_eq!(query_creator_results.documents().len(), 1);

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
    fn test_document_transfer_on_document_type_that_is_transferable_contract_v0() {
        // With a contract v0 we should not be adding the creator id
        // We do this because the creator id can not be serialized in document serialization v0
        // And document serialization v0 is necessary when the data contract is v0 or the data
        // contract config is v0.
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let card_game_path = "tests/supporting_files/contract/crypto-card-game/crypto-card-game-all-transferable-format-version-0.json";

        // let's construct the grovedb structure for the card game data contract
        let contract = json_document_to_contract(card_game_path, true, platform_version)
            .expect("expected to get data contract");

        assert!(contract.system_version_type() > 0);
        assert_eq!(contract.config().version(), 0); // this will cause us to serialize v0
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

        let mut rng = StdRng::seed_from_u64(433);

        let platform_state = platform.state.load();

        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

        let (receiver, _, _) = setup_identity(&mut platform, 450, dash_to_credits!(0.1));

        let card_document_type = contract
            .document_type_for_name("card")
            .expect("expected a profile document type");

        assert!(!card_document_type.documents_mutable());

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

        let documents_batch_transfer_transition =
            BatchTransition::new_document_transfer_transition_from_document(
                document,
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

        assert_eq!(processing_result.invalid_paid_count(), 0);

        assert_eq!(processing_result.invalid_unpaid_count(), 0);

        assert_eq!(processing_result.valid_count(), 1);

        assert_eq!(processing_result.aggregated_fees().storage_fee, 37341000); // 1383 bytes added

        assert_eq!(
            processing_result
                .aggregated_fees()
                .fee_refunds
                .calculate_refunds_amount_for_identity(identity.id()),
            Some(14992395)
        );

        assert_eq!(processing_result.aggregated_fees().processing_fee, 3369260);

        let query_sender_results = platform
            .drive
            .query_documents(query_sender_identity_documents, None, false, None, None)
            .expect("expected query result");

        let query_receiver_results = platform
            .drive
            .query_documents(query_receiver_identity_documents, None, false, None, None)
            .expect("expected query result");

        // We expect the sender to have no documents, and the receiver to have 1
        assert_eq!(query_sender_results.documents().len(), 0);

        assert_eq!(query_receiver_results.documents().len(), 1);

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
    fn test_document_transfer_on_document_type_that_is_not_transferable() {
        let platform_version = PlatformVersion::latest();
        let (mut platform, contract) = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure()
            .with_crypto_card_game_transfer_only(Transferable::Never);

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

        let documents_batch_transfer_transition =
            BatchTransition::new_document_transfer_transition_from_document(
                document,
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
    fn test_document_transfer_that_does_not_yet_exist() {
        let platform_version = PlatformVersion::latest();
        let (mut platform, contract) = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure()
            .with_crypto_card_game_transfer_only(Transferable::Never);

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

        // We expect the sender to have 0 documents, and the receiver to also have none
        assert_eq!(query_sender_results.documents().len(), 0);

        assert_eq!(query_receiver_results.documents().len(), 0);

        document.set_revision(Some(2));

        let documents_batch_transfer_transition =
            BatchTransition::new_document_transfer_transition_from_document(
                document,
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

        assert_eq!(processing_result.aggregated_fees().processing_fee, 36200);

        let query_sender_results = platform
            .drive
            .query_documents(query_sender_identity_documents, None, false, None, None)
            .expect("expected query result");

        let query_receiver_results = platform
            .drive
            .query_documents(query_receiver_identity_documents, None, false, None, None)
            .expect("expected query result");

        // We expect the sender to still have no document, and the receiver to have none as well
        assert_eq!(query_sender_results.documents().len(), 0);

        assert_eq!(query_receiver_results.documents().len(), 0);
    }

    #[test]
    fn test_document_delete_after_transfer() {
        let platform_version = PlatformVersion::latest();
        let (mut platform, contract) = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure()
            .with_crypto_card_game_transfer_only(Transferable::Always);

        let mut rng = StdRng::seed_from_u64(433);

        let platform_state = platform.state.load();

        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

        let (receiver, recipient_signer, recipient_key) =
            setup_identity(&mut platform, 450, dash_to_credits!(0.1));

        let card_document_type = contract
            .document_type_for_name("card")
            .expect("expected a profile document type");

        assert!(!card_document_type.documents_mutable());

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
                &BlockInfo::default_with_time(50000000),
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

        assert_eq!(processing_result.aggregated_fees().processing_fee, 3991900);

        let query_sender_results = platform
            .drive
            .query_documents(query_sender_identity_documents, None, false, None, None)
            .expect("expected query result");

        let query_receiver_results = platform
            .drive
            .query_documents(query_receiver_identity_documents, None, false, None, None)
            .expect("expected query result");

        // We expect the sender to have no documents, and the receiver to have 1
        assert_eq!(query_sender_results.documents().len(), 0);

        assert_eq!(query_receiver_results.documents().len(), 1);

        // Now let's try to delete the transferred document

        document.set_owner_id(receiver.id());

        let documents_batch_deletion_transition =
            BatchTransition::new_document_deletion_transition_from_document(
                document,
                card_document_type,
                &recipient_key,
                2,
                0,
                None,
                &recipient_signer,
                platform_version,
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

        assert_eq!(processing_result.aggregated_fees().processing_fee, 571240);

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
    fn test_document_transfer_on_document_that_needs_a_token() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(433);

        let platform_state = platform.state.load();

        let (contract_owner_id, _, _) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

        let (creator, signer, key) = setup_identity(&mut platform, 234, dash_to_credits!(0.1));

        let (receiver, _, _) = setup_identity(&mut platform, 450, dash_to_credits!(0.1));

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
                creator.id(),
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
            format!("select * from card where $ownerId == '{}'", creator.id());

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

        let documents_batch_transfer_transition =
            BatchTransition::new_document_transfer_transition_from_document(
                document,
                card_document_type,
                receiver.id(),
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

        let query_sender_results = platform
            .drive
            .query_documents(query_sender_identity_documents, None, false, None, None)
            .expect("expected query result");

        let query_receiver_results = platform
            .drive
            .query_documents(query_receiver_identity_documents, None, false, None, None)
            .expect("expected query result");

        // We expect the sender to have no documents, and the receiver to have 1
        assert_eq!(
            (
                query_sender_results.documents().len(),
                query_receiver_results.documents().len()
            ),
            (0, 1)
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

        // He had 5, but spent 1
        assert_eq!(token_balance, Some(4));
    }

    #[test]
    fn test_document_creator_id_unique_index_enforcement() {
        // This test verifies that a unique index on creator_id is properly enforced throughout
        // the complete document lifecycle, ensuring that only one document per creator can exist
        // at any time, regardless of ownership changes.
        //
        // ## Purpose
        // The creator_id field is immutable and set once at document creation. A unique index on
        // this field enforces a "one document per creator" constraint, which is useful for use
        // cases like:
        // - Digital collectibles where each creator can mint exactly one item
        // - Unique identity documents (e.g., verified credentials, certificates)
        // - Limited edition NFTs with single-creator constraints
        //
        // ## Why This Test Is Important
        // This test ensures that:
        // 1. The unique constraint prevents duplicate documents from the same creator
        // 2. The creator_id remains immutable during transfers (doesn't change with ownership)
        // 3. The unique constraint persists even after ownership transfers
        // 4. Only document deletion frees up the creator_id for potential reuse
        //
        // ## Test Scenario
        // This test uses a contract where the "card" document type has a unique index on $creatorId.
        // The test verifies the following sequence:
        //
        // 1. Creator creates first document → SUCCESS (creator_id slot is claimed)
        // 2. Creator tries to create second document → FAIL (creator_id already used)
        // 3. Document is transferred to receiver → SUCCESS (ownership changes, creator_id stays)
        // 4. Creator tries to create new document → FAIL (creator_id still claimed despite transfer)
        // 5. Receiver deletes the document → SUCCESS (creator_id is freed)
        // 6. Creator creates new document → SUCCESS (creator_id now available again)
        //
        // This proves that creator_id is truly immutable and the unique constraint works correctly
        // across transfers, only releasing when the document is deleted from the system entirely.
        let platform_version = PlatformVersion::latest();

        let mut platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let card_game_path = "tests/supporting_files/contract/crypto-card-game/crypto-card-game-all-transferable-unique-creator-id-index.json";

        // Load the contract with unique creator_id index
        let contract = json_document_to_contract(card_game_path, true, platform_version)
            .expect("expected to get data contract");

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

        let mut rng = StdRng::seed_from_u64(433);

        let platform_state = platform.state.load();

        // Setup two identities: creator and receiver
        let (creator, creator_signer, creator_key) =
            setup_identity(&mut platform, 958, dash_to_credits!(0.1));
        let (receiver, receiver_signer, receiver_key) =
            setup_identity(&mut platform, 450, dash_to_credits!(0.1));

        let card_document_type = contract
            .document_type_for_name("card")
            .expect("expected a card document type");

        assert!(!card_document_type.documents_mutable());

        // Step 1: Create first document by creator
        let entropy1 = Bytes32::random_with_rng(&mut rng);

        let mut document1 = card_document_type
            .random_document_with_identifier_and_entropy(
                &mut rng,
                creator.id(),
                entropy1,
                DocumentFieldFillType::DoNotFillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        document1.set("attack", 5.into());
        document1.set("defense", 8.into());

        let documents_batch_create_transition1 =
            BatchTransition::new_document_creation_transition_from_document(
                document1.clone(),
                card_document_type,
                entropy1.0,
                &creator_key,
                2,
                0,
                None,
                &creator_signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

        let documents_batch_create_serialized_transition1 = documents_batch_create_transition1
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_create_serialized_transition1.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        assert_eq!(processing_result.valid_count(), 1);
        assert_eq!(processing_result.invalid_paid_count(), 0);

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        // Step 2: Try to create a second document by the same creator
        // This should FAIL due to unique creator_id index
        let entropy2 = Bytes32::random_with_rng(&mut rng);

        let mut document2 = card_document_type
            .random_document_with_identifier_and_entropy(
                &mut rng,
                creator.id(),
                entropy2,
                DocumentFieldFillType::DoNotFillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        document2.set("attack", 3.into());
        document2.set("defense", 6.into());

        let documents_batch_create_transition2 =
            BatchTransition::new_document_creation_transition_from_document(
                document2.clone(),
                card_document_type,
                entropy2.0,
                &creator_key,
                3,
                0,
                None,
                &creator_signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

        let documents_batch_create_serialized_transition2 = documents_batch_create_transition2
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_create_serialized_transition2.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        // Should fail because creator already has a document (unique creator_id constraint)
        assert_eq!(processing_result.valid_count(), 0);
        assert_eq!(processing_result.invalid_paid_count(), 1);

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        // Step 3: Transfer the first document to the receiver
        document1.set_revision(Some(2));

        let documents_batch_transfer_transition =
            BatchTransition::new_document_transfer_transition_from_document(
                document1.clone(),
                card_document_type,
                receiver.id(),
                &creator_key,
                4,
                0,
                None,
                &creator_signer,
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

        assert_eq!(processing_result.valid_count(), 1);
        assert_eq!(processing_result.invalid_paid_count(), 0);

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        // Verify the document was transferred
        let receiver_documents_sql_string =
            format!("select * from card where $ownerId == '{}'", receiver.id());

        let query_receiver_identity_documents = DriveDocumentQuery::from_sql_expr(
            receiver_documents_sql_string.as_str(),
            &contract,
            Some(&platform.config.drive),
        )
        .expect("expected document query");

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

        assert_eq!(query_receiver_results.documents().len(), 1);

        // Step 4: Try to create a new document by the creator again after transfer
        // This should STILL FAIL because creator_id is immutable and still points to creator
        let entropy3 = Bytes32::random_with_rng(&mut rng);

        let mut document3 = card_document_type
            .random_document_with_identifier_and_entropy(
                &mut rng,
                creator.id(),
                entropy3,
                DocumentFieldFillType::DoNotFillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        document3.set("attack", 7.into());
        document3.set("defense", 4.into());

        let documents_batch_create_transition3 =
            BatchTransition::new_document_creation_transition_from_document(
                document3.clone(),
                card_document_type,
                entropy3.0,
                &creator_key,
                5,
                0,
                None,
                &creator_signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

        let documents_batch_create_serialized_transition3 = documents_batch_create_transition3
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_create_serialized_transition3.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        // Should still fail because creator_id is immutable and the unique constraint still applies
        assert_eq!(processing_result.valid_count(), 0);
        assert_eq!(processing_result.invalid_paid_count(), 1);

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        // Step 5: Receiver deletes the document
        document1.set_owner_id(receiver.id());
        document1.set_revision(Some(3));

        let documents_batch_deletion_transition =
            BatchTransition::new_document_deletion_transition_from_document(
                document1,
                card_document_type,
                &receiver_key,
                2,
                0,
                None,
                &receiver_signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch deletion transition");

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

        assert_eq!(processing_result.valid_count(), 1);
        assert_eq!(processing_result.invalid_paid_count(), 0);

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        // Verify the document was deleted
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

        assert_eq!(query_receiver_results.documents().len(), 0);

        // Step 6: Now creator should be able to create a new document
        // This should SUCCEED because the previous document with this creator_id was deleted
        let entropy4 = Bytes32::random_with_rng(&mut rng);

        let mut document4 = card_document_type
            .random_document_with_identifier_and_entropy(
                &mut rng,
                creator.id(),
                entropy4,
                DocumentFieldFillType::DoNotFillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        document4.set("attack", 9.into());
        document4.set("defense", 2.into());

        let documents_batch_create_transition4 =
            BatchTransition::new_document_creation_transition_from_document(
                document4.clone(),
                card_document_type,
                entropy4.0,
                &creator_key,
                6,
                0,
                None,
                &creator_signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

        let documents_batch_create_serialized_transition4 = documents_batch_create_transition4
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_create_serialized_transition4.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        // Should succeed now because the previous document was deleted
        assert_eq!(processing_result.valid_count(), 1);
        assert_eq!(processing_result.invalid_paid_count(), 0);

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        // Verify the new document was created
        let creator_documents_sql_string =
            format!("select * from card where $ownerId == '{}'", creator.id());

        let query_creator_identity_documents = DriveDocumentQuery::from_sql_expr(
            creator_documents_sql_string.as_str(),
            &contract,
            Some(&platform.config.drive),
        )
        .expect("expected document query");

        let query_creator_results = platform
            .drive
            .query_documents(query_creator_identity_documents, None, false, None, None)
            .expect("expected query result");

        assert_eq!(query_creator_results.documents().len(), 1);

        // Verify via creator_id query
        let creator_id_documents_sql_string =
            format!("select * from card where $creatorId == '{}'", creator.id());

        let query_creator_id_documents = DriveDocumentQuery::from_sql_expr(
            creator_id_documents_sql_string.as_str(),
            &contract,
            Some(&platform.config.drive),
        )
        .expect("expected document query");

        let query_creator_id_results = platform
            .drive
            .query_documents(query_creator_id_documents, None, false, None, None)
            .expect("expected query result");

        assert_eq!(query_creator_id_results.documents().len(), 1);

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
    fn test_document_owner_and_creator_id_unique_index_enforcement() {
        // This test verifies that a unique compound index on (owner_id, creator_id) is properly
        // enforced throughout the document lifecycle, allowing a creator to have multiple documents
        // but preventing duplicate (owner, creator) combinations.
        //
        // ## Purpose
        // A compound unique index on (owner_id, creator_id) creates a more flexible constraint than
        // a simple unique creator_id. It allows:
        // - The same creator to create multiple documents (unlike single creator_id uniqueness)
        // - Each owner to hold at most one document from any given creator
        // - Documents to move between owners while maintaining creator attribution
        //
        // This is useful for use cases like:
        // - Trading card games where one creator can make many cards, but each owner can only hold
        //   one card from that creator at a time
        // - Digital art where collectors can own one piece per artist
        // - Subscription or membership tokens where each user can have one active token per issuer
        //
        // ## Why This Test Is Important
        // This test ensures that:
        // 1. The compound constraint prevents duplicate (owner, creator) pairs
        // 2. Creators can create multiple documents when owned by different people
        // 3. Transfers can fail if they would create a duplicate (owner, creator) combination
        // 4. Transfers can succeed when they free up a (owner, creator) slot
        // 5. Deletion properly frees up the (owner, creator) constraint
        //
        // ## Test Scenario
        // This test uses a contract where the "card" document type has a unique compound index on
        // ($ownerId, $creatorId). The test verifies a complex sequence:
        //
        // 1. Creator creates document1 (owner=creator, creator=creator) → SUCCESS
        // 2. Creator tries to create document2 with same owner → FAIL (duplicate (creator, creator))
        // 3. Document1 transferred to receiver → SUCCESS (now (receiver, creator) exists)
        // 4. Creator creates document3 → SUCCESS ((creator, creator) is now available)
        // 5. Try to transfer document3 to receiver → FAIL ((receiver, creator) already exists)
        // 6. Receiver tries to transfer document1 back → FAIL ((creator, creator) occupied by document3)
        // 7. Creator deletes document3 → SUCCESS (frees (creator, creator))
        // 8. Receiver transfers document1 back to creator → SUCCESS ((creator, creator) now free)
        //
        // This proves that the compound unique index correctly tracks the combination of owner and
        // creator, allowing flexible document distribution while preventing conflicts.
        let platform_version = PlatformVersion::latest();

        let mut platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let card_game_path = "tests/supporting_files/contract/crypto-card-game/crypto-card-game-all-transferable-unique-creator-id-with-owner-id-index.json";

        // Load the contract with unique (owner_id, creator_id) compound index
        let contract = json_document_to_contract(card_game_path, true, platform_version)
            .expect("expected to get data contract");

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

        let mut rng = StdRng::seed_from_u64(433);

        let platform_state = platform.state.load();

        // Setup two identities: creator and receiver
        let (creator, creator_signer, creator_key) =
            setup_identity(&mut platform, 958, dash_to_credits!(0.1));
        let (receiver, receiver_signer, receiver_key) =
            setup_identity(&mut platform, 450, dash_to_credits!(0.1));

        let card_document_type = contract
            .document_type_for_name("card")
            .expect("expected a card document type");

        assert!(!card_document_type.documents_mutable());

        // Step 1: Creator creates first document (owner=creator, creator=creator)
        let entropy1 = Bytes32::random_with_rng(&mut rng);

        let mut document1 = card_document_type
            .random_document_with_identifier_and_entropy(
                &mut rng,
                creator.id(),
                entropy1,
                DocumentFieldFillType::DoNotFillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        document1.set("attack", 5.into());
        document1.set("defense", 8.into());

        let documents_batch_create_transition1 =
            BatchTransition::new_document_creation_transition_from_document(
                document1.clone(),
                card_document_type,
                entropy1.0,
                &creator_key,
                2,
                0,
                None,
                &creator_signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

        let documents_batch_create_serialized_transition1 = documents_batch_create_transition1
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_create_serialized_transition1.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        assert_eq!(processing_result.valid_count(), 1);
        assert_eq!(processing_result.invalid_paid_count(), 0);

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        // Step 2: Try to create second document by same creator with same owner
        // This should FAIL due to unique (owner_id, creator_id) constraint
        let entropy2 = Bytes32::random_with_rng(&mut rng);

        let mut document2 = card_document_type
            .random_document_with_identifier_and_entropy(
                &mut rng,
                creator.id(),
                entropy2,
                DocumentFieldFillType::DoNotFillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        document2.set("attack", 3.into());
        document2.set("defense", 6.into());

        let documents_batch_create_transition2 =
            BatchTransition::new_document_creation_transition_from_document(
                document2.clone(),
                card_document_type,
                entropy2.0,
                &creator_key,
                3,
                0,
                None,
                &creator_signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

        let documents_batch_create_serialized_transition2 = documents_batch_create_transition2
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_create_serialized_transition2.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        // Should fail because (creator, creator) combination already exists
        assert_eq!(processing_result.valid_count(), 0);
        assert_eq!(processing_result.invalid_paid_count(), 1);

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        // Step 3: Transfer first document to receiver (changes owner to receiver, creator stays same)
        // Now we have (owner=receiver, creator=creator)
        document1.set_revision(Some(2));

        let documents_batch_transfer_transition1 =
            BatchTransition::new_document_transfer_transition_from_document(
                document1.clone(),
                card_document_type,
                receiver.id(),
                &creator_key,
                4,
                0,
                None,
                &creator_signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition for transfer");

        let documents_batch_transfer_serialized_transition1 = documents_batch_transfer_transition1
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_transfer_serialized_transition1.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        assert_eq!(processing_result.valid_count(), 1);
        assert_eq!(processing_result.invalid_paid_count(), 0);

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        // Step 4: Now creator can create another document because (owner=creator, creator=creator) is available
        // This should SUCCEED
        let entropy3 = Bytes32::random_with_rng(&mut rng);

        let mut document3 = card_document_type
            .random_document_with_identifier_and_entropy(
                &mut rng,
                creator.id(),
                entropy3,
                DocumentFieldFillType::DoNotFillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        document3.set("attack", 7.into());
        document3.set("defense", 4.into());

        let documents_batch_create_transition3 =
            BatchTransition::new_document_creation_transition_from_document(
                document3.clone(),
                card_document_type,
                entropy3.0,
                &creator_key,
                5,
                0,
                None,
                &creator_signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

        let documents_batch_create_serialized_transition3 = documents_batch_create_transition3
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_create_serialized_transition3.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        // Should succeed because (creator, creator) is now available after transfer
        assert_eq!(processing_result.valid_count(), 1);
        assert_eq!(processing_result.invalid_paid_count(), 0);

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        // Step 5: Try to transfer document3 to receiver
        // This should FAIL because (receiver, creator) already exists from document1
        document3.set_revision(Some(2));

        let documents_batch_transfer_transition3 =
            BatchTransition::new_document_transfer_transition_from_document(
                document3.clone(),
                card_document_type,
                receiver.id(),
                &creator_key,
                6,
                0,
                None,
                &creator_signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition for transfer");

        let documents_batch_transfer_serialized_transition3 = documents_batch_transfer_transition3
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_transfer_serialized_transition3.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        // Should fail because (receiver, creator) combination already exists
        assert_eq!(processing_result.valid_count(), 0);
        assert_eq!(processing_result.invalid_paid_count(), 1);

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        // Step 6: Receiver tries to transfer document1 back to creator
        // This should FAIL because (creator, creator) now exists from document3
        document1.set_owner_id(receiver.id());
        document1.set_revision(Some(3));

        let documents_batch_transfer_transition_back =
            BatchTransition::new_document_transfer_transition_from_document(
                document1.clone(),
                card_document_type,
                creator.id(),
                &receiver_key,
                2,
                0,
                None,
                &receiver_signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition for transfer");

        let documents_batch_transfer_serialized_transition_back =
            documents_batch_transfer_transition_back
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_transfer_serialized_transition_back.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        // Should fail because (creator, creator) is occupied by document3
        assert_eq!(processing_result.valid_count(), 0);
        assert_eq!(processing_result.invalid_paid_count(), 1);

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        // Step 7: Delete document3 (which has owner=creator, creator=creator)
        document3.set_revision(Some(3));

        let documents_batch_deletion_transition =
            BatchTransition::new_document_deletion_transition_from_document(
                document3,
                card_document_type,
                &creator_key,
                7,
                0,
                None,
                &creator_signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch deletion transition");

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

        assert_eq!(processing_result.valid_count(), 1);
        assert_eq!(processing_result.invalid_paid_count(), 0);

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        // Step 8: Now receiver can transfer document1 back to creator
        // This should SUCCEED because (creator, creator) is now available
        document1.set_revision(Some(4));

        let documents_batch_transfer_transition_back2 =
            BatchTransition::new_document_transfer_transition_from_document(
                document1.clone(),
                card_document_type,
                creator.id(),
                &receiver_key,
                3,
                0,
                None,
                &receiver_signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition for transfer");

        let documents_batch_transfer_serialized_transition_back2 =
            documents_batch_transfer_transition_back2
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_transfer_serialized_transition_back2.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        // Should succeed now because (creator, creator) is available after deletion
        assert_eq!(processing_result.valid_count(), 1);
        assert_eq!(processing_result.invalid_paid_count(), 0);

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        // Verify final state: creator has the document back
        let creator_documents_sql_string =
            format!("select * from card where $ownerId == '{}'", creator.id());

        let query_creator_identity_documents = DriveDocumentQuery::from_sql_expr(
            creator_documents_sql_string.as_str(),
            &contract,
            Some(&platform.config.drive),
        )
        .expect("expected document query");

        let query_creator_results = platform
            .drive
            .query_documents(query_creator_identity_documents, None, false, None, None)
            .expect("expected query result");

        assert_eq!(query_creator_results.documents().len(), 1);

        let receiver_documents_sql_string =
            format!("select * from card where $ownerId == '{}'", receiver.id());

        let query_receiver_identity_documents = DriveDocumentQuery::from_sql_expr(
            receiver_documents_sql_string.as_str(),
            &contract,
            Some(&platform.config.drive),
        )
        .expect("expected document query");

        let query_receiver_results = platform
            .drive
            .query_documents(query_receiver_identity_documents, None, false, None, None)
            .expect("expected query result");

        assert_eq!(query_receiver_results.documents().len(), 0);

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
}
