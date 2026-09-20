//! `refersTo: deletableDocument` through the full ABCI pipeline. The
//! fixture's `comment.draftId` refers to a deletable, mutable `draft` and
//! binds the comment's `topic` to the draft's; `gatedComment.draftId` binds
//! the WRITER to the draft's owner; `pinnedComment.draftId` is an
//! `immutable` reference.
//!
//! What sets the target apart from `permanentDocument` is what happens
//! after the referenced document is deleted. Nothing stops the deletion and
//! the referring document stays, but EVERY replace re-validates the
//! reference: a dead one has to be repointed at a document that exists or
//! cleared, and an immutable one can only be cleared.

use super::*;

mod deletable_document_reference_tests {
    use super::*;
    use crate::platform_types::platform_state::PlatformState;
    use crate::platform_types::state_transitions_processing_result::StateTransitionsProcessingResult;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TempPlatform;
    use dpp::data_contract::accessors::v0::DataContractV0Setters;
    use dpp::document::Document;
    use dpp::identifier::Identifier;
    use dpp::identity::signer::Signer;
    use dpp::identity::IdentityPublicKey;
    use dpp::prelude::DataContract;
    use dpp::state_transition::StateTransition;
    use simple_signer::signer::SimpleSigner;

    /// Shared with the contract-create registration tests, which pin that
    /// the declarations themselves are accepted.
    const CONTRACT_PATH: &str = "tests/supporting_files/contract/reference-validation/reference-validation-contract-deletable-doc.json";

    /// The same contract with `comment.noteId` referring to `note`, whose
    /// type forbids deletion. Registration refuses it; applied directly, it
    /// exercises the write-time half of the same rule.
    const NOT_DELETABLE_CONTRACT_PATH: &str = "tests/supporting_files/contract/reference-validation/reference-validation-contract-deletable-doc-registration-not-deletable.json";

    fn register_contract(
        platform: &TempPlatform<MockCoreRPCLike>,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> DataContract {
        register_contract_at(platform, CONTRACT_PATH, owner_id, platform_version)
    }

    fn register_contract_at(
        platform: &TempPlatform<MockCoreRPCLike>,
        path: &str,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> DataContract {
        let mut contract = json_document_to_contract(path, true, platform_version)
            .expect("expected to parse the deletable document contract");
        contract.set_owner_id(owner_id);
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
            .expect("expected to apply the deletable document contract");
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

    fn assert_successful(result: &StateTransitionsProcessingResult, because: &str) {
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }],
            "{because}"
        );
    }

    fn assert_referenced_entity_not_found(
        result: &StateTransitionsProcessingResult,
        because: &str,
    ) {
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(StateError::ReferencedEntityNotFoundError(_)),
                ..
            }],
            "{because}"
        );
    }

    /// Creates a document of `type_name` with exactly `properties` set and
    /// returns it with the processing result.
    #[allow(clippy::too_many_arguments)]
    async fn create_document<S: Signer<IdentityPublicKey>>(
        platform: &TempPlatform<MockCoreRPCLike>,
        platform_state: &PlatformState,
        contract: &DataContract,
        type_name: &str,
        properties: &[(&str, Value)],
        owner: Identifier,
        key: &IdentityPublicKey,
        nonce: u64,
        signer: &S,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> (Document, StateTransitionsProcessingResult) {
        let document_type = contract
            .document_type_for_name(type_name)
            .expect("doctype exists");
        let entropy = Bytes32::random_with_rng(rng);
        let mut document = document_type
            .random_document_with_identifier_and_entropy(
                rng,
                owner,
                entropy,
                DocumentFieldFillType::DoNotFillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");
        for (property, value) in properties {
            document.set(property, value.clone());
        }
        // The id commits to the create transition's nonce: give the local
        // copy the id the transition will carry, since the tests reference
        // and act on it afterwards.
        document
            .set_id_for_creation(document_type, &entropy.0, nonce, platform_version)
            .expect("expected the creation id");
        let create = BatchTransition::new_document_creation_transition_from_document(
            document.clone(),
            document_type,
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
        .expect("expected the create transition");
        let result = process_and_commit(platform, platform_state, &create, platform_version);
        (document, result)
    }

    #[allow(clippy::too_many_arguments)]
    async fn replace_document<S: Signer<IdentityPublicKey>>(
        platform: &TempPlatform<MockCoreRPCLike>,
        platform_state: &PlatformState,
        contract: &DataContract,
        type_name: &str,
        document: &Document,
        key: &IdentityPublicKey,
        nonce: u64,
        signer: &S,
        platform_version: &PlatformVersion,
    ) -> StateTransitionsProcessingResult {
        let document_type = contract
            .document_type_for_name(type_name)
            .expect("doctype exists");
        let replace = BatchTransition::new_document_replacement_transition_from_document(
            document.clone(),
            document_type,
            key,
            nonce,
            0,
            None,
            signer,
            platform_version,
            None,
        )
        .await
        .expect("expected the replace transition");
        process_and_commit(platform, platform_state, &replace, platform_version)
    }

    #[allow(clippy::too_many_arguments)]
    async fn delete_document<S: Signer<IdentityPublicKey>>(
        platform: &TempPlatform<MockCoreRPCLike>,
        platform_state: &PlatformState,
        contract: &DataContract,
        type_name: &str,
        document: &Document,
        key: &IdentityPublicKey,
        nonce: u64,
        signer: &S,
        platform_version: &PlatformVersion,
    ) -> StateTransitionsProcessingResult {
        let document_type = contract
            .document_type_for_name(type_name)
            .expect("doctype exists");
        let delete = BatchTransition::new_document_deletion_transition_from_document(
            document.clone(),
            document_type,
            key,
            nonce,
            0,
            None,
            signer,
            platform_version,
            None,
        )
        .await
        .expect("expected the delete transition");
        process_and_commit(platform, platform_state, &delete, platform_version)
    }

    fn identifier_value(id: Identifier) -> Value {
        Value::Identifier(id.to_buffer())
    }

    struct Setup {
        platform: TempPlatform<MockCoreRPCLike>,
        platform_state: std::sync::Arc<PlatformState>,
        contract: DataContract,
        owner: Identifier,
        signer: SimpleSigner,
        key: IdentityPublicKey,
        rng: StdRng,
        nonce: u64,
    }

    impl Setup {
        fn new(contract_path: &str, seed: u64) -> Self {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();
            let platform_state = platform.state.load_full();
            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(1.0));
            let contract =
                register_contract_at(&platform, contract_path, identity.id(), platform_version);
            Self {
                platform,
                platform_state,
                contract,
                owner: identity.id(),
                signer,
                key,
                rng: StdRng::seed_from_u64(seed),
                nonce: 1,
            }
        }

        fn next_nonce(&mut self) -> u64 {
            self.nonce += 1;
            self.nonce
        }

        async fn create(
            &mut self,
            type_name: &str,
            properties: &[(&str, Value)],
        ) -> (Document, StateTransitionsProcessingResult) {
            let nonce = self.next_nonce();
            create_document(
                &self.platform,
                &self.platform_state,
                &self.contract,
                type_name,
                properties,
                self.owner,
                &self.key,
                nonce,
                &self.signer,
                &mut self.rng,
                PlatformVersion::latest(),
            )
            .await
        }

        /// Bumps the revision and replaces `document` as it now stands.
        async fn replace(
            &mut self,
            type_name: &str,
            document: &mut Document,
        ) -> StateTransitionsProcessingResult {
            document.increment_revision().expect("revision increments");
            let nonce = self.next_nonce();
            let result = replace_document(
                &self.platform,
                &self.platform_state,
                &self.contract,
                type_name,
                document,
                &self.key,
                nonce,
                &self.signer,
                PlatformVersion::latest(),
            )
            .await;
            // A refused replace leaves the stored revision where it was.
            if result.valid_count() == 0 {
                let revision = document.revision().expect("revision set");
                document.set_revision(Some(revision - 1));
            }
            result
        }

        async fn delete(
            &mut self,
            type_name: &str,
            document: &Document,
        ) -> StateTransitionsProcessingResult {
            let nonce = self.next_nonce();
            delete_document(
                &self.platform,
                &self.platform_state,
                &self.contract,
                type_name,
                document,
                &self.key,
                nonce,
                &self.signer,
                PlatformVersion::latest(),
            )
            .await
        }

        async fn draft(&mut self, topic: &str) -> Document {
            let (draft, result) = self.create("draft", &[("topic", topic.into())]).await;
            assert_successful(&result, "the draft is created");
            draft
        }
    }

    /// The write-time rules are `permanentDocument`'s with the permanence
    /// requirement turned around: the target must exist, the agreement
    /// must hold, and the referenced type must ALLOW deletion.
    #[tokio::test]
    async fn should_validate_a_deletable_document_reference_when_it_is_written() {
        let mut setup = Setup::new(CONTRACT_PATH, 7301);
        let draft = setup.draft("dash").await;

        let (_, result) = setup
            .create(
                "comment",
                &[
                    ("draftId", identifier_value(draft.id())),
                    ("topic", "dash".into()),
                ],
            )
            .await;
        assert_successful(&result, "an existing deletable draft may be referenced");

        let (_, result) = setup
            .create(
                "comment",
                &[
                    ("draftId", identifier_value(Identifier::random())),
                    ("topic", "dash".into()),
                ],
            )
            .await;
        assert_referenced_entity_not_found(&result, "the referenced draft must exist");

        let (_, result) = setup
            .create(
                "comment",
                &[
                    ("draftId", identifier_value(draft.id())),
                    ("topic", "btc".into()),
                ],
            )
            .await;
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(
                    StateError::ReferencedDocumentPropertyMismatchError(_)
                ),
                ..
            }],
            "the propertyAgreement is enforced on a deletable reference"
        );
    }

    /// The two document references are disjoint. Registration refuses a
    /// deletableDocument reference to a type that forbids deletion; a
    /// contract applied around registration meets the same rule at write
    /// time.
    #[tokio::test]
    async fn should_refuse_a_deletable_document_reference_to_a_permanent_type() {
        let mut setup = Setup::new(NOT_DELETABLE_CONTRACT_PATH, 7304);
        let (note, result) = setup.create("note", &[("content", "a note".into())]).await;
        assert_successful(&result, "the note is created");

        let (_, result) = setup
            .create("comment", &[("noteId", identifier_value(note.id()))])
            .await;
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(
                    StateError::ReferencedDocumentTypeNotDeletableError(_)
                ),
                ..
            }],
            "a note forbids deletion, so it is a permanentDocument target"
        );
    }

    /// Referring documents do not pin their target: the draft's owner
    /// deletes it and the comment stays. From then on every replace of the
    /// comment meets the dead reference, touched or not, until the replace
    /// repoints it at a draft that exists or clears it.
    #[tokio::test]
    async fn should_require_a_dead_reference_to_be_repointed_or_cleared_on_replace() {
        let mut setup = Setup::new(CONTRACT_PATH, 7302);
        let draft = setup.draft("dash").await;
        let other_draft = setup.draft("dash").await;
        let (mut comment, result) = setup
            .create(
                "comment",
                &[
                    ("draftId", identifier_value(draft.id())),
                    ("topic", "dash".into()),
                ],
            )
            .await;
        assert_successful(&result, "the comment is created");

        comment.set("body", "first edit".into());
        let result = setup.replace("comment", &mut comment).await;
        assert_successful(&result, "a replace passes while the draft exists");

        let result = setup.delete("draft", &draft).await;
        assert_successful(&result, "a referenced draft can still be deleted");

        // Only `body` changes, but the reference is re-validated anyway.
        comment.set("body", "second edit".into());
        let result = setup.replace("comment", &mut comment).await;
        assert_referenced_entity_not_found(
            &result,
            "a replace may not leave the reference on the deleted draft",
        );

        comment.set("draftId", identifier_value(Identifier::random()));
        let result = setup.replace("comment", &mut comment).await;
        assert_referenced_entity_not_found(
            &result,
            "repointing at another missing draft is no better",
        );

        comment.set("draftId", identifier_value(other_draft.id()));
        let result = setup.replace("comment", &mut comment).await;
        assert_successful(&result, "repointing at a draft that exists repairs it");

        // Clearing is the other repair.
        let result = setup.delete("draft", &other_draft).await;
        assert_successful(&result, "the second draft is deleted too");
        comment.remove("draftId");
        comment.remove("topic");
        let result = setup.replace("comment", &mut comment).await;
        assert_successful(&result, "clearing the dead reference repairs it");

        let result = setup.delete("comment", &comment).await;
        assert_successful(&result, "the comment can be deleted");
    }

    /// A referring document with a dead reference can always be deleted,
    /// repaired or not.
    #[tokio::test]
    async fn should_let_a_document_with_a_dead_reference_be_deleted() {
        let mut setup = Setup::new(CONTRACT_PATH, 7305);
        let draft = setup.draft("dash").await;
        let (comment, result) = setup
            .create(
                "comment",
                &[
                    ("draftId", identifier_value(draft.id())),
                    ("topic", "dash".into()),
                ],
            )
            .await;
        assert_successful(&result, "the comment is created");
        let result = setup.delete("draft", &draft).await;
        assert_successful(&result, "the draft is deleted");
        let result = setup.delete("comment", &comment).await;
        assert_successful(&result, "the comment outlives its draft and can be deleted");
    }

    /// A writer gate is re-checked on every replace, but never against a
    /// missing document: once the draft is deleted the dead reference is
    /// what fails, and repointing it puts the gate on the NEW draft's
    /// owner.
    #[tokio::test]
    async fn should_check_a_writer_gate_against_the_repointed_document() {
        let platform_version = PlatformVersion::latest();
        let mut setup = Setup::new(CONTRACT_PATH, 7303);
        let (bob, bob_signer, bob_key) =
            setup_identity(&mut setup.platform, 450, dash_to_credits!(1.0));
        let draft = setup.draft("dash").await;
        let own_other_draft = setup.draft("dash").await;
        // A draft owned by Bob, which Alice does not pass the gate on.
        let (bobs_draft, result) = create_document(
            &setup.platform,
            &setup.platform_state,
            &setup.contract,
            "draft",
            &[("topic", "dash".into())],
            bob.id(),
            &bob_key,
            2,
            &bob_signer,
            &mut setup.rng,
            platform_version,
        )
        .await;
        assert_successful(&result, "bob's draft is created");

        let (mut gated, result) = setup
            .create("gatedComment", &[("draftId", identifier_value(draft.id()))])
            .await;
        assert_successful(&result, "the draft's owner passes the gate");

        let result = setup.delete("draft", &draft).await;
        assert_successful(&result, "the draft is deleted");

        gated.set("body", "an edit".into());
        let result = setup.replace("gatedComment", &mut gated).await;
        assert_referenced_entity_not_found(&result, "the dead reference has to be repaired");

        gated.set("draftId", identifier_value(bobs_draft.id()));
        let result = setup.replace("gatedComment", &mut gated).await;
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(
                    StateError::ReferencedDocumentPropertyMismatchError(_)
                ),
                ..
            }],
            "the gate is checked against the repointed draft's owner"
        );

        gated.set("draftId", identifier_value(own_other_draft.id()));
        let result = setup.replace("gatedComment", &mut gated).await;
        assert_successful(
            &result,
            "repointing at a draft the writer owns passes the gate",
        );
    }

    /// An `immutable` reference cannot be repointed. While its draft exists
    /// it cannot be cleared either; once the draft is deleted, clearing it
    /// is the one change the immutable check lets through, and the only way
    /// left to replace the document.
    #[tokio::test]
    async fn should_let_an_immutable_reference_be_cleared_only_once_its_target_is_deleted() {
        let mut setup = Setup::new(CONTRACT_PATH, 7306);
        let draft = setup.draft("dash").await;
        let other_draft = setup.draft("dash").await;
        let (mut pinned, result) = setup
            .create(
                "pinnedComment",
                &[("draftId", identifier_value(draft.id()))],
            )
            .await;
        assert_successful(&result, "the pinned comment is created");
        let assert_immutable = |result: &StateTransitionsProcessingResult, because: &str| {
            assert_matches!(
                result.execution_results().as_slice(),
                [StateTransitionExecutionResult::PaidConsensusError {
                    error: ConsensusError::StateError(
                        StateError::DocumentImmutablePropertyChangedError(_)
                    ),
                    ..
                }],
                "{because}"
            );
        };

        pinned.remove("draftId");
        let result = setup.replace("pinnedComment", &mut pinned).await;
        assert_immutable(&result, "a live immutable reference cannot be cleared");
        pinned.set("draftId", identifier_value(draft.id()));

        let result = setup.delete("draft", &draft).await;
        assert_successful(&result, "the draft is deleted");

        pinned.set("body", "an edit".into());
        let result = setup.replace("pinnedComment", &mut pinned).await;
        assert_referenced_entity_not_found(&result, "the dead reference has to be repaired");

        pinned.set("draftId", identifier_value(other_draft.id()));
        let result = setup.replace("pinnedComment", &mut pinned).await;
        assert_immutable(
            &result,
            "an immutable reference cannot be repointed, dead or not",
        );

        pinned.remove("draftId");
        let result = setup.replace("pinnedComment", &mut pinned).await;
        assert_successful(&result, "clearing a dead immutable reference is allowed");

        // Once cleared it is an ordinary immutable property again: absent,
        // and not settable (it is not listed under immutableAllowSetting).
        pinned.set("draftId", identifier_value(other_draft.id()));
        let result = setup.replace("pinnedComment", &mut pinned).await;
        assert_immutable(&result, "a cleared immutable reference stays cleared");
    }
}
