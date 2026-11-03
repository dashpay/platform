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
    use super::super::reference_test_setup::{
        assert_successful, create_document, ReferenceTestSetup as Setup,
    };
    use super::*;
    use crate::platform_types::state_transitions_processing_result::StateTransitionsProcessingResult;
    use dpp::document::Document;
    use dpp::identifier::Identifier;

    /// Shared with the contract-create registration tests, which pin that
    /// the declarations themselves are accepted.
    const CONTRACT_PATH: &str = "tests/supporting_files/contract/reference-validation/reference-validation-contract-deletable-doc.json";

    /// The same contract with `comment.noteId` referring to `note`, whose
    /// type forbids deletion. Registration refuses it; applied directly, it
    /// exercises the write-time half of the same rule.
    const NOT_DELETABLE_CONTRACT_PATH: &str = "tests/supporting_files/contract/reference-validation/reference-validation-contract-deletable-doc-registration-not-deletable.json";

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

    fn identifier_value(id: Identifier) -> Value {
        Value::Identifier(id.to_buffer())
    }

    impl Setup {
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
