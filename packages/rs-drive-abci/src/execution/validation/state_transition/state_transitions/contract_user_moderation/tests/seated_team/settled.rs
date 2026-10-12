//! The deletion of settled documents by a seated team (protocol version 14): past a type's
//! `deleteWithin` window no moderator deletes a document alone, and a type that says who must
//! approve (`moderatorAbilities.deleteSettled`) lets the members of the seated team delete it
//! together. One member proposes, which is kept under the contract as a team action, the others
//! approve it by its id, and the approval that meets the rule deletes the document.

use super::*;
use dpp::data_contract::config::moderation::{ContractTeamAction, ContractTeamActionEvent};
use dpp::group::group_action_status::GroupActionStatus;
use drive::drive::contract::moderation::types::ContractTeamActionWrite;
use drive::util::batch::{ContractModerationOperationType, DriveOperation};

const DOCUMENT_TYPE_NOT_DELETABLE_ONCE_SETTLED: u32 = 41204;
const CONTRACT_MODERATION_TEAM_NOT_SEATED: u32 = 41205;
const DOCUMENT_NOT_SETTLED: u32 = 41206;
const CONTRACT_TEAM_ACTION_DOES_NOT_EXIST: u32 = 41207;
const CONTRACT_TEAM_ACTION_ALREADY_SIGNED: u32 = 41208;
const MODERATION_REASON_NOT_LISTED: u32 = 41203;
const SETTLED_DELETION_NOT_RESTORABLE: u32 = 41209;
const CONTRACT_TEAM_ACTION_ALREADY_COMPLETED: u32 = 41210;
const CONTRACT_TEAM_ACTION_DOCUMENT_CHANGED: u32 = 41211;
const CONTRACT_TEAM_MEMBER_ADDED_AFTER_DOCUMENT: u32 = 41212;
const DOCUMENT_NOT_FOUND: u32 = 40101;

/// A month after the documents of these tests were written, and after the seat was awarded:
/// every story and memo is settled.
const SETTLED_AT: TimestampMillis = BLOCK_TIME_MS + 30 * 24 * 3_600 * 1_000;

/// A story of a type a contract registered before a rule dating the members the leader adds
/// needed `$createdAt` may hold (5.0.0-beta.1): its rule dates them (three approvals, the leader
/// among them), but its documents carry no `$createdAt`.
const UNDATED_STORY: &str = "undatedStory";

/// The same, its rule opting out (`approversPredateDocument: false`): every member the leader
/// added counts, whenever added.
const UNDATED_LEGEND: &str = "undatedLegend";

/// How long after the award the leader's additions of these tests are made
const ADDED_AFTER_THE_SEAT: TimestampMillis = 1_000;

/// How long after the award the documents written after those additions are: a second later,
/// so that the members the leader added count toward their deletion.
const WRITTEN_AFTER_THE_SEAT: TimestampMillis = 2_000;

/// The proposal of the deletion of settled `document_id` of `document_type_name`, for the
/// listed reason with `text`
fn propose(
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

/// The approval of team action `action_id`
fn approve(action_id: Identifier) -> ContractUserModerationAction {
    ContractUserModerationAction::ApproveTeamAction { action_id }
}

/// The team action a proposal creates, or an approval approves
fn action_id_of(transition: &StateTransition) -> Identifier {
    let StateTransition::ContractUserModeration(transition) = transition else {
        panic!("expected a contract user moderation");
    };
    transition
        .team_action_id()
        .expect("expected a team action's proposal or approval")
}

impl Team {
    /// Awards the seat ([`Team::award`]) and returns the time of its block, before which no
    /// addition can be made, checking that a document written just after it is settled at
    /// `SETTLED_AT`
    fn seated(&self) -> TimestampMillis {
        let seated_at = self.award();
        assert!(
            seated_at + WRITTEN_AFTER_THE_SEAT + SETTLING_WINDOW_SECONDS * 1_000 < SETTLED_AT,
            "a document written after the seat is settled by SETTLED_AT"
        );
        seated_at
    }

    /// Stores straight to Drive a new version of the contract with `UNDATED_STORY` and
    /// `UNDATED_LEGEND`, read without full validation as a stored contract is, the team holding
    /// `deleteDocuments` on both: types registration refuses now, which a contract stored
    /// before still holds.
    fn store_undated_types(&mut self) {
        let platform_version = PlatformVersion::latest();
        let mut contract = self.setup.contract.clone();
        let mut moderation = contract
            .config()
            .moderation()
            .cloned()
            .expect("expected the moderation declaration");
        let ContractModerators::Elected(elected) = &mut moderation.moderators else {
            panic!("expected an elected declaration");
        };
        for name in [UNDATED_STORY, UNDATED_LEGEND] {
            elected.moderated_document_types.insert(
                name.to_string(),
                BTreeSet::from([ModerationAbility::DeleteDocuments]),
            );
        }
        contract.set_config(contract.config().clone().with_moderation(Some(moderation)));
        for (name, rule) in [
            (
                UNDATED_STORY,
                platform_value!({ "leader": true, "approvals": 3 }),
            ),
            (
                UNDATED_LEGEND,
                platform_value!({
                    "leader": true,
                    "approvals": 3,
                    "approversPredateDocument": false,
                }),
            ),
        ] {
            contract
                .set_document_schema(
                    name,
                    post_schema_with(platform_value!({
                        "moderatorAbilities": {
                            "delete": true,
                            "deleteWithin": SETTLING_WINDOW_SECONDS,
                            "deleteSettled": rule,
                        },
                        "documentsMutable": true,
                        "required": ["text", "$updatedAt"],
                    })),
                    false,
                    &mut vec![],
                    platform_version,
                )
                .expect("expected a stored type to be read without full validation");
        }
        contract.increment_version();
        let transaction = self.setup.platform.drive.grove.start_transaction();
        self.setup
            .platform
            .drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                Some(&transaction),
                platform_version,
            )
            .expect("expected to store the contract");
        self.setup.commit(transaction);
        // Written outside a block: the cached copy is the version before.
        self.setup
            .platform
            .drive
            .cache
            .data_contracts
            .remove(contract.id().to_buffer());
        self.setup.contract = contract;
    }

    /// `member`'s approval of team action `action_id`, written straight to Drive as a node that
    /// did not date the members the leader adds stored it
    fn approval_stored_before(&self, member: &Actor, action_id: Identifier) {
        let transaction = self.setup.platform.drive.grove.start_transaction();
        self.setup
            .platform
            .drive
            .apply_drive_operations(
                vec![DriveOperation::ContractModerationOperation(
                    ContractModerationOperationType::AddTeamActionSignature {
                        contract_id: self.setup.contract.id(),
                        action_id,
                        signer_id: member.id(),
                        write: ContractTeamActionWrite::Approve {
                            dropped_signers: vec![],
                        },
                    },
                )],
                true,
                &BlockInfo::default(),
                Some(&transaction),
                PlatformVersion::latest(),
                None,
            )
            .expect("expected to store the approval");
        self.setup.commit(transaction);
    }

    /// The leader's addition of `actor`, processed and committed `ADDED_AFTER_THE_SEAT` after
    /// the seat
    async fn add_after(&self, actor: &Actor, seated_at: TimestampMillis) {
        self.process_and_commit_at(
            &self.addition_of(actor).await,
            seated_at + ADDED_AFTER_THE_SEAT,
        );
    }

    /// A document of `document_type_name` by `actor`, created and committed at the block time of
    /// the tests
    async fn written_by(&self, actor: &Actor, document_type_name: &str) -> Document {
        let (document, mut creates) = self
            .one_document_by(actor, document_type_name, &[None])
            .await;
        self.process_and_commit(&creates.pop().expect("expected the creation"));
        document
    }

    /// A document of `document_type_name` by `actor`, created and committed in a block at
    /// `time_ms`
    async fn written_at(
        &self,
        actor: &Actor,
        document_type_name: &str,
        time_ms: TimestampMillis,
    ) -> Document {
        let (document, mut creates) = self
            .one_document_by(actor, document_type_name, &[None])
            .await;
        self.process_and_commit_at(&creates.pop().expect("expected the creation"), time_ms);
        document
    }

    /// The team action `action_id` and where it is, if the team proposed it
    fn team_action(
        &self,
        action_id: Identifier,
        transaction: &Transaction,
    ) -> Option<(GroupActionStatus, ContractTeamAction)> {
        self.setup
            .platform
            .drive
            .fetch_contract_team_action(
                self.setup.contract.id(),
                action_id,
                Some(transaction),
                PlatformVersion::latest(),
            )
            .expect("expected to fetch the team action")
    }

    /// Who approved team action `action_id`, where it is
    fn signers(
        &self,
        status: GroupActionStatus,
        action_id: Identifier,
        transaction: &Transaction,
    ) -> Vec<Identifier> {
        self.setup
            .platform
            .drive
            .fetch_contract_team_action_signers(
                self.setup.contract.id(),
                status,
                action_id,
                Some(transaction),
                PlatformVersion::latest(),
            )
            .expect("expected to fetch the approvals")
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

    /// Proves the committed state for a proposal or an approval and returns where the proof
    /// shows it: in the action still active, or in the one that ran
    fn proved_status(&self, transition: &StateTransition) -> GroupActionStatus {
        let platform_version = PlatformVersion::latest();
        let proof = self
            .setup
            .platform
            .drive
            .prove_state_transition(transition, None, platform_version)
            .expect("expected to prove the approval")
            .into_data()
            .expect("expected proof bytes");
        // No contract is needed to read the proof of a team action.
        let (_, outcome) = Drive::verify_state_transition_was_executed_with_proof(
            transition,
            &BlockInfo::default(),
            &proof,
            &|_| Ok(None),
            platform_version,
        )
        .expect("expected the proof to verify");
        match outcome.into_result() {
            StateTransitionProofResult::VerifiedContractTeamActionSignature(
                contract_id,
                action_id,
                status,
            ) => {
                assert_eq!(contract_id, self.setup.contract.id());
                assert_eq!(action_id, action_id_of(transition));
                status
            }
            other => panic!("expected the approval of a team action, got {other:?}"),
        }
    }
}

/// A settled story goes once the leader and two members of the seated team approve its deletion,
/// not before: approvals without the leader fall short however many they are, and each member
/// approves once. The member's proposal is kept as a team action naming the story and the
/// reason; the approval that meets the rule deletes the story as a moderator's deletion does,
/// for the proposal's reason, closes the action with every approval, and counts for every
/// approver; the ones before count for nobody. The proof of the proposal holds once the action
/// ran, and an approval of an action that ran is refused. The two members the leader added were
/// added before the story was written, so they count.
#[tokio::test]
async fn should_delete_a_settled_story_once_the_leader_and_two_members_approve() {
    let team = Team::new(InterimModerators::ContractOwner).await;
    let setup = &team.setup;
    let seated_at = team.seated();
    let [joiner, other_joiner, _] = &team.joiners;
    team.add_after(joiner, seated_at).await;
    team.add_after(other_joiner, seated_at).await;
    let story = team
        .written_at(&setup.stranger, STORY, seated_at + WRITTEN_AFTER_THE_SEAT)
        .await;

    // The elected member proposes, alone: the story stays.
    let proposal = setup
        .moderate(&team.member, propose(STORY, story.id(), "doxxing"))
        .await;
    let action_id = action_id_of(&proposal);
    let transaction = setup.platform.drive.grove.start_transaction();
    assert_success(&setup.process_at(&proposal, SETTLED_AT, &transaction));
    setup.commit(transaction);
    assert_eq!(
        team.proved_status(&proposal),
        GroupActionStatus::ActionActive
    );
    let transaction = setup.platform.drive.grove.start_transaction();
    let (status, action) = team
        .team_action(action_id, &transaction)
        .expect("expected the team action");
    assert_eq!(status, GroupActionStatus::ActionActive);
    assert_eq!(action.proposer_id, team.member.id());
    assert_eq!(action.proposed_at, SETTLED_AT);
    let ContractTeamActionEvent::DeleteSettledDocument {
        document_id,
        reason,
        ..
    } = &action.event;
    assert_eq!(*document_id, story.id());
    assert_eq!(reason.text, "doxxing");

    // The proposer approving too is refused, in a block and in the mempool.
    let again = setup.moderate(&team.member, approve(action_id)).await;
    assert_eq!(
        setup
            .check_tx(&again)
            .iter()
            .map(|error| error.code())
            .collect::<Vec<_>>(),
        vec![CONTRACT_TEAM_ACTION_ALREADY_SIGNED]
    );
    assert_paid_with_code(
        &setup.process_at(&again, SETTLED_AT + 1, &transaction),
        CONTRACT_TEAM_ACTION_ALREADY_SIGNED,
    );
    // Two more members approve: three approvals, as many as the rule asks for, but without the
    // leader. The story stays, and nobody is counted.
    for (member, at) in [(joiner, SETTLED_AT + 2), (other_joiner, SETTLED_AT + 3)] {
        let approval = setup.moderate(member, approve(action_id)).await;
        assert_success(&setup.process_at(&approval, at, &transaction));
    }
    assert!(team.is_stored(STORY, story.id(), &transaction));
    assert!(team.action_counts(&transaction).is_empty());

    // The leader's approval puts the leader among the approvals at last. The story goes. Three
    // earlier approvers are more than the two queries of a team read, so the team is read once
    // (`fetch_active_seats`), and both added members, added before the story, count.
    let last = setup.moderate(&team.leader, approve(action_id)).await;
    assert_success(&setup.process_at(&last, SETTLED_AT + 5, &transaction));
    assert!(!team.is_stored(STORY, story.id(), &transaction));
    assert_eq!(
        team.team_action(action_id, &transaction)
            .expect("expected the team action")
            .0,
        GroupActionStatus::ActionClosed
    );
    let mut every_approver = vec![
        team.member.id(),
        joiner.id(),
        other_joiner.id(),
        team.leader.id(),
    ];
    every_approver.sort();
    assert_eq!(
        team.signers(GroupActionStatus::ActionClosed, action_id, &transaction),
        every_approver
    );
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
    assert_eq!(removal.reason, *reason);
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
    // The approvals moved with the action, so the proposal's proof holds once it ran.
    assert_eq!(
        team.proved_status(&proposal),
        GroupActionStatus::ActionClosed
    );
    assert_eq!(team.proved_status(&last), GroupActionStatus::ActionClosed);

    // An action that ran takes no more approvals: that it ran is checked first.
    let transaction = setup.platform.drive.grove.start_transaction();
    let late = setup.moderate(&team.member, approve(action_id)).await;
    assert_paid_with_code(
        &setup.process_at(&late, SETTLED_AT + 6, &transaction),
        CONTRACT_TEAM_ACTION_ALREADY_COMPLETED,
    );
}

/// On a memo the leader alone deletes a settled document: its proposal meets the rule at once,
/// the action written closed, and a member's proposal waits for the leader's approval.
#[tokio::test]
async fn should_let_the_leader_alone_delete_a_settled_memo() {
    let team = Team::new(InterimModerators::ContractOwner).await;
    let setup = &team.setup;
    let by_the_leader = team.written_by(&setup.stranger, MEMO).await;
    let by_a_member = team.written_by(&setup.stranger, MEMO).await;
    team.award();

    let transaction = setup.platform.drive.grove.start_transaction();
    let alone = setup
        .moderate(&team.leader, propose(MEMO, by_the_leader.id(), "doxxing"))
        .await;
    assert_success(&setup.process_at(&alone, SETTLED_AT, &transaction));
    assert!(!team.is_stored(MEMO, by_the_leader.id(), &transaction));
    assert_eq!(
        team.team_action(action_id_of(&alone), &transaction)
            .expect("expected the team action")
            .0,
        GroupActionStatus::ActionClosed
    );
    assert_eq!(
        team.action_counts(&transaction),
        BTreeMap::from([(team.leader.id(), 1)])
    );

    let waiting = setup
        .moderate(&team.member, propose(MEMO, by_a_member.id(), "doxxing"))
        .await;
    assert_success(&setup.process_at(&waiting, SETTLED_AT, &transaction));
    assert!(team.is_stored(MEMO, by_a_member.id(), &transaction));
    let then = setup
        .moderate(&team.leader, approve(action_id_of(&waiting)))
        .await;
    assert_success(&setup.process_at(&then, SETTLED_AT + 1, &transaction));
    assert!(!team.is_stored(MEMO, by_a_member.id(), &transaction));
}

/// Within its window a story is deleted by one moderator alone, and a proposal is refused; once
/// settled, the reverse.
#[tokio::test]
async fn should_refuse_a_proposal_while_the_story_is_within_its_window() {
    let team = Team::new(InterimModerators::ContractOwner).await;
    let setup = &team.setup;
    let story = team.written_by(&setup.stranger, STORY).await;
    team.award();

    let window_end = BLOCK_TIME_MS + SETTLING_WINDOW_SECONDS * 1_000;
    let transaction = setup.platform.drive.grove.start_transaction();
    let early = setup
        .moderate(&team.leader, propose(STORY, story.id(), "doxxing"))
        .await;
    assert_paid_with_code(
        &setup.process_at(&early, window_end, &transaction),
        DOCUMENT_NOT_SETTLED,
    );
    assert_eq!(team.team_action(action_id_of(&early), &transaction), None);

    let alone_too_late = setup
        .moderate(&team.leader, delete_action(STORY, story.id()))
        .await;
    assert_paid_with_code(
        &setup.process_at(&alone_too_late, window_end + 1, &transaction),
        DOCUMENT_MODERATION_WINDOW_ELAPSED,
    );
    let settled = setup
        .moderate(&team.leader, propose(STORY, story.id(), "doxxing"))
        .await;
    assert_success(&setup.process_at(&settled, window_end + 1, &transaction));
}

/// Before a team is seated nobody proposes a settled deletion, the interim moderators included;
/// a type that says nothing of settled deletions takes none from anyone; nobody off the team
/// proposes or approves one; a proposal names a reason the team's proposal lists; and an
/// approval names an action the team proposed.
#[tokio::test]
async fn should_refuse_a_proposal_or_an_approval_without_a_seated_team_a_rule_or_a_seat() {
    let team = Team::new(InterimModerators::ContractOwner).await;
    let setup = &team.setup;
    let story = team.written_by(&setup.stranger, STORY).await;
    let post = team.posted_by(&setup.stranger).await;

    let transaction = setup.platform.drive.grove.start_transaction();
    let by_the_interim = setup
        .moderate(&setup.owner, propose(STORY, story.id(), "doxxing"))
        .await;
    assert_paid_with_code(
        &setup.process_at(&by_the_interim, SETTLED_AT, &transaction),
        CONTRACT_MODERATION_TEAM_NOT_SEATED,
    );
    // Before a team is seated nothing was proposed, so an approval names no action: that is
    // looked for first, before the team is read.
    let before_the_seat = setup
        .moderate(&setup.owner, approve(Identifier::from([0x43; 32])))
        .await;
    assert_paid_with_code(
        &setup.process_at(&before_the_seat, SETTLED_AT, &transaction),
        CONTRACT_TEAM_ACTION_DOES_NOT_EXIST,
    );
    setup.commit(transaction);

    team.award();
    let transaction = setup.platform.drive.grove.start_transaction();
    let on_a_post = setup
        .moderate(&team.leader, propose(POST, post.id(), "doxxing"))
        .await;
    assert_paid_with_code(
        &setup.process_at(&on_a_post, SETTLED_AT, &transaction),
        DOCUMENT_TYPE_NOT_DELETABLE_ONCE_SETTLED,
    );
    for off_the_team in [&setup.owner, &team.joiners[0]] {
        let refused = setup
            .moderate(off_the_team, propose(STORY, story.id(), "doxxing"))
            .await;
        assert_paid_with_code(
            &setup.process_at(&refused, SETTLED_AT, &transaction),
            IDENTITY_NOT_CONTRACT_MODERATOR,
        );
    }
    // A proposal names a reason document the team's proposal lists.
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
    assert_eq!(
        team.team_action(action_id_of(&unlisted), &transaction),
        None
    );

    // An approval names an action the team proposed, and only a member approves one.
    let nothing_proposed = setup
        .moderate(&team.member, approve(Identifier::from([0x42; 32])))
        .await;
    assert_paid_with_code(
        &setup.process_at(&nothing_proposed, SETTLED_AT, &transaction),
        CONTRACT_TEAM_ACTION_DOES_NOT_EXIST,
    );
    let proposal = setup
        .moderate(&team.member, propose(STORY, story.id(), "doxxing"))
        .await;
    assert_success(&setup.process_at(&proposal, SETTLED_AT, &transaction));
    for off_the_team in [&setup.owner, &team.joiners[0]] {
        let refused = setup
            .moderate(off_the_team, approve(action_id_of(&proposal)))
            .await;
        assert_paid_with_code(
            &setup.process_at(&refused, SETTLED_AT + 1, &transaction),
            IDENTITY_NOT_CONTRACT_MODERATOR,
        );
    }
}

/// A member who left the team no longer counts: the approval that reads the team drops its
/// approval, refunded to it, and a member who comes back approves again. Nothing lapses: an
/// approval given long before still counts. On a legend, whose rule counts the members the
/// leader added whenever added, the joiners added after it was written among them.
#[tokio::test]
async fn should_drop_approvals_of_members_who_left_and_count_the_ones_who_stay() {
    let team = Team::new(InterimModerators::ContractOwner).await;
    let setup = &team.setup;
    let story = team.written_by(&setup.stranger, LEGEND).await;
    let seated_at = team.seated();
    let [joiner, other_joiner, _] = &team.joiners;
    let (joiner_addition, adding_joiner) = team.added(joiner).await;
    team.process_and_commit_at(&adding_joiner, seated_at + ADDED_AFTER_THE_SEAT);
    team.add_after(other_joiner, seated_at).await;

    // The member proposes and the joiner approves.
    let proposal = setup
        .moderate(&team.member, propose(LEGEND, story.id(), "doxxing"))
        .await;
    let action_id = action_id_of(&proposal);
    let transaction = setup.platform.drive.grove.start_transaction();
    assert_success(&setup.process_at(&proposal, SETTLED_AT, &transaction));
    let by_the_joiner = setup.moderate(joiner, approve(action_id)).await;
    assert_success(&setup.process_at(&by_the_joiner, SETTLED_AT + 1, &transaction));
    setup.commit(transaction);

    // The leader removes the elected member and takes the joiner's addition back.
    team.process_and_commit(&team.removal_of(&team.member).await);
    team.process_and_commit(
        &team
            .undoing(ADDED_MODERATOR_DOCUMENT_TYPE_NAME, joiner_addition)
            .await,
    );

    // A month later, the leader approves: three approvals could meet the rule, so the team is
    // read, the two who left are dropped and refunded, and the leader's alone falls short.
    let later = SETTLED_AT + 30 * 24 * 3_600 * 1_000;
    let transaction = setup.platform.drive.grove.start_transaction();
    let by_the_leader = setup.moderate(&team.leader, approve(action_id)).await;
    let execution = setup.process_at(&by_the_leader, later, &transaction);
    let StateTransitionExecutionResult::SuccessfulExecution { fee_result, .. } = &execution else {
        panic!("expected the approval to pass, got {execution:?}");
    };
    let mut refunded: Vec<[u8; 32]> = fee_result.fee_refunds.0.keys().copied().collect();
    refunded.sort();
    let mut who_left = vec![team.member.id().to_buffer(), joiner.id().to_buffer()];
    who_left.sort();
    assert_eq!(refunded, who_left);
    assert_eq!(
        team.signers(GroupActionStatus::ActionActive, action_id, &transaction),
        vec![team.leader.id()]
    );
    assert!(team.is_stored(LEGEND, story.id(), &transaction));
    setup.commit(transaction);

    // The joiner is added again and approves again; the other joiner's approval meets the rule.
    team.process_and_commit_at(&team.addition_of(joiner).await, later);
    let transaction = setup.platform.drive.grove.start_transaction();
    let joiner_again = setup.moderate(joiner, approve(action_id)).await;
    assert_success(&setup.process_at(&joiner_again, later + 1, &transaction));
    let closing = setup.moderate(other_joiner, approve(action_id)).await;
    assert_success(&setup.process_at(&closing, later + 2, &transaction));
    assert!(!team.is_stored(LEGEND, story.id(), &transaction));
    // Only the approvals that counted close with the action, and only they are counted.
    let mut counted = vec![team.leader.id(), joiner.id(), other_joiner.id()];
    counted.sort();
    assert_eq!(
        team.signers(GroupActionStatus::ActionClosed, action_id, &transaction),
        counted
    );
    assert_eq!(
        team.action_counts(&transaction),
        BTreeMap::from([
            (team.leader.id(), 1),
            (joiner.id(), 1),
            (other_joiner.id(), 1),
        ])
    );
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
    let proposal = setup
        .moderate(&team.leader, propose(MEMO, settled.id(), "doxxing"))
        .await;
    assert_success(&setup.process_at(&proposal, SETTLED_AT, &transaction));
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
/// and nothing else: the approvals and the action it moves to the closed actions refund the
/// members who paid for them.
#[tokio::test]
async fn should_refund_the_proposer_the_action_the_deleting_approval_moves() {
    let team = Team::new(InterimModerators::ContractOwner).await;
    let setup = &team.setup;
    let memo = team.written_by(&setup.stranger, MEMO).await;
    team.award();

    let transaction = setup.platform.drive.grove.start_transaction();
    // The member's proposal falls short: the leader must approve the memo's deletion.
    let proposal = setup
        .moderate(&team.member, propose(MEMO, memo.id(), "doxxing"))
        .await;
    assert_success(&setup.process_at(&proposal, SETTLED_AT, &transaction));

    // The leader's approval deletes the memo and closes the action: the member who paid for
    // the action and its approval is refunded both, the memo's author nothing.
    let leader_approval = setup
        .moderate(&team.leader, approve(action_id_of(&proposal)))
        .await;
    let execution = setup.process_at(&leader_approval, SETTLED_AT + 1, &transaction);
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
/// (10900), and no such type is ever without the team actions tree only the contract's
/// insertion creates.
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

/// A moderator's change of a document's fields stops the approvals of its deletion: it moves
/// the document's revision but not its `$updatedAt`, so the document stays settled, and the
/// proposal names the document as it was. An approval is refused; a fresh proposal is taken.
#[tokio::test]
async fn should_refuse_an_approval_after_a_moderator_changes_the_fields_of_a_settled_document() {
    let team = Team::new(InterimModerators::ContractOwner).await;
    let setup = &team.setup;
    let seated_at = team.seated();
    let [joiner, _, _] = &team.joiners;
    team.add_after(joiner, seated_at).await;
    let (chronicle, create) = setup
        .create_document_of_type_with(&setup.stranger, CHRONICLE, |document| {
            document.properties_mut().remove("label");
        })
        .await;
    team.process_and_commit_at(&create, seated_at + WRITTEN_AFTER_THE_SEAT);

    let transaction = setup.platform.drive.grove.start_transaction();
    // The member proposes and the leader approves: one short of the rule's three.
    let proposal = setup
        .moderate(&team.member, propose(CHRONICLE, chronicle.id(), "doxxing"))
        .await;
    assert_success(&setup.process_at(&proposal, SETTLED_AT, &transaction));
    let action_id = action_id_of(&proposal);
    let by_the_leader = setup.moderate(&team.leader, approve(action_id)).await;
    assert_success(&setup.process_at(&by_the_leader, SETTLED_AT, &transaction));

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

    // The third approval would have met the rule; it is refused instead, and deletes nothing.
    let third = setup.moderate(joiner, approve(action_id)).await;
    assert_paid_with_code(
        &setup.process_at(&third, SETTLED_AT + 2, &transaction),
        CONTRACT_TEAM_ACTION_DOCUMENT_CHANGED,
    );
    assert!(team.is_stored(CHRONICLE, chronicle.id(), &transaction));
    // The chronicle is still settled: the joiner proposes its deletion as it now is.
    let afresh = setup
        .moderate(joiner, propose(CHRONICLE, chronicle.id(), "doxxing"))
        .await;
    assert_success(&setup.process_at(&afresh, SETTLED_AT + 3, &transaction));
    assert_eq!(
        team.signers(
            GroupActionStatus::ActionActive,
            action_id_of(&afresh),
            &transaction
        ),
        vec![joiner.id()]
    );
}

/// The author's replace of a settled document stops the approvals of its deletion too: it moves
/// `$updatedAt`, so the document is within its window again and no proposal is taken until it
/// settles anew, and an approval of the proposal made before is refused even then.
#[tokio::test]
async fn should_refuse_an_approval_after_the_author_replaces_a_settled_document() {
    let team = Team::new(InterimModerators::ContractOwner).await;
    let setup = &team.setup;
    let mut story = team.written_by(&setup.stranger, STORY).await;
    team.award();

    let transaction = setup.platform.drive.grove.start_transaction();
    let proposal = setup
        .moderate(&team.member, propose(STORY, story.id(), "doxxing"))
        .await;
    assert_success(&setup.process_at(&proposal, SETTLED_AT, &transaction));

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
        .moderate(&team.leader, propose(STORY, story.id(), "doxxing"))
        .await;
    assert_paid_with_code(
        &setup.process_at(&too_soon, replaced_at + 1, &transaction),
        DOCUMENT_NOT_SETTLED,
    );

    // Once it settles anew, the leader's approval of the proposal made before the rewrite is
    // refused: it is of content the team never saw.
    let settled_again_at = replaced_at + SETTLING_WINDOW_SECONDS * 1_000 + 1;
    let stale = setup
        .moderate(&team.leader, approve(action_id_of(&proposal)))
        .await;
    assert_paid_with_code(
        &setup.process_at(&stale, settled_again_at, &transaction),
        CONTRACT_TEAM_ACTION_DOCUMENT_CHANGED,
    );
    assert!(team.is_stored(STORY, story.id(), &transaction));
}

/// A rule asking for more members than the seated team can hold asks for all it can hold, the
/// seat of a member the leader removed included: the leader can not lower the bar by removing
/// members who would not approve, and taking a removal back gives the seat back.
#[tokio::test]
async fn should_count_the_seat_of_a_removed_member_toward_a_rule_asking_for_the_whole_team() {
    let team = Team::new(InterimModerators::ContractOwner).await;
    let setup = &team.setup;
    let seated_at = team.seated();
    let [first, second, _] = &team.joiners;
    team.add_after(first, seated_at).await;
    team.add_after(second, seated_at).await;
    let epic = team
        .written_at(&setup.stranger, EPIC, seated_at + WRITTEN_AFTER_THE_SEAT)
        .await;
    let (removal, remove) = team.removed(&team.member).await;
    team.process_and_commit_at(&remove, seated_at + WRITTEN_AFTER_THE_SEAT);

    let transaction = setup.platform.drive.grove.start_transaction();
    // The team can hold four: the leader, the elected member and two additions. With the
    // elected member removed, all three still seated approve, and fall one short.
    let proposal = setup
        .moderate(&team.leader, propose(EPIC, epic.id(), "doxxing"))
        .await;
    assert_success(&setup.process_at(&proposal, SETTLED_AT, &transaction));
    let action_id = action_id_of(&proposal);
    for approver in [first, second] {
        let approval = setup.moderate(approver, approve(action_id)).await;
        assert_success(&setup.process_at(&approval, SETTLED_AT, &transaction));
    }
    assert!(team.is_stored(EPIC, epic.id(), &transaction));

    // The leader takes the removal back: the member's seat is its own again, and its approval
    // completes the rule.
    let reinstating = team
        .undoing(REMOVED_MODERATOR_DOCUMENT_TYPE_NAME, removal)
        .await;
    assert_success(&setup.process_at(&reinstating, SETTLED_AT + 1, &transaction));
    let last = setup.moderate(&team.member, approve(action_id)).await;
    assert_success(&setup.process_at(&last, SETTLED_AT + 2, &transaction));
    assert!(!team.is_stored(EPIC, epic.id(), &transaction));
}

/// Past as many earlier approvers as a read of the whole team takes queries, an approval reads
/// the team once instead of each approver's seat, with the same outcome: approvers who left are
/// dropped, and so is one the leader took off and added again after the epic was written, whose
/// new seat does not count for it; the rest count. Neither approves the epic again, so a rule
/// asking for the whole team never deletes it.
#[tokio::test]
async fn should_read_the_team_once_to_drop_the_approvers_who_no_longer_count() {
    let team = Team::new(InterimModerators::ContractOwner).await;
    let setup = &team.setup;
    let seated_at = team.seated();
    let [first, second, _] = &team.joiners;
    let (first_addition, add_first) = team.added(first).await;
    team.process_and_commit_at(&add_first, seated_at + ADDED_AFTER_THE_SEAT);
    let (second_addition, add_second) = team.added(second).await;
    team.process_and_commit_at(&add_second, seated_at + ADDED_AFTER_THE_SEAT);
    let epic = team
        .written_at(&setup.stranger, EPIC, seated_at + WRITTEN_AFTER_THE_SEAT)
        .await;

    let transaction = setup.platform.drive.grove.start_transaction();
    let proposal = setup
        .moderate(&team.member, propose(EPIC, epic.id(), "doxxing"))
        .await;
    assert_success(&setup.process_at(&proposal, SETTLED_AT, &transaction));
    let action_id = action_id_of(&proposal);
    for approver in [first, second] {
        let approval = setup.moderate(approver, approve(action_id)).await;
        assert_success(&setup.process_at(&approval, SETTLED_AT, &transaction));
    }
    // The leader takes the first addition back, and the second too, adding that member again:
    // on the team, but since after the epic was written.
    for team_change in [
        team.undoing(ADDED_MODERATOR_DOCUMENT_TYPE_NAME, first_addition)
            .await,
        team.undoing(ADDED_MODERATOR_DOCUMENT_TYPE_NAME, second_addition)
            .await,
        team.addition_of(second).await,
    ] {
        assert_success(&setup.process_at(&team_change, SETTLED_AT + 1, &transaction));
    }
    // Then the leader approves: three earlier approvers, more than the two queries of a team
    // read, so the team is read once. Both added members are dropped and refunded; the member
    // and the leader fall short of the four the team can hold.
    let leader_approval = setup.moderate(&team.leader, approve(action_id)).await;
    let execution = setup.process_at(&leader_approval, SETTLED_AT + 2, &transaction);
    let StateTransitionExecutionResult::SuccessfulExecution { fee_result, .. } = &execution else {
        panic!("expected the approval to pass, got {execution:?}");
    };
    let mut refunded: Vec<[u8; 32]> = fee_result.fee_refunds.0.keys().copied().collect();
    refunded.sort();
    let mut dropped = vec![first.id().to_buffer(), second.id().to_buffer()];
    dropped.sort();
    assert_eq!(refunded, dropped);
    let mut still_counted = vec![team.member.id(), team.leader.id()];
    still_counted.sort();
    assert_eq!(
        team.signers(GroupActionStatus::ActionActive, action_id, &transaction),
        still_counted
    );

    // The member added again after the epic does not approve it again, and the one taken off is
    // off the team: the epic stays.
    let again = setup.moderate(second, approve(action_id)).await;
    assert_paid_with_code(
        &setup.process_at(&again, SETTLED_AT + 3, &transaction),
        CONTRACT_TEAM_MEMBER_ADDED_AFTER_DOCUMENT,
    );
    let off_the_team = setup.moderate(first, approve(action_id)).await;
    assert_paid_with_code(
        &setup.process_at(&off_the_team, SETTLED_AT + 3, &transaction),
        IDENTITY_NOT_CONTRACT_MODERATOR,
    );
    assert!(team.is_stored(EPIC, epic.id(), &transaction));
}

/// Two members may propose the deletion of one document: once one of the actions runs, the other
/// takes no approval, the document gone (40101), and stays active.
#[tokio::test]
async fn should_refuse_an_approval_of_a_sibling_proposal_whose_document_is_gone() {
    let team = Team::new(InterimModerators::ContractOwner).await;
    let setup = &team.setup;
    let memo = team.written_by(&setup.stranger, MEMO).await;
    team.award();

    let transaction = setup.platform.drive.grove.start_transaction();
    // The member proposes first and waits for the leader; the leader proposes too, and its
    // proposal meets the memo's rule at once.
    let by_the_member = setup
        .moderate(&team.member, propose(MEMO, memo.id(), "doxxing"))
        .await;
    assert_success(&setup.process_at(&by_the_member, SETTLED_AT, &transaction));
    let by_the_leader = setup
        .moderate(&team.leader, propose(MEMO, memo.id(), "spam"))
        .await;
    assert_success(&setup.process_at(&by_the_leader, SETTLED_AT + 1, &transaction));
    assert!(!team.is_stored(MEMO, memo.id(), &transaction));

    let too_late = setup
        .moderate(&team.leader, approve(action_id_of(&by_the_member)))
        .await;
    assert_paid_with_code(
        &setup.process_at(&too_late, SETTLED_AT + 2, &transaction),
        DOCUMENT_NOT_FOUND,
    );
    assert_eq!(
        team.team_action(action_id_of(&by_the_member), &transaction)
            .expect("expected the team action")
            .0,
        GroupActionStatus::ActionActive
    );
}

/// A member the leader added counts toward a story's deletion only when added before the story
/// was written: one added in the same block or after neither proposes nor approves it, in a
/// block or in the mempool, each refusal paid, and nothing it sent is kept. The leader and the
/// elected member count however old the story, one written before the seat included, and a
/// member added before the story completes the rule.
#[tokio::test]
async fn should_refuse_members_added_after_a_story_and_count_those_added_before() {
    let team = Team::new(InterimModerators::ContractOwner).await;
    let setup = &team.setup;
    let before_the_seat = team.written_by(&setup.stranger, STORY).await;
    let seated_at = team.seated();
    let [early, late, _] = &team.joiners;
    team.add_after(early, seated_at).await;
    let story_written_at = seated_at + WRITTEN_AFTER_THE_SEAT;
    let story = team
        .written_at(&setup.stranger, STORY, story_written_at)
        .await;
    // In the block that wrote the story: not before it.
    team.process_and_commit_at(&team.addition_of(late).await, story_written_at);

    let transaction = setup.platform.drive.grove.start_transaction();
    let by_the_late = setup
        .moderate(late, propose(STORY, story.id(), "doxxing"))
        .await;
    assert_paid_with_code(
        &setup.process_at(&by_the_late, SETTLED_AT, &transaction),
        CONTRACT_TEAM_MEMBER_ADDED_AFTER_DOCUMENT,
    );
    assert_eq!(
        team.team_action(action_id_of(&by_the_late), &transaction),
        None
    );
    // Nor does the early member reach the story written before the seat, which no addition
    // predates, in a block or in the mempool (judged at the award, when only that story is
    // settled); the elected member does.
    let by_the_early = setup
        .moderate(early, propose(STORY, before_the_seat.id(), "doxxing"))
        .await;
    assert_eq!(
        setup
            .check_tx(&by_the_early)
            .iter()
            .map(|error| error.code())
            .collect::<Vec<_>>(),
        vec![CONTRACT_TEAM_MEMBER_ADDED_AFTER_DOCUMENT]
    );
    assert_paid_with_code(
        &setup.process_at(&by_the_early, SETTLED_AT, &transaction),
        CONTRACT_TEAM_MEMBER_ADDED_AFTER_DOCUMENT,
    );
    let by_the_member = setup
        .moderate(
            &team.member,
            propose(STORY, before_the_seat.id(), "doxxing"),
        )
        .await;
    assert_success(&setup.process_at(&by_the_member, SETTLED_AT, &transaction));

    // The elected member proposes the story; the late member's approval is refused and kept
    // nowhere, the early member's counts, and the leader's meets the rule.
    let proposal = setup
        .moderate(&team.member, propose(STORY, story.id(), "spam"))
        .await;
    let action_id = action_id_of(&proposal);
    assert_success(&setup.process_at(&proposal, SETTLED_AT, &transaction));
    let refused = setup.moderate(late, approve(action_id)).await;
    assert_paid_with_code(
        &setup.process_at(&refused, SETTLED_AT + 1, &transaction),
        CONTRACT_TEAM_MEMBER_ADDED_AFTER_DOCUMENT,
    );
    assert_eq!(
        team.signers(GroupActionStatus::ActionActive, action_id, &transaction),
        vec![team.member.id()]
    );
    for (approver, at) in [(early, SETTLED_AT + 2), (&team.leader, SETTLED_AT + 3)] {
        let approval = setup.moderate(approver, approve(action_id)).await;
        assert_success(&setup.process_at(&approval, at, &transaction));
    }
    assert!(!team.is_stored(STORY, story.id(), &transaction));
    let mut counted = vec![team.member.id(), early.id(), team.leader.id()];
    counted.sort();
    assert_eq!(
        team.signers(GroupActionStatus::ActionClosed, action_id, &transaction),
        counted
    );
}

/// A member the leader takes off and adds again after a story was written sits in the new
/// addition, too late for the story: the approval it gave before no longer counts, and the
/// approval that reads the team drops it, refunded, as it drops a member who left. It does not
/// approve again; a member added before the story completes the rule instead.
#[tokio::test]
async fn should_drop_the_approval_of_a_member_added_again_after_the_story() {
    let team = Team::new(InterimModerators::ContractOwner).await;
    let setup = &team.setup;
    let seated_at = team.seated();
    let [joiner, other_joiner, _] = &team.joiners;
    let (joiner_addition, adding_joiner) = team.added(joiner).await;
    team.process_and_commit_at(&adding_joiner, seated_at + ADDED_AFTER_THE_SEAT);
    team.add_after(other_joiner, seated_at).await;
    let story = team
        .written_at(&setup.stranger, STORY, seated_at + WRITTEN_AFTER_THE_SEAT)
        .await;

    // The joiner proposes, added before the story.
    let proposal = setup
        .moderate(joiner, propose(STORY, story.id(), "doxxing"))
        .await;
    let action_id = action_id_of(&proposal);
    team.process_and_commit_at(&proposal, SETTLED_AT);

    // The leader takes the joiner off and adds it again.
    team.process_and_commit_at(
        &team
            .undoing(ADDED_MODERATOR_DOCUMENT_TYPE_NAME, joiner_addition)
            .await,
        SETTLED_AT + 1,
    );
    team.process_and_commit_at(&team.addition_of(joiner).await, SETTLED_AT + 1);

    // The joiner's approval is still among the action's, but approving again is refused for the
    // late addition, not as already given: that approval no longer counts.
    let transaction = setup.platform.drive.grove.start_transaction();
    let stale = setup.moderate(joiner, approve(action_id)).await;
    assert_paid_with_code(
        &setup.process_at(&stale, SETTLED_AT + 2, &transaction),
        CONTRACT_TEAM_MEMBER_ADDED_AFTER_DOCUMENT,
    );

    // The elected member approves; the leader's approval could then meet the rule, so it reads
    // each earlier approver's seat: the joiner's approval is dropped and refunded, and the two
    // left fall short.
    let by_the_member = setup.moderate(&team.member, approve(action_id)).await;
    assert_success(&setup.process_at(&by_the_member, SETTLED_AT + 2, &transaction));
    let by_the_leader = setup.moderate(&team.leader, approve(action_id)).await;
    let execution = setup.process_at(&by_the_leader, SETTLED_AT + 3, &transaction);
    let StateTransitionExecutionResult::SuccessfulExecution { fee_result, .. } = &execution else {
        panic!("expected the approval to pass, got {execution:?}");
    };
    assert_eq!(
        fee_result.fee_refunds.0.keys().copied().collect::<Vec<_>>(),
        vec![joiner.id().to_buffer()]
    );
    let mut still_counted = vec![team.member.id(), team.leader.id()];
    still_counted.sort();
    assert_eq!(
        team.signers(GroupActionStatus::ActionActive, action_id, &transaction),
        still_counted
    );
    assert!(team.is_stored(STORY, story.id(), &transaction));

    // The joiner does not approve again; the other joiner, added before the story, completes it.
    let again = setup.moderate(joiner, approve(action_id)).await;
    assert_paid_with_code(
        &setup.process_at(&again, SETTLED_AT + 4, &transaction),
        CONTRACT_TEAM_MEMBER_ADDED_AFTER_DOCUMENT,
    );
    let closing = setup.moderate(other_joiner, approve(action_id)).await;
    assert_success(&setup.process_at(&closing, SETTLED_AT + 5, &transaction));
    assert!(!team.is_stored(STORY, story.id(), &transaction));
    let mut counted = vec![team.member.id(), team.leader.id(), other_joiner.id()];
    counted.sort();
    assert_eq!(
        team.signers(GroupActionStatus::ActionClosed, action_id, &transaction),
        counted
    );
    assert_eq!(
        team.action_counts(&transaction),
        BTreeMap::from([
            (team.leader.id(), 1),
            (team.member.id(), 1),
            (other_joiner.id(), 1),
        ])
    );
}

/// A stored type whose rule dates the members the leader adds, but whose documents carry no
/// `$createdAt`, registered before such a rule needed it: a document recording no creation time
/// predates no addition, so no added member proposes or approves its deletion (41212), and the
/// approvals of added members a node stored before are dropped, refunded, by the next approval
/// that reads the team, by one point read each or by one read of the whole team. The leader and
/// the elected member count.
#[tokio::test]
async fn should_count_no_added_member_on_a_stored_type_whose_documents_carry_no_creation_time() {
    let mut team = Team::new(InterimModerators::ContractOwner).await;
    team.store_undated_types();
    let team = team;
    let setup = &team.setup;
    let seated_at = team.seated();
    let [joiner, other_joiner, _] = &team.joiners;
    team.add_after(joiner, seated_at).await;
    team.add_after(other_joiner, seated_at).await;
    let story = team
        .written_at(
            &setup.stranger,
            UNDATED_STORY,
            seated_at + WRITTEN_AFTER_THE_SEAT,
        )
        .await;
    assert_eq!(
        setup
            .stored_document(UNDATED_STORY, story.id(), None)
            .expect("expected the story")
            .created_at(),
        None
    );

    // A joiner's proposal is refused; the elected member proposes twice, for two reasons.
    let transaction = setup.platform.drive.grove.start_transaction();
    let by_the_joiner = setup
        .moderate(joiner, propose(UNDATED_STORY, story.id(), "doxxing"))
        .await;
    assert_paid_with_code(
        &setup.process_at(&by_the_joiner, SETTLED_AT, &transaction),
        CONTRACT_TEAM_MEMBER_ADDED_AFTER_DOCUMENT,
    );
    let one_by_one = setup
        .moderate(&team.member, propose(UNDATED_STORY, story.id(), "doxxing"))
        .await;
    let whole_team = setup
        .moderate(&team.member, propose(UNDATED_STORY, story.id(), "spam"))
        .await;
    for proposal in [&one_by_one, &whole_team] {
        assert_success(&setup.process_at(proposal, SETTLED_AT, &transaction));
    }
    setup.commit(transaction);
    let (one_by_one, whole_team) = (action_id_of(&one_by_one), action_id_of(&whole_team));

    // Approvals of the joiners a node stored before: one on the first action, two on the
    // second.
    team.approval_stored_before(joiner, one_by_one);
    team.approval_stored_before(joiner, whole_team);
    team.approval_stored_before(other_joiner, whole_team);

    let transaction = setup.platform.drive.grove.start_transaction();
    let by_the_other_joiner = setup.moderate(other_joiner, approve(one_by_one)).await;
    assert_paid_with_code(
        &setup.process_at(&by_the_other_joiner, SETTLED_AT + 1, &transaction),
        CONTRACT_TEAM_MEMBER_ADDED_AFTER_DOCUMENT,
    );
    // The leader's approvals could meet the rule, so each reads the team: two earlier approvers
    // one point read each, three by one read of the whole team. The joiners' approvals are
    // dropped and refunded; the member and the leader fall short of three, and the story stays.
    let mut counted = vec![team.member.id(), team.leader.id()];
    counted.sort();
    for (action_id, dropped, at) in [
        (one_by_one, vec![joiner], SETTLED_AT + 2),
        (whole_team, vec![joiner, other_joiner], SETTLED_AT + 3),
    ] {
        let by_the_leader = setup.moderate(&team.leader, approve(action_id)).await;
        let execution = setup.process_at(&by_the_leader, at, &transaction);
        let StateTransitionExecutionResult::SuccessfulExecution { fee_result, .. } = &execution
        else {
            panic!("expected the approval to pass, got {execution:?}");
        };
        let mut refunded: Vec<[u8; 32]> = fee_result.fee_refunds.0.keys().copied().collect();
        refunded.sort();
        let mut dropped: Vec<[u8; 32]> = dropped
            .iter()
            .map(|member| member.id().to_buffer())
            .collect();
        dropped.sort();
        assert_eq!(refunded, dropped);
        assert_eq!(
            team.signers(GroupActionStatus::ActionActive, action_id, &transaction),
            counted
        );
    }
    assert!(team.is_stored(UNDATED_STORY, story.id(), &transaction));
}

/// A stored type opting out (`approversPredateDocument: false`) needs no creation time: members
/// the leader added propose and approve the deletion of its documents, which carry none, and
/// complete the rule with the leader.
#[tokio::test]
async fn should_let_added_members_delete_from_a_stored_opted_out_type_without_creation_times() {
    let mut team = Team::new(InterimModerators::ContractOwner).await;
    team.store_undated_types();
    let team = team;
    let setup = &team.setup;
    let seated_at = team.seated();
    let [joiner, other_joiner, _] = &team.joiners;
    team.add_after(joiner, seated_at).await;
    team.add_after(other_joiner, seated_at).await;
    let legend = team
        .written_at(
            &setup.stranger,
            UNDATED_LEGEND,
            seated_at + WRITTEN_AFTER_THE_SEAT,
        )
        .await;

    let transaction = setup.platform.drive.grove.start_transaction();
    let proposal = setup
        .moderate(joiner, propose(UNDATED_LEGEND, legend.id(), "doxxing"))
        .await;
    assert_success(&setup.process_at(&proposal, SETTLED_AT, &transaction));
    let action_id = action_id_of(&proposal);
    for (approver, at) in [
        (other_joiner, SETTLED_AT + 1),
        (&team.leader, SETTLED_AT + 2),
    ] {
        let approval = setup.moderate(approver, approve(action_id)).await;
        assert_success(&setup.process_at(&approval, at, &transaction));
    }
    assert!(!team.is_stored(UNDATED_LEGEND, legend.id(), &transaction));
    let mut counted = vec![joiner.id(), other_joiner.id(), team.leader.id()];
    counted.sort();
    assert_eq!(
        team.signers(GroupActionStatus::ActionClosed, action_id, &transaction),
        counted
    );
}
