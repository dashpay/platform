//! `deleteConstraints` through the full ABCI pipeline (protocol version 14): the
//! rules the stored document must meet for its owner to delete it. The fixture's
//! `poll` may be deleted only while no `vote` points at it (`noVotes`, a `countOf`
//! matched by the poll's own `$id`) and while it is not locked (`notLocked`, a
//! stored property), and may be edited only before its first vote
//! (`editableBeforeTheFirstVote`, the same total in `propertyConstraints`). An
//! `address` keeps at least one per owner (`keepsOne`, a `countOf` of its own
//! type, which the deleted document no longer counts toward).
//!
//! A delete that breaks a rule is refused with `DocumentDeleteConstraintViolatedError`
//! (40147), a paid state error, and the document stays.

use super::*;

mod delete_constraints_tests {
    use super::super::reference_test_setup::{assert_successful, ReferenceTestSetup as Setup};
    use super::*;
    use crate::platform_types::state_transitions_processing_result::StateTransitionsProcessingResult;
    use dpp::consensus::basic::document::PropertyConstraintViolation;
    use dpp::consensus::basic::BasicError;
    use dpp::consensus::codes::ErrorWithCode;
    use dpp::document::Document;
    use dpp::identifier::Identifier;

    const CONTRACT_PATH: &str =
        "tests/supporting_files/contract/delete-constraints/delete-constraints-contract.json";

    fn identifier_value(id: Identifier) -> Value {
        Value::Identifier(id.to_buffer())
    }

    /// The delete was refused, paid, by the delete rule `rule` of `document_type`
    /// for the document `document_id`.
    fn assert_delete_refused_by(
        result: &StateTransitionsProcessingResult,
        document_id: Identifier,
        document_type: &str,
        rule: &str,
        because: &str,
    ) {
        match result.execution_results().as_slice() {
            [StateTransitionExecutionResult::PaidConsensusError {
                error:
                    ConsensusError::StateError(StateError::DocumentDeleteConstraintViolatedError(error)),
                ..
            }] => {
                assert_eq!(error.document_id(), &document_id, "{because}");
                assert_eq!(error.document_type_name(), document_type, "{because}");
                assert_eq!(error.constraint(), rule, "{because}");
                assert_eq!(
                    error.violation(),
                    PropertyConstraintViolation::NotMet,
                    "{because}"
                );
                assert_eq!(
                    ConsensusError::StateError(StateError::DocumentDeleteConstraintViolatedError(
                        error.clone()
                    ))
                    .code(),
                    40147
                );
            }
            other => panic!("{because}: expected a paid 40147 refusal, got {other:?}"),
        }
    }

    impl Setup {
        async fn poll(&mut self, properties: &[(&str, Value)]) -> Document {
            let mut all = vec![("question", Value::from("which way?"))];
            all.extend(properties.iter().cloned());
            let (poll, result) = self.create("poll", &all).await;
            assert_successful(&result, "the poll is created");
            poll
        }

        async fn vote(&mut self, poll: &Document, choice: u64) -> Document {
            let (vote, result) = self
                .create(
                    "vote",
                    &[
                        ("pollId", identifier_value(poll.id())),
                        ("choice", Value::U64(choice)),
                    ],
                )
                .await;
            assert_successful(&result, "the vote is created");
            vote
        }
    }

    /// The poll example: deletable before its first vote, refused once a vote points at
    /// it, and deletable again once no vote does. Another poll's votes do not count.
    #[tokio::test]
    async fn should_let_a_poll_be_deleted_only_while_no_vote_points_at_it() {
        let mut setup = Setup::new(CONTRACT_PATH, 8401);

        let unvoted = setup.poll(&[]).await;
        let result = setup.delete("poll", &unvoted).await;
        assert_successful(&result, "a poll no vote points at is deleted");

        let poll = setup.poll(&[]).await;
        let other_poll = setup.poll(&[]).await;
        let vote = setup.vote(&poll, 1).await;
        setup.vote(&other_poll, 0).await;

        let result = setup.delete("poll", &poll).await;
        assert_delete_refused_by(
            &result,
            poll.id(),
            "poll",
            "noVotes",
            "a vote points at the poll",
        );

        // The refused delete left the poll in place: a vote can still point at it
        setup.vote(&poll, 2).await;

        let (votes, _) = setup
            .platform
            .drive
            .query_documents(
                drive::query::DriveDocumentQuery::all_items_query(
                    &setup.contract,
                    setup.contract.document_type_for_name("vote").expect("vote"),
                    None,
                ),
                None,
                false,
                None,
                None,
            )
            .map(|outcome| (outcome.documents_owned(), ()))
            .expect("expected to query the votes");
        let votes_of_poll = votes
            .into_iter()
            .filter(|stored| stored.get("pollId") == Some(&identifier_value(poll.id())))
            .collect::<Vec<_>>();
        assert_eq!(votes_of_poll.len(), 2);

        // Once no vote points at it any more, the poll may go
        for stored in &votes_of_poll {
            let result = setup.delete("vote", stored).await;
            assert_successful(&result, "a voter deletes its vote");
        }
        assert!(votes_of_poll.iter().any(|stored| stored.id() == vote.id()));
        let result = setup.delete("poll", &poll).await;
        assert_successful(
            &result,
            "with its votes gone the poll is deleted, the other poll's vote not counting",
        );
        let result = setup.delete("poll", &other_poll).await;
        assert_delete_refused_by(
            &result,
            other_poll.id(),
            "poll",
            "noVotes",
            "the other poll keeps its vote",
        );
    }

    /// A rule reads the stored document: a locked poll is not deleted, however few
    /// votes it has, and is once unlocked.
    #[tokio::test]
    async fn should_judge_a_delete_rule_on_the_stored_document() {
        let mut setup = Setup::new(CONTRACT_PATH, 8402);
        let mut poll = setup.poll(&[("status", "locked".into())]).await;

        let result = setup.delete("poll", &poll).await;
        assert_delete_refused_by(
            &result,
            poll.id(),
            "poll",
            "notLocked",
            "the stored poll is locked",
        );

        poll.set("status", "open".into());
        let result = setup.replace("poll", &mut poll).await;
        assert_successful(&result, "the unvoted poll is unlocked");
        let result = setup.delete("poll", &poll).await;
        assert_successful(&result, "the unlocked poll is deleted");
    }

    /// `$id` matches a `propertyConstraints` total too: the poll is edited freely before
    /// its first vote, and an edit after it is refused (10422).
    #[tokio::test]
    async fn should_match_a_property_constraints_total_by_the_document_id() {
        let mut setup = Setup::new(CONTRACT_PATH, 8403);
        let mut poll = setup.poll(&[]).await;

        poll.set("question", "which way now?".into());
        let result = setup.replace("poll", &mut poll).await;
        assert_successful(&result, "an unvoted poll is edited");

        setup.vote(&poll, 0).await;
        poll.set("question", "which way after all?".into());
        let result = setup.replace("poll", &mut poll).await;
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::BasicError(
                    BasicError::DocumentPropertyConstraintViolatedError(_)
                ),
                ..
            }],
            "a voted poll is no longer edited"
        );
    }

    /// A total of the type's own documents is read as it will be once the deleted
    /// document is gone: an owner with two addresses deletes one, and then not the last.
    #[tokio::test]
    async fn should_count_own_type_totals_without_the_deleted_document() {
        let mut setup = Setup::new(CONTRACT_PATH, 8404);
        let (home, result) = setup.create("address", &[("label", "home".into())]).await;
        assert_successful(&result, "the first address is created");

        let result = setup.delete("address", &home).await;
        assert_delete_refused_by(
            &result,
            home.id(),
            "address",
            "keepsOne",
            "the only address is not deleted",
        );

        let (work, result) = setup.create("address", &[("label", "work".into())]).await;
        assert_successful(&result, "the second address is created");
        let result = setup.delete("address", &home).await;
        assert_successful(&result, "one of two addresses is deleted");
        let result = setup.delete("address", &work).await;
        assert_delete_refused_by(
            &result,
            work.id(),
            "address",
            "keepsOne",
            "the last address is not deleted",
        );
    }
}
