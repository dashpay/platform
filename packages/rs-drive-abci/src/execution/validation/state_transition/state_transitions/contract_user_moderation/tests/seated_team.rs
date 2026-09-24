//! Elected moderation teams moderating from their stored charter (protocol version 14), through
//! the whole pipeline: a contest for the seat of an elected contract is awarded, and from then on
//! the seated charter's team moderates instead of the interim moderators, is protected, grows
//! within the target's `maxAddedModerators`, and may charge less than the declared moderators
//! fee.
//!
//! The proposal and the elected charter are filed through the pipeline, as their references
//! require; the join requests are written to Drive directly, since their key requirements and
//! envelope are covered where those keywords are.

use super::*;
use crate::execution::check_tx::CheckTxLevel;
use crate::execution::check_tx::CheckTxLevel::Recheck;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use crate::execution::validation::state_transition::common::seated_moderation_charter::fetch_seated_moderation_charter;
use crate::execution::validation::state_transition::processor::state::StateTransitionStateValidation;
use crate::execution::validation::state_transition::transformer::StateTransitionActionTransformer;
use crate::execution::validation::state_transition::ValidationMode;
use dpp::block::epoch::Epoch;
use dpp::block::extended_block_info::v0::ExtendedBlockInfoV0;
use dpp::data_contract::document_type::action_fees::agreement::{
    AgreedFeeMultiplier, DocumentActionFeeAgreement,
};
use dpp::data_contract::document_type::action_fees::{
    ActionFeePricing, ContractFeePot, DocumentActionFee,
};
use dpp::document::DocumentV0;
use dpp::fee::fee_result::FeeResult;
use dpp::moderation_charter::{
    moderators_share_of, ElectedCharter, ModerationCharterRewardSplit, SubmittedCharter,
    ADDED_MODERATOR_DOCUMENT_TYPE_NAME, ELECTED_CHARTER_DOCUMENT_TYPE_NAME,
    JOIN_REQUEST_DOCUMENT_TYPE_NAME, REASON_DOCUMENT_TYPE_NAME,
    REMOVED_MODERATOR_DOCUMENT_TYPE_NAME, SUBMITTED_CHARTER_DOCUMENT_TYPE_NAME,
};
use dpp::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
use dpp::state_transition::batch_transition::batched_transition::BatchedTransitionRef;
use dpp::state_transition::batch_transition::methods::StateTransitionCreationOptions;
use dpp::state_transition::batch_transition::BatchTransitionV0;
use dpp::state_transition::contract_fee_claim_transition::methods::ContractFeeClaimTransitionMethodsV0;
use dpp::state_transition::contract_fee_claim_transition::ContractFeeClaimTransition;
use dpp::version::DefaultForPlatformVersion;
use drive::drive::credit_pools::epochs::operations_factory::EpochOperations;
use drive::query::VotePollsByEndDateDriveQuery;
use drive::state_transition_action::batch::batched_transition::document_transition::DocumentTransitionAction;
use drive::state_transition_action::batch::batched_transition::BatchedTransitionAction;
use drive::state_transition_action::StateTransitionAction;
use drive::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
use drive::util::batch::GroveDbOpBatch;
use drive::util::object_size_info::DocumentInfo::DocumentRefInfo;
use drive::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use std::sync::Arc;

mod pot;
mod reasons;

const REFERENCED_ENTITY_NOT_FOUND: u32 = 40120;
const CONTRACT_MODERATION_ABILITY_NOT_GRANTED: u32 = 41201;
const MODERATION_CHARTER_ADDED_MODERATOR_LIMIT_REACHED: u32 = 41202;
const DOCUMENT_ACTION_FEE_MODERATORS_SHARE_MISMATCH: u32 = 40139;
const CONTRACT_FEE_CLAIM_NOT_ALLOWED: u32 = 41113;
const DOCUMENT_ACTION_FEE_AGREEMENT_MISMATCH: u32 = 40133;

/// What creating a post costs on top of the gas: 0.0001 Dash for the contract owner and 0.001
/// Dash for the moderation team.
const OWNER_PART: Credits = 10_000_000;
const MODERATORS_PART: Credits = 100_000_000;
/// The percentage of the declared moderators part the team's proposal takes.
const MODERATORS_SHARE: u8 = 60;
/// How many members the leader may add after the election.
const MAX_ADDED_MODERATORS: u16 = 2;
/// The challenge cool-down of a contestable seat: two weeks.
const CHALLENGE_COOL_DOWN: u32 = 1_209_600;
/// When the suspensions of these tests end: after the award too, which the mempool judges
/// against, the award's block time being the last committed one.
const LATER: TimestampMillis = 4_000_000_000_000;

/// A document type whose creation charges a fee priced by the fee multiplier, moderated with
/// bans only.
const REPLY: &str = "reply";
/// A document type whose creation charges a fixed fee, not moderated.
const NOTE: &str = "note";

/// An elected declaration keeping all three lists, moderating `post` with `abilities` and
/// `reply` with bans, with `interim` until a team is seated, room for `MAX_ADDED_MODERATORS`
/// additions, the owner protected from the team when `owner_protected`, and the seat
/// contestable after `challenge_cool_down` when there is one
fn elected_posts(
    interim: InterimModerators,
    abilities: &[ModerationAbility],
    owner_protected: bool,
    challenge_cool_down: Option<u32>,
) -> ContractModerationConfig {
    ContractModerationConfig {
        banlist: true,
        suspensions: true,
        warnings: true,
        moderators: ContractModerators::Elected(Box::new(ElectedModerators {
            join_window: DEFAULT_ELECTION_WINDOW_SECONDS,
            vote_window: DEFAULT_ELECTION_WINDOW_SECONDS,
            challenge_cool_down,
            election_delay: None,
            max_added_moderators: MAX_ADDED_MODERATORS,
            moderated_document_types: BTreeMap::from([
                (POST.to_string(), abilities.iter().copied().collect()),
                (REPLY.to_string(), BTreeSet::from([ModerationAbility::Ban])),
            ]),
            interim,
            owner_protected,
        })),
    }
}

/// Every ability on `post`
const ALL_ABILITIES: [ModerationAbility; 4] = [
    ModerationAbility::Ban,
    ModerationAbility::Suspend,
    ModerationAbility::Warn,
    ModerationAbility::DeleteDocuments,
];

/// An agreement to a post's creation fee naming `moderators` for the moderators part
fn agreeing_to(moderators: Credits) -> DocumentActionFeeAgreement {
    DocumentActionFeeAgreement::for_declared_fee(
        ActionFeePricing::Fixed,
        DocumentActionFee {
            owner: OWNER_PART,
            moderators,
        },
        AgreedFeeMultiplier {
            known_permille: 1_000,
            increase_tolerance_percent: 0,
        },
    )
}

/// The moderators part the seated charter's share gives
fn discounted() -> Credits {
    moderators_share_of(MODERATORS_PART, MODERATORS_SHARE)
}

/// The `reason` document the team's proposal lists, which every action of the team in these
/// tests names.
const LISTED_REASON: Identifier = Identifier::new([0xE1; 32]);
/// A `reason` document the team's proposal does not list.
const UNLISTED_REASON: Identifier = Identifier::new([0xE2; 32]);

/// `action` naming the reason document `reason_document_id`, when it carries a reason
fn citing(
    action: ContractUserModerationAction,
    reason_document_id: Identifier,
) -> ContractUserModerationAction {
    match action {
        ContractUserModerationAction::Ban {
            identity_id,
            reason,
        } => ContractUserModerationAction::Ban {
            identity_id,
            reason: reason.with_reason_document(reason_document_id),
        },
        ContractUserModerationAction::Suspend {
            identity_id,
            until,
            reason,
        } => ContractUserModerationAction::Suspend {
            identity_id,
            until,
            reason: reason.with_reason_document(reason_document_id),
        },
        ContractUserModerationAction::Warn {
            identity_id,
            reason,
        } => ContractUserModerationAction::Warn {
            identity_id,
            reason: reason.with_reason_document(reason_document_id),
        },
        ContractUserModerationAction::DeleteDocument {
            document_type_name,
            document_id,
            reason,
        } => ContractUserModerationAction::DeleteDocument {
            document_type_name,
            document_id,
            reason: reason.with_reason_document(reason_document_id),
        },
        reversal => reversal,
    }
}

// The team's actions name the listed reason: the helpers of the parent module, citing it.
fn ban_action(identity_id: Identifier) -> ContractUserModerationAction {
    citing(super::ban_action(identity_id), LISTED_REASON)
}

fn suspend_action(identity_id: Identifier, until: TimestampMillis) -> ContractUserModerationAction {
    citing(super::suspend_action(identity_id, until), LISTED_REASON)
}

fn warn_action(identity_id: Identifier, text: &str) -> ContractUserModerationAction {
    citing(super::warn_action(identity_id, text), LISTED_REASON)
}

fn delete_action(
    document_type_name: &str,
    document_id: Identifier,
) -> ContractUserModerationAction {
    citing(
        super::delete_action(document_type_name, document_id),
        LISTED_REASON,
    )
}

/// The banlist entry `ban_action` leaves
fn banned() -> Option<ContractBan> {
    Some(ContractBan {
        reason: ban_reason().with_reason_document(LISTED_REASON),
    })
}

/// A `reason` document of the charter contract at `reason_id`, owned by `owner`, written to
/// Drive as a reason create leaves it
fn write_reason(
    setup: &Setup,
    charters: &DataContract,
    reason_id: Identifier,
    owner: &Actor,
    code: &str,
) {
    let platform_version = PlatformVersion::latest();
    let document_type = charters
        .document_type_for_name(REASON_DOCUMENT_TYPE_NAME)
        .expect("expected the reason type");
    let document = Document::V0(DocumentV0 {
        id: reason_id,
        owner_id: owner.id(),
        properties: BTreeMap::from([
            ("code".to_string(), Value::Text(code.to_string())),
            ("label".to_string(), Value::Text(format!("Reason {code}"))),
        ]),
        created_at: Some(BLOCK_TIME_MS),
        ..Default::default()
    });
    setup
        .platform
        .drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((&document, None)),
                    owner_id: Some(owner.id().to_buffer()),
                },
                contract: charters,
                document_type,
            },
            false,
            BlockInfo::default(),
            true,
            None,
            platform_version,
            None,
        )
        .expect("expected to write the reason");
}

/// A contract with an elected declaration and a team on its way: the leader filed a proposal
/// and put it to the vote with one member, and three more identities asked to join it. The
/// contest is open until `award` ends it.
struct Team {
    setup: Setup,
    leader: Actor,
    member: Actor,
    joiners: [Actor; 3],
    charters: Arc<DataContract>,
    submitted_charter_id: Identifier,
    elected_charter_id: Identifier,
}

impl Team {
    async fn new(interim: InterimModerators) -> Self {
        Self::with_abilities(interim, &ALL_ABILITIES).await
    }

    async fn with_abilities(interim: InterimModerators, abilities: &[ModerationAbility]) -> Self {
        Self::build(
            interim,
            abilities,
            false,
            Some(CHALLENGE_COOL_DOWN),
            vec![LISTED_REASON],
        )
        .await
    }

    /// A team whose proposal lists `reasons`
    async fn with_reasons(interim: InterimModerators, reasons: Vec<Identifier>) -> Self {
        Self::build(
            interim,
            &ALL_ABILITIES,
            false,
            Some(CHALLENGE_COOL_DOWN),
            reasons,
        )
        .await
    }

    async fn build(
        interim: InterimModerators,
        abilities: &[ModerationAbility],
        owner_protected: bool,
        challenge_cool_down: Option<u32>,
        reasons: Vec<Identifier>,
    ) -> Self {
        let platform_version = PlatformVersion::latest();
        let mut setup = Setup::new_at_with(
            Some(elected_posts(
                interim,
                abilities,
                owner_protected,
                challenge_cool_down,
            )),
            platform_version,
            |c| {
                for (name, pricing) in [(POST, "fixed"), (REPLY, "feeMultiplier"), (NOTE, "fixed")]
                {
                    add_document_type(
                        c,
                        name,
                        post_schema_with(platform_value!({
                            "actionFees": {
                                "pricing": pricing,
                                "create": { "owner": OWNER_PART, "moderators": MODERATORS_PART },
                            },
                        })),
                    )
                }
            },
        )
        .await;
        let leader = Actor::new(&mut setup.platform, 21);
        let member = Actor::new(&mut setup.platform, 22);
        let joiners = [
            Actor::new(&mut setup.platform, 23),
            Actor::new(&mut setup.platform, 24),
            Actor::new(&mut setup.platform, 25),
        ];
        let charters = setup
            .platform
            .drive
            .cache
            .system_data_contracts
            .load_moderation_charters(platform_version)
            .expect("expected the moderation charters contract");

        // Two grounds anyone may cite; the proposal lists those it was given.
        for (reason_id, code) in [(LISTED_REASON, "SPM"), (UNLISTED_REASON, "OFF")] {
            write_reason(&setup, &charters, reason_id, &joiners[2], code);
        }
        let proposal = SubmittedCharter {
            target_contract_id: setup.contract.id(),
            description: "We keep the posts civil".to_string(),
            reasons,
            moderators_share: Some(MODERATORS_SHARE),
            reward_split: ModerationCharterRewardSplit {
                leader: 10,
                equal: 40,
                actions: 50,
            },
        };
        let mut team = Self {
            setup,
            leader,
            member,
            joiners,
            charters,
            submitted_charter_id: Identifier::default(),
            elected_charter_id: Identifier::default(),
        };
        let (proposal, filing) = team
            .charter_document(
                &team.leader,
                SUBMITTED_CHARTER_DOCUMENT_TYPE_NAME,
                proposal.to_document_properties(),
            )
            .await;
        team.process_and_commit(&filing);
        team.submitted_charter_id = proposal.id();

        for (index, actor) in [&team.member]
            .into_iter()
            .chain(team.joiners.iter())
            .enumerate()
        {
            team.write_join_request(actor, index as u8);
        }

        let charter = ElectedCharter {
            target_contract_id: team.setup.contract.id(),
            submitted_charter_id: team.submitted_charter_id,
            members: vec![team.member.id()],
        };
        let (charter, application) = team
            .charter_document(
                &team.leader,
                ELECTED_CHARTER_DOCUMENT_TYPE_NAME,
                charter.to_document_properties(),
            )
            .await;
        team.process_and_commit(&application);
        team.elected_charter_id = charter.id();
        team
    }

    /// A creation by `actor` of a moderation charters document, and the document it creates
    async fn charter_document(
        &self,
        actor: &Actor,
        document_type_name: &str,
        properties: BTreeMap<String, Value>,
    ) -> (Document, StateTransition) {
        let platform_version = PlatformVersion::latest();
        let document_type = self
            .charters
            .document_type_for_name(document_type_name)
            .expect("expected the charter document type");
        let nonce = actor.contract_nonce();
        let (entropy, document) = {
            let mut rng = self.setup.rng.borrow_mut();
            let entropy = Bytes32::random_with_rng(&mut rng);
            let mut document = document_type
                .random_document_with_identifier_and_entropy(
                    &mut rng,
                    actor.id(),
                    entropy,
                    DocumentFieldFillType::DoNotFillIfNotRequired,
                    DocumentFieldFillSize::MinDocumentFillSize,
                    platform_version,
                )
                .expect("expected a random document");
            document.set_properties(properties);
            document
                .set_id_for_creation(document_type, &entropy.0, nonce, platform_version)
                .expect("expected to set the document id");
            (entropy, document)
        };
        let transition = BatchTransition::new_document_creation_transition_from_document(
            document.clone(),
            document_type,
            entropy.0,
            &actor.key,
            nonce,
            0,
            None,
            &actor.signer,
            platform_version,
            None,
        )
        .await
        .expect("expected to build the charter document creation");
        (document, transition)
    }

    /// `actor`'s offer to join the proposal, written to Drive as a join request create leaves it
    fn write_join_request(&self, actor: &Actor, discriminator: u8) {
        let platform_version = PlatformVersion::latest();
        let document_type = self
            .charters
            .document_type_for_name(JOIN_REQUEST_DOCUMENT_TYPE_NAME)
            .expect("expected the join request type");
        let document = Document::V0(DocumentV0 {
            id: Identifier::from([0xA0 + discriminator; 32]),
            owner_id: actor.id(),
            properties: BTreeMap::from([
                (
                    "submittedCharterId".to_string(),
                    Value::Identifier(self.submitted_charter_id.to_buffer()),
                ),
                (
                    "recipientId".to_string(),
                    Value::Identifier(self.leader.id().to_buffer()),
                ),
                ("recipientKeyId".to_string(), Value::U32(3)),
                ("senderKeyId".to_string(), Value::U32(3)),
                ("encryptedMessage".to_string(), Value::Bytes(vec![7; 48])),
            ]),
            created_at: Some(BLOCK_TIME_MS),
            ..Default::default()
        });
        self.setup
            .platform
            .drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&document, None)),
                        owner_id: Some(actor.id().to_buffer()),
                    },
                    contract: &self.charters,
                    document_type,
                },
                false,
                BlockInfo::default(),
                true,
                None,
                platform_version,
                None,
            )
            .expect("expected to write the join request");
    }

    fn process_and_commit(&self, transition: &StateTransition) {
        let transaction = self.setup.platform.drive.grove.start_transaction();
        assert_success(&self.setup.process(transition, &transaction));
        self.setup.commit(transaction);
    }

    /// Ends the contest for the seat as the block at the end of the join window does: with a
    /// single contender, the charter is awarded there.
    fn award(&self) {
        let platform_version = PlatformVersion::latest();
        let platform = &self.setup.platform;
        let (end_time, _) = VotePollsByEndDateDriveQuery {
            start_time: None,
            end_time: None,
            limit: None,
            offset: None,
            order_ascending: true,
        }
        .execute_no_proof(&platform.drive, None, &mut vec![], platform_version)
        .expect("expected the end date of the contest")
        .into_iter()
        .next()
        .expect("expected an open contest");
        let block_info = BlockInfo {
            time_ms: end_time,
            height: 2,
            core_height: 42,
            epoch: Default::default(),
        };
        let mut platform_state = (**platform.state.load()).clone();
        platform_state.set_last_committed_block_info(Some(
            ExtendedBlockInfoV0 {
                basic_info: block_info,
                app_hash: platform
                    .drive
                    .grove
                    .root_hash(None, &platform_version.drive.grove_version)
                    .unwrap()
                    .expect("expected the root hash"),
                quorum_hash: [0u8; 32],
                block_id_hash: [0u8; 32],
                proposer_pro_tx_hash: [0u8; 32],
                signature: [0u8; 96],
                round: 0,
            }
            .into(),
        ));
        platform.state.store(Arc::new(platform_state));
        let platform_state = platform.state.load();
        let transaction = platform.drive.grove.start_transaction();
        platform
            .check_for_ended_vote_polls(
                &platform_state,
                &platform_state,
                &block_info,
                Some(&transaction),
                platform_version,
            )
            .expect("expected to end the contest");
        self.setup.commit(transaction);
        assert_eq!(
            self.stored_charter(ELECTED_CHARTER_DOCUMENT_TYPE_NAME, self.elected_charter_id)
                .map(|document| document.owner_id()),
            Some(self.leader.id()),
            "the contest is awarded to its single contender, the leader"
        );
    }

    /// The charter contract's document of `document_type_name` stored at `document_id`
    fn stored_charter(
        &self,
        document_type_name: &str,
        document_id: Identifier,
    ) -> Option<Document> {
        let document_type = self
            .charters
            .document_type_for_name(document_type_name)
            .expect("expected the charter document type");
        let query = DriveDocumentQuery::new_primary_key_single_item_query(
            &self.charters,
            document_type,
            document_id,
        );
        self.setup
            .platform
            .drive
            .query_documents(query, None, false, None, None)
            .expect("expected to query the charter document")
            .documents_owned()
            .pop()
    }

    /// The leader's addition of `actor` to the seated team
    async fn addition_of(&self, actor: &Actor) -> StateTransition {
        self.added(actor).await.1
    }

    /// The leader's addition of `actor` to the seated team, and the `addedModerator` it creates
    async fn added(&self, actor: &Actor) -> (Document, StateTransition) {
        let properties = BTreeMap::from([
            (
                "electedCharterId".to_string(),
                Value::Identifier(self.elected_charter_id.to_buffer()),
            ),
            (
                "submittedCharterId".to_string(),
                Value::Identifier(self.submitted_charter_id.to_buffer()),
            ),
            (
                "memberId".to_string(),
                Value::Identifier(actor.id().to_buffer()),
            ),
        ]);
        self.charter_document(&self.leader, ADDED_MODERATOR_DOCUMENT_TYPE_NAME, properties)
            .await
    }

    /// The leader's removal of `actor` from the seated team
    async fn removal_of(&self, actor: &Actor) -> StateTransition {
        self.removed(actor).await.1
    }

    /// The leader's removal of `actor` from the seated team, and the `removedModerator` it
    /// creates
    async fn removed(&self, actor: &Actor) -> (Document, StateTransition) {
        let properties = BTreeMap::from([
            (
                "electedCharterId".to_string(),
                Value::Identifier(self.elected_charter_id.to_buffer()),
            ),
            (
                "memberId".to_string(),
                Value::Identifier(actor.id().to_buffer()),
            ),
        ]);
        self.charter_document(
            &self.leader,
            REMOVED_MODERATOR_DOCUMENT_TYPE_NAME,
            properties,
        )
        .await
    }

    /// The leader's deletion of its team change `document` of `document_type_name`, which
    /// undoes it
    async fn undoing(&self, document_type_name: &str, document: Document) -> StateTransition {
        let document_type = self
            .charters
            .document_type_for_name(document_type_name)
            .expect("expected the charter document type");
        BatchTransition::new_document_deletion_transition_from_document(
            document,
            document_type,
            &self.leader.key,
            self.leader.contract_nonce(),
            0,
            None,
            &self.leader.signer,
            PlatformVersion::latest(),
            None,
        )
        .await
        .expect("expected to build the charter document deletion")
    }

    /// A post by `actor`, agreeing to `agreement` (`None`: no agreement at all), and the post
    async fn post_by(
        &self,
        actor: &Actor,
        agreement: Option<DocumentActionFeeAgreement>,
    ) -> (Document, StateTransition) {
        let (document, mut transitions) = self.one_document_by(actor, POST, &[agreement]).await;
        let transition = transitions.pop().expect("expected the post creation");
        (document, transition)
    }

    /// One document of `document_type_name` by `actor`, the same document under the same
    /// nonce, created once for each of `agreements`: transitions that differ in their agreement
    /// alone
    async fn one_document_by(
        &self,
        actor: &Actor,
        document_type_name: &str,
        agreements: &[Option<DocumentActionFeeAgreement>],
    ) -> (Document, Vec<StateTransition>) {
        let platform_version = PlatformVersion::latest();
        let document_type = self
            .setup
            .contract
            .document_type_for_name(document_type_name)
            .expect("expected the document type");
        let nonce = actor.contract_nonce();
        let (entropy, document) = {
            let mut rng = self.setup.rng.borrow_mut();
            let entropy = Bytes32::random_with_rng(&mut rng);
            let mut document = document_type
                .random_document_with_identifier_and_entropy(
                    &mut rng,
                    actor.id(),
                    entropy,
                    DocumentFieldFillType::FillIfNotRequired,
                    DocumentFieldFillSize::MinDocumentFillSize,
                    platform_version,
                )
                .expect("expected a random post");
            document
                .set_id_for_creation(document_type, &entropy.0, nonce, platform_version)
                .expect("expected to set the post id");
            (entropy, document)
        };
        let mut transitions = vec![];
        for agreement in agreements {
            transitions.push(
                BatchTransition::new_document_creation_transition_from_document(
                    document.clone(),
                    document_type,
                    entropy.0,
                    &actor.key,
                    nonce,
                    0,
                    None,
                    &actor.signer,
                    platform_version,
                    Some(StateTransitionCreationOptions {
                        action_fee_agreement: *agreement,
                        ..Default::default()
                    }),
                )
                .await
                .expect("expected to build the post creation"),
            );
        }
        (document, transitions)
    }

    /// A post by `actor` that agrees to the declared fee, created and committed
    async fn posted_by(&self, actor: &Actor) -> Document {
        let (post, create) = self
            .post_by(actor, Some(agreeing_to(MODERATORS_PART)))
            .await;
        self.process_and_commit(&create);
        post
    }

    /// The credits in the contract's moderators pot
    fn moderators_pot(&self, transaction: &Transaction) -> Credits {
        self.setup
            .platform
            .drive
            .fetch_contract_fee_pot(
                self.setup.contract.id(),
                ContractFeePot::Moderators,
                Some(transaction),
                PlatformVersion::latest(),
            )
            .expect("expected to fetch the moderators pot")
            .credits
    }

    /// The codes check tx refuses `transition` with at `level`
    fn check_tx_codes(&self, transition: &StateTransition, level: CheckTxLevel) -> Vec<u32> {
        let version = PlatformVersion::latest();
        let state = self.setup.platform.state.load();
        let platform_ref = PlatformRef {
            drive: &self.setup.platform.drive,
            state: &state,
            config: &self.setup.platform.config,
            core_rpc: &self.setup.platform.core_rpc,
        };
        let raw = transition
            .serialize_to_bytes()
            .expect("expected to serialize");
        self.setup
            .platform
            .check_tx(&raw, level, &platform_ref, version)
            .expect("expected to check tx")
            .errors
            .iter()
            .map(|error| error.code())
            .collect()
    }
}

/// The fees a processed transition paid, the action fee aside
fn fees_of(execution: &StateTransitionExecutionResult) -> FeeResult {
    match execution {
        StateTransitionExecutionResult::SuccessfulExecution { fee_result, .. } => {
            fee_result.clone()
        }
        StateTransitionExecutionResult::PaidConsensusError { actual_fees, .. } => {
            actual_fees.clone()
        }
        other => panic!("expected a paid execution, got {other:?}"),
    }
}

/// The gas a processed transition paid, the action fee aside
fn gas_of(execution: &StateTransitionExecutionResult) -> Credits {
    fees_of(execution).total_base_fee()
}

/// The interim moderates until the contest for the seat is awarded; from then on the leader and
/// the elected member moderate, with every ability the declaration gives them, and the interim
/// moderators, the owner and everyone else are refused. What the interim did stands.
#[tokio::test]
async fn should_seat_the_winner_of_the_contest_and_let_its_team_moderate_instead_of_the_interim() {
    let team = Team::new(InterimModerators::AppointedModerators(
        [THE_MODERATOR].into(),
    ))
    .await;
    let setup = &team.setup;
    let interim = &setup.moderator;

    // Before the award: the interim moderates and the contender's team does not.
    let transaction = setup.platform.drive.grove.start_transaction();
    let ban = setup.moderate(interim, ban_action(setup.user.id())).await;
    assert_success(&setup.process(&ban, &transaction));
    for actor in [&team.leader, &team.member] {
        let ban = setup.moderate(actor, ban_action(setup.stranger.id())).await;
        assert_paid_with_code(
            &setup.process(&ban, &transaction),
            IDENTITY_NOT_CONTRACT_MODERATOR,
        );
    }
    setup.commit(transaction);
    let post = team.posted_by(&setup.stranger).await;

    team.award();

    // The team moderates: the leader and the elected member ban, suspend, warn, lift and
    // delete, in a block and in the mempool.
    let transaction = setup.platform.drive.grove.start_transaction();
    let suspend = setup
        .moderate(&team.leader, suspend_action(setup.stranger.id(), LATER))
        .await;
    assert!(setup.check_tx(&suspend).is_empty());
    assert_success(&setup.process(&suspend, &transaction));
    let warn = setup
        .moderate(&team.member, warn_action(setup.stranger.id(), "calm down"))
        .await;
    assert_success(&setup.process(&warn, &transaction));
    let unsuspend = setup
        .moderate(&team.member, unsuspend_action(setup.stranger.id()))
        .await;
    assert_success(&setup.process(&unsuspend, &transaction));
    let stored = setup
        .stored_document(POST, post.id(), Some(&transaction))
        .expect("expected the post to be stored");
    let delete = setup
        .moderate(&team.member, delete_action(POST, post.id()))
        .await;
    assert_success(&setup.process(&delete, &transaction));
    // The leader undoes the member's deletion: a restore needs the same ability.
    let restore = setup
        .moderate(
            &team.leader,
            restore_action(POST, setup.document_bytes(POST, &stored)),
        )
        .await;
    assert_success(&setup.process(&restore, &transaction));
    assert_eq!(
        setup.stored_document(POST, post.id(), Some(&transaction)),
        Some(stored)
    );
    // What the interim did stands, and the team may undo it.
    assert_eq!(
        setup.status(setup.user.id(), Some(&transaction)).ban,
        banned()
    );
    let unban = setup
        .moderate(&team.leader, unban_action(setup.user.id()))
        .await;
    assert_success(&setup.process(&unban, &transaction));
    let ban = setup
        .moderate(&team.member, ban_action(setup.user.id()))
        .await;
    assert_success(&setup.process(&ban, &transaction));

    // Nobody else does: the interim moderator, the owner and a stranger, in the mempool too.
    for actor in [interim, &setup.owner, &team.joiners[0]] {
        let ban = setup.moderate(actor, ban_action(setup.stranger.id())).await;
        let mempool_errors = setup.check_tx(&ban);
        assert_eq!(
            mempool_errors.iter().map(|e| e.code()).collect::<Vec<_>>(),
            vec![IDENTITY_NOT_CONTRACT_MODERATOR]
        );
        assert_paid_with_code(
            &setup.process(&ban, &transaction),
            IDENTITY_NOT_CONTRACT_MODERATOR,
        );
        let delete = setup.moderate(actor, delete_action(POST, post.id())).await;
        assert_paid_with_code(
            &setup.process(&delete, &transaction),
            IDENTITY_NOT_CONTRACT_MODERATOR,
        );
    }
}

/// A seat is never contested again in protocol version 14, whatever the target declares:
/// challenges come later, so once a charter is seated another leader's elected charter for the
/// same target is refused, with the target's seat contestable or not, and the seated charter
/// keeps the seat.
#[tokio::test]
async fn should_keep_the_seat_of_a_seated_team_whether_or_not_it_is_contestable() {
    let platform_version = PlatformVersion::latest();
    for challenge_cool_down in [Some(CHALLENGE_COOL_DOWN), None] {
        let team = Team::build(
            InterimModerators::ContractOwner,
            &ALL_ABILITIES,
            false,
            challenge_cool_down,
            vec![LISTED_REASON],
        )
        .await;
        team.award();

        // A rival files its own proposal for the seated target, which a proposal may, and puts
        // it to the vote.
        let rival = &team.joiners[0];
        let target_contract_id = team.setup.contract.id();
        let proposal = SubmittedCharter {
            target_contract_id,
            description: "We would keep the posts civil too".to_string(),
            reasons: vec![],
            moderators_share: None,
            reward_split: ModerationCharterRewardSplit {
                leader: 100,
                equal: 0,
                actions: 0,
            },
        };
        let (proposal, filing) = team
            .charter_document(
                rival,
                SUBMITTED_CHARTER_DOCUMENT_TYPE_NAME,
                proposal.to_document_properties(),
            )
            .await;
        team.process_and_commit(&filing);
        let charter = ElectedCharter {
            target_contract_id,
            submitted_charter_id: proposal.id(),
            members: vec![],
        };
        let (_, application) = team
            .charter_document(
                rival,
                ELECTED_CHARTER_DOCUMENT_TYPE_NAME,
                charter.to_document_properties(),
            )
            .await;
        let transaction = team.setup.platform.drive.grove.start_transaction();
        // The seated charter holds the unique index for the target: the contest was awarded.
        assert_paid_with_code(
            &team.setup.process(&application, &transaction),
            DUPLICATE_UNIQUE_INDEX,
        );

        // The seated charter keeps the seat.
        let mut reads =
            StateTransitionExecutionContext::default_for_platform_version(platform_version)
                .expect("expected an execution context");
        let seated = fetch_seated_moderation_charter(
            &team.setup.platform.drive,
            target_contract_id,
            &Default::default(),
            &mut reads,
            Some(&transaction),
            platform_version,
        )
        .expect("expected to read the seated charter")
        .expect("expected a seated charter");
        assert_eq!(
            seated.leader_id,
            team.leader.id(),
            "seat contestable: {challenge_cool_down:?}"
        );
    }
}

/// The leader adds members from the join requests and removes elected members: an added member
/// moderates and is protected until the leader deletes its addition, a removed elected member
/// no longer moderates and can be moderated until the leader deletes the removal, and the
/// leader and the active members can be neither banned nor have their documents deleted, while
/// the interim moderators and the owner lost that.
#[tokio::test]
async fn should_follow_additions_and_removals_and_protect_the_team() {
    let team = Team::new(InterimModerators::AppointedModerators(
        [THE_MODERATOR].into(),
    ))
    .await;
    let setup = &team.setup;
    let added = &team.joiners[0];
    let member_post = team.posted_by(&team.member).await;
    let added_post = team.posted_by(added).await;
    team.award();

    let transaction = setup.platform.drive.grove.start_transaction();
    // Before its addition the joiner does not moderate.
    let ban = setup.moderate(added, ban_action(setup.user.id())).await;
    assert_paid_with_code(
        &setup.process(&ban, &transaction),
        IDENTITY_NOT_CONTRACT_MODERATOR,
    );
    let (addition, adding) = team.added(added).await;
    assert_success(&setup.process(&adding, &transaction));
    let ban = setup.moderate(added, ban_action(setup.user.id())).await;
    assert_success(&setup.process(&ban, &transaction));

    // The leader, an elected member and an added member are protected, from a ban and from a
    // deletion of what they wrote.
    for (by, target) in [
        (&team.member, &team.leader),
        (&team.leader, &team.member),
        (&team.member, added),
    ] {
        let ban = setup.moderate(by, ban_action(target.id())).await;
        assert_paid_with_code(
            &setup.process(&ban, &transaction),
            CONTRACT_MODERATION_TARGET_NOT_ALLOWED,
        );
    }
    for post in [&member_post, &added_post] {
        let delete = setup
            .moderate(&team.leader, delete_action(POST, post.id()))
            .await;
        assert_paid_with_code(
            &setup.process(&delete, &transaction),
            CONTRACT_MODERATION_TARGET_NOT_ALLOWED,
        );
    }
    // The interim moderator and the owner (the declaration leaves it unprotected) are not.
    for target in [&setup.moderator, &setup.owner] {
        let warn = setup
            .moderate(&team.leader, warn_action(target.id(), "noted"))
            .await;
        assert_success(&setup.process(&warn, &transaction));
    }

    // Taken off, the added member by deleting its addition and the elected member by a
    // removal, they moderate no more, and are moderated.
    let taking_off = team
        .undoing(ADDED_MODERATOR_DOCUMENT_TYPE_NAME, addition)
        .await;
    assert_success(&setup.process(&taking_off, &transaction));
    let (removal, removing) = team.removed(&team.member).await;
    assert_success(&setup.process(&removing, &transaction));
    for removed in [added, &team.member] {
        let unban = setup.moderate(removed, unban_action(setup.user.id())).await;
        assert_paid_with_code(
            &setup.process(&unban, &transaction),
            IDENTITY_NOT_CONTRACT_MODERATOR,
        );
        let warn = setup
            .moderate(&team.leader, warn_action(removed.id(), "you left"))
            .await;
        assert_success(&setup.process(&warn, &transaction));
    }
    let delete = setup
        .moderate(&team.leader, delete_action(POST, added_post.id()))
        .await;
    assert_success(&setup.process(&delete, &transaction));

    // Deleting the removal puts the elected member back.
    let reinstating = team
        .undoing(REMOVED_MODERATOR_DOCUMENT_TYPE_NAME, removal)
        .await;
    assert_success(&setup.process(&reinstating, &transaction));
    let unban = setup
        .moderate(&team.member, unban_action(setup.user.id()))
        .await;
    assert_success(&setup.process(&unban, &transaction));
    let ban = setup
        .moderate(&team.leader, ban_action(team.member.id()))
        .await;
    assert_paid_with_code(
        &setup.process(&ban, &transaction),
        CONTRACT_MODERATION_TARGET_NOT_ALLOWED,
    );
}

/// A removal names an elected member of the charter and nobody else: an added member is taken
/// off by deleting its addition, and an identity never on the team has nothing to remove.
#[tokio::test]
async fn should_remove_only_an_elected_member() {
    let team = Team::new(InterimModerators::ContractOwner).await;
    let setup = &team.setup;
    team.award();

    let transaction = setup.platform.drive.grove.start_transaction();
    let added = &team.joiners[0];
    assert_success(&setup.process(&team.addition_of(added).await, &transaction));
    for never_elected in [added, &team.joiners[1]] {
        assert_paid_with_code(
            &setup.process(&team.removal_of(never_elected).await, &transaction),
            REFERENCED_ENTITY_NOT_FOUND,
        );
    }
    // The added member is still on the team.
    let ban = setup.moderate(added, ban_action(setup.user.id())).await;
    assert_success(&setup.process(&ban, &transaction));
}

/// The target's `maxAddedModerators` caps the additions a seated charter holds: the Nth passes,
/// the (N+1)th is refused, paid, and deleting an addition frees its slot.
#[tokio::test]
async fn should_cap_the_members_a_leader_adds_and_free_a_slot_when_an_addition_is_deleted() {
    let team = Team::new(InterimModerators::ContractOwner).await;
    let setup = &team.setup;
    team.award();

    let transaction = setup.platform.drive.grove.start_transaction();
    let [first, second, third] = &team.joiners;
    let (first_addition, adding_first) = team.added(first).await;
    assert_success(&setup.process(&adding_first, &transaction));
    assert_success(&setup.process(&team.addition_of(second).await, &transaction));
    let over_the_cap = team.addition_of(third).await;
    assert_paid_with_code(
        &setup.process(&over_the_cap, &transaction),
        MODERATION_CHARTER_ADDED_MODERATOR_LIMIT_REACHED,
    );
    // The third joiner did not make it onto the team.
    let ban = setup.moderate(third, ban_action(setup.user.id())).await;
    assert_paid_with_code(
        &setup.process(&ban, &transaction),
        IDENTITY_NOT_CONTRACT_MODERATOR,
    );

    let taking_off_first = team
        .undoing(ADDED_MODERATOR_DOCUMENT_TYPE_NAME, first_addition)
        .await;
    assert_success(&setup.process(&taking_off_first, &transaction));
    assert_success(&setup.process(&team.addition_of(third).await, &transaction));
    let ban = setup.moderate(third, ban_action(setup.user.id())).await;
    assert_success(&setup.process(&ban, &transaction));
    // The team is full again.
    assert_paid_with_code(
        &setup.process(&team.addition_of(first).await, &transaction),
        MODERATION_CHARTER_ADDED_MODERATOR_LIMIT_REACHED,
    );
}

/// A seated team acts with the declaration's abilities and no others; the interim moderators
/// before it acted with every ability the contract backs.
#[tokio::test]
async fn should_refuse_a_seated_team_an_ability_the_declaration_does_not_give_it() {
    let team =
        Team::with_abilities(InterimModerators::ContractOwner, &[ModerationAbility::Ban]).await;
    let setup = &team.setup;
    let post = team.posted_by(&setup.user).await;

    // The owner in the interim suspends and warns, which the declaration gives no team.
    let transaction = setup.platform.drive.grove.start_transaction();
    let suspend = setup
        .moderate(&setup.owner, suspend_action(setup.stranger.id(), LATER))
        .await;
    assert_success(&setup.process(&suspend, &transaction));
    drop(transaction);

    team.award();
    let transaction = setup.platform.drive.grove.start_transaction();
    let ban = setup
        .moderate(&team.leader, ban_action(setup.user.id()))
        .await;
    assert_success(&setup.process(&ban, &transaction));
    for action in [
        suspend_action(setup.stranger.id(), LATER),
        warn_action(setup.stranger.id(), "no"),
        clear_warnings_action(setup.stranger.id()),
        delete_action(POST, post.id()),
        // Refused before the bytes are even decoded.
        restore_action(POST, vec![]),
    ] {
        let refused = setup.moderate(&team.member, action.clone()).await;
        assert_paid_with_code(
            &setup.process(&refused, &transaction),
            CONTRACT_MODERATION_ABILITY_NOT_GRANTED,
        );
    }
}

/// A `notYetUsable` interim blocks the moderated type until the contest for the seat is
/// awarded, and not a block longer.
#[tokio::test]
async fn should_end_the_interim_block_once_a_charter_is_seated() {
    let team = Team::new(InterimModerators::NotYetUsable).await;
    let setup = &team.setup;

    let (_, blocked) = team
        .post_by(&setup.user, Some(agreeing_to(MODERATORS_PART)))
        .await;
    let transaction = setup.platform.drive.grove.start_transaction();
    assert_paid_with_code(
        &setup.process(&blocked, &transaction),
        CONTRACT_MODERATED_DOCUMENT_TYPE_NOT_YET_USABLE,
    );
    drop(transaction);

    team.award();
    assert!(setup.check_tx(&blocked).is_empty());
    let transaction = setup.platform.drive.grove.start_transaction();
    assert_success(&setup.process(&blocked, &transaction));
    // Nobody moderated in the interim; the seated team does now.
    let ban = setup
        .moderate(&team.leader, ban_action(setup.stranger.id()))
        .await;
    assert_success(&setup.process(&ban, &transaction));
}

/// An action agreeing to the declared moderators part pays it, before and after a charter is
/// seated, and reads no charter. One agreeing to the seated charter's share of it pays the
/// share, into the pot, and the gas of the charter lookup and the proposal fetch on top: from
/// the same state, it costs exactly those two reads more than the one paying the declared part.
#[tokio::test]
async fn should_charge_the_declared_moderators_part_without_a_read_and_a_discount_with_one() {
    let team = Team::new(InterimModerators::ContractOwner).await;
    let setup = &team.setup;
    let platform_version = PlatformVersion::latest();
    // The same post, agreeing to the declared part or to the charter's share.
    let (_, posts) = team
        .one_document_by(
            &setup.user,
            POST,
            &[
                Some(agreeing_to(MODERATORS_PART)),
                Some(agreeing_to(discounted())),
            ],
        )
        .await;
    let [full, at_the_share] = [&posts[0], &posts[1]];

    let transaction = setup.platform.drive.grove.start_transaction();
    let pot_before = team.moderators_pot(&transaction);
    assert_success(&setup.process(full, &transaction));
    assert_eq!(
        team.moderators_pot(&transaction) - pot_before,
        MODERATORS_PART
    );
    drop(transaction);

    team.award();

    // The declared part, with a charter seated: charged in full.
    let transaction = setup.platform.drive.grove.start_transaction();
    let seated_full = setup.process(full, &transaction);
    assert_success(&seated_full);
    assert_eq!(
        team.moderators_pot(&transaction) - pot_before,
        MODERATORS_PART
    );
    drop(transaction);

    // The charter's share, from the same state: charged the share.
    assert_eq!(discounted(), 60_000_000);
    let transaction = setup.platform.drive.grove.start_transaction();
    let user_before = setup.balance(setup.user.id(), Some(&transaction));
    let seated_discount = setup.process(at_the_share, &transaction);
    assert_success(&seated_discount);
    assert_eq!(team.moderators_pot(&transaction) - pot_before, discounted());
    assert_eq!(
        user_before - setup.balance(setup.user.id(), Some(&transaction)),
        gas_of(&seated_discount) + OWNER_PART + discounted()
    );
    drop(transaction);

    // What the two reads cost, read on their own from the same state, in a transaction as a
    // block reads them.
    let mut reads = StateTransitionExecutionContext::default_for_platform_version(platform_version)
        .expect("expected an execution context");
    let transaction = setup.platform.drive.grove.start_transaction();
    let charter = fetch_seated_moderation_charter(
        &setup.platform.drive,
        setup.contract.id(),
        &Default::default(),
        &mut reads,
        Some(&transaction),
        platform_version,
    )
    .expect("expected to read the seated charter")
    .expect("expected a seated charter");
    assert_eq!(charter.leader_id, team.leader.id());
    assert_eq!(
        charter
            .fetch_moderators_share(
                &setup.platform.drive,
                &Default::default(),
                &mut reads,
                Some(&transaction),
                platform_version,
            )
            .expect("expected to read the share"),
        MODERATORS_SHARE
    );
    let mut read_fee = FeeResult::default();
    ValidationOperation::add_many_to_fee_result(
        reads.operations_slice(),
        &mut read_fee,
        platform_version,
    )
    .expect("expected to price the reads");
    assert!(read_fee.processing_fee > 0);
    let (full_fees, discount_fees) = (fees_of(&seated_full), fees_of(&seated_discount));
    assert_eq!(discount_fees.storage_fee, full_fees.storage_fee);
    assert_eq!(
        discount_fees.processing_fee - full_fees.processing_fee,
        read_fee.processing_fee,
        "the declared part reads nothing; the share reads the charter and its proposal"
    );
}

/// Less than the declared moderators part is the seated charter's share or nothing: a discount
/// before a charter is seated, one below the share and one above it are refused, paid and
/// charged no action fee, in a block and on every recheck of the mempool, which agrees with
/// the block on the share too.
#[tokio::test]
async fn should_refuse_any_discount_but_the_seated_charters_share_and_agree_on_recheck() {
    let team = Team::new(InterimModerators::ContractOwner).await;
    let setup = &team.setup;
    let (_, at_the_share) = team
        .post_by(&setup.user, Some(agreeing_to(discounted())))
        .await;
    let refused_without_a_fee = |transition: &StateTransition| {
        let transaction = setup.platform.drive.grove.start_transaction();
        let pot_before = team.moderators_pot(&transaction);
        assert_paid_with_code(
            &setup.process(transition, &transaction),
            DOCUMENT_ACTION_FEE_MODERATORS_SHARE_MISMATCH,
        );
        assert_eq!(team.moderators_pot(&transaction), pot_before);
    };

    // No charter is seated: no discount.
    assert_eq!(
        team.check_tx_codes(&at_the_share, CheckTxLevel::FirstTimeCheck),
        vec![DOCUMENT_ACTION_FEE_MODERATORS_SHARE_MISMATCH]
    );
    assert_eq!(
        team.check_tx_codes(&at_the_share, Recheck),
        vec![DOCUMENT_ACTION_FEE_MODERATORS_SHARE_MISMATCH]
    );
    refused_without_a_fee(&at_the_share);

    team.award();
    assert_eq!(
        team.check_tx_codes(&at_the_share, CheckTxLevel::FirstTimeCheck),
        Vec::<u32>::new()
    );
    assert_eq!(
        team.check_tx_codes(&at_the_share, Recheck),
        Vec::<u32>::new()
    );
    for moderators in [discounted() - 1, discounted() + 1, 0] {
        let (_, other_discount) = team
            .post_by(&setup.user, Some(agreeing_to(moderators)))
            .await;
        assert_eq!(
            team.check_tx_codes(&other_discount, Recheck),
            vec![DOCUMENT_ACTION_FEE_MODERATORS_SHARE_MISMATCH],
            "{moderators}"
        );
        refused_without_a_fee(&other_discount);
    }
}

/// The interim team claims the moderators pot until a charter is seated, and not after: the pot
/// carries over to the seated team, unsettled.
#[tokio::test]
async fn should_stop_the_interim_team_claiming_the_moderators_pot_once_a_charter_is_seated() {
    let team = Team::new(InterimModerators::ContractOwner).await;
    let setup = &team.setup;
    team.posted_by(&setup.user).await;
    let claim = || async {
        ContractFeeClaimTransition::try_from_identity_with_signer(
            &setup.owner.identity,
            &CRITICAL_KEY_ID,
            setup.contract.id(),
            ContractFeePot::Moderators,
            setup.owner.contract_nonce(),
            0,
            &setup.owner.signer,
            PlatformVersion::latest(),
            None,
        )
        .await
        .expect("expected to build the claim")
    };

    let transaction = setup.platform.drive.grove.start_transaction();
    assert_success(&setup.process(&claim().await, &transaction));
    drop(transaction);

    team.award();
    let transaction = setup.platform.drive.grove.start_transaction();
    assert_paid_with_code(
        &setup.process(&claim().await, &transaction),
        CONTRACT_FEE_CLAIM_NOT_ALLOWED,
    );
    assert_eq!(team.moderators_pot(&transaction), MODERATORS_PART);
}

/// With `ownerProtected`, a seated team protects the contract owner as it protects its own
/// members, though the owner is not one: it can not be banned and its documents can not be
/// deleted, and it does not moderate.
#[tokio::test]
async fn should_protect_the_owner_from_a_seated_team_when_the_declaration_says_so() {
    let team = Team::build(
        InterimModerators::ContractOwner,
        &ALL_ABILITIES,
        true,
        Some(CHALLENGE_COOL_DOWN),
        vec![LISTED_REASON],
    )
    .await;
    let setup = &team.setup;
    let owner_post = team.posted_by(&setup.owner).await;
    team.award();

    let transaction = setup.platform.drive.grove.start_transaction();
    let ban = setup
        .moderate(&team.leader, ban_action(setup.owner.id()))
        .await;
    assert_paid_with_code(
        &setup.process(&ban, &transaction),
        CONTRACT_MODERATION_TARGET_NOT_ALLOWED,
    );
    let delete = setup
        .moderate(&team.member, delete_action(POST, owner_post.id()))
        .await;
    assert_paid_with_code(
        &setup.process(&delete, &transaction),
        CONTRACT_MODERATION_TARGET_NOT_ALLOWED,
    );
    let ban = setup
        .moderate(&setup.owner, ban_action(setup.user.id()))
        .await;
    assert_paid_with_code(
        &setup.process(&ban, &transaction),
        IDENTITY_NOT_CONTRACT_MODERATOR,
    );
}

/// A discount is the seated charter's to give on the types the contract moderates, and only
/// there: on a type it does not moderate an agreement to less is the plain mismatch. On a fee
/// priced by the fee multiplier the share applies to the declared part, and the epoch's
/// multiplier to the share.
#[tokio::test]
async fn should_discount_only_a_moderated_type_and_scale_the_share_by_the_fee_multiplier() {
    let team = Team::new(InterimModerators::ContractOwner).await;
    let setup = &team.setup;
    team.award();

    let (_, note) = team
        .one_document_by(&setup.user, NOTE, &[Some(agreeing_to(discounted()))])
        .await;
    let transaction = setup.platform.drive.grove.start_transaction();
    assert_paid_with_code(
        &setup.process(&note[0], &transaction),
        DOCUMENT_ACTION_FEE_AGREEMENT_MISMATCH,
    );
    drop(transaction);

    // The epoch's fee multiplier is 1.5.
    let mut batch = GroveDbOpBatch::new();
    batch.push(
        Epoch::new(0)
            .expect("expected epoch 0")
            .update_fee_multiplier_operation(1_500),
    );
    setup
        .platform
        .drive
        .grove_apply_batch(batch, false, None, &PlatformVersion::latest().drive)
        .expect("expected to set the fee multiplier of epoch 0");
    let agreement = DocumentActionFeeAgreement::for_declared_fee(
        ActionFeePricing::FeeMultiplier,
        DocumentActionFee {
            owner: OWNER_PART,
            moderators: discounted(),
        },
        AgreedFeeMultiplier {
            known_permille: 1_500,
            increase_tolerance_percent: 0,
        },
    );
    let (_, reply) = team
        .one_document_by(&setup.user, REPLY, &[Some(agreement)])
        .await;
    let transaction = setup.platform.drive.grove.start_transaction();
    let pot_before = team.moderators_pot(&transaction);
    assert_success(&setup.process(&reply[0], &transaction));
    assert_eq!(
        team.moderators_pot(&transaction) - pot_before,
        discounted() * 3 / 2
    );
}

/// The document transition a single-transition batch carries
fn document_transition_of(transition: &StateTransition) -> DocumentTransition {
    let StateTransition::Batch(batch) = transition else {
        panic!("expected a batch");
    };
    match batch.transitions_iter().next() {
        Some(BatchedTransitionRef::Document(document_transition)) => document_transition.clone(),
        _ => panic!("expected a document transition"),
    }
}

/// Two additions in one batch are counted together: with one slot left, the first is accepted
/// and the second refused, though neither is in state when the other is judged. Driven through
/// the transformer and the batch state validation directly: `max_transitions_in_documents_batch`
/// is 1 at every protocol version, so no such batch reaches them from the network today.
#[tokio::test]
async fn should_count_the_additions_an_earlier_create_of_the_same_batch_was_accepted_for() {
    let team = Team::new(InterimModerators::ContractOwner).await;
    let setup = &team.setup;
    team.award();
    let [first, second, third] = &team.joiners;
    team.process_and_commit(&team.addition_of(first).await);

    let batch: StateTransition = BatchTransition::from(BatchTransitionV0 {
        owner_id: team.leader.id(),
        transitions: vec![
            document_transition_of(&team.addition_of(second).await),
            document_transition_of(&team.addition_of(third).await),
        ],
        user_fee_increase: 0,
        signature_public_key_id: 0,
        signature: Default::default(),
    })
    .into();

    let platform_version = PlatformVersion::latest();
    let state = setup.platform.state.load();
    let platform_ref = PlatformRef {
        drive: &setup.platform.drive,
        state: &state,
        config: &setup.platform.config,
        core_rpc: &setup.platform.core_rpc,
    };
    let mut execution_context =
        StateTransitionExecutionContext::default_for_platform_version(platform_version)
            .expect("expected an execution context");
    let transformed = batch
        .transform_into_action(
            &platform_ref,
            &BlockInfo::default(),
            &None,
            ValidationMode::Validator,
            &mut execution_context,
            None,
        )
        .expect("expected to transform the batch");
    assert!(transformed.errors.is_empty(), "{:?}", transformed.errors);
    let validated = batch
        .validate_state(
            transformed.data,
            &platform_ref,
            ValidationMode::Validator,
            &BlockInfo::default(),
            &mut execution_context,
            None,
        )
        .expect("expected to validate the batch against state");
    assert_eq!(
        validated
            .errors
            .iter()
            .map(|error| error.code())
            .collect::<Vec<_>>(),
        vec![MODERATION_CHARTER_ADDED_MODERATOR_LIMIT_REACHED]
    );
    let Some(StateTransitionAction::BatchAction(action)) = validated.data else {
        panic!("expected a batch action back from state validation");
    };
    let survived_as_creates: Vec<bool> = action
        .transitions()
        .iter()
        .map(|transition| {
            matches!(
                transition,
                BatchedTransitionAction::DocumentAction(DocumentTransitionAction::CreateAction(_))
            )
        })
        .collect();
    assert_eq!(survived_as_creates, vec![true, false]);
}

/// The addition cap hooks into the batch's state validation that protocol version 13 runs too,
/// but no batch of that version reaches it: the moderation charters contract is not in state
/// before protocol version 14, so an `addedModerator` create is refused when its contract is
/// fetched, before state validation, as any document create on a missing contract always was.
#[tokio::test]
async fn should_refuse_an_addition_before_protocol_version_14_before_the_cap_is_judged() {
    let platform_version = PlatformVersion::get(13).expect("protocol version 13");
    let setup = Setup::new_at(None, platform_version).await;
    let charters = setup
        .platform
        .drive
        .cache
        .system_data_contracts
        .load_moderation_charters(PlatformVersion::latest())
        .expect("expected the moderation charters contract");
    let document_type = charters
        .document_type_for_name(ADDED_MODERATOR_DOCUMENT_TYPE_NAME)
        .expect("expected the addedModerator type");
    let nonce = setup.owner.contract_nonce();
    let (entropy, document) = {
        let mut rng = setup.rng.borrow_mut();
        let entropy = Bytes32::random_with_rng(&mut rng);
        let mut document = document_type
            .random_document_with_identifier_and_entropy(
                &mut rng,
                setup.owner.id(),
                entropy,
                DocumentFieldFillType::DoNotFillIfNotRequired,
                DocumentFieldFillSize::MinDocumentFillSize,
                PlatformVersion::latest(),
            )
            .expect("expected a random document");
        document.set_properties(BTreeMap::from([
            ("electedCharterId".to_string(), Value::Identifier([1; 32])),
            ("submittedCharterId".to_string(), Value::Identifier([2; 32])),
            (
                "memberId".to_string(),
                Value::Identifier(setup.user.id().to_buffer()),
            ),
        ]));
        document
            .set_id_for_creation(document_type, &entropy.0, nonce, platform_version)
            .expect("expected to set the document id");
        (entropy, document)
    };
    let addition = BatchTransition::new_document_creation_transition_from_document(
        document,
        document_type,
        entropy.0,
        &setup.owner.key,
        nonce,
        0,
        None,
        &setup.owner.signer,
        platform_version,
        None,
    )
    .await
    .expect("expected to build the addition");

    let transaction = setup.platform.drive.grove.start_transaction();
    let execution = setup.process(&addition, &transaction);
    let code = match &execution {
        StateTransitionExecutionResult::PaidConsensusError { error, .. }
        | StateTransitionExecutionResult::UnpaidConsensusError(error) => error.code(),
        other => panic!("expected the addition to be refused, got {other:?}"),
    };
    assert_eq!(code, DATA_CONTRACT_NOT_PRESENT);
}
