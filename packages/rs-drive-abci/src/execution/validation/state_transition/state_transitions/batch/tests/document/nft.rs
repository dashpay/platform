use super::*;

mod nft_tests {
    use super::*;
    use crate::test::helpers::fast_forward_to_block::fast_forward_to_block;
    use dpp::tokens::token_payment_info::v0::TokenPaymentInfoV0;
    use dpp::tokens::token_payment_info::TokenPaymentInfo;
    #[test]
    fn test_document_set_price_on_document_without_ability_to_purchase() {
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

        // We expect the sender to have 1 document, and the receiver to have none
        assert_eq!(query_sender_results.documents().len(), 1);

        document.set_revision(Some(2));

        let documents_batch_update_price_transition =
            BatchTransition::new_document_update_price_transition_from_document(
                document.clone(),
                card_document_type,
                dash_to_credits!(0.1),
                &key,
                3,
                0,
                None,
                &signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition for the update price");

        let documents_batch_transfer_serialized_transition =
            documents_batch_update_price_transition
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

        let result = processing_result.into_execution_results().remove(0);

        let StateTransitionExecutionResult::PaidConsensusError(consensus_error, _) = result else {
            panic!("expected a paid consensus error");
        };
        assert_eq!(consensus_error.to_string(), "Document transition action card is in trade mode No Trading that does not support the seller setting the price is not supported");
    }

    #[test]
    fn test_document_set_price() {
        let platform_version = PlatformVersion::latest();
        let (mut platform, contract) = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure()
            .with_crypto_card_game_nft(TradeMode::DirectPurchase);

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

        let documents_batch_update_price_transition =
            BatchTransition::new_document_update_price_transition_from_document(
                document.clone(),
                card_document_type,
                dash_to_credits!(0.1),
                &key,
                3,
                0,
                None,
                &signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition for the update price");

        let documents_batch_transfer_serialized_transition =
            documents_batch_update_price_transition
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

        assert_eq!(processing_result.aggregated_fees().processing_fee, 2473880);

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

        // The sender document should have the desired price

        let document = query_sender_results.documents().first().unwrap();

        let price: Credits = document
            .properties()
            .get_integer("$price")
            .expect("expected to get back price");

        assert_eq!(dash_to_credits!(0.1), price);

        assert_eq!(document.revision(), Some(2));
    }

    #[test]
    fn test_document_set_price_and_purchase() {
        let platform_version = PlatformVersion::latest();
        let (mut platform, contract) = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure()
            .with_crypto_card_game_nft(TradeMode::DirectPurchase);

        let mut rng = StdRng::seed_from_u64(433);

        let platform_state = platform.state.load();

        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

        let (purchaser, recipient_signer, recipient_key) =
            setup_identity(&mut platform, 450, dash_to_credits!(1.0));

        let seller_balance = platform
            .drive
            .fetch_identity_balance(identity.id().to_buffer(), None, platform_version)
            .expect("expected to get identity balance")
            .expect("expected that identity exists");

        assert_eq!(seller_balance, dash_to_credits!(0.1));

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

        assert_eq!(
            processing_result
                .aggregated_fees()
                .clone()
                .into_balance_change(identity.id())
                .change(),
            &BalanceChange::RemoveFromBalance {
                required_removed_balance: 123579000,
                desired_removed_balance: 126435860,
            }
        );

        let original_creation_cost = 126435860;

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let seller_balance = platform
            .drive
            .fetch_identity_balance(identity.id().to_buffer(), None, platform_version)
            .expect("expected to get identity balance")
            .expect("expected that identity exists");

        // the seller should have received 0.1 and already had 0.1 minus the processing fee and storage fee
        assert_eq!(
            seller_balance,
            dash_to_credits!(0.1) - original_creation_cost
        );

        let sender_documents_sql_string =
            format!("select * from card where $ownerId == '{}'", identity.id());

        let query_sender_identity_documents = DriveDocumentQuery::from_sql_expr(
            sender_documents_sql_string.as_str(),
            &contract,
            Some(&platform.config.drive),
        )
        .expect("expected document query");

        let receiver_documents_sql_string =
            format!("select * from card where $ownerId == '{}'", purchaser.id());

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

        let documents_batch_update_price_transition =
            BatchTransition::new_document_update_price_transition_from_document(
                document.clone(),
                card_document_type,
                dash_to_credits!(0.1),
                &key,
                3,
                0,
                None,
                &signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition for the update price");

        let documents_batch_transfer_serialized_transition =
            documents_batch_update_price_transition
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

        assert_eq!(processing_result.aggregated_fees().storage_fee, 216000); // we added 8 bytes for the price

        assert_eq!(
            processing_result
                .aggregated_fees()
                .fee_refunds
                .calculate_refunds_amount_for_identity(identity.id()),
            None
        );

        assert_eq!(processing_result.aggregated_fees().processing_fee, 2473880);

        let seller_balance = platform
            .drive
            .fetch_identity_balance(identity.id().to_buffer(), None, platform_version)
            .expect("expected to get identity balance")
            .expect("expected that identity exists");

        // the seller should have received 0.1 and already had 0.1 minus the processing fee and storage fee
        assert_eq!(
            seller_balance,
            dash_to_credits!(0.1) - original_creation_cost - 2689880
        );

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

        // We expect the sender to still have their document, and the receiver to have none
        assert_eq!(query_sender_results.documents().len(), 1);

        assert_eq!(query_receiver_results.documents().len(), 0);

        // The sender document should have the desired price

        let mut document = query_sender_results.documents_owned().remove(0);

        let price: Credits = document
            .properties()
            .get_integer("$price")
            .expect("expected to get back price");

        assert_eq!(dash_to_credits!(0.1), price);

        // At this point we want to have the receiver purchase the document

        document.set_revision(Some(3));

        let documents_batch_purchase_transition =
            BatchTransition::new_document_purchase_transition_from_document(
                document.clone(),
                card_document_type,
                purchaser.id(),
                dash_to_credits!(0.1), //same price as requested
                &recipient_key,
                1, // 1 because he's never done anything
                0,
                None,
                &recipient_signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition for the purchase");

        let documents_batch_purchase_serialized_transition = documents_batch_purchase_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_purchase_serialized_transition],
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

        assert_eq!(processing_result.aggregated_fees().storage_fee, 64611000);

        assert_eq!(processing_result.aggregated_fees().processing_fee, 4080480);

        assert_eq!(
            processing_result
                .aggregated_fees()
                .fee_refunds
                .calculate_refunds_amount_for_identity(identity.id()),
            Some(22704503)
        );

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

        let seller_balance = platform
            .drive
            .fetch_identity_balance(identity.id().to_buffer(), None, platform_version)
            .expect("expected to get identity balance")
            .expect("expected that identity exists");

        // the seller should have received 0.1 and already had 0.1 minus the processing fee and storage fee
        assert_eq!(
            seller_balance,
            dash_to_credits!(0.2) - original_creation_cost + 20014623
        );

        let buyers_balance = platform
            .drive
            .fetch_identity_balance(purchaser.id().to_buffer(), None, platform_version)
            .expect("expected to get purchaser balance")
            .expect("expected that purchaser exists");

        // the buyer paid 0.1, but also storage and processing fees
        assert_eq!(buyers_balance, dash_to_credits!(0.9) - 68691480);
    }

    #[test]
    fn test_document_set_price_and_purchase_different_epoch_documents_mutable() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let card_game_path = "tests/supporting_files/contract/crypto-card-game/crypto-card-game-direct-purchase-documents-mutable.json";

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

        let (purchaser, recipient_signer, recipient_key) =
            setup_identity(&mut platform, 450, dash_to_credits!(1.0));

        let seller_balance = platform
            .drive
            .fetch_identity_balance(identity.id().to_buffer(), None, platform_version)
            .expect("expected to get identity balance")
            .expect("expected that identity exists");

        assert_eq!(seller_balance, dash_to_credits!(0.1));

        let card_document_type = contract
            .document_type_for_name("card")
            .expect("expected a profile document type");

        assert!(card_document_type.documents_mutable());

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

        assert_eq!(
            processing_result
                .aggregated_fees()
                .clone()
                .into_balance_change(identity.id())
                .change(),
            &BalanceChange::RemoveFromBalance {
                required_removed_balance: 138159000,
                desired_removed_balance: 141234660,
            }
        );

        let original_creation_cost = 141234660;

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let seller_balance = platform
            .drive
            .fetch_identity_balance(identity.id().to_buffer(), None, platform_version)
            .expect("expected to get identity balance")
            .expect("expected that identity exists");

        // the seller already had 0.1 minus the processing fee and storage fee
        assert_eq!(
            seller_balance,
            dash_to_credits!(0.1) - original_creation_cost
        );

        let sender_documents_sql_string =
            format!("select * from card where $ownerId == '{}'", identity.id());

        let query_sender_identity_documents = DriveDocumentQuery::from_sql_expr(
            sender_documents_sql_string.as_str(),
            &contract,
            Some(&platform.config.drive),
        )
        .expect("expected document query");

        let receiver_documents_sql_string =
            format!("select * from card where $ownerId == '{}'", purchaser.id());

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

        // now let's modify the document

        fast_forward_to_block(&platform, 500_000, 100, 3, 1, false); //next epoch

        document.set("description", "chopsticks".into());
        document.bump_revision();

        let documents_batch_update_transition =
            BatchTransition::new_document_replacement_transition_from_document(
                document.clone(),
                card_document_type,
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

        assert_eq!(
            processing_result.invalid_paid_count(),
            0,
            "{:?}",
            processing_result.execution_results()
        );

        assert_eq!(
            processing_result.invalid_unpaid_count(),
            0,
            "{:?}",
            processing_result.execution_results()
        );

        assert_eq!(
            processing_result.valid_count(),
            1,
            "{:?}",
            processing_result.execution_results()
        );

        assert_eq!(processing_result.aggregated_fees().storage_fee, 378000);

        assert_eq!(
            processing_result
                .aggregated_fees()
                .fee_refunds
                .calculate_refunds_amount_for_identity(identity.id()),
            None
        );

        assert_eq!(processing_result.aggregated_fees().processing_fee, 2717400);

        let seller_balance = platform
            .drive
            .fetch_identity_balance(identity.id().to_buffer(), None, platform_version)
            .expect("expected to get identity balance")
            .expect("expected that identity exists");

        // the seller should have received 0.1 and already had 0.1 minus the processing fee and storage fee
        assert_eq!(
            seller_balance,
            dash_to_credits!(0.1) - original_creation_cost - 2717400 - 378000
        );

        // now let's update price, but first go to next epoch

        fast_forward_to_block(&platform, 1_200_000_000, 900, 42, 2, false); //next epoch

        document.bump_revision();

        let documents_batch_update_price_transition =
            BatchTransition::new_document_update_price_transition_from_document(
                document.clone(),
                card_document_type,
                dash_to_credits!(0.1),
                &key,
                4,
                0,
                None,
                &signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition for the update price");

        let documents_batch_transfer_serialized_transition =
            documents_batch_update_price_transition
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

        assert_eq!(
            processing_result.invalid_paid_count(),
            0,
            "{:?}",
            processing_result.execution_results()
        );

        assert_eq!(
            processing_result.invalid_unpaid_count(),
            0,
            "{:?}",
            processing_result.execution_results()
        );

        assert_eq!(processing_result.valid_count(), 1);

        assert_eq!(processing_result.aggregated_fees().storage_fee, 216000); // we added 8 bytes for the price

        assert_eq!(
            processing_result
                .aggregated_fees()
                .fee_refunds
                .calculate_refunds_amount_for_identity(identity.id()),
            None
        );

        assert_eq!(processing_result.aggregated_fees().processing_fee, 2721160);

        let seller_balance = platform
            .drive
            .fetch_identity_balance(identity.id().to_buffer(), None, platform_version)
            .expect("expected to get identity balance")
            .expect("expected that identity exists");

        // the seller should have received 0.1 and already had 0.1 minus the processing fee and storage fee
        assert_eq!(
            seller_balance,
            dash_to_credits!(0.1) - original_creation_cost - 2717400 - 378000 - 2721160 - 216000
        );

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

        // We expect the sender to still have their document, and the receiver to have none
        assert_eq!(query_sender_results.documents().len(), 1);

        assert_eq!(query_receiver_results.documents().len(), 0);

        // The sender document should have the desired price

        let mut document = query_sender_results.documents_owned().remove(0);

        let price: Credits = document
            .properties()
            .get_integer("$price")
            .expect("expected to get back price");

        assert_eq!(dash_to_credits!(0.1), price);

        // At this point we want to have the receiver purchase the document at the next epoch

        fast_forward_to_block(&platform, 1_700_000_000, 1200, 42, 3, false); //next epoch

        document.bump_revision();

        let documents_batch_purchase_transition =
            BatchTransition::new_document_purchase_transition_from_document(
                document.clone(),
                card_document_type,
                purchaser.id(),
                dash_to_credits!(0.1), //same price as requested
                &recipient_key,
                1, // 1 because he's never done anything
                0,
                None,
                &recipient_signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition for the purchase");

        let documents_batch_purchase_serialized_transition = documents_batch_purchase_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_purchase_serialized_transition],
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

        assert_eq!(
            processing_result.invalid_paid_count(),
            0,
            "{:?}",
            processing_result.execution_results()
        );

        assert_eq!(
            processing_result.invalid_unpaid_count(),
            0,
            "{:?}",
            processing_result.execution_results()
        );

        assert_eq!(
            processing_result.valid_count(),
            1,
            "{:?}",
            processing_result.execution_results()
        );

        assert_eq!(processing_result.aggregated_fees().storage_fee, 64611000);

        assert_eq!(processing_result.aggregated_fees().processing_fee, 4345280);

        assert_eq!(
            processing_result
                .aggregated_fees()
                .fee_refunds
                .calculate_refunds_amount_for_identity(identity.id()),
            Some(52987722)
        );

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

        let seller_balance = platform
            .drive
            .fetch_identity_balance(identity.id().to_buffer(), None, platform_version)
            .expect("expected to get identity balance")
            .expect("expected that identity exists");

        // the seller should have received 0.1 and already had 0.1 minus the processing fee and storage fee
        assert_eq!(
            seller_balance,
            dash_to_credits!(0.2) - original_creation_cost + 46955162
        );

        let buyers_balance = platform
            .drive
            .fetch_identity_balance(purchaser.id().to_buffer(), None, platform_version)
            .expect("expected to get purchaser balance")
            .expect("expected that purchaser exists");

        // the buyer paid 0.1, but also storage and processing fees
        assert_eq!(buyers_balance, dash_to_credits!(0.9) - 68956280);
    }

    #[test]
    fn test_document_set_price_and_purchase_different_epoch() {
        let platform_version = PlatformVersion::latest();
        let (mut platform, contract) = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure()
            .with_crypto_card_game_nft(TradeMode::DirectPurchase);

        let mut rng = StdRng::seed_from_u64(433);

        let platform_state = platform.state.load();

        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

        let (purchaser, recipient_signer, recipient_key) =
            setup_identity(&mut platform, 450, dash_to_credits!(1.0));

        let seller_balance = platform
            .drive
            .fetch_identity_balance(identity.id().to_buffer(), None, platform_version)
            .expect("expected to get identity balance")
            .expect("expected that identity exists");

        assert_eq!(seller_balance, dash_to_credits!(0.1));

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

        assert_eq!(
            processing_result
                .aggregated_fees()
                .clone()
                .into_balance_change(identity.id())
                .change(),
            &BalanceChange::RemoveFromBalance {
                required_removed_balance: 123579000,
                desired_removed_balance: 126435860,
            }
        );

        let original_creation_cost = 126435860;

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let seller_balance = platform
            .drive
            .fetch_identity_balance(identity.id().to_buffer(), None, platform_version)
            .expect("expected to get identity balance")
            .expect("expected that identity exists");

        // the seller already had 0.1 minus the processing fee and storage fee
        assert_eq!(
            seller_balance,
            dash_to_credits!(0.1) - original_creation_cost
        );

        let sender_documents_sql_string =
            format!("select * from card where $ownerId == '{}'", identity.id());

        let query_sender_identity_documents = DriveDocumentQuery::from_sql_expr(
            sender_documents_sql_string.as_str(),
            &contract,
            Some(&platform.config.drive),
        )
        .expect("expected document query");

        let receiver_documents_sql_string =
            format!("select * from card where $ownerId == '{}'", purchaser.id());

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

        // now let's update price, but first go to next epoch

        fast_forward_to_block(&platform, 1_200_000_000, 900, 42, 1, false); //next epoch

        document.set_revision(Some(2));

        let documents_batch_update_price_transition =
            BatchTransition::new_document_update_price_transition_from_document(
                document.clone(),
                card_document_type,
                dash_to_credits!(0.1),
                &key,
                3,
                0,
                None,
                &signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition for the update price");

        let documents_batch_transfer_serialized_transition =
            documents_batch_update_price_transition
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

        assert_eq!(processing_result.aggregated_fees().storage_fee, 216000); // we added 8 bytes for the price

        assert_eq!(
            processing_result
                .aggregated_fees()
                .fee_refunds
                .calculate_refunds_amount_for_identity(identity.id()),
            None
        );

        assert_eq!(processing_result.aggregated_fees().processing_fee, 2473880);

        let seller_balance = platform
            .drive
            .fetch_identity_balance(identity.id().to_buffer(), None, platform_version)
            .expect("expected to get identity balance")
            .expect("expected that identity exists");

        // the seller should have received 0.1 and already had 0.1 minus the processing fee and storage fee
        assert_eq!(
            seller_balance,
            dash_to_credits!(0.1) - original_creation_cost - 2689880
        );

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

        // We expect the sender to still have their document, and the receiver to have none
        assert_eq!(query_sender_results.documents().len(), 1);

        assert_eq!(query_receiver_results.documents().len(), 0);

        // The sender document should have the desired price

        let mut document = query_sender_results.documents_owned().remove(0);

        let price: Credits = document
            .properties()
            .get_integer("$price")
            .expect("expected to get back price");

        assert_eq!(dash_to_credits!(0.1), price);

        // At this point we want to have the receiver purchase the document at the next epoch

        fast_forward_to_block(&platform, 1_700_000_000, 1200, 42, 2, false); //next epoch

        document.set_revision(Some(3));

        let documents_batch_purchase_transition =
            BatchTransition::new_document_purchase_transition_from_document(
                document.clone(),
                card_document_type,
                purchaser.id(),
                dash_to_credits!(0.1), //same price as requested
                &recipient_key,
                1, // 1 because he's never done anything
                0,
                None,
                &recipient_signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition for the purchase");

        let documents_batch_purchase_serialized_transition = documents_batch_purchase_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_purchase_serialized_transition],
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

        assert_eq!(processing_result.aggregated_fees().storage_fee, 64611000);

        assert_eq!(processing_result.aggregated_fees().processing_fee, 4080480);

        assert_eq!(
            processing_result
                .aggregated_fees()
                .fee_refunds
                .calculate_refunds_amount_for_identity(identity.id()),
            Some(22704503)
        );

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

        let seller_balance = platform
            .drive
            .fetch_identity_balance(identity.id().to_buffer(), None, platform_version)
            .expect("expected to get identity balance")
            .expect("expected that identity exists");

        // the seller should have received 0.1 and already had 0.1 minus the processing fee and storage fee
        assert_eq!(
            seller_balance,
            dash_to_credits!(0.2) - original_creation_cost + 20014623
        );

        let buyers_balance = platform
            .drive
            .fetch_identity_balance(purchaser.id().to_buffer(), None, platform_version)
            .expect("expected to get purchaser balance")
            .expect("expected that purchaser exists");

        // the buyer paid 0.1, but also storage and processing fees
        assert_eq!(buyers_balance, dash_to_credits!(0.9) - 68691480);
    }

    #[test]
    fn test_document_set_price_and_try_purchase_at_different_amount() {
        let platform_version = PlatformVersion::latest();
        let (mut platform, contract) = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure()
            .with_crypto_card_game_nft(TradeMode::DirectPurchase);

        let mut rng = StdRng::seed_from_u64(433);

        let platform_state = platform.state.load();

        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

        let (purchaser, recipient_signer, recipient_key) =
            setup_identity(&mut platform, 450, dash_to_credits!(1.0));

        let seller_balance = platform
            .drive
            .fetch_identity_balance(identity.id().to_buffer(), None, platform_version)
            .expect("expected to get identity balance")
            .expect("expected that identity exists");

        assert_eq!(seller_balance, dash_to_credits!(0.1));

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

        let documents_batch_update_price_transition =
            BatchTransition::new_document_update_price_transition_from_document(
                document.clone(),
                card_document_type,
                dash_to_credits!(0.5),
                &key,
                3,
                0,
                None,
                &signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition for the update price");

        let documents_batch_transfer_serialized_transition =
            documents_batch_update_price_transition
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

        assert_eq!(processing_result.valid_count(), 1);

        // At this point we want to have the receiver purchase the document

        document.set_revision(Some(3));

        let documents_batch_purchase_transition =
            BatchTransition::new_document_purchase_transition_from_document(
                document.clone(),
                card_document_type,
                purchaser.id(),
                dash_to_credits!(0.35), //different than requested price
                &recipient_key,
                1, // 1 because he's never done anything
                0,
                None,
                &recipient_signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition for the purchase");

        let documents_batch_purchase_serialized_transition = documents_batch_purchase_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_purchase_serialized_transition],
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

        assert_eq!(processing_result.invalid_paid_count(), 1);

        let result = processing_result.into_execution_results().remove(0);

        let StateTransitionExecutionResult::PaidConsensusError(consensus_error, _) = result else {
            panic!("expected a paid consensus error");
        };
        assert_eq!(consensus_error.to_string(), "5rJccTdtJfg6AxSKyrptWUug3PWjveEitTTLqBn9wHdk document can not be purchased for 35000000000, it's sale price is 50000000000 (in credits)");
    }

    #[test]
    fn test_document_set_price_and_purchase_from_ones_self() {
        let platform_version = PlatformVersion::latest();
        let (mut platform, contract) = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure()
            .with_crypto_card_game_nft(TradeMode::DirectPurchase);

        let mut rng = StdRng::seed_from_u64(433);

        let platform_state = platform.state.load();

        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.5));

        let seller_balance = platform
            .drive
            .fetch_identity_balance(identity.id().to_buffer(), None, platform_version)
            .expect("expected to get identity balance")
            .expect("expected that identity exists");

        assert_eq!(seller_balance, dash_to_credits!(0.5));

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

        let documents_batch_update_price_transition =
            BatchTransition::new_document_update_price_transition_from_document(
                document.clone(),
                card_document_type,
                dash_to_credits!(0.1),
                &key,
                3,
                0,
                None,
                &signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition for the update price");

        let documents_batch_transfer_serialized_transition =
            documents_batch_update_price_transition
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

        assert_eq!(processing_result.valid_count(), 1);

        // At this point we want to have the receiver purchase the document

        document.set_revision(Some(3));

        let documents_batch_purchase_transition =
            BatchTransition::new_document_purchase_transition_from_document(
                document.clone(),
                card_document_type,
                identity.id(),
                dash_to_credits!(0.1), //same price as requested
                &key,
                1, // 1 because he's never done anything
                0,
                None,
                &signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition for the purchase");

        let documents_batch_purchase_serialized_transition = documents_batch_purchase_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_purchase_serialized_transition],
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

        assert_eq!(processing_result.invalid_paid_count(), 1);

        let result = processing_result.into_execution_results().remove(0);

        let StateTransitionExecutionResult::PaidConsensusError(consensus_error, _) = result else {
            panic!("expected a paid consensus error");
        };
        assert_eq!(consensus_error.to_string(), "Document transition action on document type: card identity trying to purchase a document that is already owned by the purchaser is not supported");
    }

    #[test]
    fn test_document_set_price_and_purchase_then_try_buy_back() {
        // In this test we try to buy back a document after it has been sold

        let platform_version = PlatformVersion::latest();
        let (mut platform, contract) = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure()
            .with_crypto_card_game_nft(TradeMode::DirectPurchase);

        let mut rng = StdRng::seed_from_u64(433);

        let platform_state = platform.state.load();

        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

        let (purchaser, recipient_signer, recipient_key) =
            setup_identity(&mut platform, 450, dash_to_credits!(1.0));

        let seller_balance = platform
            .drive
            .fetch_identity_balance(identity.id().to_buffer(), None, platform_version)
            .expect("expected to get identity balance")
            .expect("expected that identity exists");

        assert_eq!(seller_balance, dash_to_credits!(0.1));

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

        let documents_batch_update_price_transition =
            BatchTransition::new_document_update_price_transition_from_document(
                document.clone(),
                card_document_type,
                dash_to_credits!(0.1),
                &key,
                3,
                0,
                None,
                &signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition for the update price");

        let documents_batch_transfer_serialized_transition =
            documents_batch_update_price_transition
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

        assert_eq!(processing_result.valid_count(), 1);

        // At this point we want to have the receiver purchase the document

        document.set_revision(Some(3));

        let documents_batch_purchase_transition =
            BatchTransition::new_document_purchase_transition_from_document(
                document.clone(),
                card_document_type,
                purchaser.id(),
                dash_to_credits!(0.1), //same price as requested
                &recipient_key,
                1, // 1 because he's never done anything
                0,
                None,
                &recipient_signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition for the purchase");

        let documents_batch_purchase_serialized_transition = documents_batch_purchase_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_purchase_serialized_transition],
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

        assert_eq!(processing_result.valid_count(), 1);

        // Let's verify some stuff

        let sender_documents_sql_string =
            format!("select * from card where $ownerId == '{}'", identity.id());

        let query_sender_identity_documents = DriveDocumentQuery::from_sql_expr(
            sender_documents_sql_string.as_str(),
            &contract,
            Some(&platform.config.drive),
        )
        .expect("expected document query");

        let receiver_documents_sql_string =
            format!("select * from card where $ownerId == '{}'", purchaser.id());

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

        // We expect the sender to still have their document, and the receiver to have none
        assert_eq!(query_sender_results.documents().len(), 0);

        assert_eq!(query_receiver_results.documents().len(), 1);

        // The sender document should have the desired price

        let mut document = query_receiver_results.documents_owned().remove(0);

        let price: Option<Credits> = document
            .properties()
            .get_optional_integer("$price")
            .expect("expected to get back price");

        assert_eq!(price, None);

        assert_eq!(document.owner_id(), purchaser.id());

        // At this point we want to have the sender to try to buy back the document

        document.set_revision(Some(4));

        let documents_batch_purchase_transition =
            BatchTransition::new_document_purchase_transition_from_document(
                document.clone(),
                card_document_type,
                identity.id(),
                dash_to_credits!(0.1), //same price as old requested
                &key,
                4, // 1 because he's never done anything
                0,
                None,
                &signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition for the purchase");

        let documents_batch_purchase_serialized_transition = documents_batch_purchase_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_purchase_serialized_transition],
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

        assert_eq!(processing_result.invalid_paid_count(), 1);

        let result = processing_result.into_execution_results().remove(0);

        let StateTransitionExecutionResult::PaidConsensusError(consensus_error, _) = result else {
            panic!("expected a paid consensus error");
        };
        assert_eq!(
            consensus_error.to_string(),
            "5rJccTdtJfg6AxSKyrptWUug3PWjveEitTTLqBn9wHdk document not for sale"
        );
    }

    #[test]
    fn test_document_set_price_and_purchase_with_enough_credits_to_buy_but_not_enough_to_pay_for_processing(
    ) {
        let platform_version = PlatformVersion::latest();
        let (mut platform, contract) = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure()
            .with_crypto_card_game_nft(TradeMode::DirectPurchase);

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

        let documents_batch_update_price_transition =
            BatchTransition::new_document_update_price_transition_from_document(
                document.clone(),
                card_document_type,
                dash_to_credits!(0.1),
                &key,
                3,
                0,
                None,
                &signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition for the update price");

        let documents_batch_transfer_serialized_transition =
            documents_batch_update_price_transition
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

        assert_eq!(processing_result.aggregated_fees().processing_fee, 2473880);

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

        // We expect the sender to still have their document, and the receiver to have none
        assert_eq!(query_sender_results.documents().len(), 1);

        assert_eq!(query_receiver_results.documents().len(), 0);

        // The sender document should have the desired price

        let mut document = query_sender_results.documents_owned().remove(0);

        let price: Credits = document
            .properties()
            .get_integer("$price")
            .expect("expected to get back price");

        assert_eq!(dash_to_credits!(0.1), price);

        // At this point we want to have the receiver purchase the document

        document.set_revision(Some(3));

        let documents_batch_purchase_transition =
            BatchTransition::new_document_purchase_transition_from_document(
                document.clone(),
                card_document_type,
                receiver.id(),
                dash_to_credits!(0.1), //same price as requested
                &recipient_key,
                1, // 1 because he's never done anything
                0,
                None,
                &recipient_signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition for the purchase");

        let documents_batch_purchase_serialized_transition = documents_batch_purchase_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_purchase_serialized_transition],
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

        // nothing can go through because the purchaser doesn't have enough balance

        assert_eq!(processing_result.invalid_paid_count(), 0);

        assert_eq!(processing_result.invalid_unpaid_count(), 1);

        assert_eq!(processing_result.valid_count(), 0);

        assert_eq!(processing_result.aggregated_fees().processing_fee, 0);
    }

    #[test]
    fn test_document_set_price_on_not_owned_document() {
        let platform_version = PlatformVersion::latest();
        let (mut platform, contract) = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure()
            .with_crypto_card_game_nft(TradeMode::DirectPurchase);

        let mut rng = StdRng::seed_from_u64(433);

        let platform_state = platform.state.load();

        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

        let (other_identity, other_identity_signer, other_identity_key) =
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

        document.set_revision(Some(2));

        document.set_owner_id(other_identity.id()); // we do this to trick the system

        let documents_batch_update_price_transition =
            BatchTransition::new_document_update_price_transition_from_document(
                document.clone(),
                card_document_type,
                dash_to_credits!(0.1),
                &other_identity_key,
                1,
                0,
                None,
                &other_identity_signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition for the update price");

        let documents_batch_transfer_serialized_transition =
            documents_batch_update_price_transition
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

        assert_eq!(processing_result.invalid_paid_count(), 1);

        assert_eq!(processing_result.invalid_unpaid_count(), 0);

        assert_eq!(processing_result.valid_count(), 0);

        assert_eq!(processing_result.aggregated_fees().processing_fee, 36200);

        let sender_documents_sql_string =
            format!("select * from card where $ownerId == '{}'", identity.id());

        let query_sender_identity_documents = DriveDocumentQuery::from_sql_expr(
            sender_documents_sql_string.as_str(),
            &contract,
            Some(&platform.config.drive),
        )
        .expect("expected document query");

        let query_sender_results = platform
            .drive
            .query_documents(query_sender_identity_documents, None, false, None, None)
            .expect("expected query result");

        // The sender document should not have the desired price

        let document = query_sender_results.documents().first().unwrap();

        assert_eq!(
            document
                .properties()
                .get_optional_integer::<u64>("$price")
                .expect("expected None"),
            None
        );
    }

    #[test]
    fn test_document_set_price_and_purchase_with_token_costs() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(433);

        let platform_state = platform.state.load();

        let (contract_owner_id, _, _) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

        let (creator, signer, key) = setup_identity(&mut platform, 234, dash_to_credits!(0.1));

        let (purchaser, recipient_signer, recipient_key) =
            setup_identity(&mut platform, 450, dash_to_credits!(1.0));

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
        add_tokens_to_identity(&mut platform, gold_token_id, purchaser.id(), 5);

        let token_supply = platform
            .drive
            .fetch_token_total_supply(gold_token_id.to_buffer(), None, platform_version)
            .expect("expected to fetch total supply");

        assert_eq!(token_supply, Some(20));

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
                    minimum_token_cost: Some(10),
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

        let sender_documents_sql_string =
            format!("select * from card where $ownerId == '{}'", creator.id());

        let query_sender_identity_documents = DriveDocumentQuery::from_sql_expr(
            sender_documents_sql_string.as_str(),
            &contract,
            Some(&platform.config.drive),
        )
        .expect("expected document query");

        let receiver_documents_sql_string =
            format!("select * from card where $ownerId == '{}'", purchaser.id());

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

        let documents_batch_update_price_transition =
            BatchTransition::new_document_update_price_transition_from_document(
                document.clone(),
                card_document_type,
                dash_to_credits!(0.1),
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
            .expect("expect to create documents batch transition for the update price");

        let documents_batch_transfer_serialized_transition =
            documents_batch_update_price_transition
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

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
        );

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

        // We expect the sender to still have their document, and the receiver to have none
        assert_eq!(query_sender_results.documents().len(), 1);

        assert_eq!(query_receiver_results.documents().len(), 0);

        // The sender document should have the desired price

        let mut document = query_sender_results.documents_owned().remove(0);

        let price: Credits = document
            .properties()
            .get_integer("$price")
            .expect("expected to get back price");

        assert_eq!(dash_to_credits!(0.1), price);

        // At this point we want to have the receiver purchase the document

        document.set_revision(Some(3));

        let documents_batch_purchase_transition =
            BatchTransition::new_document_purchase_transition_from_document(
                document.clone(),
                card_document_type,
                purchaser.id(),
                dash_to_credits!(0.1), //same price as requested
                &recipient_key,
                1, // 1 because he's never done anything
                0,
                Some(TokenPaymentInfo::V0(TokenPaymentInfoV0 {
                    payment_token_contract_id: None,
                    token_contract_position: 0,
                    minimum_token_cost: Some(2),
                    maximum_token_cost: Some(3),
                    gas_fees_paid_by: Default::default(),
                })),
                &recipient_signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition for the purchase");

        let documents_batch_purchase_serialized_transition = documents_batch_purchase_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_purchase_serialized_transition],
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
        assert_eq!(query_sender_results.documents().len(), 0);

        assert_eq!(query_receiver_results.documents().len(), 1);

        let token_balance = platform
            .drive
            .fetch_identity_token_balance(
                gas_token_id.to_buffer(),
                purchaser.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");

        // He had never had any gas
        assert_eq!(token_balance, None);

        let gold_token_balance = platform
            .drive
            .fetch_identity_token_balance(
                gold_token_id.to_buffer(),
                purchaser.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");

        // It costs 3 to purchase on top of the credits and he had 5
        assert_eq!(gold_token_balance, Some(2));
    }

    #[test]
    fn test_document_creator_id_unique_index_enforcement_with_purchase() {
        // This test verifies that a unique index on creator_id is properly enforced throughout
        // the complete document lifecycle with purchase operations, ensuring that only one
        // document per creator can exist at any time.
        //
        // ## Purpose
        // The creator_id field is immutable and set once at document creation. A unique index on
        // this field enforces a "one document per creator" constraint. This test extends the
        // transfer test to include purchase operations (setting price and purchasing).
        //
        // ## Why This Test Is Important
        // This test ensures that:
        // 1. The unique constraint prevents duplicate documents from the same creator
        // 2. The creator_id remains immutable during purchases (doesn't change with ownership)
        // 3. The unique constraint persists even after purchase operations
        // 4. Only document deletion frees up the creator_id for potential reuse
        // 5. Price setting and purchasing work correctly with unique creator_id constraints
        //
        // ## Test Scenario
        // This test uses a contract with tradeMode=1 (direct purchase) where the "card" document
        // type has a unique index on $creatorId. The test verifies:
        //
        // 1. Creator creates first document with price → SUCCESS
        // 2. Creator tries to create second document → FAIL (creator_id already used)
        // 3. Buyer purchases the document → SUCCESS (ownership changes, creator_id stays)
        // 4. Creator tries to create new document → FAIL (creator_id still claimed despite purchase)
        // 5. New buyer deletes the document → SUCCESS (creator_id is freed)
        // 6. Creator creates new document → SUCCESS (creator_id now available again)
        let platform_version = PlatformVersion::latest();

        let mut platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let card_game_path = "tests/supporting_files/contract/crypto-card-game/crypto-card-game-direct-purchase-unique-creator-id-index.json";

        // Load the contract with unique creator_id index and tradeMode=1 (direct purchase)
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

        // Setup two identities: creator and buyer
        let (creator, creator_signer, creator_key) =
            setup_identity(&mut platform, 958, dash_to_credits!(0.5));
        let (buyer, buyer_signer, buyer_key) =
            setup_identity(&mut platform, 450, dash_to_credits!(0.5));

        let card_document_type = contract
            .document_type_for_name("card")
            .expect("expected a card document type");

        assert!(!card_document_type.documents_mutable());

        // Step 1: Creator creates first document with price
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

        // Set price on document1
        document1.set_revision(Some(2));

        let price = dash_to_credits!(0.1);

        let documents_batch_update_price_transition =
            BatchTransition::new_document_update_price_transition_from_document(
                document1.clone(),
                card_document_type,
                price,
                &creator_key,
                3,
                0,
                None,
                &creator_signer,
                platform_version,
                None,
            )
            .expect("expect to create update price transition");

        let documents_batch_update_price_serialized_transition =
            documents_batch_update_price_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_update_price_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default_with_time(50000000),
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

        // Step 2: Try to create second document by same creator
        // This should FAIL due to unique creator_id constraint
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
                4,
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

        // Step 3: Buyer purchases document1 (changes owner to buyer, creator stays same)
        document1.set_revision(Some(3));

        let documents_batch_purchase_transition =
            BatchTransition::new_document_purchase_transition_from_document(
                document1.clone(),
                card_document_type,
                buyer.id(),
                price,
                &buyer_key,
                2,
                0,
                None,
                &buyer_signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition for purchase");

        let documents_batch_purchase_serialized_transition = documents_batch_purchase_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_purchase_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default_with_time(100000000),
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

        // Verify the document was purchased
        let buyer_documents_sql_string =
            format!("select * from card where $ownerId == '{}'", buyer.id());

        let query_buyer_identity_documents = DriveDocumentQuery::from_sql_expr(
            buyer_documents_sql_string.as_str(),
            &contract,
            Some(&platform.config.drive),
        )
        .expect("expected document query");

        let query_buyer_results = platform
            .drive
            .query_documents(
                query_buyer_identity_documents.clone(),
                None,
                false,
                None,
                None,
            )
            .expect("expected query result");

        assert_eq!(query_buyer_results.documents().len(), 1);

        // Step 4: Try to create a new document by the creator again after purchase
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

        // Step 5: Buyer deletes the document
        document1.set_owner_id(buyer.id());
        document1.set_revision(Some(4));

        let documents_batch_deletion_transition =
            BatchTransition::new_document_deletion_transition_from_document(
                document1,
                card_document_type,
                &buyer_key,
                3,
                0,
                None,
                &buyer_signer,
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
        let query_buyer_results = platform
            .drive
            .query_documents(
                query_buyer_identity_documents.clone(),
                None,
                false,
                None,
                None,
            )
            .expect("expected query result");

        assert_eq!(query_buyer_results.documents().len(), 0);

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
    fn test_document_owner_and_creator_id_unique_index_enforcement_with_purchase() {
        // This test verifies that a unique compound index on (owner_id, creator_id) is properly
        // enforced throughout the document lifecycle with purchase operations, allowing a creator
        // to have multiple documents but preventing duplicate (owner, creator) combinations.
        //
        // ## Purpose
        // A compound unique index on (owner_id, creator_id) creates a more flexible constraint than
        // a simple unique creator_id. It allows the same creator to create multiple documents, but
        // each owner can hold at most one document from any given creator. This test extends the
        // transfer test to include purchase operations.
        //
        // ## Why This Test Is Important
        // This test ensures that:
        // 1. The compound constraint prevents duplicate (owner, creator) pairs
        // 2. Creators can create multiple documents when owned by different people
        // 3. Purchases can fail if they would create a duplicate (owner, creator) combination
        // 4. Price setting works correctly with compound unique constraints
        // 5. Deletion properly frees up the (owner, creator) constraint
        //
        // ## Test Scenario
        // This test uses a contract with tradeMode=1 where the "card" document type has a unique
        // compound index on ($ownerId, $creatorId). The test verifies:
        //
        // 1. Creator creates document1 with price (owner=creator, creator=creator) → SUCCESS
        // 2. Creator tries to create document2 with same owner → FAIL (duplicate (creator, creator))
        // 3. Buyer1 purchases document1 → SUCCESS (now (buyer1, creator) exists)
        // 4. Creator creates document3 with price → SUCCESS ((creator, creator) is now available)
        // 5. Buyer1 tries to purchase document3 → FAIL ((buyer1, creator) already exists)
        // 6. Buyer2 purchases document3 → SUCCESS ((buyer2, creator) is available)
        // 7. Buyer1 tries to purchase from another seller → FAIL (simulated via attempting to create)
        // 8. Buyer2 deletes document → SUCCESS (frees (buyer2, creator))
        // 9. Buyer1 can now receive from creator → SUCCESS (via purchase or transfer)
        let platform_version = PlatformVersion::latest();

        let mut platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let card_game_path = "tests/supporting_files/contract/crypto-card-game/crypto-card-game-direct-purchase-unique-creator-id-with-owner-id-index.json";

        // Load the contract with unique (owner_id, creator_id) compound index and tradeMode=1
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

        // Setup three identities: creator, buyer1, and buyer2
        let (creator, creator_signer, creator_key) =
            setup_identity(&mut platform, 958, dash_to_credits!(0.5));
        let (buyer1, buyer1_signer, buyer1_key) =
            setup_identity(&mut platform, 450, dash_to_credits!(0.5));
        let (buyer2, buyer2_signer, buyer2_key) =
            setup_identity(&mut platform, 789, dash_to_credits!(0.5));

        let card_document_type = contract
            .document_type_for_name("card")
            .expect("expected a card document type");

        assert!(!card_document_type.documents_mutable());

        let price = dash_to_credits!(0.1);

        // Step 1: Creator creates document1 with price (owner=creator, creator=creator)
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

        // Set price on document1
        document1.set_revision(Some(2));

        let documents_batch_update_price_transition1 =
            BatchTransition::new_document_update_price_transition_from_document(
                document1.clone(),
                card_document_type,
                price,
                &creator_key,
                3,
                0,
                None,
                &creator_signer,
                platform_version,
                None,
            )
            .expect("expect to create update price transition");

        let documents_batch_update_price_serialized_transition1 =
            documents_batch_update_price_transition1
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_update_price_serialized_transition1.clone()],
                &platform_state,
                &BlockInfo::default_with_time(50000000),
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
                4,
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

        // Step 3: Buyer1 purchases document1 (changes owner to buyer1, creator stays same)
        // Now we have (owner=buyer1, creator=creator)
        document1.set_revision(Some(3));

        let documents_batch_purchase_transition1 =
            BatchTransition::new_document_purchase_transition_from_document(
                document1.clone(),
                card_document_type,
                buyer1.id(),
                price,
                &buyer1_key,
                2,
                0,
                None,
                &buyer1_signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition for purchase");

        let documents_batch_purchase_serialized_transition1 = documents_batch_purchase_transition1
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_purchase_serialized_transition1.clone()],
                &platform_state,
                &BlockInfo::default_with_time(100000000),
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

        // Should succeed because (creator, creator) is now available after purchase
        assert_eq!(processing_result.valid_count(), 1);
        assert_eq!(processing_result.invalid_paid_count(), 0);

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        // Set price on document3
        document3.set_revision(Some(2));

        let documents_batch_update_price_transition3 =
            BatchTransition::new_document_update_price_transition_from_document(
                document3.clone(),
                card_document_type,
                price,
                &creator_key,
                6,
                0,
                None,
                &creator_signer,
                platform_version,
                None,
            )
            .expect("expect to create update price transition");

        let documents_batch_update_price_serialized_transition3 =
            documents_batch_update_price_transition3
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_update_price_serialized_transition3.clone()],
                &platform_state,
                &BlockInfo::default_with_time(150000000),
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

        // Step 5: Try buyer1 to purchase document3
        // This should FAIL because (buyer1, creator) already exists from document1
        document3.set_revision(Some(3));

        let documents_batch_purchase_transition_buyer1 =
            BatchTransition::new_document_purchase_transition_from_document(
                document3.clone(),
                card_document_type,
                buyer1.id(),
                price,
                &buyer1_key,
                3,
                0,
                None,
                &buyer1_signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition for purchase");

        let documents_batch_purchase_serialized_transition_buyer1 =
            documents_batch_purchase_transition_buyer1
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_purchase_serialized_transition_buyer1.clone()],
                &platform_state,
                &BlockInfo::default_with_time(200000000),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        // Should fail because (buyer1, creator) combination already exists
        assert_eq!(processing_result.valid_count(), 0);
        assert_eq!(processing_result.invalid_paid_count(), 1);

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        // Step 6: Buyer2 purchases document3
        // This should SUCCEED because (buyer2, creator) is available
        document3.set_revision(Some(3));

        let documents_batch_purchase_transition_buyer2 =
            BatchTransition::new_document_purchase_transition_from_document(
                document3.clone(),
                card_document_type,
                buyer2.id(),
                price,
                &buyer2_key,
                2,
                0,
                None,
                &buyer2_signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition for purchase");

        let documents_batch_purchase_serialized_transition_buyer2 =
            documents_batch_purchase_transition_buyer2
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_purchase_serialized_transition_buyer2.clone()],
                &platform_state,
                &BlockInfo::default_with_time(250000000),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        // Should succeed because (buyer2, creator) is available
        assert_eq!(processing_result.valid_count(), 1);
        assert_eq!(processing_result.invalid_paid_count(), 0);

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        // Step 7: Buyer2 deletes document3 (which has owner=buyer2, creator=creator)
        document3.set_owner_id(buyer2.id());
        document3.set_revision(Some(4));

        let documents_batch_deletion_transition =
            BatchTransition::new_document_deletion_transition_from_document(
                document3,
                card_document_type,
                &buyer2_key,
                3,
                0,
                None,
                &buyer2_signer,
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

        // Verify final state
        let buyer1_documents_sql_string =
            format!("select * from card where $ownerId == '{}'", buyer1.id());

        let query_buyer1_identity_documents = DriveDocumentQuery::from_sql_expr(
            buyer1_documents_sql_string.as_str(),
            &contract,
            Some(&platform.config.drive),
        )
        .expect("expected document query");

        let query_buyer1_results = platform
            .drive
            .query_documents(query_buyer1_identity_documents, None, false, None, None)
            .expect("expected query result");

        assert_eq!(query_buyer1_results.documents().len(), 1);

        let buyer2_documents_sql_string =
            format!("select * from card where $ownerId == '{}'", buyer2.id());

        let query_buyer2_identity_documents = DriveDocumentQuery::from_sql_expr(
            buyer2_documents_sql_string.as_str(),
            &contract,
            Some(&platform.config.drive),
        )
        .expect("expected document query");

        let query_buyer2_results = platform
            .drive
            .query_documents(query_buyer2_identity_documents, None, false, None, None)
            .expect("expected query result");

        assert_eq!(query_buyer2_results.documents().len(), 0);

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
