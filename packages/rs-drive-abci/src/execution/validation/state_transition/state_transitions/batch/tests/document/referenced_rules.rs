//! Rules that read the document a reference points at, and `where` entries
//! that are not an equality, through the full ABCI pipeline (protocol version
//! 14). A `propertyConstraints` rule reading `{ "referenced": [reference,
//! field] }` is judged once the references hold, on the document the
//! reference's check read, and a reference a rule reads through is checked
//! again on every replace, so the rule sees the document as it is then. A
//! `where` entry with a literal, `notEqual` or an ordering is checked against
//! the document found as an equality is. Every transition here is processed in
//! a block whose time and heights are 0, unless a test names a later time.

use super::*;

mod referenced_rules_tests {
    use super::super::reference_test_setup::{assert_successful, ReferenceTestSetup};
    use super::*;
    use crate::platform_types::state_transitions_processing_result::StateTransitionsProcessingResult;
    use dpp::consensus::basic::BasicError;
    use dpp::document::{Document, DocumentV0Setters};

    const CONTRACT_PATH: &str = "tests/supporting_files/contract/reference-validation/reference-validation-contract-referenced-rules.json";

    fn assert_rule_broken(result: &StateTransitionsProcessingResult, rule: &str) {
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::BasicError(
                    BasicError::DocumentPropertyConstraintViolatedError(e)
                ),
                ..
            }] if e.constraint() == rule,
            "expected rule {rule} to refuse the write"
        );
    }

    fn assert_where_refused(result: &StateTransitionsProcessingResult, condition: &str) {
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(
                    StateError::ReferencedDocumentPropertyMismatchError(e)
                ),
                ..
            }] if e.referring_property() == condition,
            "expected the where entry {condition} to refuse the write"
        );
    }

    fn assert_not_found(result: &StateTransitionsProcessingResult) {
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(StateError::ReferencedEntityNotFoundError(_)),
                ..
            }]
        );
    }

    fn id_value(document: &Document) -> Value {
        Value::Identifier(document.id().to_buffer())
    }

    async fn poll(setup: &mut ReferenceTestSetup, extra: &[(&str, Value)]) -> Document {
        let mut properties = vec![
            ("option0", Value::Text("yes".to_string())),
            ("option1", Value::Text("no".to_string())),
        ];
        properties.extend(extra.iter().cloned());
        let (poll, result) = setup.create("poll", &properties).await;
        assert_successful(&result, "the poll must be created");
        poll
    }

    /// A vote is judged by the poll it names, read once for the reference and
    /// the rules alike: before it ends, on a single-choice poll, and for one of
    /// the options the poll holds.
    #[tokio::test]
    async fn should_judge_a_vote_by_the_poll_it_refers_to() {
        let mut setup = ReferenceTestSetup::new(CONTRACT_PATH, 6101);
        let two_options = poll(&mut setup, &[]).await;
        let (_, result) = setup
            .create(
                "vote",
                &[("pollId", id_value(&two_options)), ("choice", Value::U8(1))],
            )
            .await;
        assert_successful(&result, "a vote for the second of two options holds");
        let (_, result) = setup
            .create(
                "vote",
                &[("pollId", id_value(&two_options)), ("choice", Value::U8(2))],
            )
            .await;
        assert_rule_broken(&result, "choiceIsAnOption");

        let three_options =
            poll(&mut setup, &[("option2", Value::Text("maybe".to_string()))]).await;
        let (_, result) = setup
            .create(
                "vote",
                &[
                    ("pollId", id_value(&three_options)),
                    ("choice", Value::U8(2)),
                ],
            )
            .await;
        assert_successful(&result, "the third option is there");

        // The block's time is 0, so a poll ending at 0 has ended
        let ended = poll(&mut setup, &[("endsAt", Value::U64(0))]).await;
        let (_, result) = setup
            .create(
                "vote",
                &[("pollId", id_value(&ended)), ("choice", Value::U8(0))],
            )
            .await;
        assert_rule_broken(&result, "castBeforeTheEnd");

        let multiple = poll(&mut setup, &[("multiChoice", Value::Bool(true))]).await;
        let (_, result) = setup
            .create(
                "vote",
                &[("pollId", id_value(&multiple)), ("choice", Value::U8(0))],
            )
            .await;
        assert_rule_broken(&result, "singleChoicePoll");

        // A missing poll is the reference's refusal, before any rule reading it
        let (_, result) = setup
            .create(
                "vote",
                &[
                    ("pollId", Value::Identifier([9; 32])),
                    ("choice", Value::U8(0)),
                ],
            )
            .await;
        assert_not_found(&result);
    }

    /// A rule reads the referenced document on every replace, even one the
    /// reference itself would not be checked again for: the auction a bid
    /// names is permanent, yet its reserve price may have risen since, and an
    /// edit of the bid's note is judged against the auction as it is now. The
    /// `where` of the reference is checked again with it.
    #[tokio::test]
    async fn should_read_a_permanent_target_again_on_every_replace() {
        let mut setup = ReferenceTestSetup::new(CONTRACT_PATH, 6102);
        let (mut auction, result) = setup
            .create(
                "auction",
                &[
                    ("reservePrice", Value::U32(100)),
                    ("status", Value::Text("open".to_string())),
                ],
            )
            .await;
        assert_successful(&result, "the auction must be created");

        let (_, result) = setup
            .create(
                "bid",
                &[
                    ("auctionId", id_value(&auction)),
                    ("amount", Value::U32(50)),
                ],
            )
            .await;
        assert_rule_broken(&result, "atLeastReserve");
        let (mut bid, result) = setup
            .create(
                "bid",
                &[
                    ("auctionId", id_value(&auction)),
                    ("amount", Value::U32(150)),
                ],
            )
            .await;
        assert_successful(&result, "a bid over the reserve holds");

        auction.set("reservePrice", Value::U32(200));
        let result = setup.replace("auction", &mut auction).await;
        assert_successful(&result, "the auction's owner raises the reserve");

        bid.set("note", Value::Text("still keen".to_string()));
        let result = setup.replace("bid", &mut bid).await;
        assert_rule_broken(&result, "atLeastReserve");

        bid.set("amount", Value::U32(250));
        let result = setup.replace("bid", &mut bid).await;
        assert_successful(&result, "a bid raised over the new reserve holds");

        auction.set("status", Value::Text("closed".to_string()));
        let result = setup.replace("auction", &mut auction).await;
        assert_successful(&result, "the auction closes");
        bid.set("amount", Value::U32(300));
        let result = setup.replace("bid", &mut bid).await;
        assert_where_refused(&result, "{ \"in\": [\"open\", \"extended\"] }");
    }

    /// A rule reading both the referenced document and, through `$old.`, the
    /// document a replace replaces: each raise must top the stored amount by
    /// the auction's reserve, a create topping nothing, as its stored amount
    /// is absent. A replace is refused or accepted by both values together,
    /// the auction read as it is at each replace.
    #[tokio::test]
    async fn should_judge_a_rule_reading_the_referenced_and_the_stored_document() {
        let mut setup = ReferenceTestSetup::new(CONTRACT_PATH, 6110);
        let (mut auction, result) = setup
            .create(
                "auction",
                &[
                    ("reservePrice", Value::U32(100)),
                    ("status", Value::Text("open".to_string())),
                ],
            )
            .await;
        assert_successful(&result, "the auction must be created");

        // A create has no stored amount: the reserve alone
        let (_, result) = setup
            .create(
                "raise",
                &[
                    ("auctionId", id_value(&auction)),
                    ("amount", Value::U32(50)),
                ],
            )
            .await;
        assert_rule_broken(&result, "raisesByTheReserve");
        let (mut raise, result) = setup
            .create(
                "raise",
                &[
                    ("auctionId", id_value(&auction)),
                    ("amount", Value::U32(120)),
                ],
            )
            .await;
        assert_successful(&result, "a first raise over the reserve holds");

        // Over the reserve, yet not 100 over the stored 120
        raise.set("amount", Value::U32(200));
        let result = setup.replace("raise", &mut raise).await;
        assert_rule_broken(&result, "raisesByTheReserve");
        raise.set("amount", Value::U32(220));
        let result = setup.replace("raise", &mut raise).await;
        assert_successful(&result, "a raise 100 over the stored 120 holds");

        // The reserve as it is now, over the stored 220
        auction.set("reservePrice", Value::U32(10));
        let result = setup.replace("auction", &mut auction).await;
        assert_successful(&result, "the auction's owner lowers the reserve");
        raise.set("amount", Value::U32(229));
        let result = setup.replace("raise", &mut raise).await;
        assert_rule_broken(&result, "raisesByTheReserve");
        raise.set("amount", Value::U32(230));
        let result = setup.replace("raise", &mut raise).await;
        assert_successful(&result, "a raise 10 over the stored 220 holds");
    }

    /// The `where` entries that are not an equality: a literal the referenced
    /// side must equal, a value it must differ from, and an ordering bounded by
    /// the time of the write, which holds only when both sides have a value.
    #[tokio::test]
    async fn should_check_where_literals_inequalities_and_orderings() {
        let mut setup = ReferenceTestSetup::new(CONTRACT_PATH, 6103);
        let (pending, result) = setup
            .create("order", &[("status", Value::Text("pending".to_string()))])
            .await;
        assert_successful(&result, "the order must be created");
        let (_, result) = setup
            .create(
                "review",
                &[("orderId", id_value(&pending)), ("stars", Value::U8(5))],
            )
            .await;
        assert_where_refused(&result, "{ \"const\": \"delivered\" }");
        let (delivered, result) = setup
            .create("order", &[("status", Value::Text("delivered".to_string()))])
            .await;
        assert_successful(&result, "the order must be created");
        let (_, result) = setup
            .create(
                "review",
                &[("orderId", id_value(&delivered)), ("stars", Value::U8(5))],
            )
            .await;
        assert_successful(&result, "a delivered order is reviewed");

        // The writer owns the order, so a rebuttal of it differs from no one
        let (_, result) = setup
            .create("rebuttal", &[("orderId", id_value(&delivered))])
            .await;
        assert_where_refused(&result, "{ \"notEqual\": \"$ownerId\" }");

        let open = poll(&mut setup, &[("endsAt", Value::U64(5))]).await;
        let (mut vote, result) = setup
            .create("deadlineVote", &[("pollId", id_value(&open))])
            .await;
        assert_successful(&result, "a poll ending after the write takes a vote");
        // Checked again on every replace: the bound changes with each write
        vote.set("note", Value::Text("edited".to_string()));
        let result = setup.replace("deadlineVote", &mut vote).await;
        assert_successful(&result, "the poll is still open at the replace");

        let endless = poll(&mut setup, &[]).await;
        let (_, result) = setup
            .create("deadlineVote", &[("pollId", id_value(&endless))])
            .await;
        assert_where_refused(&result, "{ \"greaterThan\": \"$updatedAt\" }");

        let multiple = poll(
            &mut setup,
            &[
                ("endsAt", Value::U64(5)),
                ("multiChoice", Value::Bool(true)),
            ],
        )
        .await;
        let (_, result) = setup
            .create("deadlineVote", &[("pollId", id_value(&multiple))])
            .await;
        assert_where_refused(&result, "{ \"notEqual\": { \"const\": true } }");
    }

    fn block_at(time_ms: u64) -> BlockInfo {
        BlockInfo {
            time_ms,
            ..Default::default()
        }
    }

    /// An ordering bounded by `$updatedAt` is checked again on every replace,
    /// against the time of the replace's block, though the reference is a
    /// permanentDocument one no rule reads through and the replace leaves it as
    /// it was: an entry made before its ballot ends may be edited until then,
    /// and no longer once it has ended.
    #[tokio::test]
    async fn should_check_an_ordering_by_the_time_of_each_replace() {
        let mut setup = ReferenceTestSetup::new(CONTRACT_PATH, 6105);
        let (ballot, result) = setup
            .create("ballot", &[("endsAt", Value::U64(5_000))])
            .await;
        assert_successful(&result, "the ballot must be created");
        let (mut entry, result) = setup
            .create("ballotEntry", &[("ballotId", id_value(&ballot))])
            .await;
        assert_successful(&result, "an entry made before the ballot ends holds");

        entry.set("note", Value::Text("before the end".to_string()));
        let result = setup
            .replace_at("ballotEntry", &mut entry, &block_at(4_999))
            .await;
        assert_successful(&result, "the ballot ends after this replace");

        entry.set("note", Value::Text("at the end".to_string()));
        let result = setup
            .replace_at("ballotEntry", &mut entry, &block_at(5_000))
            .await;
        assert_where_refused(&result, "{ \"greaterThan\": \"$updatedAt\" }");
    }

    /// `where` conditions hold on every form of document reference: on each
    /// element of a typed array, on each leaf of an `anyOf` (which holds
    /// through either, refusing with its last leaf's refusal when none does),
    /// and on the document a `findBy` finds.
    #[tokio::test]
    async fn should_check_where_conditions_on_every_reference_form() {
        let mut setup = ReferenceTestSetup::new(CONTRACT_PATH, 6106);
        let (open_ballot, result) = setup
            .create("ballot", &[("endsAt", Value::U64(5_000))])
            .await;
        assert_successful(&result, "the ballot must be created");
        let (short_ballot, result) = setup.create("ballot", &[("endsAt", Value::U64(50))]).await;
        assert_successful(&result, "the ballot must be created");

        let (_, result) = setup
            .create(
                "ballotList",
                &[("ballotIds", Value::Array(vec![id_value(&open_ballot)]))],
            )
            .await;
        assert_successful(&result, "every element names a ballot ending after 100");
        let (_, result) = setup
            .create(
                "ballotList",
                &[(
                    "ballotIds",
                    Value::Array(vec![id_value(&open_ballot), id_value(&short_ballot)]),
                )],
            )
            .await;
        assert_where_refused(&result, "{ \"greaterThan\": { \"const\": 100 } }");

        let (open_auction, result) = setup
            .create(
                "auction",
                &[
                    ("reservePrice", Value::U32(1)),
                    ("status", Value::Text("open".to_string())),
                ],
            )
            .await;
        assert_successful(&result, "the auction must be created");
        let (closed_auction, result) = setup
            .create(
                "auction",
                &[
                    ("reservePrice", Value::U32(1)),
                    ("status", Value::Text("closed".to_string())),
                ],
            )
            .await;
        assert_successful(&result, "the auction must be created");
        for target in [&open_ballot, &open_auction] {
            let (_, result) = setup
                .create("ballotOrAuction", &[("targetId", id_value(target))])
                .await;
            assert_successful(&result, "one leaf holds");
        }
        // The ballot leaf's condition fails, and the auction leaf finds no
        // auction, the last refusal
        let (_, result) = setup
            .create("ballotOrAuction", &[("targetId", id_value(&short_ballot))])
            .await;
        assert_not_found(&result);
        let (_, result) = setup
            .create(
                "ballotOrAuction",
                &[("targetId", id_value(&closed_auction))],
            )
            .await;
        assert_where_refused(&result, "{ \"const\": \"open\" }");

        for (code, capacity) in [("small", 1u32), ("large", 5)] {
            let (_, result) = setup
                .create(
                    "event",
                    &[
                        ("code", Value::Text(code.to_string())),
                        ("capacity", Value::U32(capacity)),
                    ],
                )
                .await;
            assert_successful(&result, "the event must be created");
        }
        let (_, result) = setup
            .create("pass", &[("eventCode", Value::Text("large".to_string()))])
            .await;
        assert_successful(&result, "the event found holds more than one");
        let (_, result) = setup
            .create("pass", &[("eventCode", Value::Text("small".to_string()))])
            .await;
        assert_where_refused(&result, "{ \"greaterThan\": { \"const\": 1 } }");
    }

    /// A reference inside an object is read through by its dotted path.
    #[tokio::test]
    async fn should_read_through_a_reference_inside_an_object() {
        let mut setup = ReferenceTestSetup::new(CONTRACT_PATH, 6107);
        let single = poll(&mut setup, &[]).await;
        let multiple = poll(&mut setup, &[("multiChoice", Value::Bool(true))]).await;
        let meta =
            |poll: &Document| Value::Map(vec![(Value::Text("pollId".to_string()), id_value(poll))]);
        let (_, result) = setup.create("nestedVote", &[("meta", meta(&single))]).await;
        assert_successful(&result, "a single-choice poll takes the vote");
        let (_, result) = setup
            .create("nestedVote", &[("meta", meta(&multiple))])
            .await;
        assert_rule_broken(&result, "singleChoicePoll");
    }

    /// A reference found by `findBy` reads the document the key finds, for the
    /// rules too.
    #[tokio::test]
    async fn should_read_a_document_found_by_find_by() {
        let mut setup = ReferenceTestSetup::new(CONTRACT_PATH, 6104);
        let (_, result) = setup
            .create(
                "event",
                &[
                    ("code", Value::Text("concert".to_string())),
                    ("capacity", Value::U32(2)),
                ],
            )
            .await;
        assert_successful(&result, "the event must be created");
        let (_, result) = setup
            .create(
                "ticket",
                &[
                    ("eventCode", Value::Text("concert".to_string())),
                    ("seat", Value::U32(1)),
                ],
            )
            .await;
        assert_successful(&result, "the second seat is there");
        let (_, result) = setup
            .create(
                "ticket",
                &[
                    ("eventCode", Value::Text("concert".to_string())),
                    ("seat", Value::U32(2)),
                ],
            )
            .await;
        assert_rule_broken(&result, "seatExists");
        let (_, result) = setup
            .create(
                "ticket",
                &[
                    ("eventCode", Value::Text("opera".to_string())),
                    ("seat", Value::U32(0)),
                ],
            )
            .await;
        assert_not_found(&result);
    }
}
