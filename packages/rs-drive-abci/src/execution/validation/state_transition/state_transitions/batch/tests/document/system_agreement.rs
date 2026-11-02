//! `propertyAgreement` against the referenced document's system identifiers
//! through the full ABCI pipeline. The fixture's `message.noteId` refers to
//! a transferable `note` and binds `authorId` to the note's `$ownerId` and
//! `originalAuthorId` to its `$creatorId`. Transferring the note is what
//! tells the two bindings apart: the owner moves, the creator stays.

use super::*;

mod system_agreement_tests {
    use super::*;
    use crate::platform_types::platform_state::PlatformState;
    use crate::platform_types::state_transitions_processing_result::StateTransitionsProcessingResult;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TempPlatform;
    use dpp::data_contract::accessors::v0::DataContractV0Setters;
    use dpp::data_contract::config::DataContractConfig;
    use dpp::document::Document;
    use dpp::identifier::Identifier;
    use dpp::identity::signer::Signer;
    use dpp::identity::IdentityPublicKey;
    use dpp::prelude::DataContract;
    use dpp::state_transition::StateTransition;

    /// Shared with the contract-create registration tests, which pin that
    /// the declarations themselves are accepted. `message.noteId` binds
    /// `authorId` to the note's `$ownerId` and `originalAuthorId` to its
    /// `$creatorId`.
    const SYSTEM_CONTRACT_PATH: &str = "tests/supporting_files/contract/reference-validation/reference-validation-contract-agreement-system-valid.json";

    /// `message.noteId` binds the WRITER (`$ownerId`) to the note's
    /// `$ownerId`: only the note's current owner may write a message on it.
    const WRITER_CONTRACT_PATH: &str = "tests/supporting_files/contract/reference-validation/reference-validation-contract-agreement-writer-valid.json";

    fn register_contract(
        platform: &TempPlatform<MockCoreRPCLike>,
        path: &str,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> DataContract {
        let mut contract = json_document_to_contract(path, true, platform_version)
            .expect("expected to parse the system agreement contract");
        contract.set_owner_id(owner_id);
        // A note records its creator id only under a format-1 config, which
        // the JSON fixture does not spell out.
        contract.set_config(
            DataContractConfig::default_for_version(platform_version)
                .expect("expected the default contract config"),
        );
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
            .expect("expected to apply the system agreement contract");
        contract
    }

    fn process_and_commit(
        platform: &TempPlatform<MockCoreRPCLike>,
        platform_state: &PlatformState,
        transition: &StateTransition,
        platform_version: &PlatformVersion,
    ) -> StateTransitionsProcessingResult {
        let serialized = transition
            .serialize_to_bytes()
            .expect("expected the batch transition to serialize");
        let transaction = platform.drive.grove.start_transaction();
        let processing_result = platform
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
        processing_result
    }

    /// Creates a `note` owned by `owner` and returns it.
    #[allow(clippy::too_many_arguments)]
    async fn create_note<S: Signer<IdentityPublicKey>>(
        platform: &TempPlatform<MockCoreRPCLike>,
        platform_state: &PlatformState,
        contract: &DataContract,
        owner: Identifier,
        key: &IdentityPublicKey,
        nonce: u64,
        signer: &S,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Document {
        let note_type = contract
            .document_type_for_name("note")
            .expect("note doctype exists");
        let entropy = Bytes32::random_with_rng(rng);
        let mut note = note_type
            .random_document_with_identifier_and_entropy(
                rng,
                owner,
                entropy,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random note");
        note.set_id_for_creation(note_type, &entropy.0, nonce, platform_version)
            .expect("expected to set the document id");
        note.set("content", "a note".into());
        let create = BatchTransition::new_document_creation_transition_from_document(
            note.clone(),
            note_type,
            entropy.0,
            key,
            nonce,
            0,
            None,
            signer,
            platform_version,
            None,
        )
        .await
        .expect("expected the note create transition");
        let result = process_and_commit(platform, platform_state, &create, platform_version);
        assert_eq!(
            result.valid_count(),
            1,
            "the note must be created: {:?}",
            result.execution_results()
        );
        note
    }

    /// Transfers `note` from its current owner (who signs) to `recipient`.
    #[allow(clippy::too_many_arguments)]
    async fn transfer_note<S: Signer<IdentityPublicKey>>(
        platform: &TempPlatform<MockCoreRPCLike>,
        platform_state: &PlatformState,
        contract: &DataContract,
        note: &Document,
        recipient: Identifier,
        key: &IdentityPublicKey,
        nonce: u64,
        signer: &S,
        platform_version: &PlatformVersion,
    ) {
        let note_type = contract
            .document_type_for_name("note")
            .expect("note doctype exists");
        let mut note = note.clone();
        note.set_revision(Some(2));
        let transfer = BatchTransition::new_document_transfer_transition_from_document(
            note,
            note_type,
            recipient,
            key,
            nonce,
            0,
            None,
            signer,
            platform_version,
            None,
        )
        .await
        .expect("expected the note transfer transition");
        let result = process_and_commit(platform, platform_state, &transfer, platform_version);
        assert_eq!(
            result.valid_count(),
            1,
            "the note must be transferred: {:?}",
            result.execution_results()
        );
    }

    /// Submits a `message` on `note_id` owned by `owner`. `None` OMITS the
    /// property rather than leaving the random fill's value in place.
    #[allow(clippy::too_many_arguments)]
    async fn submit_message<S: Signer<IdentityPublicKey>>(
        platform: &TempPlatform<MockCoreRPCLike>,
        platform_state: &PlatformState,
        contract: &DataContract,
        owner: Identifier,
        note_id: Identifier,
        author_id: Option<Identifier>,
        original_author_id: Option<Identifier>,
        key: &IdentityPublicKey,
        nonce: u64,
        signer: &S,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> StateTransitionsProcessingResult {
        let message_type = contract
            .document_type_for_name("message")
            .expect("message doctype exists");
        let entropy = Bytes32::random_with_rng(rng);
        let mut message = message_type
            .random_document_with_identifier_and_entropy(
                rng,
                owner,
                entropy,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random message");
        message
            .set_id_for_creation(message_type, &entropy.0, nonce, platform_version)
            .expect("expected to set the document id");
        message.set("noteId", Value::Identifier(note_id.to_buffer()));
        for (property, value) in [
            ("authorId", author_id),
            ("originalAuthorId", original_author_id),
        ] {
            match value {
                Some(id) => message.set(property, Value::Identifier(id.to_buffer())),
                None => {
                    message.remove(property);
                }
            }
        }
        let create = BatchTransition::new_document_creation_transition_from_document(
            message,
            message_type,
            entropy.0,
            key,
            nonce,
            0,
            None,
            signer,
            platform_version,
            None,
        )
        .await
        .expect("expected the message create transition");
        process_and_commit(platform, platform_state, &create, platform_version)
    }

    fn assert_mismatch(result: &StateTransitionsProcessingResult, because: &str) {
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(
                    StateError::ReferencedDocumentPropertyMismatchError(_)
                ),
                ..
            }],
            "{because}"
        );
    }

    /// Before the transfer both identifiers are Alice; after it the owner
    /// is Bob while the creator is still Alice. Each binding is checked
    /// against the note as it stands when the message is written.
    #[tokio::test]
    async fn test_owner_and_creator_agreements_diverge_across_a_transfer() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();
        let platform_state = platform.state.load();
        let mut rng = StdRng::seed_from_u64(4246);

        let (alice, alice_signer, alice_key) =
            setup_identity(&mut platform, 958, dash_to_credits!(1.0));
        let (bob, bob_signer, bob_key) = setup_identity(&mut platform, 450, dash_to_credits!(1.0));
        let contract = register_contract(
            &platform,
            SYSTEM_CONTRACT_PATH,
            alice.id(),
            platform_version,
        );

        let note = create_note(
            &platform,
            &platform_state,
            &contract,
            alice.id(),
            &alice_key,
            2,
            &alice_signer,
            &mut rng,
            platform_version,
        )
        .await;

        // Owner and creator are both Alice: a message echoing both passes.
        let result = submit_message(
            &platform,
            &platform_state,
            &contract,
            bob.id(),
            note.id(),
            Some(alice.id()),
            Some(alice.id()),
            &bob_key,
            2,
            &bob_signer,
            &mut rng,
            platform_version,
        )
        .await;
        assert_eq!(
            result.valid_count(),
            1,
            "a message agreeing with the note's owner and creator must pass: {:?}",
            result.execution_results()
        );

        let result = submit_message(
            &platform,
            &platform_state,
            &contract,
            bob.id(),
            note.id(),
            Some(bob.id()),
            Some(alice.id()),
            &bob_key,
            3,
            &bob_signer,
            &mut rng,
            platform_version,
        )
        .await;
        assert_mismatch(
            &result,
            "an authorId that is not the note's current owner must be refused",
        );

        // `$ownerId` is always present on the referenced side, so omitting
        // the referring property is the one-side-absent mismatch.
        let result = submit_message(
            &platform,
            &platform_state,
            &contract,
            bob.id(),
            note.id(),
            None,
            Some(alice.id()),
            &bob_key,
            4,
            &bob_signer,
            &mut rng,
            platform_version,
        )
        .await;
        assert_mismatch(
            &result,
            "a message omitting authorId cannot agree with a note that has an owner",
        );

        transfer_note(
            &platform,
            &platform_state,
            &contract,
            &note,
            bob.id(),
            &alice_key,
            3,
            &alice_signer,
            platform_version,
        )
        .await;

        // The owner moved to Bob; the creator is still Alice.
        let result = submit_message(
            &platform,
            &platform_state,
            &contract,
            bob.id(),
            note.id(),
            Some(bob.id()),
            Some(alice.id()),
            &bob_key,
            5,
            &bob_signer,
            &mut rng,
            platform_version,
        )
        .await;
        assert_eq!(
            result.valid_count(),
            1,
            "after the transfer the owner is Bob and the creator Alice: {:?}",
            result.execution_results()
        );

        let result = submit_message(
            &platform,
            &platform_state,
            &contract,
            bob.id(),
            note.id(),
            Some(alice.id()),
            Some(alice.id()),
            &bob_key,
            6,
            &bob_signer,
            &mut rng,
            platform_version,
        )
        .await;
        assert_mismatch(&result, "$ownerId follows the note through the transfer");

        let result = submit_message(
            &platform,
            &platform_state,
            &contract,
            bob.id(),
            note.id(),
            Some(bob.id()),
            Some(bob.id()),
            &bob_key,
            7,
            &bob_signer,
            &mut rng,
            platform_version,
        )
        .await;
        assert_mismatch(
            &result,
            "$creatorId never changes, whoever owns the note now",
        );
    }

    /// Submits a `message` on `note_id` owned (and signed) by `owner` under
    /// the writer-gated contract, whose message carries no author fields,
    /// and returns the document as submitted for a later replace.
    #[allow(clippy::too_many_arguments)]
    async fn submit_gated_message<S: Signer<IdentityPublicKey>>(
        platform: &TempPlatform<MockCoreRPCLike>,
        platform_state: &PlatformState,
        contract: &DataContract,
        owner: Identifier,
        note_id: Identifier,
        key: &IdentityPublicKey,
        nonce: u64,
        signer: &S,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> (StateTransitionsProcessingResult, Document) {
        let message_type = contract
            .document_type_for_name("message")
            .expect("message doctype exists");
        let entropy = Bytes32::random_with_rng(rng);
        let mut message = message_type
            .random_document_with_identifier_and_entropy(
                rng,
                owner,
                entropy,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random message");
        message
            .set_id_for_creation(message_type, &entropy.0, nonce, platform_version)
            .expect("expected to set the document id");
        message.set("noteId", Value::Identifier(note_id.to_buffer()));
        let create = BatchTransition::new_document_creation_transition_from_document(
            message.clone(),
            message_type,
            entropy.0,
            key,
            nonce,
            0,
            None,
            signer,
            platform_version,
            None,
        )
        .await
        .expect("expected the message create transition");
        (
            process_and_commit(platform, platform_state, &create, platform_version),
            message,
        )
    }

    /// Replaces ONLY the `content` of a gated message, leaving its `noteId`
    /// untouched, signed by `key`'s identity.
    #[allow(clippy::too_many_arguments)]
    async fn replace_gated_message_content<S: Signer<IdentityPublicKey>>(
        platform: &TempPlatform<MockCoreRPCLike>,
        platform_state: &PlatformState,
        contract: &DataContract,
        message: &mut Document,
        content: &str,
        key: &IdentityPublicKey,
        nonce: u64,
        signer: &S,
        platform_version: &PlatformVersion,
    ) -> StateTransitionsProcessingResult {
        let message_type = contract
            .document_type_for_name("message")
            .expect("message doctype exists");
        message.set("content", content.into());
        message
            .increment_revision()
            .expect("expected the revision to increment");
        let replace = BatchTransition::new_document_replacement_transition_from_document(
            message.clone(),
            message_type,
            key,
            nonce,
            0,
            None,
            signer,
            platform_version,
            None,
        )
        .await
        .expect("expected the message replace transition");
        process_and_commit(platform, platform_state, &replace, platform_version)
    }

    /// `{ "$ownerId": "$ownerId" }` is a write gate: only the note's current
    /// owner may create a message on it. The gate follows a transfer of the
    /// note, and it is checked at write time only, so a message written
    /// before the transfer is not disturbed by it.
    #[tokio::test]
    async fn test_writer_owner_agreement_gates_creation_to_the_referenced_owner() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();
        let platform_state = platform.state.load();
        let mut rng = StdRng::seed_from_u64(4247);

        let (alice, alice_signer, alice_key) =
            setup_identity(&mut platform, 958, dash_to_credits!(1.0));
        let (bob, bob_signer, bob_key) = setup_identity(&mut platform, 450, dash_to_credits!(1.0));
        let contract = register_contract(
            &platform,
            WRITER_CONTRACT_PATH,
            alice.id(),
            platform_version,
        );

        let note = create_note(
            &platform,
            &platform_state,
            &contract,
            alice.id(),
            &alice_key,
            2,
            &alice_signer,
            &mut rng,
            platform_version,
        )
        .await;

        // The note's owner may write; anyone else may not.
        let (result, _) = submit_gated_message(
            &platform,
            &platform_state,
            &contract,
            alice.id(),
            note.id(),
            &alice_key,
            3,
            &alice_signer,
            &mut rng,
            platform_version,
        )
        .await;
        assert_eq!(
            result.valid_count(),
            1,
            "the note's owner must pass the writer gate: {:?}",
            result.execution_results()
        );

        let (result, _) = submit_gated_message(
            &platform,
            &platform_state,
            &contract,
            bob.id(),
            note.id(),
            &bob_key,
            2,
            &bob_signer,
            &mut rng,
            platform_version,
        )
        .await;
        assert_mismatch(
            &result,
            "an identity that does not own the note must be refused",
        );

        transfer_note(
            &platform,
            &platform_state,
            &contract,
            &note,
            bob.id(),
            &alice_key,
            4,
            &alice_signer,
            platform_version,
        )
        .await;

        // The gate follows the note to its new owner.
        let (result, _) = submit_gated_message(
            &platform,
            &platform_state,
            &contract,
            bob.id(),
            note.id(),
            &bob_key,
            3,
            &bob_signer,
            &mut rng,
            platform_version,
        )
        .await;
        assert_eq!(
            result.valid_count(),
            1,
            "after the transfer the new owner must pass the writer gate: {:?}",
            result.execution_results()
        );

        let (result, _) = submit_gated_message(
            &platform,
            &platform_state,
            &contract,
            alice.id(),
            note.id(),
            &alice_key,
            5,
            &alice_signer,
            &mut rng,
            platform_version,
        )
        .await;
        assert_mismatch(
            &result,
            "the previous owner no longer passes the writer gate",
        );
    }

    /// The writer gate holds on EVERY replace, not only when the reference
    /// changes: `$ownerId` is transition metadata that never appears among
    /// the changed fields. Once the note has moved, its previous owner may
    /// no longer touch a message they wrote on it, even an unrelated field.
    #[tokio::test]
    async fn test_writer_owner_agreement_holds_on_every_replace() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();
        let platform_state = platform.state.load();
        let mut rng = StdRng::seed_from_u64(4248);

        let (alice, alice_signer, alice_key) =
            setup_identity(&mut platform, 958, dash_to_credits!(1.0));
        let (bob, _, _) = setup_identity(&mut platform, 450, dash_to_credits!(1.0));
        let contract = register_contract(
            &platform,
            WRITER_CONTRACT_PATH,
            alice.id(),
            platform_version,
        );

        let note = create_note(
            &platform,
            &platform_state,
            &contract,
            alice.id(),
            &alice_key,
            2,
            &alice_signer,
            &mut rng,
            platform_version,
        )
        .await;
        let (result, mut message) = submit_gated_message(
            &platform,
            &platform_state,
            &contract,
            alice.id(),
            note.id(),
            &alice_key,
            3,
            &alice_signer,
            &mut rng,
            platform_version,
        )
        .await;
        assert_eq!(
            result.valid_count(),
            1,
            "the note's owner must pass the writer gate: {:?}",
            result.execution_results()
        );

        // While Alice still owns the note she may edit an unrelated field.
        let result = replace_gated_message_content(
            &platform,
            &platform_state,
            &contract,
            &mut message,
            "first edit",
            &alice_key,
            4,
            &alice_signer,
            platform_version,
        )
        .await;
        assert_eq!(
            result.valid_count(),
            1,
            "the note's owner must pass the writer gate on replace: {:?}",
            result.execution_results()
        );

        transfer_note(
            &platform,
            &platform_state,
            &contract,
            &note,
            bob.id(),
            &alice_key,
            5,
            &alice_signer,
            platform_version,
        )
        .await;

        // The same edit, with the reference untouched, is now refused: the
        // gate is re-checked although no changed field is bound to it.
        let result = replace_gated_message_content(
            &platform,
            &platform_state,
            &contract,
            &mut message,
            "second edit",
            &alice_key,
            6,
            &alice_signer,
            platform_version,
        )
        .await;
        assert_mismatch(
            &result,
            "the previous owner must fail the writer gate on a replace of an unrelated field",
        );
    }
}
