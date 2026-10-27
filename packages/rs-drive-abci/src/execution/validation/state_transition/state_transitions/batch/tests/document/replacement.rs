use super::*;

mod replacement_tests {
    use super::*;
    use crate::platform_types::platform_state::PlatformState;
    use crate::test::helpers::fast_forward_to_block::fast_forward_to_block;
    use dpp::data_contract::accessors::v0::DataContractV0Setters;
    use dpp::data_contract::DataContract;
    use dpp::document::Document;
    use dpp::fee::fee_result::FeeResult;
    use dpp::identifier::Identifier;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::identity::KeyID;
    use dpp::prelude::IdentityNonce;
    use dpp::state_transition::data_contract_update_transition::methods::DataContractUpdateTransitionMethodsV0;
    use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
    use dpp::state_transition::StateTransition;
    use dpp::tokens::token_payment_info::v0::TokenPaymentInfoV0;
    use dpp::tokens::token_payment_info::TokenPaymentInfo;
    use drive::util::test_helpers::setup_contract;
    use std::collections::BTreeMap;
    use std::sync::Arc;

    const REFERENCE_VALIDATION_CONTRACT_PATH: &str =
        "tests/supporting_files/contract/reference-validation/reference-validation-contract.json";
    const REFERENCE_VALIDATION_NESTED_CONTRACT_PATH: &str =
        "tests/supporting_files/contract/reference-validation/reference-validation-contract-nested.json";
    const REFERENCE_VALIDATION_OPTIONAL_CONTRACT_PATH: &str =
        "tests/supporting_files/contract/reference-validation/reference-validation-contract-optional.json";

    /// Creates a document from `contract_path`'s message type, applies `create_setup`
    /// to it, processes the creation (asserting success), then applies `replace_mutation`
    /// and processes the replacement, returning its execution result.
    /// propertyAgreement on replace: changing the REFERRING property while
    /// leaving the reference untouched must re-validate the agreement —
    /// the changed-fields gate binds the agreement's referring properties,
    /// not only the reference property itself.
    #[tokio::test]
    async fn should_document_replace_fail_when_agreement_property_diverges() {
        use crate::platform_types::platform_state::PlatformState;
        use crate::rpc::core::MockCoreRPCLike;
        use crate::test::helpers::setup::TempPlatform;
        use dpp::state_transition::StateTransition;

        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();
        let mut rng = StdRng::seed_from_u64(4433);
        let platform_state = platform.state.load();

        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

        let contract = setup_contract(
            &platform.drive,
            "tests/supporting_files/contract/reference-validation/reference-validation-contract-agreement-valid.json",
            None,
            None,
            None::<fn(&mut DataContract)>,
            None,
            None,
        );

        let process = |platform: &TempPlatform<MockCoreRPCLike>,
                       platform_state: &PlatformState,
                       transition: &StateTransition| {
            let serialized = transition
                .serialize_to_bytes()
                .expect("expected the transition to serialize");
            let transaction = platform.drive.grove.start_transaction();
            let result = platform
                .platform
                .process_raw_state_transitions(
                    &[serialized],
                    platform_state,
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
            result
                .execution_results()
                .first()
                .expect("expected one execution result")
                .clone()
        };

        // A note with topic "alpha" for the message to agree with.
        let note_type = contract
            .document_type_for_name("note")
            .expect("expected the note document type");
        let note_entropy = Bytes32::random_with_rng(&mut rng);
        let mut note = note_type
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity.id(),
                note_entropy,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random note");
        note.set_id_for_creation(note_type, &note_entropy.0, 2, platform_version)
            .expect("expected to set the document id");
        note.set("topic", "alpha".into());
        let note_create = BatchTransition::new_document_creation_transition_from_document(
            note.clone(),
            note_type,
            note_entropy.0,
            &key,
            2,
            0,
            None,
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("expected the note create transition");
        assert_matches!(
            process(&platform, &platform_state, &note_create),
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );

        // An agreeing message referencing it.
        let message_type = contract
            .document_type_for_name("message")
            .expect("expected the message document type");
        let message_entropy = Bytes32::random_with_rng(&mut rng);
        let mut message = message_type
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity.id(),
                message_entropy,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random message");
        message
            .set_id_for_creation(message_type, &message_entropy.0, 3, platform_version)
            .expect("expected to set the document id");
        message.set(
            "noteId",
            dpp::platform_value::Value::Identifier(note.id().to_buffer()),
        );
        message.set("topic", "alpha".into());
        let message_create = BatchTransition::new_document_creation_transition_from_document(
            message.clone(),
            message_type,
            message_entropy.0,
            &key,
            3,
            0,
            None,
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("expected the message create transition");
        assert_matches!(
            process(&platform, &platform_state, &message_create),
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );

        // Replace ONLY the referring property: the reference is untouched,
        // so this must be caught by the agreement-aware changed-fields gate.
        message.set("topic", "beta".into());
        message.increment_revision().expect("revision increments");
        let message_replace = BatchTransition::new_document_replacement_transition_from_document(
            message,
            message_type,
            &key,
            4,
            0,
            None,
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("expected the message replace transition");
        assert_matches!(
            process(&platform, &platform_state, &message_replace),
            StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(
                    StateError::ReferencedDocumentPropertyMismatchError(_)
                ),
                ..
            }
        );
    }

    async fn run_reference_validation_create_then_replace<C, R>(
        contract_path: &str,
        create_setup: C,
        replace_mutation: R,
    ) -> StateTransitionExecutionResult
    where
        C: FnOnce(&mut Document, Identifier, Identifier),
        R: FnOnce(&mut Document, Identifier, Identifier),
    {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(433);

        let platform_state = platform.state.load();

        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));
        let (other_identity, ..) = setup_identity(&mut platform, 959, dash_to_credits!(0.1));

        let contract = setup_contract(
            &platform.drive,
            contract_path,
            None,
            None,
            None::<fn(&mut DataContract)>,
            None,
            None,
        );

        let message = contract
            .document_type_for_name("message")
            .expect("expected a message document type");

        let entropy = Bytes32::random_with_rng(&mut rng);

        let mut document = message
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity.id(),
                entropy,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");
        document
            .set_id_for_creation(message, &entropy.0, 2, platform_version)
            .expect("expected to set the document id");

        create_setup(&mut document, identity.id(), other_identity.id());

        let documents_batch_create_transition =
            BatchTransition::new_document_creation_transition_from_document(
                document.clone(),
                message,
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

        document.increment_revision().unwrap();
        replace_mutation(&mut document, identity.id(), other_identity.id());

        let documents_batch_replace_transition =
            BatchTransition::new_document_replacement_transition_from_document(
                document,
                message,
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

        let documents_batch_replace_serialized_transition = documents_batch_replace_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[documents_batch_replace_serialized_transition],
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

        processing_result
            .execution_results()
            .first()
            .expect("expected one execution result")
            .clone()
    }

    async fn run_reference_validation_replace_with_contract<F>(
        contract_path: &str,
        to_user_id: F,
        change_note: bool,
    ) -> (StateTransitionExecutionResult, FeeResult)
    where
        F: FnOnce(Identifier, Identifier) -> Identifier,
    {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(433);

        let platform_state = platform.state.load();

        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));
        let (other_identity, ..) = setup_identity(&mut platform, 959, dash_to_credits!(0.1));

        let contract = setup_contract(
            &platform.drive,
            contract_path,
            None,
            None,
            None::<fn(&mut DataContract)>,
            None,
            None,
        );

        let message = contract
            .document_type_for_name("message")
            .expect("expected a message document type");

        let entropy = Bytes32::random_with_rng(&mut rng);

        let mut document = message
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity.id(),
                entropy,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");
        document
            .set_id_for_creation(message, &entropy.0, 2, platform_version)
            .expect("expected to set the document id");

        document.set("toUserId", identity.id().into());
        document.set("note", "before".into());

        let documents_batch_create_transition =
            BatchTransition::new_document_creation_transition_from_document(
                document.clone(),
                message,
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

        document.increment_revision().unwrap();
        if change_note {
            document.set("note", "after".into());
        }
        document.set(
            "toUserId",
            to_user_id(identity.id(), other_identity.id()).into(),
        );

        let documents_batch_replace_transition =
            BatchTransition::new_document_replacement_transition_from_document(
                document,
                message,
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

        let documents_batch_replace_serialized_transition = documents_batch_replace_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[documents_batch_replace_serialized_transition],
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

        let result = processing_result
            .execution_results()
            .first()
            .expect("expected one execution result")
            .clone();

        (result, processing_result.aggregated_fees().clone())
    }

    #[tokio::test]
    async fn test_document_replace_on_document_type_that_is_mutable() {
        run_document_replace_on_document_type_that_is_mutable_at_protocol_version(
            PlatformVersion::latest().protocol_version,
            // v14: replaced documents carry the contract-version stamp, and
            // GroveDB V4 writes through the Merk node it retains from reading
            // the old value, billing slightly fewer reads than the V3 path
            // Protocol version 14 adds +740 per document write (the contract's version
            // item is one more node to rehash) and the larger DashPay v2 schema
            // increases byte-billed contract-tree reads.
            // The app-connect and moderation charters contracts each add one sibling to the
            // genesis contracts tree, increasing the bytes billed when reading that tree
            // (protocol 14 only).
            1546140,
        )
        .await;
    }

    /// PROTOCOL_VERSION_13: fee predating every v14 change on this path —
    /// both the contract-version stamp and the dashpay payment-address
    /// contract (#4380), whose v2 schema is gated behind
    /// `SYSTEM_DATA_CONTRACT_VERSIONS_V3` (v14 only; v13 genesis stores
    /// dashpay v1, so its smaller node shifts the byte-billed contracts-
    /// subtree reads). That gate is why this value is below the pre-stamp
    /// v14 baseline by more than the stamp bytes — and this pin is what
    /// fails if the gate ever leaks into v13. Pinned so v13 chain history
    /// stays bit-for-bit reproducible.
    #[tokio::test]
    async fn test_document_replace_on_document_type_that_is_mutable_protocol_version_13() {
        run_document_replace_on_document_type_that_is_mutable_at_protocol_version(13, 1411320)
            .await;
    }

    /// PROTOCOL_VERSION_11: pre-B7 happy-path fee — transformer's local
    /// execution context was dropped, so per-transition grovedb reads
    /// were not billed. Pinned so v11 chain history stays bit-for-bit
    /// reproducible.
    #[tokio::test]
    async fn test_document_replace_on_document_type_that_is_mutable_protocol_version_11() {
        run_document_replace_on_document_type_that_is_mutable_at_protocol_version(11, 1399260)
            .await;
    }

    const TRANSIENT_NOTE_CONTRACT_PATH: &str =
        "tests/supporting_files/contract/transient/transient-note-contract.json";

    /// A transient value is judged on the transition and dropped before its
    /// document is stored; from protocol version 14 a replace drops it too.
    #[tokio::test]
    async fn should_store_a_replaced_document_without_its_transient_values() {
        assert_eq!(
            run_replace_carrying_a_transient_value(PlatformVersion::latest()).await,
            None
        );
    }

    /// Protocol version 13 stored whatever a replace carried, transient values
    /// included: pinned so its chain history stays reproducible.
    #[tokio::test]
    async fn should_store_the_transient_values_of_a_replace_at_protocol_version_13() {
        let platform_version = PlatformVersion::get(13).expect("expected protocol version 13");
        assert_eq!(
            run_replace_carrying_a_transient_value(platform_version).await,
            Some(Value::Text("y".to_string()))
        );
    }

    /// Creates a `note` whose transient `code` is `x`, checks the stored
    /// document has no `code`, replaces it with `code` `y`, and returns the
    /// `code` the stored document holds after the replace.
    async fn run_replace_carrying_a_transient_value(
        platform_version: &PlatformVersion,
    ) -> Option<Value> {
        let mut platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(platform_version.protocol_version)
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(433);

        let platform_state = platform.state.load();

        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

        let contract = setup_contract(
            &platform.drive,
            TRANSIENT_NOTE_CONTRACT_PATH,
            None,
            None,
            None::<fn(&mut DataContract)>,
            None,
            Some(platform_version),
        );
        let note = contract
            .document_type_for_name("note")
            .expect("expected a note document type");

        let entropy = Bytes32::random_with_rng(&mut rng);
        let mut document = note
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity.id(),
                entropy,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");
        document
            .set_id_for_creation(note, &entropy.0, 2, platform_version)
            .expect("expected to set the document id");
        document.set("body", "a".into());
        document.set("code", "x".into());
        let document_id = document.id();

        let stored_code = || {
            let query =
                DriveDocumentQuery::new_primary_key_single_item_query(&contract, note, document_id);
            platform
                .drive
                .query_documents(
                    query,
                    None,
                    false,
                    None,
                    Some(platform_version.protocol_version),
                )
                .expect("expected to query the note")
                .documents_owned()
                .pop()
                .expect("expected the stored note")
                .properties()
                .get("code")
                .cloned()
        };
        let process_and_commit = |transition: Vec<u8>| {
            let transaction = platform.drive.grove.start_transaction();
            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[transition],
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
        };

        let create = BatchTransition::new_document_creation_transition_from_document(
            document.clone(),
            note,
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
        process_and_commit(
            create
                .serialize_to_bytes()
                .expect("expected serialized create"),
        );
        assert_eq!(
            stored_code(),
            None,
            "a create never stores a transient value"
        );

        document.increment_revision().unwrap();
        document.set("body", "b".into());
        document.set("code", "y".into());
        let replace = BatchTransition::new_document_replacement_transition_from_document(
            document,
            note,
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
        process_and_commit(
            replace
                .serialize_to_bytes()
                .expect("expected serialized replace"),
        );

        stored_code()
    }

    async fn run_document_replace_on_document_type_that_is_mutable_at_protocol_version(
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

        fast_forward_to_block(&platform, 1_200_000_000, 900, 42, 1, false); //next epoch

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
        document
            .set_id_for_creation(profile, &entropy.0, 2, platform_version)
            .expect("expected to set the document id");

        set_valid_profile_payment_addresses(&mut document, profile);

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
            .await
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

        assert_eq!(
            processing_result.aggregated_fees().processing_fee,
            expected_processing_fee,
            "PROTOCOL_VERSION_{}: happy-path replace processing fee must match the version-specific baseline",
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

    async fn perform_document_replace_on_profile_after_epoch_change(
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
        document
            .set_id_for_creation(profile, &entropy.0, 2, platform_version)
            .expect("expected to set the document id");

        set_valid_profile_payment_addresses(&mut document, profile);

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
                .await
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

    #[tokio::test]
    async fn test_document_replace_on_document_type_that_is_mutable_different_epoch_bigger_size() {
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
        )
        .await;
    }

    #[tokio::test]
    async fn test_document_replace_on_document_type_that_is_mutable_different_epoch_smaller_size() {
        perform_document_replace_on_profile_after_epoch_change(
            "Sam",
            vec![(
                "S",
                StorageFlags::SingleEpochOwned(0, Identifier::default().to_buffer()),
            )],
        )
        .await;
    }

    #[tokio::test]
    async fn test_document_replace_on_document_type_that_is_mutable_different_epoch_same_size() {
        perform_document_replace_on_profile_after_epoch_change(
            "Sam",
            vec![(
                "Max",
                StorageFlags::SingleEpochOwned(0, Identifier::default().to_buffer()),
            )],
        )
        .await;
    }

    #[tokio::test]
    async fn test_document_replace_on_document_type_that_is_mutable_different_epoch_bigger_size_then_bigger_size(
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
        )
        .await;
    }

    #[tokio::test]
    async fn test_document_replace_on_document_type_that_is_mutable_different_epoch_bigger_size_then_bigger_size_by_3_bytes(
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
        )
        .await;
    }

    #[tokio::test]
    async fn test_document_replace_on_document_type_that_is_mutable_different_epoch_bigger_size_then_smaller_size(
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
        )
        .await;
    }

    #[tokio::test]
    async fn test_document_replace_on_document_type_that_is_mutable_different_epoch_bigger_size_then_back_to_original(
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
        )
        .await;
    }

    /// Helper for the paired Replace-on-immutable-doc test. The same scenario
    /// is exercised at PROTOCOL_VERSION_11 (legacy bump-only fee) and at
    /// PROTOCOL_VERSION_12 (fee covers fetch + validation).
    async fn run_document_replace_on_document_type_that_is_not_mutable_at_protocol_version(
        protocol_version: dpp::version::ProtocolVersion,
        expected_processing_fee: dpp::fee::Credits,
    ) {
        let platform_version = PlatformVersion::get(protocol_version)
            .expect("expected platform version for the requested protocol_version");
        let mut platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(protocol_version)
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
        document
            .set_id_for_creation(
                contact_request_document_type,
                &entropy.0,
                2,
                platform_version,
            )
            .expect("expected to set the document id");

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
            .await
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

        assert_eq!(
            processing_result.aggregated_fees().processing_fee,
            expected_processing_fee,
            "PROTOCOL_VERSION_{}: processing fee must match the version-specific baseline",
            protocol_version,
        );
    }

    /// PROTOCOL_VERSION_12+: bump emission charges the user for the fetch +
    /// structure validation that ran before the failure.
    #[tokio::test]
    async fn test_document_replace_on_document_type_that_is_not_mutable() {
        run_document_replace_on_document_type_that_is_not_mutable_at_protocol_version(
            PlatformVersion::latest().protocol_version,
            460940, // v14: stamped documents (see happy-path baseline note)
        )
        .await;
    }

    /// PROTOCOL_VERSION_13: pre-stamp fee — document serialization format 3
    /// (the contract-version stamp) activates at v14, so v13 costs must be
    /// exactly what they were before the `requiredSince` changes. Pinned so
    /// v13 chain history stays bit-for-bit reproducible.
    #[tokio::test]
    async fn test_document_replace_on_document_type_that_is_not_mutable_protocol_version_13() {
        run_document_replace_on_document_type_that_is_not_mutable_at_protocol_version(13, 460920)
            .await;
    }

    /// PROTOCOL_VERSION_11: pre-fix bump-only fee (no charge for the fetch
    /// + validation work). Pinned so v11 chain history stays bit-for-bit
    /// reproducible.
    #[tokio::test]
    async fn test_document_replace_on_document_type_that_is_not_mutable_protocol_version_11() {
        run_document_replace_on_document_type_that_is_not_mutable_at_protocol_version(11, 41880)
            .await;
    }

    /// Pins the bump-emission contract on Replace's revision-mismatch path.
    ///
    /// Without the bump, a failed Replace returns errors-only with no action.
    /// Fee accounting then charges the user (PaidConsensusError) but the
    /// identity_contract_nonce in state never advances — the same exact bytes
    /// can be re-broadcast indefinitely.
    ///
    /// The test asserts:
    ///   1. After a Replace that fails `check_revision_is_bumped_by_one`, the
    ///      stored contract nonce MUST advance past the submitted nonce.
    ///   2. Re-submitting the same bytes through CheckTx FirstTimeCheck MUST
    ///      be rejected with `InvalidIdentityNonceError`.
    #[tokio::test]
    async fn replayed_failed_replace_with_consumed_nonce_must_be_rejected_at_check_tx() {
        use crate::execution::check_tx::CheckTxLevel;
        use crate::execution::validation::state_transition::check_tx_verification::state_transition_to_execution_event_for_check_tx;
        use crate::platform_types::platform::PlatformRef;
        use dpp::serialization::PlatformDeserializableUntrusted;
        use dpp::state_transition::StateTransition;

        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(437);

        let platform_state = platform.state.load();

        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.5));

        let dashpay = platform
            .drive
            .cache
            .system_data_contracts
            .load_dashpay(platform_version)
            .expect("expected the dashpay system contract");
        let dashpay_contract = dashpay.clone();

        // Use the mutable `profile` doc type — same contract-and-doc-type that
        // mainnet 35C0 was operating on (DPNS-like profile-replace flow).
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
        document
            .set_id_for_creation(profile, &entropy.0, 2, platform_version)
            .expect("expected to set the document id");

        set_valid_profile_payment_addresses(&mut document, profile);
        // Random fillers can produce a non-URI avatarUrl that fails JSON-schema
        // validation on Create. Pin it to a valid URI like the sibling tests do.
        document.set("avatarUrl", "http://test.com/bob.jpg".into());
        document.set("displayName", "Original".into());

        // 1) Create at nonce 2 — consumes nonce 2; doc lands at revision 1.
        let create_transition = BatchTransition::new_document_creation_transition_from_document(
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
        .await
        .expect("expected to build create transition");

        let create_serialized = create_transition
            .serialize_to_bytes()
            .expect("expected to serialize create");

        let transaction = platform.drive.grove.start_transaction();
        let create_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![create_serialized],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process create");
        assert_eq!(create_result.valid_count(), 1);
        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit create");

        let (post_create_nonce_raw, _) = platform
            .drive
            .fetch_identity_contract_nonce_with_fees(
                identity.id().to_buffer(),
                dashpay_contract.id().to_buffer(),
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to fetch contract nonce after create");
        let post_create_nonce =
            post_create_nonce_raw.expect("contract nonce must be present after create");

        // 2) Build a Replace at nonce 3 with revision 3. Doc is at revision
        //    1, so check_revision_is_bumped_by_one_during_replace_v0 returns
        //    InvalidDocumentRevisionError(Some(1), 3) and we hit the
        //    failure-with-bump path in the transformer.
        let mut altered_document = document.clone();
        altered_document.set_revision(Some(3));
        altered_document.set("displayName", "Out of order".into());

        let replace_transition =
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
            .await
            .expect("expected to build replace transition");

        let replace_serialized = replace_transition
            .serialize_to_bytes()
            .expect("expected to serialize replace");

        let transaction = platform.drive.grove.start_transaction();
        let replace_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![replace_serialized.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process replace");
        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit failed replace");

        assert_eq!(
            replace_result.invalid_paid_count(),
            1,
            "Replace must commit as invalid_paid (PaidConsensusError); execution_results={:?}",
            replace_result.execution_results()
        );
        assert_eq!(replace_result.valid_count(), 0);

        // 3) Direct invariant: the bump must have advanced the contract nonce
        //    in state. If the stored nonce is still post-create, the bump
        //    silently dropped — that is the bug.
        let (post_replace_nonce_raw, _) = platform
            .drive
            .fetch_identity_contract_nonce_with_fees(
                identity.id().to_buffer(),
                dashpay_contract.id().to_buffer(),
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to fetch contract nonce after failed replace");
        let post_replace_nonce =
            post_replace_nonce_raw.expect("contract nonce must be present after failed replace");

        assert_ne!(
            post_replace_nonce, post_create_nonce,
            "failed Replace's bump action did not advance the contract \
             nonce — stored nonce is still {:#x} (= post-create value), so \
             the same serialized bytes can be replayed",
            post_create_nonce
        );

        // 4) Re-submitting identical bytes through CheckTx FirstTimeCheck must
        //    hit the nonce check first and reject.
        let replayed_state_transition =
            StateTransition::deserialize_from_bytes_untrusted(&replace_serialized)
                .expect("expected to deserialize replayed transition");

        let platform_state = platform.state.load();
        let platform_ref = PlatformRef {
            drive: &platform.drive,
            state: &platform_state,
            config: &platform.config,
            core_rpc: &platform.core_rpc,
        };

        let check_tx_result = state_transition_to_execution_event_for_check_tx(
            &platform_ref,
            &replayed_state_transition,
            CheckTxLevel::FirstTimeCheck,
            &platform.check_tx_proof_verifier,
            platform_version,
        )
        .expect("expected check_tx to not return an Err");

        assert!(
            !check_tx_result.is_valid(),
            "CheckTx FirstTimeCheck must reject identical bytes after the \
             failed-Replace bump consumed the nonce"
        );
        assert!(
            check_tx_result.errors.iter().any(|e| matches!(
                e,
                ConsensusError::StateError(StateError::InvalidIdentityNonceError(_))
            )),
            "expected InvalidIdentityNonceError on replay; got {:?}",
            check_tx_result.errors
        );
    }

    #[tokio::test]
    async fn test_document_replace_on_document_type_that_is_not_mutable_but_is_transferable() {
        run_document_replace_on_document_type_that_is_not_mutable_but_is_transferable_at_protocol_version(
            PlatformVersion::latest().protocol_version,
            457680, // v14: stamped documents (see happy-path baseline note)
        )
        .await;
    }

    /// PROTOCOL_VERSION_13: pre-stamp fee — document serialization format 3
    /// (the contract-version stamp) activates at v14, so v13 costs must be
    /// exactly what they were before the `requiredSince` changes. Pinned so
    /// v13 chain history stays bit-for-bit reproducible.
    #[tokio::test]
    async fn test_document_replace_on_document_type_that_is_not_mutable_but_is_transferable_protocol_version_13(
    ) {
        run_document_replace_on_document_type_that_is_not_mutable_but_is_transferable_at_protocol_version(
            13,
            457660,
        )
        .await;
    }

    /// PROTOCOL_VERSION_11: pre-B7 bump-only fee (transformer's local
    /// execution context dropped the per-transition reads). Pinned so
    /// v11 chain history stays bit-for-bit reproducible.
    #[tokio::test]
    async fn test_document_replace_on_document_type_that_is_not_mutable_but_is_transferable_protocol_version_11(
    ) {
        run_document_replace_on_document_type_that_is_not_mutable_but_is_transferable_at_protocol_version(
            11,
            445700,
        )
        .await;
    }

    async fn run_document_replace_on_document_type_that_is_not_mutable_but_is_transferable_at_protocol_version(
        protocol_version: dpp::version::ProtocolVersion,
        expected_processing_fee: dpp::fee::Credits,
    ) {
        let platform_version = PlatformVersion::get(protocol_version)
            .expect("expected platform version for the requested protocol_version");
        let (mut platform, contract) = TestPlatformBuilder::new()
            .with_initial_protocol_version(protocol_version)
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
        document
            .set_id_for_creation(card_document_type, &entropy.0, 2, platform_version)
            .expect("expected to set the document id");

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
            platform_version,
        )
        .expect("expected document query");

        let receiver_documents_sql_string =
            format!("select * from card where $ownerId == '{}'", receiver.id());

        let query_receiver_identity_documents = DriveDocumentQuery::from_sql_expr(
            receiver_documents_sql_string.as_str(),
            &contract,
            Some(&platform.config.drive),
            platform_version,
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
            .await
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

        assert_eq!(
            processing_result.aggregated_fees().processing_fee,
            expected_processing_fee,
            "PROTOCOL_VERSION_{}: paid-error replace processing fee must match the version-specific baseline",
            protocol_version,
        );

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

    /// Helper for the paired Replace-on-missing-document test.
    ///
    /// Both versions land as PaidConsensusError because the Replace
    /// missing-target-document path emits a `BumpIdentityDataContractNonce`
    /// action on every protocol version (it was the one legacy v0 bump
    /// site, preserved to keep PROTOCOL_VERSION_11 chain replay bit-for-bit
    /// reproducible). Only the fee differs.
    async fn run_document_replace_that_does_not_yet_exist_at_protocol_version(
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

        set_valid_profile_payment_addresses(&mut document, profile);

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
            .await
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

        assert_eq!(
            processing_result.invalid_paid_count(),
            1,
            "PROTOCOL_VERSION_{}: must land as PaidConsensusError",
            protocol_version,
        );

        assert_eq!(processing_result.invalid_unpaid_count(), 0);

        assert_eq!(processing_result.valid_count(), 0);

        assert_eq!(
            processing_result.aggregated_fees().processing_fee,
            expected_processing_fee,
            "PROTOCOL_VERSION_{}: processing fee must match the version-specific baseline",
            protocol_version,
        );
    }

    /// PROTOCOL_VERSION_12+ — bump emission for this specific path is
    /// unconditional (pre-existing legacy behavior), but the document
    /// query now bills its cost on top of v11's bump-only fee.
    #[tokio::test]
    async fn test_document_replace_that_does_not_yet_exist() {
        run_document_replace_that_does_not_yet_exist_at_protocol_version(
            PlatformVersion::latest().protocol_version,
            520340,
        )
        .await;
    }

    /// PROTOCOL_VERSION_11 — pins the legacy fee + bump-emission behavior.
    /// This is the one Replace failure path that already emitted a bump on
    /// v11; the bump-emission helper must not strip it on v0.
    #[tokio::test]
    async fn test_document_replace_that_does_not_yet_exist_protocol_version_11() {
        run_document_replace_that_does_not_yet_exist_at_protocol_version(11, 516040).await;
    }

    #[tokio::test]
    async fn test_double_document_replace() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(433);

        fast_forward_to_block(&platform, 1_200_000_000, 900, 42, 1, false); //next epoch

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
        document
            .set_id_for_creation(profile, &entropy.0, 2, platform_version)
            .expect("expected to set the document id");

        set_valid_profile_payment_addresses(&mut document, profile);

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
            platform_version,
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

        assert_eq!(document.to_string(), "v0 : id:Hek9BmBiymTrsxccySNNsQCyhgX1J7fUAafSMeDb1Pd6 owner_id:CisQdz2ej7EwWv8JbetSXBNsV4xsf8QsSS8tqp4tEf7V created_at:1970-01-14 21:20:00 updated_at:1970-01-14 21:20:00 avatarFingerprint:bytes d7b0e2b357c10312 avatarHash:bytes32 YonaRoE0hMgat53AYt5LTlQlIkKLReGpB7xNAqJ5HM8= avatarUrl:string http://test.com/bob.[...(23)] corePaymentAddress:bytes 000000000000000000000000000000000000000000 displayName:string QBwBNNXXYCngB0er platformPaymentAddress:bytes 010000000000000000000000000000000000000000 publicMessage:string 8XG7KBGNvm2 shieldedAddress:bytes b3bb8852a93313580b0f9cef98328f2fa69b49e87f74043b160f4edb43e8cbe62c4985b6ec6094dd7554da  ");

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
            .await
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
            .await
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

        assert_eq!(document.to_string(), "v0 : id:Hek9BmBiymTrsxccySNNsQCyhgX1J7fUAafSMeDb1Pd6 owner_id:CisQdz2ej7EwWv8JbetSXBNsV4xsf8QsSS8tqp4tEf7V created_at:1970-01-14 21:20:00 updated_at:1970-01-14 21:20:00 avatarFingerprint:bytes d7b0e2b357c10312 avatarHash:bytes32 YonaRoE0hMgat53AYt5LTlQlIkKLReGpB7xNAqJ5HM8= avatarUrl:string http://test.com/drap[...(26)] corePaymentAddress:bytes 000000000000000000000000000000000000000000 displayName:string Ody platformPaymentAddress:bytes 010000000000000000000000000000000000000000 publicMessage:string 8XG7KBGNvm2 shieldedAddress:bytes b3bb8852a93313580b0f9cef98328f2fa69b49e87f74043b160f4edb43e8cbe62c4985b6ec6094dd7554da  ");

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
    async fn test_double_document_replace_different_height_same_epoch() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(433);

        fast_forward_to_block(&platform, 1_200_000_000, 900, 42, 1, false); //next epoch

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
        document
            .set_id_for_creation(profile, &entropy.0, 2, platform_version)
            .expect("expected to set the document id");

        set_valid_profile_payment_addresses(&mut document, profile);

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
            platform_version,
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

        assert_eq!(document.to_string(), "v0 : id:Hek9BmBiymTrsxccySNNsQCyhgX1J7fUAafSMeDb1Pd6 owner_id:CisQdz2ej7EwWv8JbetSXBNsV4xsf8QsSS8tqp4tEf7V created_at:1970-01-14 21:20:00 updated_at:1970-01-14 21:20:00 avatarFingerprint:bytes d7b0e2b357c10312 avatarHash:bytes32 YonaRoE0hMgat53AYt5LTlQlIkKLReGpB7xNAqJ5HM8= avatarUrl:string http://test.com/bob.[...(23)] corePaymentAddress:bytes 000000000000000000000000000000000000000000 displayName:string QBwBNNXXYCngB0er platformPaymentAddress:bytes 010000000000000000000000000000000000000000 publicMessage:string 8XG7KBGNvm2 shieldedAddress:bytes b3bb8852a93313580b0f9cef98328f2fa69b49e87f74043b160f4edb43e8cbe62c4985b6ec6094dd7554da  ");

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
            .await
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
            .await
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

        assert_eq!(document.to_string(), "v0 : id:Hek9BmBiymTrsxccySNNsQCyhgX1J7fUAafSMeDb1Pd6 owner_id:CisQdz2ej7EwWv8JbetSXBNsV4xsf8QsSS8tqp4tEf7V created_at:1970-01-14 21:20:00 updated_at:1970-01-17 04:53:20 avatarFingerprint:bytes d7b0e2b357c10312 avatarHash:bytes32 YonaRoE0hMgat53AYt5LTlQlIkKLReGpB7xNAqJ5HM8= avatarUrl:string http://test.com/cat.[...(23)] corePaymentAddress:bytes 000000000000000000000000000000000000000000 displayName:string Samuel platformPaymentAddress:bytes 010000000000000000000000000000000000000000 publicMessage:string 8XG7KBGNvm2 shieldedAddress:bytes b3bb8852a93313580b0f9cef98328f2fa69b49e87f74043b160f4edb43e8cbe62c4985b6ec6094dd7554da  ");

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

        assert_eq!(document.to_string(), "v0 : id:Hek9BmBiymTrsxccySNNsQCyhgX1J7fUAafSMeDb1Pd6 owner_id:CisQdz2ej7EwWv8JbetSXBNsV4xsf8QsSS8tqp4tEf7V created_at:1970-01-14 21:20:00 updated_at:1970-01-19 12:26:40 avatarFingerprint:bytes d7b0e2b357c10312 avatarHash:bytes32 YonaRoE0hMgat53AYt5LTlQlIkKLReGpB7xNAqJ5HM8= avatarUrl:string http://test.com/drap[...(26)] corePaymentAddress:bytes 000000000000000000000000000000000000000000 displayName:string Ody platformPaymentAddress:bytes 010000000000000000000000000000000000000000 publicMessage:string 8XG7KBGNvm2 shieldedAddress:bytes b3bb8852a93313580b0f9cef98328f2fa69b49e87f74043b160f4edb43e8cbe62c4985b6ec6094dd7554da  ");

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
    async fn test_double_document_replace_no_change_different_height_same_epoch() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(433);

        fast_forward_to_block(&platform, 1_200_000_000, 900, 42, 1, false); //next epoch

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
        document
            .set_id_for_creation(profile, &entropy.0, 2, platform_version)
            .expect("expected to set the document id");

        set_valid_profile_payment_addresses(&mut document, profile);

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
            platform_version,
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

        assert_eq!(document.to_string(), "v0 : id:Hek9BmBiymTrsxccySNNsQCyhgX1J7fUAafSMeDb1Pd6 owner_id:CisQdz2ej7EwWv8JbetSXBNsV4xsf8QsSS8tqp4tEf7V created_at:1970-01-14 21:20:00 updated_at:1970-01-14 21:20:00 avatarFingerprint:bytes d7b0e2b357c10312 avatarHash:bytes32 YonaRoE0hMgat53AYt5LTlQlIkKLReGpB7xNAqJ5HM8= avatarUrl:string http://test.com/bob.[...(23)] corePaymentAddress:bytes 000000000000000000000000000000000000000000 displayName:string QBwBNNXXYCngB0er platformPaymentAddress:bytes 010000000000000000000000000000000000000000 publicMessage:string 8XG7KBGNvm2 shieldedAddress:bytes b3bb8852a93313580b0f9cef98328f2fa69b49e87f74043b160f4edb43e8cbe62c4985b6ec6094dd7554da  ");

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
            .await
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
            .await
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

        assert_eq!(document.to_string(), "v0 : id:Hek9BmBiymTrsxccySNNsQCyhgX1J7fUAafSMeDb1Pd6 owner_id:CisQdz2ej7EwWv8JbetSXBNsV4xsf8QsSS8tqp4tEf7V created_at:1970-01-14 21:20:00 updated_at:1970-01-17 04:53:20 avatarFingerprint:bytes d7b0e2b357c10312 avatarHash:bytes32 YonaRoE0hMgat53AYt5LTlQlIkKLReGpB7xNAqJ5HM8= avatarUrl:string http://test.com/bob.[...(23)] corePaymentAddress:bytes 000000000000000000000000000000000000000000 displayName:string QBwBNNXXYCngB0er platformPaymentAddress:bytes 010000000000000000000000000000000000000000 publicMessage:string 8XG7KBGNvm2 shieldedAddress:bytes b3bb8852a93313580b0f9cef98328f2fa69b49e87f74043b160f4edb43e8cbe62c4985b6ec6094dd7554da  ");

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

        assert_eq!(document.to_string(), "v0 : id:Hek9BmBiymTrsxccySNNsQCyhgX1J7fUAafSMeDb1Pd6 owner_id:CisQdz2ej7EwWv8JbetSXBNsV4xsf8QsSS8tqp4tEf7V created_at:1970-01-14 21:20:00 updated_at:1970-01-19 12:26:40 avatarFingerprint:bytes d7b0e2b357c10312 avatarHash:bytes32 YonaRoE0hMgat53AYt5LTlQlIkKLReGpB7xNAqJ5HM8= avatarUrl:string http://test.com/bob.[...(23)] corePaymentAddress:bytes 000000000000000000000000000000000000000000 displayName:string QBwBNNXXYCngB0er platformPaymentAddress:bytes 010000000000000000000000000000000000000000 publicMessage:string 8XG7KBGNvm2 shieldedAddress:bytes b3bb8852a93313580b0f9cef98328f2fa69b49e87f74043b160f4edb43e8cbe62c4985b6ec6094dd7554da  ");

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
    async fn test_double_document_replace_different_height_different_epoch() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(433);

        fast_forward_to_block(&platform, 1_200_000_000, 900, 42, 1, false); //next epoch

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
        document
            .set_id_for_creation(profile, &entropy.0, 2, platform_version)
            .expect("expected to set the document id");

        set_valid_profile_payment_addresses(&mut document, profile);

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
            platform_version,
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

        assert_eq!(document.to_string(), "v0 : id:Hek9BmBiymTrsxccySNNsQCyhgX1J7fUAafSMeDb1Pd6 owner_id:CisQdz2ej7EwWv8JbetSXBNsV4xsf8QsSS8tqp4tEf7V created_at:1970-01-14 21:20:00 updated_at:1970-01-14 21:20:00 avatarFingerprint:bytes d7b0e2b357c10312 avatarHash:bytes32 YonaRoE0hMgat53AYt5LTlQlIkKLReGpB7xNAqJ5HM8= avatarUrl:string http://test.com/bob.[...(23)] corePaymentAddress:bytes 000000000000000000000000000000000000000000 displayName:string QBwBNNXXYCngB0er platformPaymentAddress:bytes 010000000000000000000000000000000000000000 publicMessage:string 8XG7KBGNvm2 shieldedAddress:bytes b3bb8852a93313580b0f9cef98328f2fa69b49e87f74043b160f4edb43e8cbe62c4985b6ec6094dd7554da  ");

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
            .await
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
            .await
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

        assert_eq!(document.to_string(), "v0 : id:Hek9BmBiymTrsxccySNNsQCyhgX1J7fUAafSMeDb1Pd6 owner_id:CisQdz2ej7EwWv8JbetSXBNsV4xsf8QsSS8tqp4tEf7V created_at:1970-01-14 21:20:00 updated_at:1970-01-17 04:53:20 avatarFingerprint:bytes d7b0e2b357c10312 avatarHash:bytes32 YonaRoE0hMgat53AYt5LTlQlIkKLReGpB7xNAqJ5HM8= avatarUrl:string http://test.com/cat.[...(23)] corePaymentAddress:bytes 000000000000000000000000000000000000000000 displayName:string Samuel platformPaymentAddress:bytes 010000000000000000000000000000000000000000 publicMessage:string 8XG7KBGNvm2 shieldedAddress:bytes b3bb8852a93313580b0f9cef98328f2fa69b49e87f74043b160f4edb43e8cbe62c4985b6ec6094dd7554da  ");

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

        assert_eq!(document.to_string(), "v0 : id:Hek9BmBiymTrsxccySNNsQCyhgX1J7fUAafSMeDb1Pd6 owner_id:CisQdz2ej7EwWv8JbetSXBNsV4xsf8QsSS8tqp4tEf7V created_at:1970-01-14 21:20:00 updated_at:1970-01-19 12:26:40 avatarFingerprint:bytes d7b0e2b357c10312 avatarHash:bytes32 YonaRoE0hMgat53AYt5LTlQlIkKLReGpB7xNAqJ5HM8= avatarUrl:string http://test.com/drap[...(26)] corePaymentAddress:bytes 000000000000000000000000000000000000000000 displayName:string Ody platformPaymentAddress:bytes 010000000000000000000000000000000000000000 publicMessage:string 8XG7KBGNvm2 shieldedAddress:bytes b3bb8852a93313580b0f9cef98328f2fa69b49e87f74043b160f4edb43e8cbe62c4985b6ec6094dd7554da  ");

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
    async fn test_document_replace_on_document_type_that_requires_a_token() {
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
        document
            .set_id_for_creation(card_document_type, &entropy.0, 2, platform_version)
            .expect("expected to set the document id");

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
            .await
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
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
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

    #[tokio::test]
    async fn should_document_replace_fail_when_referenced_identity_missing() {
        let (result, _) = run_reference_validation_replace_with_contract(
            REFERENCE_VALIDATION_CONTRACT_PATH,
            |_, _| Identifier::random(),
            false,
        )
        .await;

        assert_matches!(
            result,
            PaidConsensusError {
                error: ConsensusError::StateError(StateError::ReferencedEntityNotFoundError(_)),
                ..
            }
        );
    }

    #[tokio::test]
    async fn should_document_replace_validate_only_changed_fields() {
        let (_, fee_without_reference) = run_reference_validation_replace_with_contract(
            REFERENCE_VALIDATION_CONTRACT_PATH,
            |identity_id, _| identity_id,
            true,
        )
        .await;

        let (_, fee_with_reference) = run_reference_validation_replace_with_contract(
            REFERENCE_VALIDATION_CONTRACT_PATH,
            |_, other_id| other_id,
            true,
        )
        .await;

        assert!(
            fee_with_reference.processing_fee > fee_without_reference.processing_fee,
            "expected identity reference validation to increase processing fee"
        );
    }

    #[tokio::test]
    async fn should_document_replace_fail_when_nested_reference_changed_to_missing_identity() {
        // Regression: changed_data_fields holds top-level keys ("meta"), while
        // reference properties are tracked by flattened path ("meta.nestedUserId");
        // a nested reference under a changed object must still be validated.
        let result = run_reference_validation_create_then_replace(
            REFERENCE_VALIDATION_NESTED_CONTRACT_PATH,
            |document, owner_id, other_id| {
                document.set("toUserId", owner_id.into());
                document.set("otherUserId", other_id.into());
                document.set("meta.nestedUserId", owner_id.into());
            },
            |document, _, _| {
                document.set("meta.nestedUserId", Identifier::random().into());
            },
        )
        .await;

        assert_matches!(
            result,
            PaidConsensusError {
                error: ConsensusError::StateError(StateError::ReferencedEntityNotFoundError(_)),
                ..
            }
        );
    }

    #[tokio::test]
    async fn should_document_replace_succeed_when_optional_reference_removed() {
        let result = run_reference_validation_create_then_replace(
            REFERENCE_VALIDATION_OPTIONAL_CONTRACT_PATH,
            |document, owner_id, _| {
                document.set("optionalUserId", owner_id.into());
            },
            |document, _, _| {
                document.remove("optionalUserId");
            },
        )
        .await;

        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
    }

    #[tokio::test]
    async fn should_document_replace_fail_when_reference_field_changed_to_missing_identity() {
        let (result, _) = run_reference_validation_replace_with_contract(
            REFERENCE_VALIDATION_CONTRACT_PATH,
            |_, _| Identifier::random(),
            true,
        )
        .await;

        assert_matches!(
            result,
            PaidConsensusError {
                error: ConsensusError::StateError(StateError::ReferencedEntityNotFoundError(_)),
                ..
            }
        );
    }

    const REFERENCE_VALIDATION_IDENTITY_KEY_CONTRACT_PATH: &str =
        "tests/supporting_files/contract/reference-validation/reference-validation-contract-identity-key.json";

    /// Committed state the identity-key reference replace tests can point at:
    /// the test identity, its enabled critical authentication key and its
    /// master key, which the helper disables in state.
    struct IdentityKeyReferenceTargets {
        identity_id: Identifier,
        enabled_key_id: KeyID,
        disabled_key_id: KeyID,
    }

    /// Registers the identity-key fixture contract, disables the test
    /// identity's master key in state, creates a `message` document shaped by
    /// `create_mutator` (asserting success), then replaces it shaped by
    /// `replace_mutator` and returns the replace execution result.
    async fn run_identity_key_reference_create_then_replace<C, R>(
        create_mutator: C,
        replace_mutator: R,
    ) -> StateTransitionExecutionResult
    where
        C: FnOnce(&mut Document, &IdentityKeyReferenceTargets),
        R: FnOnce(&mut Document, &IdentityKeyReferenceTargets),
    {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(433);

        let platform_state = platform.state.load();

        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

        // Key 0 is the master key; documents are signed with the critical key,
        // so disabling it leaves the transitions below valid
        platform
            .drive
            .disable_identity_keys(
                identity.id().to_buffer(),
                vec![0],
                1,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to disable the master key");

        let targets = IdentityKeyReferenceTargets {
            identity_id: identity.id(),
            enabled_key_id: key.id(),
            disabled_key_id: 0,
        };

        let contract = setup_contract(
            &platform.drive,
            REFERENCE_VALIDATION_IDENTITY_KEY_CONTRACT_PATH,
            None,
            None,
            None::<fn(&mut DataContract)>,
            None,
            None,
        );

        let message = contract
            .document_type_for_name("message")
            .expect("expected a message document type");

        let entropy = Bytes32::random_with_rng(&mut rng);

        let mut document = message
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity.id(),
                entropy,
                // The reference properties are optional; each test sets only
                // what it exercises
                DocumentFieldFillType::DoNotFillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random message document");
        document
            .set_id_for_creation(message, &entropy.0, 2, platform_version)
            .expect("expected to set the document id");

        create_mutator(&mut document, &targets);

        let documents_batch_create_transition =
            BatchTransition::new_document_creation_transition_from_document(
                document.clone(),
                message,
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

        document.increment_revision().unwrap();
        replace_mutator(&mut document, &targets);

        let documents_batch_replace_transition =
            BatchTransition::new_document_replacement_transition_from_document(
                document,
                message,
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

        let documents_batch_replace_serialized_transition = documents_batch_replace_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[documents_batch_replace_serialized_transition],
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

        processing_result
            .execution_results()
            .first()
            .expect("expected one execution result")
            .clone()
    }

    /// Registers the key-requirements fixture contract, adds the keys of
    /// [`IdentityKeyRequirementTargets`] to the test identity, creates a `message`
    /// document referencing the key that meets the requirements (asserting success),
    /// then replaces it shaped by `replace_mutator` and returns the replace execution
    /// result.
    async fn run_identity_key_requirement_create_then_replace<R>(
        replace_mutator: R,
    ) -> StateTransitionExecutionResult
    where
        R: FnOnce(&mut Document, &IdentityKeyRequirementTargets),
    {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(433);

        let platform_state = platform.state.load();

        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

        let contract = setup_contract(
            &platform.drive,
            REFERENCE_VALIDATION_IDENTITY_KEY_REQUIREMENTS_CONTRACT_PATH,
            None,
            None,
            None::<fn(&mut DataContract)>,
            None,
            None,
        );

        let targets = add_identity_key_requirement_targets(
            &mut platform,
            &identity,
            key.id(),
            contract.id(),
            platform_version,
        );

        let message = contract
            .document_type_for_name("message")
            .expect("expected a message document type");

        let entropy = Bytes32::random_with_rng(&mut rng);

        let mut document = message
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity.id(),
                entropy,
                DocumentFieldFillType::DoNotFillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random message document");
        document
            .set_id_for_creation(message, &entropy.0, 2, platform_version)
            .expect("expected to set the document id");

        document.set("recipientId", targets.identity_id.into());
        document.set(
            "recipientKeyId",
            (targets.decryption_key_bound_to_inbox_id as i64).into(),
        );
        document.set("note", "hello".into());

        let documents_batch_create_transition =
            BatchTransition::new_document_creation_transition_from_document(
                document.clone(),
                message,
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

        document.increment_revision().unwrap();
        replace_mutator(&mut document, &targets);

        let documents_batch_replace_transition =
            BatchTransition::new_document_replacement_transition_from_document(
                document,
                message,
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

        let documents_batch_replace_serialized_transition = documents_batch_replace_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[documents_batch_replace_serialized_transition],
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

        processing_result
            .execution_results()
            .first()
            .expect("expected one execution result")
            .clone()
    }

    /// keyRequirements on replace: repointing the reference at a key that
    /// fails a requirement is refused, through the key id alone.
    #[tokio::test]
    async fn should_document_replace_fail_when_repointed_at_a_key_that_fails_the_requirement() {
        let result = run_identity_key_requirement_create_then_replace(|document, targets| {
            document.set(
                "recipientKeyId",
                (targets.encryption_key_bound_to_inbox_id as i64).into(),
            );
        })
        .await;

        assert_matches!(
            result,
            PaidConsensusError {
                error: ConsensusError::StateError(
                    StateError::ReferencedIdentityKeyRequirementNotMetError(ref e)
                ),
                ..
            } if e.document_type_name() == "message"
                && e.path() == "recipientId"
                && e.field() == "purpose"
                && e.required() == "decryption"
                && e.actual() == "encryption"
        );
    }

    /// keyRequirements on replace: a replace that leaves the reference and its
    /// key id alone is not re-checked, and one that repoints it at another key
    /// meeting the requirements passes.
    #[tokio::test]
    async fn should_document_replace_succeed_when_the_reference_is_untouched_or_still_met() {
        let result = run_identity_key_requirement_create_then_replace(|document, _| {
            document.set("note", "changed".into());
        })
        .await;

        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );

        let result = run_identity_key_requirement_create_then_replace(|document, targets| {
            document.set("recipientId", targets.identity_id.into());
            document.set(
                "recipientKeyId",
                (targets.decryption_key_bound_to_inbox_id as i64).into(),
            );
            document.set("note", "changed".into());
        })
        .await;

        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
    }

    /// identityPublicKey on replace: changing only the key id property while
    /// leaving the identity id untouched must re-validate the reference:
    /// the referenced key is the (identity id, key id) pair, so the
    /// changed-fields gate binds the key id property the way it binds an
    /// agreement's referring property.
    #[tokio::test]
    async fn should_document_replace_fail_when_only_key_id_changed_to_disabled_key() {
        let result = run_identity_key_reference_create_then_replace(
            |document, targets| {
                document.set("toUserId", targets.identity_id.into());
                document.set("toKeyIndex", (targets.enabled_key_id as i64).into());
            },
            |document, targets| {
                document.set("toKeyIndex", (targets.disabled_key_id as i64).into());
            },
        )
        .await;

        assert_matches!(
            result,
            PaidConsensusError {
                error: ConsensusError::StateError(StateError::ReferencedIdentityKeyDisabledError(
                    _
                )),
                ..
            }
        );
    }

    #[tokio::test]
    async fn should_document_replace_fail_when_only_key_id_changed_to_missing_key() {
        let result = run_identity_key_reference_create_then_replace(
            |document, targets| {
                document.set("toUserId", targets.identity_id.into());
                document.set("toKeyIndex", (targets.enabled_key_id as i64).into());
            },
            |document, _| {
                document.set("toKeyIndex", 99i64.into());
            },
        )
        .await;

        assert_matches!(
            result,
            PaidConsensusError {
                error: ConsensusError::StateError(StateError::ReferencedIdentityKeyNotFoundError(
                    _
                )),
                ..
            }
        );
    }

    #[tokio::test]
    async fn should_document_replace_succeed_when_key_reference_and_key_id_untouched() {
        let result = run_identity_key_reference_create_then_replace(
            |document, targets| {
                document.set("toUserId", targets.identity_id.into());
                document.set("toKeyIndex", (targets.enabled_key_id as i64).into());
            },
            |document, _| {
                document.set("note", "changed".into());
            },
        )
        .await;

        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
    }

    const REFERENCE_VALIDATION_OWNER_KEY_CONTRACT_PATH: &str =
        "tests/supporting_files/contract/reference-validation/reference-validation-contract-owner-key.json";

    /// Committed state the key id reference replace tests can point at: the
    /// writer and a second identity, each with an enabled critical key and a
    /// master key (key 0) that the helper may disable between the create and
    /// the replace.
    struct KeyIdReferenceTargets {
        writer_id: Identifier,
        enabled_key_id: KeyID,
        master_key_id: KeyID,
        other_id: Identifier,
        other_enabled_key_id: KeyID,
    }

    /// Whose master key the helper disables between the create and the
    /// replace, so that a replace refetching the key is observable.
    enum DisableMasterKeyBetween {
        Nobody,
        Writer,
        Other,
    }

    /// Registers the key id reference fixture at `contract_path` (a `message`
    /// type whose key id property carries `refersTo: identityPublicKey` with
    /// an `identityProperty`), creates a `message` document shaped by
    /// `create_mutator` (asserting success), disables the chosen master key
    /// in state, replaces the document shaped by `replace_mutator` and
    /// returns the replace execution result. Master keys are enabled at
    /// create time, so a create may reference one.
    async fn run_key_id_reference_create_then_replace<C, R>(
        contract_path: &str,
        create_mutator: C,
        disable_between: DisableMasterKeyBetween,
        replace_mutator: R,
    ) -> StateTransitionExecutionResult
    where
        C: FnOnce(&mut Document, &KeyIdReferenceTargets),
        R: FnOnce(&mut Document, &KeyIdReferenceTargets),
    {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(434);

        let platform_state = platform.state.load();

        let (identity, signer, key) = setup_identity(&mut platform, 959, dash_to_credits!(0.1));
        let (other, _, other_key) = setup_identity(&mut platform, 452, dash_to_credits!(0.1));

        let targets = KeyIdReferenceTargets {
            writer_id: identity.id(),
            enabled_key_id: key.id(),
            master_key_id: 0,
            other_id: other.id(),
            other_enabled_key_id: other_key.id(),
        };

        let contract = setup_contract(
            &platform.drive,
            contract_path,
            None,
            None,
            None::<fn(&mut DataContract)>,
            None,
            None,
        );

        let message = contract
            .document_type_for_name("message")
            .expect("expected a message document type");

        let entropy = Bytes32::random_with_rng(&mut rng);

        let mut document = message
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity.id(),
                entropy,
                // The key id property is optional; each test sets what it
                // exercises
                DocumentFieldFillType::DoNotFillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random message document");
        document
            .set_id_for_creation(message, &entropy.0, 2, platform_version)
            .expect("expected to set the document id");

        create_mutator(&mut document, &targets);

        let documents_batch_create_transition =
            BatchTransition::new_document_creation_transition_from_document(
                document.clone(),
                message,
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

        let disabled_identity = match disable_between {
            DisableMasterKeyBetween::Nobody => None,
            DisableMasterKeyBetween::Writer => Some(identity.id()),
            DisableMasterKeyBetween::Other => Some(other.id()),
        };
        if let Some(identity_id) = disabled_identity {
            // Documents are signed with the critical key, so the replace
            // below stays valid
            platform
                .drive
                .disable_identity_keys(
                    identity_id.to_buffer(),
                    vec![0],
                    1,
                    &BlockInfo::default(),
                    true,
                    None,
                    platform_version,
                )
                .expect("expected to disable the master key");
        }

        document.increment_revision().unwrap();
        replace_mutator(&mut document, &targets);

        let documents_batch_replace_transition =
            BatchTransition::new_document_replacement_transition_from_document(
                document,
                message,
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

        let documents_batch_replace_serialized_transition = documents_batch_replace_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[documents_batch_replace_serialized_transition],
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

        processing_result
            .execution_results()
            .first()
            .expect("expected one execution result")
            .clone()
    }

    /// The key id form on replace: a changed key id is re-validated against
    /// the owner's keys, the identity being the writer by construction.
    #[tokio::test]
    async fn should_document_replace_fail_when_owner_key_id_changed_to_a_missing_key() {
        let result = run_key_id_reference_create_then_replace(
            REFERENCE_VALIDATION_OWNER_KEY_CONTRACT_PATH,
            |document, targets| {
                document.set("senderKeyId", (targets.enabled_key_id as i64).into());
            },
            DisableMasterKeyBetween::Nobody,
            |document, _| {
                document.set("senderKeyId", 99i64.into());
            },
        )
        .await;

        assert_matches!(
            result,
            PaidConsensusError {
                error: ConsensusError::StateError(StateError::ReferencedIdentityKeyNotFoundError(
                    e
                )),
                ..
            } if e.key_id() == 99 && e.path() == "senderKeyId"
        );
    }

    #[tokio::test]
    async fn should_document_replace_fail_when_owner_key_id_changed_to_a_disabled_key() {
        let result = run_key_id_reference_create_then_replace(
            REFERENCE_VALIDATION_OWNER_KEY_CONTRACT_PATH,
            |document, targets| {
                document.set("senderKeyId", (targets.enabled_key_id as i64).into());
            },
            DisableMasterKeyBetween::Writer,
            |document, targets| {
                document.set("senderKeyId", (targets.master_key_id as i64).into());
            },
        )
        .await;

        assert_matches!(
            result,
            PaidConsensusError {
                error: ConsensusError::StateError(StateError::ReferencedIdentityKeyDisabledError(
                    _
                )),
                ..
            }
        );
    }

    /// An untouched key id is re-validated on every replace, as the writer
    /// gate is: the identity is the writer, which is transition metadata and
    /// never among the changed fields. The document was created naming the
    /// master key while it was enabled, the key is disabled before the
    /// replace, and a replace of another property is refused.
    #[tokio::test]
    async fn should_document_replace_fail_when_untouched_owner_key_id_names_a_now_disabled_key() {
        let result = run_key_id_reference_create_then_replace(
            REFERENCE_VALIDATION_OWNER_KEY_CONTRACT_PATH,
            |document, targets| {
                document.set("senderKeyId", (targets.master_key_id as i64).into());
            },
            DisableMasterKeyBetween::Writer,
            |document, _| {
                document.set("note", "changed".into());
            },
        )
        .await;

        assert_matches!(
            result,
            PaidConsensusError {
                error: ConsensusError::StateError(StateError::ReferencedIdentityKeyDisabledError(
                    _
                )),
                ..
            }
        );
    }

    /// The same untouched replace passes while the key stays enabled: the
    /// refetch is a check, not a change.
    #[tokio::test]
    async fn should_document_replace_succeed_when_untouched_owner_key_id_still_names_an_enabled_key(
    ) {
        let result = run_key_id_reference_create_then_replace(
            REFERENCE_VALIDATION_OWNER_KEY_CONTRACT_PATH,
            |document, targets| {
                document.set("senderKeyId", (targets.enabled_key_id as i64).into());
            },
            DisableMasterKeyBetween::Writer,
            |document, _| {
                document.set("note", "changed".into());
            },
        )
        .await;

        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
    }

    const REFERENCE_VALIDATION_OWNER_KEY_TRANSFERABLE_CONTRACT_PATH: &str =
        "tests/supporting_files/contract/reference-validation/reference-validation-contract-owner-key-transferable.json";

    /// The identities of a transfer: the writer (creator), whose master key
    /// stays enabled, and the receiver, whose master key is disabled.
    struct TransferTargets {
        writer_id: Identifier,
        receiver_id: Identifier,
        receiver_key_id: KeyID,
    }

    /// A transferable key id reference fixture at `contract_path`: the writer
    /// creates a `message` naming its own master key (key 0, enabled) as
    /// `senderKeyId`, transfers it to a receiver whose master key was
    /// disabled in state beforehand, and the receiver replaces it shaped by
    /// `replace_mutator`. Returns the replace execution result and the
    /// identities.
    async fn run_key_id_reference_create_transfer_then_replace<R>(
        contract_path: &str,
        replace_mutator: R,
    ) -> (StateTransitionExecutionResult, TransferTargets)
    where
        R: FnOnce(&mut Document, &TransferTargets),
    {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(435);

        let platform_state = platform.state.load();

        let (identity, signer, key) = setup_identity(&mut platform, 959, dash_to_credits!(0.1));
        let (receiver, receiver_signer, receiver_key) =
            setup_identity(&mut platform, 451, dash_to_credits!(0.1));

        // The receiver's master key is disabled, so a key id of 0 names an
        // enabled key of the writer and a disabled key of the receiver
        platform
            .drive
            .disable_identity_keys(
                receiver.id().to_buffer(),
                vec![0],
                1,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to disable the receiver's master key");

        let contract = setup_contract(
            &platform.drive,
            contract_path,
            None,
            None,
            None::<fn(&mut DataContract)>,
            None,
            None,
        );

        let targets = TransferTargets {
            writer_id: identity.id(),
            receiver_id: receiver.id(),
            receiver_key_id: receiver_key.id(),
        };

        let message = contract
            .document_type_for_name("message")
            .expect("expected a message document type");

        let entropy = Bytes32::random_with_rng(&mut rng);

        let mut document = message
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity.id(),
                entropy,
                DocumentFieldFillType::DoNotFillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random message document");
        document
            .set_id_for_creation(message, &entropy.0, 2, platform_version)
            .expect("expected to set the document id");
        document.set("senderKeyId", 0i64.into());

        let create_transition = BatchTransition::new_document_creation_transition_from_document(
            document.clone(),
            message,
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

        let transaction = platform.drive.grove.start_transaction();
        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[create_transition
                    .serialize_to_bytes()
                    .expect("expected a serialized create transition")],
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

        // The transfer is not a reference check: the document changes hands
        // with its key id as written
        document.set_revision(Some(2));
        let transfer_transition = BatchTransition::new_document_transfer_transition_from_document(
            document.clone(),
            message,
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

        let transaction = platform.drive.grove.start_transaction();
        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[transfer_transition
                    .serialize_to_bytes()
                    .expect("expected a serialized transfer transition")],
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

        document.set_owner_id(receiver.id());
        document.set_revision(Some(3));
        replace_mutator(&mut document, &targets);

        let replace_transition =
            BatchTransition::new_document_replacement_transition_from_document(
                document,
                message,
                &receiver_key,
                1,
                0,
                None,
                &receiver_signer,
                platform_version,
                None,
            )
            .await
            .expect("expect to create documents batch transition");

        let transaction = platform.drive.grove.start_transaction();
        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[replace_transition
                    .serialize_to_bytes()
                    .expect("expected a serialized replace transition")],
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

        (
            processing_result
                .execution_results()
                .first()
                .expect("expected one execution result")
                .clone(),
            targets,
        )
    }

    /// After a transfer the writer is the receiver, and the replace
    /// re-validates the reference against it whether or not the key id
    /// changed: the stored key id names the receiver's disabled master key,
    /// and a replace of another property is refused.
    #[tokio::test]
    async fn should_document_replace_fail_after_transfer_when_untouched_owner_key_id_is_not_the_new_owners_key(
    ) {
        let (result, targets) = run_key_id_reference_create_transfer_then_replace(
            REFERENCE_VALIDATION_OWNER_KEY_TRANSFERABLE_CONTRACT_PATH,
            |document, _| {
                document.set("note", "changed by the receiver".into());
            },
        )
        .await;

        assert_matches!(
            result,
            PaidConsensusError {
                error: ConsensusError::StateError(StateError::ReferencedIdentityKeyDisabledError(
                    e
                )),
                ..
            } if *e.identity_id() == targets.receiver_id
        );
    }

    #[tokio::test]
    async fn should_document_replace_succeed_after_transfer_when_owner_key_id_is_repointed_at_the_new_owners_key(
    ) {
        let (result, _) = run_key_id_reference_create_transfer_then_replace(
            REFERENCE_VALIDATION_OWNER_KEY_TRANSFERABLE_CONTRACT_PATH,
            |document, targets| {
                document.set("senderKeyId", (targets.receiver_key_id as i64).into());
            },
        )
        .await;

        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
    }

    const REFERENCE_VALIDATION_CREATOR_KEY_CONTRACT_PATH: &str =
        "tests/supporting_files/contract/reference-validation/reference-validation-contract-creator-key.json";
    const REFERENCE_VALIDATION_IDENTITY_PROPERTY_KEY_CONTRACT_PATH: &str =
        "tests/supporting_files/contract/reference-validation/reference-validation-contract-identity-property-key.json";

    /// `$creatorId` follows the creator through a transfer: the receiver may
    /// repoint the key id at the creator's master key (enabled on the
    /// creator, disabled on the receiver), which the owner form would refuse.
    #[tokio::test]
    async fn should_document_replace_succeed_after_transfer_when_creator_key_id_names_the_creators_key(
    ) {
        let (result, _) = run_key_id_reference_create_transfer_then_replace(
            REFERENCE_VALIDATION_CREATOR_KEY_CONTRACT_PATH,
            |document, _| {
                document.set("senderKeyId", 0i64.into());
                document.set("note", "changed by the receiver".into());
            },
        )
        .await;

        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
    }

    #[tokio::test]
    async fn should_document_replace_fail_after_transfer_when_creator_key_id_names_a_key_the_creator_lacks(
    ) {
        let (result, targets) = run_key_id_reference_create_transfer_then_replace(
            REFERENCE_VALIDATION_CREATOR_KEY_CONTRACT_PATH,
            |document, _| {
                document.set("senderKeyId", 99i64.into());
            },
        )
        .await;

        assert_matches!(
            result,
            PaidConsensusError {
                error: ConsensusError::StateError(StateError::ReferencedIdentityKeyNotFoundError(
                    e
                )),
                ..
            } if *e.identity_id() == targets.writer_id && e.key_id() == 99
        );
    }

    const REFERENCE_VALIDATION_CREATOR_KEY_BEFORE_CREATOR_IDS_CONTRACT_PATH: &str =
        "tests/supporting_files/contract/reference-validation/reference-validation-contract-creator-key-before-creator-ids.json";
    const REFERENCE_VALIDATION_CREATOR_KEY_BEFORE_CREATOR_IDS_UPDATE_PATH: &str =
        "tests/supporting_files/contract/reference-validation/reference-validation-contract-creator-key-before-creator-ids-update.json";

    /// Processes `transition` at the protocol version of `platform_state`,
    /// commits, and returns its execution result.
    fn process_and_commit_one(
        platform: &TempPlatform<MockCoreRPCLike>,
        platform_state: &PlatformState,
        transition: &StateTransition,
    ) -> StateTransitionExecutionResult {
        let platform_version = platform_state
            .current_platform_version()
            .expect("expected the current platform version");
        let transaction = platform.drive.grove.start_transaction();
        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[transition
                    .serialize_to_bytes()
                    .expect("expected a serialized transition")],
                platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process the state transition");
        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit the transaction");
        processing_result.into_execution_results().remove(0)
    }

    /// A document written before its type recorded creator ids meets a
    /// `$creatorId` key reference a later contract update adds. A transferable
    /// `message` of a format-1 contract with a version 1 config is created at
    /// protocol version 9, which records no creator id for any type; the chain
    /// moves to the latest protocol version, where the type records them; a
    /// contract update adds `senderKeyId` with `identityProperty: $creatorId`,
    /// which registration admits on such a type; and the writer replaces the
    /// old message shaped by `replace_mutator`. Returns the replace execution
    /// result.
    async fn run_replace_of_a_message_written_before_creator_ids<R>(
        replace_mutator: R,
    ) -> StateTransitionExecutionResult
    where
        R: FnOnce(&mut Document),
    {
        let platform_version_9 = PlatformVersion::get(9).expect("expected protocol version 9");
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(9)
            .build_with_mock_rpc()
            .set_initial_state_structure();
        let platform_state = platform.state.load();
        let mut rng = StdRng::seed_from_u64(9437);

        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.5));

        let mut contract = json_document_to_contract(
            REFERENCE_VALIDATION_CREATOR_KEY_BEFORE_CREATOR_IDS_CONTRACT_PATH,
            true,
            platform_version_9,
        )
        .expect("expected to parse the contract at protocol version 9");
        contract.set_owner_id(identity.id());
        // What makes the type record creator ids from protocol version 10 on
        assert!(contract.system_version_type() > 0 && contract.config().version() > 0);
        platform
            .drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version_9,
            )
            .expect("expected to apply the contract");

        let message = contract
            .document_type_for_name("message")
            .expect("expected a message document type");
        assert!(message.documents_transferable().is_transferable());

        let entropy = Bytes32::random_with_rng(&mut rng);
        let mut document = message
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity.id(),
                entropy,
                DocumentFieldFillType::DoNotFillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version_9,
            )
            .expect("expected a random message document");
        document
            .set_id_for_creation(message, &entropy.0, 1, platform_version_9)
            .expect("expected to set the document id");
        document.set("note", "written at protocol version 9".into());

        let create_transition = BatchTransition::new_document_creation_transition_from_document(
            document.clone(),
            message,
            entropy.0,
            &key,
            1,
            0,
            None,
            &signer,
            platform_version_9,
            None,
        )
        .await
        .expect("expected a create transition");
        assert_matches!(
            process_and_commit_one(&platform, &platform_state, &create_transition),
            StateTransitionExecutionResult::SuccessfulExecution { .. },
            "the message is created at protocol version 9"
        );

        let query = DriveDocumentQuery::from_sql_expr(
            "select * from message",
            &contract,
            Some(&platform.config.drive),
            platform_version_9,
        )
        .expect("expected a document query");
        let stored = platform
            .drive
            .query_documents(query, None, false, None, None)
            .expect("expected a query result")
            .documents()
            .to_vec();
        assert_eq!(stored.len(), 1);
        assert_eq!(
            stored[0].creator_id(),
            None,
            "protocol version 9 records no creator id, even on a transferable type"
        );

        // The chain moves to the latest protocol version
        let mut upgraded_state = platform.state.load().as_ref().clone();
        upgraded_state.set_current_protocol_version_in_consensus(platform_version.protocol_version);
        upgraded_state.set_next_epoch_protocol_version(platform_version.protocol_version);
        platform.state.store(Arc::new(upgraded_state));
        let platform_state = platform.state.load();

        let mut updated_contract = json_document_to_contract(
            REFERENCE_VALIDATION_CREATOR_KEY_BEFORE_CREATOR_IDS_UPDATE_PATH,
            true,
            platform_version,
        )
        .expect("expected to parse the updated contract");
        updated_contract.set_owner_id(identity.id());
        updated_contract.set_config(contract.config().clone());

        let update_transition = DataContractUpdateTransition::new_from_data_contract(
            updated_contract.clone(),
            &identity.clone().into_partial_identity_info(),
            key.id(),
            2,
            0,
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("expected an update transition");
        assert_matches!(
            process_and_commit_one(&platform, &platform_state, &update_transition),
            StateTransitionExecutionResult::SuccessfulExecution { .. },
            "the update adding a $creatorId key reference to a type that records creator ids \
             is accepted, documents written before it did notwithstanding"
        );

        let updated_message = updated_contract
            .document_type_for_name("message")
            .expect("expected the updated message document type");
        let mut replacement = document;
        replacement.set_revision(Some(2));
        replace_mutator(&mut replacement);

        let replace_transition =
            BatchTransition::new_document_replacement_transition_from_document(
                replacement,
                updated_message,
                &key,
                3,
                0,
                None,
                &signer,
                platform_version,
                None,
            )
            .await
            .expect("expected a replace transition");

        process_and_commit_one(&platform, &platform_state, &replace_transition)
    }

    /// The old message records no creator, so a `$creatorId` key id set on it
    /// names no identity's key: the replace is refused, paid, with the error a
    /// key id set while its identity property is not gets. Key 0 is the
    /// writer's enabled master key, so the missing creator is the only fault.
    #[tokio::test]
    async fn should_document_replace_fail_when_creator_key_id_is_set_on_a_document_that_records_no_creator(
    ) {
        let result = run_replace_of_a_message_written_before_creator_ids(|document| {
            document.set("senderKeyId", 0i64.into());
        })
        .await;

        assert_matches!(
            result,
            PaidConsensusError {
                error: ConsensusError::StateError(
                    StateError::ReferencedKeyIdPropertyInvalidError(e)
                ),
                ..
            } if e.key_id_property() == "senderKeyId"
                && e.path() == "senderKeyId"
                && e.message().contains("records no $creatorId")
        );
    }

    /// The creator is only read when the key id changes, so the old message
    /// stays replaceable while its key id stays unset.
    #[tokio::test]
    async fn should_document_replace_succeed_on_a_document_that_records_no_creator_while_the_creator_key_id_stays_unset(
    ) {
        let result = run_replace_of_a_message_written_before_creator_ids(|document| {
            document.set("note", "replaced at the latest protocol version".into());
        })
        .await;

        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
    }

    /// A property path binds both sides of the pair: changing only the
    /// identity property re-validates the key id against the new identity.
    #[tokio::test]
    async fn should_document_replace_fail_when_only_the_identity_property_changed_to_an_identity_without_the_key(
    ) {
        let result = run_key_id_reference_create_then_replace(
            REFERENCE_VALIDATION_IDENTITY_PROPERTY_KEY_CONTRACT_PATH,
            |document, targets| {
                document.set("toUserId", targets.other_id.into());
                document.set(
                    "recipientKeyId",
                    (targets.other_enabled_key_id as i64).into(),
                );
            },
            DisableMasterKeyBetween::Nobody,
            |document, _| {
                document.set("toUserId", Identifier::random().into());
            },
        )
        .await;

        assert_matches!(
            result,
            PaidConsensusError {
                error: ConsensusError::StateError(StateError::ReferencedIdentityKeyNotFoundError(
                    _
                )),
                ..
            }
        );
    }

    /// An untouched pair is not refetched: the named identity's key is
    /// disabled between the create and a replace of another property, and
    /// the replace still passes.
    #[tokio::test]
    async fn should_document_replace_succeed_without_refetching_an_untouched_identity_property_pair(
    ) {
        let result = run_key_id_reference_create_then_replace(
            REFERENCE_VALIDATION_IDENTITY_PROPERTY_KEY_CONTRACT_PATH,
            |document, targets| {
                document.set("toUserId", targets.other_id.into());
                document.set("recipientKeyId", (targets.master_key_id as i64).into());
            },
            DisableMasterKeyBetween::Other,
            |document, _| {
                document.set("note", "changed".into());
            },
        )
        .await;

        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
    }
}
