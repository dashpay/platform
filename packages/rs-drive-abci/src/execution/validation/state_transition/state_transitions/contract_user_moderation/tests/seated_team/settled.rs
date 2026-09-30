//! The deletion of settled documents by a seated team's approvals (protocol version 14): past a
//! type's `deleteWithin` window no moderator deletes a document alone, and a type that says who
//! must approve (`moderatorAbilities.deleteSettled`) lets the members of the seated team delete
//! it together, each approval kept under the contract until the one that meets the rule.

use super::*;
use dpp::data_contract::config::moderation::ContractSettledDeletion;
use drive::drive::contract::moderation::types::{
    ContractSettledDeletionEntry, ContractSettledDeletionsQuery,
};

const DOCUMENT_TYPE_NOT_DELETABLE_ONCE_SETTLED: u32 = 41204;
const CONTRACT_MODERATION_TEAM_NOT_SEATED: u32 = 41205;
const DOCUMENT_NOT_SETTLED: u32 = 41206;
const SETTLED_DELETION_REASON_MISMATCH: u32 = 41207;
const SETTLED_DELETION_ALREADY_APPROVED: u32 = 41208;
const MODERATION_REASON_NOT_LISTED: u32 = 41203;
const SETTLED_DELETION_NOT_RESTORABLE: u32 = 41209;

/// A month after the documents of these tests were written, and after the seat was awarded:
/// every story and memo is settled.
const SETTLED_AT: TimestampMillis = BLOCK_TIME_MS + 30 * 24 * 3_600 * 1_000;

/// An approval of the deletion of settled `document_id` of `document_type_name`, for the listed
/// reason with `text`
fn approve(
    document_type_name: &str,
    document_id: Identifier,
    text: &str,
) -> ContractUserModerationAction {
    ContractUserModerationAction::DeleteSettledDocument {
        document_type_name: document_type_name.to_string(),
        document_id,
        reason: ContractModerationReason::from_text(text).with_reason_document(LISTED_REASON),
    }
}

impl Team {
    /// A document of `document_type_name` by `actor`, created and committed at the block time of
    /// the tests
    async fn written_by(&self, actor: &Actor, document_type_name: &str) -> Document {
        let (document, mut creates) = self
            .one_document_by(actor, document_type_name, &[None])
            .await;
        self.process_and_commit(&creates.pop().expect("expected the creation"));
        document
    }

    /// The approvals the contract keeps of the deletion of `document_id`, if any
    fn settled_deletion(
        &self,
        document_type_name: &str,
        document_id: Identifier,
        transaction: &Transaction,
    ) -> Option<ContractSettledDeletion> {
        self.setup
            .platform
            .drive
            .fetch_contract_settled_deletions(
                self.setup.contract.id(),
                &ContractSettledDeletionsQuery {
                    document_type_name: document_type_name.to_string(),
                    selection: ContractDocumentRemovalsSelection::DocumentIds(vec![document_id]),
                },
                Some(transaction),
                PlatformVersion::latest(),
            )
            .expect("expected to fetch the approvals")
            .pop()
            .map(
                |ContractSettledDeletionEntry {
                     settled_deletion, ..
                 }| settled_deletion,
            )
    }

    /// Whether the document of `document_type_name` at `document_id` is stored
    fn is_stored(
        &self,
        document_type_name: &str,
        document_id: Identifier,
        transaction: &Transaction,
    ) -> bool {
        self.setup
            .stored_document(document_type_name, document_id, Some(transaction))
            .is_some()
    }

    /// The moderation action counts of the contract's team
    fn action_counts(&self, transaction: &Transaction) -> BTreeMap<Identifier, u32> {
        self.setup
            .platform
            .drive
            .fetch_contract_moderation_action_counts(
                self.setup.contract.id(),
                31,
                Some(transaction),
                PlatformVersion::latest(),
            )
            .expect("expected to read the counts")
    }

    /// Proves the committed state for an approval and returns the approvals the proof shows
    fn proved_approvals(&self, transition: &StateTransition) -> ContractSettledDeletion {
        let platform_version = PlatformVersion::latest();
        let proof = self
            .setup
            .platform
            .drive
            .prove_state_transition(transition, None, platform_version)
            .expect("expected to prove the approval")
            .into_data()
            .expect("expected proof bytes");
        // No contract is needed to read an approval's proof.
        let (_, outcome) = Drive::verify_state_transition_was_executed_with_proof(
            transition,
            &BlockInfo::default(),
            &proof,
            &|_| Ok(None),
            platform_version,
        )
        .expect("expected the proof to verify");
        match outcome.into_result() {
            StateTransitionProofResult::VerifiedContractSettledDeletion(
                contract_id,
                document_type_name,
                _,
                settled_deletion,
            ) => {
                assert_eq!(contract_id, self.setup.contract.id());
                assert_eq!(document_type_name, STORY);
                settled_deletion
            }
            other => panic!("expected the approvals of a settled deletion, got {other:?}"),
        }
    }
}

/// A settled story goes once the leader and two members of the seated team approve its deletion
/// for one reason, not before: approvals without the leader fall short however many they are,
/// each member approves once, and a different reason is refused. The one that meets the rule
/// deletes the story as a moderator's deletion does, leaves its removal record, marks the
/// approvals deleted and counts for every approver; the ones before count for nobody.
#[tokio::test]
async fn should_delete_a_settled_story_once_the_leader_and_two_members_approve() {
    let team = Team::new(InterimModerators::ContractOwner).await;
    let setup = &team.setup;
    let story = team.written_by(&setup.stranger, STORY).await;
    team.award();
    let [joiner, other_joiner, _] = &team.joiners;
    team.process_and_commit(&team.addition_of(joiner).await);
    team.process_and_commit(&team.addition_of(other_joiner).await);

    // The elected member starts it, alone: the story stays.
    let first = setup
        .moderate(&team.member, approve(STORY, story.id(), "doxxing"))
        .await;
    let transaction = setup.platform.drive.grove.start_transaction();
    assert_success(&setup.process_at(&first, SETTLED_AT, &transaction));
    setup.commit(transaction);
    let open = team.proved_approvals(&first);
    assert_eq!(open.approvals, vec![team.member.id()]);
    assert_eq!(open.proposed_at, SETTLED_AT);
    assert_eq!(open.deleted_at, None);

    let transaction = setup.platform.drive.grove.start_transaction();
    // Twice is refused, in a block and in the mempool.
    let again = setup
        .moderate(&team.member, approve(STORY, story.id(), "doxxing"))
        .await;
    assert_eq!(
        setup
            .check_tx(&again)
            .iter()
            .map(|error| error.code())
            .collect::<Vec<_>>(),
        vec![SETTLED_DELETION_ALREADY_APPROVED]
    );
    assert_paid_with_code(
        &setup.process_at(&again, SETTLED_AT + 1, &transaction),
        SETTLED_DELETION_ALREADY_APPROVED,
    );
    // Two more members for the same reason: three approvals, as many as the rule asks for,
    // but without the leader. The story stays, and nobody is counted.
    for (member, at) in [(joiner, SETTLED_AT + 2), (other_joiner, SETTLED_AT + 3)] {
        let approval = setup
            .moderate(member, approve(STORY, story.id(), "doxxing"))
            .await;
        assert_success(&setup.process_at(&approval, at, &transaction));
    }
    assert!(team.is_stored(STORY, story.id(), &transaction));
    assert!(team.action_counts(&transaction).is_empty());

    // The leader, for another reason: refused.
    let elsewhere = setup
        .moderate(&team.leader, approve(STORY, story.id(), "spam"))
        .await;
    assert_paid_with_code(
        &setup.process_at(&elsewhere, SETTLED_AT + 4, &transaction),
        SETTLED_DELETION_REASON_MISMATCH,
    );
    // For the same reason: the leader among the approvals at last. The story goes.
    let last = setup
        .moderate(&team.leader, approve(STORY, story.id(), "doxxing"))
        .await;
    assert_success(&setup.process_at(&last, SETTLED_AT + 5, &transaction));
    assert!(!team.is_stored(STORY, story.id(), &transaction));
    let deleted = team
        .settled_deletion(STORY, story.id(), &transaction)
        .expect("expected the approvals");
    assert_eq!(
        deleted.approvals,
        vec![
            team.member.id(),
            joiner.id(),
            other_joiner.id(),
            team.leader.id()
        ]
    );
    assert_eq!(deleted.proposed_at, SETTLED_AT);
    assert_eq!(deleted.deleted_at, Some(SETTLED_AT + 5));
    let removal = setup
        .platform
        .drive
        .fetch_contract_document_removals(
            setup.contract.id(),
            &ContractDocumentRemovalsQuery {
                document_type_name: STORY.to_string(),
                selection: ContractDocumentRemovalsSelection::DocumentIds(vec![story.id()]),
            },
            Some(&transaction),
            PlatformVersion::latest(),
        )
        .expect("expected to fetch the removal")
        .pop()
        .expect("expected the removal record")
        .removal;
    assert_eq!(removal.document_owner_id, setup.stranger.id());
    assert_eq!(removal.moderator_id, team.leader.id());
    assert_eq!(removal.reason, deleted.reason);
    // Every approver counts once, for the deletion their approvals made.
    assert_eq!(
        team.action_counts(&transaction),
        BTreeMap::from([
            (team.leader.id(), 1),
            (team.member.id(), 1),
            (joiner.id(), 1),
            (other_joiner.id(), 1),
        ])
    );
    setup.commit(transaction);
    assert_eq!(
        team.proved_approvals(&last).deleted_at,
        Some(SETTLED_AT + 5)
    );
}

/// On a memo the leader alone deletes a settled document: its approval meets the rule at once,
/// and a member's approval waits for it.
#[tokio::test]
async fn should_let_the_leader_alone_delete_a_settled_memo() {
    let team = Team::new(InterimModerators::ContractOwner).await;
    let setup = &team.setup;
    let by_the_leader = team.written_by(&setup.stranger, MEMO).await;
    let by_a_member = team.written_by(&setup.stranger, MEMO).await;
    team.award();

    let transaction = setup.platform.drive.grove.start_transaction();
    let alone = setup
        .moderate(&team.leader, approve(MEMO, by_the_leader.id(), "doxxing"))
        .await;
    assert_success(&setup.process_at(&alone, SETTLED_AT, &transaction));
    assert!(!team.is_stored(MEMO, by_the_leader.id(), &transaction));

    let waiting = setup
        .moderate(&team.member, approve(MEMO, by_a_member.id(), "doxxing"))
        .await;
    assert_success(&setup.process_at(&waiting, SETTLED_AT, &transaction));
    assert!(team.is_stored(MEMO, by_a_member.id(), &transaction));
    let then = setup
        .moderate(&team.leader, approve(MEMO, by_a_member.id(), "doxxing"))
        .await;
    assert_success(&setup.process_at(&then, SETTLED_AT + 1, &transaction));
    assert!(!team.is_stored(MEMO, by_a_member.id(), &transaction));
}

/// Within its window a story is deleted by one moderator alone, and an approval is refused; once
/// settled, the reverse.
#[tokio::test]
async fn should_refuse_an_approval_while_the_story_is_within_its_window() {
    let team = Team::new(InterimModerators::ContractOwner).await;
    let setup = &team.setup;
    let story = team.written_by(&setup.stranger, STORY).await;
    team.award();

    let window_end = BLOCK_TIME_MS + SETTLING_WINDOW_SECONDS * 1_000;
    let transaction = setup.platform.drive.grove.start_transaction();
    let early = setup
        .moderate(&team.leader, approve(STORY, story.id(), "doxxing"))
        .await;
    assert_paid_with_code(
        &setup.process_at(&early, window_end, &transaction),
        DOCUMENT_NOT_SETTLED,
    );
    assert_eq!(team.settled_deletion(STORY, story.id(), &transaction), None);

    let alone_too_late = setup
        .moderate(&team.leader, delete_action(STORY, story.id()))
        .await;
    assert_paid_with_code(
        &setup.process_at(&alone_too_late, window_end + 1, &transaction),
        DOCUMENT_MODERATION_WINDOW_ELAPSED,
    );
    let settled = setup
        .moderate(&team.leader, approve(STORY, story.id(), "doxxing"))
        .await;
    assert_success(&setup.process_at(&settled, window_end + 1, &transaction));
}

/// Before a team is seated nobody approves a settled deletion, the interim moderators included;
/// a type that says nothing of settled deletions takes none from anyone; nobody off the team
/// approves one; and the first approval names a reason the team's proposal lists.
#[tokio::test]
async fn should_refuse_an_approval_without_a_seated_team_a_rule_or_a_seat() {
    let team = Team::new(InterimModerators::ContractOwner).await;
    let setup = &team.setup;
    let story = team.written_by(&setup.stranger, STORY).await;
    let post = team.posted_by(&setup.stranger).await;

    let transaction = setup.platform.drive.grove.start_transaction();
    let by_the_interim = setup
        .moderate(&setup.owner, approve(STORY, story.id(), "doxxing"))
        .await;
    assert_paid_with_code(
        &setup.process_at(&by_the_interim, SETTLED_AT, &transaction),
        CONTRACT_MODERATION_TEAM_NOT_SEATED,
    );
    setup.commit(transaction);

    team.award();
    let transaction = setup.platform.drive.grove.start_transaction();
    let on_a_post = setup
        .moderate(&team.leader, approve(POST, post.id(), "doxxing"))
        .await;
    assert_paid_with_code(
        &setup.process_at(&on_a_post, SETTLED_AT, &transaction),
        DOCUMENT_TYPE_NOT_DELETABLE_ONCE_SETTLED,
    );
    for off_the_team in [&setup.owner, &team.joiners[0]] {
        let refused = setup
            .moderate(off_the_team, approve(STORY, story.id(), "doxxing"))
            .await;
        assert_paid_with_code(
            &setup.process_at(&refused, SETTLED_AT, &transaction),
            IDENTITY_NOT_CONTRACT_MODERATOR,
        );
    }
    // The first approval names a reason document the team's proposal lists.
    let unlisted = setup
        .moderate(
            &team.leader,
            ContractUserModerationAction::DeleteSettledDocument {
                document_type_name: STORY.to_string(),
                document_id: story.id(),
                reason: ContractModerationReason::from_text("doxxing")
                    .with_reason_document(UNLISTED_REASON),
            },
        )
        .await;
    assert_paid_with_code(
        &setup.process_at(&unlisted, SETTLED_AT, &transaction),
        MODERATION_REASON_NOT_LISTED,
    );
    assert_eq!(team.settled_deletion(STORY, story.id(), &transaction), None);
}

/// A member who left the team no longer counts: the next approval drops it, and keeps the
/// others. Approvals nobody left on the team stands behind hold nothing: the next one starts
/// afresh, for its own reason, so a member the leader removed can not hold the reason. Approvals
/// that lapsed, a week after the first, start afresh with the next one too.
#[tokio::test]
async fn should_drop_approvers_who_left_and_start_afresh_once_none_remains_or_they_lapse() {
    let team = Team::new(InterimModerators::ContractOwner).await;
    let setup = &team.setup;
    let story = team.written_by(&setup.stranger, STORY).await;
    let other_story = team.written_by(&setup.stranger, STORY).await;
    team.award();
    let [joiner, other_joiner, _] = &team.joiners;
    let (joiner_addition, adding_joiner) = team.added(joiner).await;
    team.process_and_commit(&adding_joiner);
    team.process_and_commit(&team.addition_of(other_joiner).await);

    // The member and a joiner approve the story; the joiner alone approves the other one, for
    // a reason nobody else stands behind.
    let transaction = setup.platform.drive.grove.start_transaction();
    for (member, document, text, at) in [
        (&team.member, &story, "doxxing", SETTLED_AT),
        (joiner, &story, "doxxing", SETTLED_AT + 1),
        (joiner, &other_story, "i do not like it", SETTLED_AT + 2),
    ] {
        let approval = setup
            .moderate(member, approve(STORY, document.id(), text))
            .await;
        assert_success(&setup.process_at(&approval, at, &transaction));
    }
    setup.commit(transaction);
    // The leader removes the elected member and takes the joiner's addition back.
    team.process_and_commit(&team.removal_of(&team.member).await);
    team.process_and_commit(
        &team
            .undoing(ADDED_MODERATOR_DOCUMENT_TYPE_NAME, joiner_addition)
            .await,
    );
    let (joiner_readdition, readding_joiner) = team.added(joiner).await;
    team.process_and_commit(&readding_joiner);

    // The joiner is back on the team; the member is not, and is dropped.
    let transaction = setup.platform.drive.grove.start_transaction();
    let by_the_other_joiner = setup
        .moderate(other_joiner, approve(STORY, story.id(), "doxxing"))
        .await;
    assert_success(&setup.process_at(&by_the_other_joiner, SETTLED_AT + 3, &transaction));
    let pruned = team
        .settled_deletion(STORY, story.id(), &transaction)
        .expect("expected the approvals");
    assert_eq!(pruned.approvals, vec![joiner.id(), other_joiner.id()]);
    assert_eq!(pruned.proposed_at, SETTLED_AT);
    setup.commit(transaction);

    // The other story's only approver was off the team for a while and is back: its approval
    // still stands, and its reason with it.
    let transaction = setup.platform.drive.grove.start_transaction();
    let elsewhere = setup
        .moderate(&team.leader, approve(STORY, other_story.id(), "doxxing"))
        .await;
    assert_paid_with_code(
        &setup.process_at(&elsewhere, SETTLED_AT + 4, &transaction),
        SETTLED_DELETION_REASON_MISMATCH,
    );
    setup.commit(transaction);

    // Once it is off the team again, nothing holds the reason: the leader starts afresh.
    team.process_and_commit(
        &team
            .undoing(ADDED_MODERATOR_DOCUMENT_TYPE_NAME, joiner_readdition)
            .await,
    );
    let transaction = setup.platform.drive.grove.start_transaction();
    let afresh = setup
        .moderate(&team.leader, approve(STORY, other_story.id(), "doxxing"))
        .await;
    assert_success(&setup.process_at(&afresh, SETTLED_AT + 5, &transaction));
    let restarted = team
        .settled_deletion(STORY, other_story.id(), &transaction)
        .expect("expected the approvals");
    assert_eq!(restarted.approvals, vec![team.leader.id()]);
    assert_eq!(restarted.proposed_at, SETTLED_AT + 5);
    assert_eq!(restarted.reason.text, "doxxing");

    // A week after the first approval of the story, the approvals lapse.
    let lapsed_at = SETTLED_AT
        + PlatformVersion::latest()
            .system_limits
            .contract_settled_deletion_approval_window_ms
        + 1;
    let after_the_lapse = setup
        .moderate(&team.leader, approve(STORY, story.id(), "harassment"))
        .await;
    assert_success(&setup.process_at(&after_the_lapse, lapsed_at, &transaction));
    let lapsed = team
        .settled_deletion(STORY, story.id(), &transaction)
        .expect("expected the approvals");
    assert_eq!(lapsed.approvals, vec![team.leader.id()]);
    assert_eq!(lapsed.proposed_at, lapsed_at);
    assert_eq!(lapsed.reason.text, "harassment");
    assert!(team.is_stored(STORY, story.id(), &transaction));
}

/// A deletion the team approved together stands: no member restores it, the leader included. A
/// member's deletion within the window is restored as ever.
#[tokio::test]
async fn should_refuse_to_restore_a_deletion_the_team_approved() {
    let team = Team::new(InterimModerators::ContractOwner).await;
    let setup = &team.setup;
    let settled = team.written_by(&setup.stranger, MEMO).await;
    let fresh = team.written_by(&setup.stranger, MEMO).await;
    team.award();
    let bytes_of = |document: &Document| {
        let stored = setup
            .stored_document(MEMO, document.id(), None)
            .expect("expected the memo");
        setup.document_bytes(MEMO, &stored)
    };
    let (settled_bytes, fresh_bytes) = (bytes_of(&settled), bytes_of(&fresh));

    let transaction = setup.platform.drive.grove.start_transaction();
    // The leader alone deletes the settled memo; the member deletes the fresh one within its
    // window.
    let approval = setup
        .moderate(&team.leader, approve(MEMO, settled.id(), "doxxing"))
        .await;
    assert_success(&setup.process_at(&approval, SETTLED_AT, &transaction));
    let deletion = setup
        .moderate(&team.member, delete_action(MEMO, fresh.id()))
        .await;
    assert_success(&setup.process_at(&deletion, BLOCK_TIME_MS + 1_000, &transaction));

    for member in [&team.leader, &team.member] {
        let restore = setup
            .moderate(member, restore_action(MEMO, settled_bytes.clone()))
            .await;
        assert_paid_with_code(
            &setup.process_at(&restore, SETTLED_AT + 1, &transaction),
            SETTLED_DELETION_NOT_RESTORABLE,
        );
    }
    assert!(!team.is_stored(MEMO, settled.id(), &transaction));

    let restore = setup
        .moderate(&team.member, restore_action(MEMO, fresh_bytes))
        .await;
    assert_success(&setup.process_at(&restore, BLOCK_TIME_MS + 2_000, &transaction));
    assert!(team.is_stored(MEMO, fresh.id(), &transaction));
}

/// The approval that deletes a document forfeits the refund of whoever paid for the document,
/// and nothing else: the approvals record it rewrites shorter refunds the member who paid for
/// it.
#[tokio::test]
async fn should_refund_the_member_who_paid_for_the_approvals_the_deletion_rewrites() {
    let team = Team::new(InterimModerators::ContractOwner).await;
    let setup = &team.setup;
    let memo = team.written_by(&setup.stranger, MEMO).await;
    team.award();

    let transaction = setup.platform.drive.grove.start_transaction();
    // The member's approval falls short (the leader must approve the memo's deletion), for a
    // long reason, and lapses a week later.
    let member_approval = setup
        .moderate(&team.member, approve(MEMO, memo.id(), &"x".repeat(200)))
        .await;
    assert_success(&setup.process_at(&member_approval, SETTLED_AT, &transaction));

    // The leader's approval, for a short reason, starts afresh and deletes the memo: its
    // record is shorter than the member's it replaces.
    let lapsed_at = SETTLED_AT
        + PlatformVersion::latest()
            .system_limits
            .contract_settled_deletion_approval_window_ms
        + 1;
    let leader_approval = setup
        .moderate(&team.leader, approve(MEMO, memo.id(), "spam"))
        .await;
    let execution = setup.process_at(&leader_approval, lapsed_at, &transaction);
    let StateTransitionExecutionResult::SuccessfulExecution { fee_result, .. } = &execution else {
        panic!("expected the approval to pass, got {execution:?}");
    };
    assert_eq!(
        fee_result.fee_refunds.0.keys().copied().collect::<Vec<_>>(),
        vec![team.member.id().to_buffer()]
    );
    assert!(!team.is_stored(MEMO, memo.id(), &transaction));
}

/// No update adds a type that says who of the team approves the deletion of its settled
/// documents: the elected declaration, which no update changes, does not give the team
/// `deleteDocuments` on a type the contract was not created with, so the update is refused
/// (10900), and no such type is ever without the approvals tree only the contract's insertion
/// creates.
#[tokio::test]
async fn should_refuse_an_update_adding_a_type_that_deletes_settled_documents() {
    let team = Team::new(InterimModerators::ContractOwner).await;
    let setup = &team.setup;
    let mut changed = setup.contract.clone();
    changed.increment_version();
    let schema = changed
        .document_type_for_name(STORY)
        .expect("expected the story type")
        .schema()
        .clone();
    add_document_type(&mut changed, "saga", schema);
    let update = setup.contract_update(changed).await;
    let transaction = setup.platform.drive.grove.start_transaction();
    assert_unpaid_with_code(
        &setup.process(&update, &transaction),
        INVALID_CONTRACT_MODERATION_CONFIG,
    );
}

/// A moderator's change of a document's fields closes the approvals of its deletion: it moves
/// the document's revision but not its `$updatedAt`, so the document stays settled, and the
/// approvals given before are of the document as it was. The next approval starts afresh.
#[tokio::test]
async fn should_start_afresh_after_a_moderator_changes_the_fields_of_a_settled_document() {
    let team = Team::new(InterimModerators::ContractOwner).await;
    let setup = &team.setup;
    let (chronicle, create) = setup
        .create_document_of_type_with(&setup.stranger, CHRONICLE, |document| {
            document.properties_mut().remove("label");
        })
        .await;
    team.process_and_commit(&create);
    team.award();
    let [joiner, _, _] = &team.joiners;
    team.process_and_commit(&team.addition_of(joiner).await);

    let transaction = setup.platform.drive.grove.start_transaction();
    // The member and the leader approve: one short of the rule's three.
    for approver in [&team.member, &team.leader] {
        let approval = setup
            .moderate(approver, approve(CHRONICLE, chronicle.id(), "doxxing"))
            .await;
        assert_success(&setup.process_at(&approval, SETTLED_AT, &transaction));
    }

    // The leader labels the chronicle: a new revision, the same `$updatedAt`.
    let label = setup
        .moderate(
            &team.leader,
            ContractUserModerationAction::ChangeDocumentFields {
                document_type_name: CHRONICLE.to_string(),
                document_id: chronicle.id(),
                fields: BTreeMap::from([("label".to_string(), Value::Text("disputed".into()))]),
                reason: ContractModerationReason::from_text("checked")
                    .with_reason_document(LISTED_REASON),
            },
        )
        .await;
    assert_success(&setup.process_at(&label, SETTLED_AT + 1, &transaction));

    // The third approval would have met the rule; it starts afresh instead, the chronicle still
    // settled, and deletes nothing.
    let third = setup
        .moderate(joiner, approve(CHRONICLE, chronicle.id(), "doxxing"))
        .await;
    assert_success(&setup.process_at(&third, SETTLED_AT + 2, &transaction));
    assert!(team.is_stored(CHRONICLE, chronicle.id(), &transaction));
    let restarted = team
        .settled_deletion(CHRONICLE, chronicle.id(), &transaction)
        .expect("expected the approvals");
    assert_eq!(restarted.approvals, vec![joiner.id()]);
    assert_eq!(restarted.proposed_at, SETTLED_AT + 2);
    assert_eq!(restarted.deleted_at, None);
}

/// The author's replace of a settled document closes the approvals of its deletion too: it
/// moves `$updatedAt`, so the document is within its window again and no approval is taken
/// until it settles anew, and then the next approval starts afresh.
#[tokio::test]
async fn should_start_afresh_after_the_author_replaces_a_settled_document() {
    let team = Team::new(InterimModerators::ContractOwner).await;
    let setup = &team.setup;
    let mut story = team.written_by(&setup.stranger, STORY).await;
    team.award();

    let transaction = setup.platform.drive.grove.start_transaction();
    for approver in [&team.member, &team.leader] {
        let approval = setup
            .moderate(approver, approve(STORY, story.id(), "doxxing"))
            .await;
        assert_success(&setup.process_at(&approval, SETTLED_AT, &transaction));
    }

    // The author rewrites the story: new content, within its window again.
    let replaced_at = SETTLED_AT + 1;
    story.set("text", "rewritten".into());
    story
        .increment_revision()
        .expect("expected to bump the revision");
    let replace = BatchTransition::new_document_replacement_transition_from_document(
        story.clone(),
        setup
            .contract
            .document_type_for_name(STORY)
            .expect("expected the story type"),
        &setup.stranger.key,
        setup.stranger.contract_nonce(),
        0,
        None,
        &setup.stranger.signer,
        PlatformVersion::latest(),
        None,
    )
    .await
    .expect("expected to build the replacement");
    assert_success(&setup.process_at(&replace, replaced_at, &transaction));
    let too_soon = setup
        .moderate(&team.member, approve(STORY, story.id(), "doxxing"))
        .await;
    assert_paid_with_code(
        &setup.process_at(&too_soon, replaced_at + 1, &transaction),
        DOCUMENT_NOT_SETTLED,
    );

    // Once it settles anew, the leader's approval starts afresh rather than joining the two
    // given before the rewrite.
    let settled_again_at = replaced_at + SETTLING_WINDOW_SECONDS * 1_000 + 1;
    let afresh = setup
        .moderate(&team.leader, approve(STORY, story.id(), "doxxing"))
        .await;
    assert_success(&setup.process_at(&afresh, settled_again_at, &transaction));
    let restarted = team
        .settled_deletion(STORY, story.id(), &transaction)
        .expect("expected the approvals");
    assert_eq!(restarted.approvals, vec![team.leader.id()]);
    assert_eq!(restarted.proposed_at, settled_again_at);
    assert!(team.is_stored(STORY, story.id(), &transaction));
}
