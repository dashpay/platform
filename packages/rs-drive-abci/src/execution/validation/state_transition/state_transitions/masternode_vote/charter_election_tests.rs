//! Moderation elections (protocol version 14): an `electedCharter` contest of the moderation
//! charters contract runs on the join window and the vote window its target contract declares,
//! and every application prefunds the masternode votes with the moderation fund (0.5 Dash), what
//! the votes leave of it released as processing fees at clean-up. Every other contest, DPNS
//! included, keeps the generic windows and fund.

use crate::execution::validation::state_transition::state_transitions::tests::{
    create_dpns_identity_name_contest, setup_identity, setup_masternode_voting_identity,
};
use crate::platform_types::platform_state::PlatformStateV0Methods;
use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult;
use crate::rpc::core::MockCoreRPCLike;
use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
use dpp::block::block_info::BlockInfo;
use dpp::block::epoch::Epoch;
use dpp::block::extended_block_info::v0::ExtendedBlockInfoV0;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::ConsensusError;
use dpp::dash_to_credits;
use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
use dpp::data_contract::config::moderation::{
    ContractModerationConfig, ContractModerators, ElectedModerators, InterimModerators,
    ModerationAbility,
};
use dpp::data_contract::DataContract;
use dpp::document::{Document, DocumentV0, DocumentV0Getters};
use dpp::fee::fee_result::FeeResult;
use dpp::fee::Credits;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::{Identity, IdentityPublicKey, TimestampMillis};
use dpp::moderation_charter::{
    property_names, ELECTED_CHARTER_DOCUMENT_TYPE_NAME, MODERATION_CHARTERS_CONTRACT_ID,
    SUBMITTED_CHARTER_DOCUMENT_TYPE_NAME,
};
use dpp::platform_value::{Bytes32, Identifier, Value};
use dpp::prelude::IdentityNonce;
use dpp::serialization::PlatformSerializable;
use dpp::state_transition::batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
use dpp::state_transition::batch_transition::BatchTransition;
use dpp::state_transition::masternode_vote_transition::methods::MasternodeVoteTransitionMethodsV0;
use dpp::state_transition::masternode_vote_transition::MasternodeVoteTransition;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use dpp::voting::vote_info_storage::contested_document_vote_poll_stored_info::{
    ContestedDocumentVotePollStatus, ContestedDocumentVotePollStoredInfoV0Getters,
};
use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use dpp::voting::vote_polls::VotePoll;
use dpp::voting::votes::resource_vote::v0::ResourceVoteV0;
use dpp::voting::votes::resource_vote::ResourceVote;
use dpp::voting::votes::Vote;
use drive::drive::contract::paths::contract_root_path;
use drive::drive::document::ContestWindows;
use drive::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::resolve::ContestedDocumentResourceVotePollResolver;
use drive::error::Error as DriveError;
use drive::grovedb::Error as GroveError;
use drive::query::VotePollsByEndDateDriveQuery;
use drive::util::test_helpers::setup_contract;
use platform_version::version::PlatformVersion;
use rand::prelude::StdRng;
use rand::{Rng, SeedableRng};
use simple_signer::signer::SimpleSigner;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

/// The contract whose moderation seat is contended for: any contract will do, with an elected
/// moderation declaration written into its config.
const TARGET_CONTRACT: &str =
    "tests/supporting_files/contract/family/family-contract-countable.json";

const ONE_DAY: u32 = 86_400;
const ONE_WEEK: u32 = 604_800;
const FOUR_WEEKS: u32 = 2_419_200;
const DAY_MS: TimestampMillis = 86_400_000;
const TWO_HOURS_MS: TimestampMillis = 7_200_000;

type IdentityInfo = (Identity, SimpleSigner, IdentityPublicKey);

/// An identity writing to the charter contract, with the identity contract nonce its next
/// transition uses.
struct Applicant {
    info: IdentityInfo,
    next_nonce: IdentityNonce,
}

impl Applicant {
    fn id(&self) -> Identifier {
        self.info.0.id()
    }
}

fn block(time_ms: TimestampMillis, height: u64) -> BlockInfo {
    BlockInfo {
        time_ms,
        height,
        core_height: 42,
        epoch: Default::default(),
    }
}

fn setup() -> (
    TempPlatform<MockCoreRPCLike>,
    &'static PlatformVersion,
    Arc<DataContract>,
    StdRng,
) {
    let platform_version = PlatformVersion::latest();
    let platform = TestPlatformBuilder::new()
        .with_latest_protocol_version()
        .build_with_mock_rpc()
        .set_genesis_state();
    let charters = platform
        .drive
        .cache
        .system_data_contracts
        .load_moderation_charters(platform_version)
        .expect("expected the moderation charters contract at genesis");
    (
        platform,
        platform_version,
        charters,
        StdRng::seed_from_u64(0xC4A2_7E25),
    )
}

fn elected_moderation(join_window: u32, vote_window: u32) -> ContractModerationConfig {
    ContractModerationConfig {
        banlist: true,
        suspensions: false,
        warnings: false,
        moderators: ContractModerators::Elected(Box::new(ElectedModerators {
            join_window,
            vote_window,
            challenge_cool_down: Some(1_209_600),
            election_delay: None,
            max_added_moderators: 0,
            moderated_document_types: BTreeMap::from([(
                "person".to_string(),
                BTreeSet::from([ModerationAbility::Ban]),
            )]),
            interim: InterimModerators::ContractOwner,
            owner_protected: false,
        })),
    }
}

/// Writes the target contract `[id_byte; 32]` to state directly, the way the fixtures are,
/// with `moderation` in its config.
fn write_target(
    platform: &TempPlatform<MockCoreRPCLike>,
    id_byte: u8,
    moderation: Option<ContractModerationConfig>,
    platform_version: &PlatformVersion,
) -> Identifier {
    let contract = setup_contract(
        &platform.drive,
        TARGET_CONTRACT,
        Some([id_byte; 32]),
        None,
        Some(|contract: &mut DataContract| {
            contract.set_config(contract.config().clone().with_moderation(moderation));
        }),
        None,
        Some(platform_version),
    );
    // A rewritten target must be read again, as a node that never cached it would
    platform.drive.cache.data_contracts.clear();
    contract.id()
}

/// A target contract declaring elected moderation with these windows, in seconds.
fn elected_target(
    platform: &TempPlatform<MockCoreRPCLike>,
    id_byte: u8,
    join_window: u32,
    vote_window: u32,
    platform_version: &PlatformVersion,
) -> Identifier {
    write_target(
        platform,
        id_byte,
        Some(elected_moderation(join_window, vote_window)),
        platform_version,
    )
}

fn applicant(platform: &mut TempPlatform<MockCoreRPCLike>, rng: &mut StdRng) -> Applicant {
    Applicant {
        info: setup_identity(platform, rng.gen(), dash_to_credits!(3.0)),
        next_nonce: 1,
    }
}

/// The contest for the moderation seat of `target`.
fn charter_poll(target: Identifier) -> ContestedDocumentResourceVotePoll {
    ContestedDocumentResourceVotePoll {
        contract_id: MODERATION_CHARTERS_CONTRACT_ID,
        document_type_name: ELECTED_CHARTER_DOCUMENT_TYPE_NAME.to_string(),
        index_name: "byTargetContract".to_string(),
        index_values: vec![Value::Identifier(target.to_buffer())],
    }
}

/// A serialized create of a `document_type_name` document of the charter contract holding
/// `properties`, signed by `applicant`, and the id of the document it creates.
async fn create_transition(
    charters: &DataContract,
    applicant: &mut Applicant,
    document_type_name: &str,
    properties: BTreeMap<String, Value>,
    rng: &mut StdRng,
    platform_version: &PlatformVersion,
) -> (Vec<u8>, Identifier) {
    let document_type = charters
        .document_type_for_name(document_type_name)
        .expect("expected the charter document type");
    let entropy = Bytes32::random_with_rng(rng);
    let nonce = applicant.next_nonce;
    applicant.next_nonce += 1;
    let mut document: Document = DocumentV0 {
        owner_id: applicant.id(),
        properties,
        ..Default::default()
    }
    .into();
    document
        .set_id_for_creation(document_type, &entropy.0, nonce, platform_version)
        .expect("expected to set the document id");
    let id = document.id();
    let (_, signer, key) = &applicant.info;
    let transition = BatchTransition::new_document_creation_transition_from_document(
        document,
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
    .expect("expected to create the batch transition");
    (
        transition
            .serialize_to_bytes()
            .expect("expected to serialize the batch transition"),
        id,
    )
}

/// Processes one transition in a block at `time_ms` and returns its execution result.
fn process(
    platform: &TempPlatform<MockCoreRPCLike>,
    transition: Vec<u8>,
    time_ms: TimestampMillis,
    platform_version: &PlatformVersion,
) -> StateTransitionExecutionResult {
    let platform_state = platform.state.load();
    let transaction = platform.drive.grove.start_transaction();
    let processing_result = platform
        .platform
        .process_raw_state_transitions(
            &[transition],
            &platform_state,
            &block(time_ms, 1),
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

/// Processes one transition that must pass, and returns the fee it paid.
fn process_valid(
    platform: &TempPlatform<MockCoreRPCLike>,
    transition: Vec<u8>,
    time_ms: TimestampMillis,
    platform_version: &PlatformVersion,
) -> FeeResult {
    match process(platform, transition, time_ms, platform_version) {
        StateTransitionExecutionResult::SuccessfulExecution { fee_result, .. } => fee_result,
        other => panic!("expected the transition to pass, got {other:?}"),
    }
}

/// Processes one transition that must be refused, paid, and returns why.
fn process_refused(
    platform: &TempPlatform<MockCoreRPCLike>,
    transition: Vec<u8>,
    time_ms: TimestampMillis,
    platform_version: &PlatformVersion,
) -> ConsensusError {
    match process(platform, transition, time_ms, platform_version) {
        StateTransitionExecutionResult::PaidConsensusError { error, .. } => error,
        other => panic!("expected a paid refusal, got {other:?}"),
    }
}

/// `applicant`'s proposal for `target`, filed at `time_ms`; returns its id.
async fn propose(
    platform: &TempPlatform<MockCoreRPCLike>,
    charters: &DataContract,
    applicant: &mut Applicant,
    target: Identifier,
    time_ms: TimestampMillis,
    rng: &mut StdRng,
    platform_version: &PlatformVersion,
) -> Identifier {
    let (proposal, proposal_id) = create_transition(
        charters,
        applicant,
        SUBMITTED_CHARTER_DOCUMENT_TYPE_NAME,
        BTreeMap::from([
            (
                property_names::TARGET_CONTRACT_ID.to_string(),
                Value::Identifier(target.to_buffer()),
            ),
            (
                property_names::DESCRIPTION.to_string(),
                Value::Text("Keeps the family tree civil".to_string()),
            ),
            (property_names::REASONS.to_string(), Value::Array(vec![])),
            (
                property_names::REWARD_SPLIT.to_string(),
                Value::Map(vec![
                    (
                        Value::Text(property_names::REWARD_SPLIT_LEADER.to_string()),
                        Value::U8(10),
                    ),
                    (
                        Value::Text(property_names::REWARD_SPLIT_EQUAL.to_string()),
                        Value::U8(40),
                    ),
                    (
                        Value::Text(property_names::REWARD_SPLIT_ACTIONS.to_string()),
                        Value::U8(50),
                    ),
                ]),
            ),
        ]),
        rng,
        platform_version,
    )
    .await;
    process_valid(platform, proposal, time_ms, platform_version);
    proposal_id
}

/// The serialized application of `applicant` for `target` on its proposal `proposal_id`: an
/// `electedCharter` create with no members, which opens or joins the contest.
async fn application(
    charters: &DataContract,
    applicant: &mut Applicant,
    target: Identifier,
    proposal_id: Identifier,
    rng: &mut StdRng,
    platform_version: &PlatformVersion,
) -> Vec<u8> {
    application_naming_the_target_as(
        charters,
        applicant,
        Value::Identifier(target.to_buffer()),
        proposal_id,
        rng,
        platform_version,
    )
    .await
}

/// [`application`] with the target written as `target`: an identifier property is accepted as
/// an identifier, as bytes or as an array of byte values.
async fn application_naming_the_target_as(
    charters: &DataContract,
    applicant: &mut Applicant,
    target: Value,
    proposal_id: Identifier,
    rng: &mut StdRng,
    platform_version: &PlatformVersion,
) -> Vec<u8> {
    create_transition(
        charters,
        applicant,
        ELECTED_CHARTER_DOCUMENT_TYPE_NAME,
        BTreeMap::from([
            (property_names::TARGET_CONTRACT_ID.to_string(), target),
            (
                property_names::SUBMITTED_CHARTER_ID.to_string(),
                Value::Identifier(proposal_id.to_buffer()),
            ),
            (property_names::MEMBERS.to_string(), Value::Array(vec![])),
        ]),
        rng,
        platform_version,
    )
    .await
    .0
}

/// `applicant` files a proposal for `target` at `time_ms` and applies with it a second later.
/// Returns the application's block time, when the contest starts or is joined, and its fee.
async fn apply(
    platform: &TempPlatform<MockCoreRPCLike>,
    charters: &DataContract,
    applicant: &mut Applicant,
    target: Identifier,
    time_ms: TimestampMillis,
    rng: &mut StdRng,
    platform_version: &PlatformVersion,
) -> (TimestampMillis, FeeResult) {
    let proposal_id = propose(
        platform,
        charters,
        applicant,
        target,
        time_ms,
        rng,
        platform_version,
    )
    .await;
    let application = application(
        charters,
        applicant,
        target,
        proposal_id,
        rng,
        platform_version,
    )
    .await;
    let fee = process_valid(platform, application, time_ms + 1000, platform_version);
    (time_ms + 1000, fee)
}

/// A masternode votes `choice` in `poll` at `time_ms`.
async fn vote(
    platform: &mut TempPlatform<MockCoreRPCLike>,
    poll: &ContestedDocumentResourceVotePoll,
    choice: ResourceVoteChoice,
    masternode_seed: u64,
    time_ms: TimestampMillis,
    platform_version: &PlatformVersion,
) {
    let (pro_tx_hash, _, signer, voting_key) =
        setup_masternode_voting_identity(platform, masternode_seed, platform_version);
    let vote = Vote::ResourceVote(ResourceVote::V0(ResourceVoteV0 {
        vote_poll: VotePoll::ContestedDocumentResourceVotePoll(poll.clone()),
        resource_vote_choice: choice,
    }));
    let transition = MasternodeVoteTransition::try_from_vote_with_signer(
        vote,
        &signer,
        pro_tx_hash,
        &voting_key,
        1,
        platform_version,
        None,
    )
    .await
    .expect("expected to make the vote")
    .serialize_to_bytes()
    .expect("expected to serialize the vote");
    process_valid(platform, transition, time_ms, platform_version);
}

fn end_dates(
    platform: &TempPlatform<MockCoreRPCLike>,
    platform_version: &PlatformVersion,
) -> Vec<(TimestampMillis, VotePoll)> {
    VotePollsByEndDateDriveQuery {
        start_time: None,
        end_time: None,
        limit: None,
        offset: None,
        order_ascending: true,
    }
    .execute_no_proof(&platform.drive, None, &mut vec![], platform_version)
    .expect("expected the end date entries")
    .into_iter()
    .flat_map(|(time, polls)| polls.into_iter().map(move |poll| (time, poll)))
    .collect()
}

/// Ends every poll due at `time_ms`, as the block at that time would.
fn end_polls_at(
    platform: &TempPlatform<MockCoreRPCLike>,
    time_ms: TimestampMillis,
    height: u64,
    platform_version: &PlatformVersion,
) {
    let mut platform_state = (**platform.state.load()).clone();
    let block_info = block(time_ms, height);
    platform_state.set_last_committed_block_info(Some(
        ExtendedBlockInfoV0 {
            basic_info: block_info,
            app_hash: platform
                .drive
                .grove
                .root_hash(None, &platform_version.drive.grove_version)
                .unwrap()
                .unwrap(),
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
        .expect("ending the polls due must never fail");
    platform
        .drive
        .grove
        .commit_transaction(transaction)
        .unwrap()
        .expect("expected to commit the transaction");
}

fn status(
    platform: &TempPlatform<MockCoreRPCLike>,
    poll: &ContestedDocumentResourceVotePoll,
    platform_version: &PlatformVersion,
) -> ContestedDocumentVotePollStatus {
    let resolved = poll
        .resolve(&platform.drive, None, platform_version)
        .expect("expected to resolve the contest");
    let (_, stored_info) = platform
        .drive
        .fetch_contested_document_vote_poll_stored_info(&resolved, None, None, platform_version)
        .expect("expected to read the contest");
    stored_info
        .expect("expected the contest to exist")
        .vote_poll_status()
}

fn prefunded_balance(
    platform: &TempPlatform<MockCoreRPCLike>,
    poll: &ContestedDocumentResourceVotePoll,
    platform_version: &PlatformVersion,
) -> Credits {
    platform
        .drive
        .fetch_prefunded_specialized_balance(
            poll.specialized_balance_id()
                .expect("expected the balance id")
                .to_buffer(),
            None,
            platform_version,
        )
        .expect("expected to read the balance")
        .unwrap_or_default()
}

fn balance_of(
    platform: &TempPlatform<MockCoreRPCLike>,
    identity_id: Identifier,
    platform_version: &PlatformVersion,
) -> Credits {
    platform
        .drive
        .fetch_identity_balance(identity_id.to_buffer(), None, platform_version)
        .expect("expected to read the balance")
        .expect("expected the identity")
}

/// The processing credits epoch 0 will distribute; the item is written by the first credit, so
/// before any it is missing, which is none.
fn processing_credits(
    platform: &TempPlatform<MockCoreRPCLike>,
    platform_version: &PlatformVersion,
) -> Credits {
    match platform
        .drive
        .get_epoch_processing_credits_for_distribution(
            &Epoch::new(0).expect("epoch"),
            None,
            platform_version,
        ) {
        Ok(credits) => credits,
        Err(DriveError::GroveDB(error)) if matches!(*error, GroveError::PathKeyNotFound(_)) => 0,
        Err(error) => panic!("expected the epoch's processing credits: {error:?}"),
    }
}

#[tokio::test]
async fn should_award_a_single_applicant_when_the_target_join_window_closes() {
    let (mut platform, platform_version, charters, mut rng) = setup();
    let target = elected_target(&platform, 0xA1, ONE_DAY, ONE_WEEK, platform_version);
    let mut alice = applicant(&mut platform, &mut rng);

    let (start, _) = apply(
        &platform,
        &charters,
        &mut alice,
        target,
        10_000,
        &mut rng,
        platform_version,
    )
    .await;

    let poll = charter_poll(target);
    assert_eq!(
        end_dates(&platform, platform_version),
        vec![(
            start + DAY_MS,
            VotePoll::ContestedDocumentResourceVotePoll(poll.clone())
        )],
        "a single applicant's election ends with the target's one-day join window"
    );

    end_polls_at(&platform, start + DAY_MS - 1, 10, platform_version);
    assert!(matches!(
        status(&platform, &poll, platform_version),
        ContestedDocumentVotePollStatus::Started(_)
    ));

    end_polls_at(&platform, start + DAY_MS, 11, platform_version);
    assert_eq!(
        status(&platform, &poll, platform_version),
        ContestedDocumentVotePollStatus::Awarded(alice.id())
    );
    assert!(end_dates(&platform, platform_version).is_empty());
}

#[tokio::test]
async fn should_move_the_end_to_the_join_and_vote_windows_when_a_second_applicant_joins() {
    let (mut platform, platform_version, charters, mut rng) = setup();
    let target = elected_target(&platform, 0xA2, FOUR_WEEKS, ONE_DAY, platform_version);
    let mut alice = applicant(&mut platform, &mut rng);
    let mut bob = applicant(&mut platform, &mut rng);
    let mut carol = applicant(&mut platform, &mut rng);
    let poll = charter_poll(target);

    let (start, _) = apply(
        &platform,
        &charters,
        &mut alice,
        target,
        10_000,
        &mut rng,
        platform_version,
    )
    .await;

    // Two weeks in: past the generic join window of every network, inside the target's
    let two_weeks = 14 * DAY_MS;
    apply(
        &platform,
        &charters,
        &mut bob,
        target,
        start + two_weeks - 1000,
        &mut rng,
        platform_version,
    )
    .await;

    let join_end = start + u64::from(FOUR_WEEKS) * 1000;
    let vote_end = join_end + DAY_MS;
    assert_eq!(
        end_dates(&platform, platform_version),
        vec![(
            vote_end,
            VotePoll::ContestedDocumentResourceVotePoll(poll.clone())
        )],
        "the second applicant moves the end to the join window and the vote window"
    );

    // One more applicant after the target's join window closed is refused
    let proposal_id = propose(
        &platform,
        &charters,
        &mut carol,
        target,
        join_end,
        &mut rng,
        platform_version,
    )
    .await;
    let late = application(
        &charters,
        &mut carol,
        target,
        proposal_id,
        &mut rng,
        platform_version,
    )
    .await;
    let refusal = process_refused(&platform, late, join_end + 1000, platform_version);
    let ConsensusError::StateError(StateError::DocumentContestNotJoinableError(error)) = refusal
    else {
        panic!("expected the contest not to be joinable, got {refusal:?}");
    };
    assert_eq!(
        error.joinable_time(),
        u64::from(FOUR_WEEKS) * 1000,
        "the refusal names the target's join window"
    );

    vote(
        &mut platform,
        &poll,
        ResourceVoteChoice::TowardsIdentity(bob.id()),
        0xB0B1,
        join_end + DAY_MS / 2,
        platform_version,
    )
    .await;
    vote(
        &mut platform,
        &poll,
        ResourceVoteChoice::TowardsIdentity(bob.id()),
        0xB0B2,
        join_end + DAY_MS / 2,
        platform_version,
    )
    .await;
    vote(
        &mut platform,
        &poll,
        ResourceVoteChoice::TowardsIdentity(alice.id()),
        0xA11C,
        join_end + DAY_MS / 2,
        platform_version,
    )
    .await;

    end_polls_at(&platform, join_end, 10, platform_version);
    assert!(
        matches!(
            status(&platform, &poll, platform_version),
            ContestedDocumentVotePollStatus::Started(_)
        ),
        "the join window closing no longer ends the election"
    );

    end_polls_at(&platform, vote_end, 11, platform_version);
    assert_eq!(
        status(&platform, &poll, platform_version),
        ContestedDocumentVotePollStatus::Awarded(bob.id())
    );
    assert!(end_dates(&platform, platform_version).is_empty());
}

#[tokio::test]
async fn should_end_elections_of_targets_with_different_windows_at_different_heights() {
    let (mut platform, platform_version, charters, mut rng) = setup();
    let one_day_target = elected_target(&platform, 0xA3, ONE_DAY, ONE_WEEK, platform_version);
    let four_week_target = elected_target(&platform, 0xA4, FOUR_WEEKS, ONE_WEEK, platform_version);
    let mut alice = applicant(&mut platform, &mut rng);
    let mut bob = applicant(&mut platform, &mut rng);

    let (start, _) = apply(
        &platform,
        &charters,
        &mut alice,
        one_day_target,
        10_000,
        &mut rng,
        platform_version,
    )
    .await;
    let (same_start, _) = apply(
        &platform,
        &charters,
        &mut bob,
        four_week_target,
        10_000,
        &mut rng,
        platform_version,
    )
    .await;
    assert_eq!(start, same_start);

    let one_day_poll = charter_poll(one_day_target);
    let four_week_poll = charter_poll(four_week_target);
    let four_weeks_end = start + u64::from(FOUR_WEEKS) * 1000;
    assert_eq!(
        end_dates(&platform, platform_version),
        vec![
            (
                start + DAY_MS,
                VotePoll::ContestedDocumentResourceVotePoll(one_day_poll.clone())
            ),
            (
                four_weeks_end,
                VotePoll::ContestedDocumentResourceVotePoll(four_week_poll.clone())
            ),
        ]
    );

    end_polls_at(&platform, start + DAY_MS, 10, platform_version);
    assert_eq!(
        status(&platform, &one_day_poll, platform_version),
        ContestedDocumentVotePollStatus::Awarded(alice.id()),
        "the one-day election ends at height 10"
    );
    assert!(matches!(
        status(&platform, &four_week_poll, platform_version),
        ContestedDocumentVotePollStatus::Started(_)
    ));

    end_polls_at(&platform, four_weeks_end - 1, 11, platform_version);
    assert!(matches!(
        status(&platform, &four_week_poll, platform_version),
        ContestedDocumentVotePollStatus::Started(_)
    ));

    end_polls_at(&platform, four_weeks_end, 12, platform_version);
    assert_eq!(
        status(&platform, &four_week_poll, platform_version),
        ContestedDocumentVotePollStatus::Awarded(bob.id()),
        "the four-week election ends at height 12"
    );
}

#[tokio::test]
async fn should_prefund_each_application_with_half_a_dash_and_release_the_remainder_at_clean_up() {
    let (mut platform, platform_version, charters, mut rng) = setup();
    let fund_fees = &platform_version.fee_version.vote_resolution_fund_fees;
    let moderation_fund = fund_fees.moderation_vote_resolution_fund_required_amount;
    assert_eq!(moderation_fund, dash_to_credits!(0.5));
    let vote_cost = fund_fees.contested_document_single_vote_cost;

    let target = elected_target(&platform, 0xA5, ONE_DAY, ONE_DAY, platform_version);
    let poll = charter_poll(target);
    let mut alice = applicant(&mut platform, &mut rng);
    let mut bob = applicant(&mut platform, &mut rng);

    let proposal_id = propose(
        &platform,
        &charters,
        &mut alice,
        target,
        10_000,
        &mut rng,
        platform_version,
    )
    .await;
    let alice_before = balance_of(&platform, alice.id(), platform_version);
    let alice_application = application(
        &charters,
        &mut alice,
        target,
        proposal_id,
        &mut rng,
        platform_version,
    )
    .await;
    let start = 11_000;
    let fee = process_valid(&platform, alice_application, start, platform_version);
    assert_eq!(
        alice_before - balance_of(&platform, alice.id(), platform_version),
        moderation_fund + fee.total_base_fee(),
        "applying costs the moderation fund on top of the document fee"
    );
    assert_eq!(
        prefunded_balance(&platform, &poll, platform_version),
        moderation_fund
    );

    apply(
        &platform,
        &charters,
        &mut bob,
        target,
        start + 60_000,
        &mut rng,
        platform_version,
    )
    .await;
    assert_eq!(
        prefunded_balance(&platform, &poll, platform_version),
        2 * moderation_fund,
        "every applicant prefunds the votes"
    );

    for (seed, choice) in [
        (0xF001, ResourceVoteChoice::TowardsIdentity(bob.id())),
        (0xF002, ResourceVoteChoice::TowardsIdentity(bob.id())),
        (0xF003, ResourceVoteChoice::Abstain),
    ] {
        vote(
            &mut platform,
            &poll,
            choice,
            seed,
            start + DAY_MS / 2,
            platform_version,
        )
        .await;
    }
    let remainder = 2 * moderation_fund - 3 * vote_cost;
    assert_eq!(
        prefunded_balance(&platform, &poll, platform_version),
        remainder
    );

    let processing_before = processing_credits(&platform, platform_version);
    end_polls_at(&platform, start + 2 * DAY_MS, 10, platform_version);
    assert_eq!(
        status(&platform, &poll, platform_version),
        ContestedDocumentVotePollStatus::Awarded(bob.id())
    );
    assert_eq!(
        prefunded_balance(&platform, &poll, platform_version),
        0,
        "the clean-up empties the prefunded balance"
    );
    assert_eq!(
        processing_credits(&platform, platform_version) - processing_before,
        remainder,
        "what the votes left is released as processing fees"
    );
}

#[tokio::test]
async fn should_keep_the_generic_windows_and_fund_for_a_dpns_contest() {
    let (mut platform, platform_version, _, _) = setup();
    let platform_state = platform.state.load();
    let (_, _, dpns_contract) = create_dpns_identity_name_contest(
        &mut platform,
        &platform_state,
        7,
        "quantum",
        platform_version,
    )
    .await;

    let [(end_date, VotePoll::ContestedDocumentResourceVotePoll(poll))]: [_; 1] =
        end_dates(&platform, platform_version)
            .try_into()
            .expect("expected one contest");
    assert_eq!(poll.contract_id, dpns_contract.id());

    let ContestedDocumentVotePollStatus::Started(start_block) =
        status(&platform, &poll, platform_version)
    else {
        panic!("expected the contest to run");
    };
    let generic_poll_duration =
        ContestWindows::generic(platform.drive.config.network, platform_version).poll_duration_ms;
    assert_eq!(end_date - start_block.time_ms, generic_poll_duration);
    assert_eq!(
        prefunded_balance(&platform, &poll, platform_version),
        2 * platform_version
            .fee_version
            .vote_resolution_fund_fees
            .contested_document_vote_resolution_fund_required_amount,
        "each DPNS contender prefunds the contested document fund"
    );
}

/// Elected moderation is frozen at a contract's creation, so a target cannot change kind; the
/// target is rewritten here with no moderation at all. Nothing at the election's end reads the
/// target, and a later applicant is refused with a consensus error, never an internal one.
#[tokio::test]
async fn should_end_an_election_whose_target_changed_kind() {
    let (mut platform, platform_version, charters, mut rng) = setup();
    let target = elected_target(&platform, 0xA6, ONE_DAY, ONE_WEEK, platform_version);
    let poll = charter_poll(target);
    let mut alice = applicant(&mut platform, &mut rng);
    let mut bob = applicant(&mut platform, &mut rng);

    let proposal_id = propose(
        &platform,
        &charters,
        &mut bob,
        target,
        5_000,
        &mut rng,
        platform_version,
    )
    .await;
    let (start, _) = apply(
        &platform,
        &charters,
        &mut alice,
        target,
        10_000,
        &mut rng,
        platform_version,
    )
    .await;

    write_target(&platform, 0xA6, None, platform_version);

    let late = application(
        &charters,
        &mut bob,
        target,
        proposal_id,
        &mut rng,
        platform_version,
    )
    .await;
    // Two hours in: past the generic join window of test networks, so only the reference
    // validation can refuse it, and it names why
    let refusal = process_refused(&platform, late, start + TWO_HOURS_MS, platform_version);
    assert!(
        matches!(
            refusal,
            ConsensusError::StateError(StateError::ReferencedContractRequirementNotMetError(_))
        ),
        "expected the target's missing elected moderation to refuse it, got {refusal:?}"
    );

    end_polls_at(&platform, start + DAY_MS, 10, platform_version);
    assert_eq!(
        status(&platform, &poll, platform_version),
        ContestedDocumentVotePollStatus::Awarded(alice.id())
    );
    assert!(end_dates(&platform, platform_version).is_empty());
}

/// A contract can not be deleted, so a target cannot disappear; its stored contract is removed
/// from state here. Nothing at the election's end reads the target, and a later applicant is
/// refused with a consensus error, never an internal one.
#[tokio::test]
async fn should_end_an_election_whose_target_disappeared() {
    let (mut platform, platform_version, charters, mut rng) = setup();
    let target = elected_target(&platform, 0xA7, ONE_DAY, ONE_WEEK, platform_version);
    let poll = charter_poll(target);
    let mut alice = applicant(&mut platform, &mut rng);
    let mut bob = applicant(&mut platform, &mut rng);

    let proposal_id = propose(
        &platform,
        &charters,
        &mut bob,
        target,
        5_000,
        &mut rng,
        platform_version,
    )
    .await;
    let (start, _) = apply(
        &platform,
        &charters,
        &mut alice,
        target,
        10_000,
        &mut rng,
        platform_version,
    )
    .await;

    let target_bytes = target.to_buffer();
    platform
        .drive
        .grove_delete(
            (&contract_root_path(&target_bytes)).into(),
            &[0],
            None,
            &mut vec![],
            &platform_version.drive,
        )
        .expect("expected to remove the stored target contract");
    platform.drive.cache.data_contracts.clear();
    assert!(platform
        .drive
        .get_contract_with_fetch_info(target_bytes, false, None, platform_version)
        .expect("expected to look the target up")
        .is_none());

    let late = application(
        &charters,
        &mut bob,
        target,
        proposal_id,
        &mut rng,
        platform_version,
    )
    .await;
    // Two hours in: past the generic join window of test networks, so only the reference
    // validation can refuse it, and it names why
    let refusal = process_refused(&platform, late, start + TWO_HOURS_MS, platform_version);
    assert!(
        matches!(
            refusal,
            ConsensusError::StateError(StateError::ReferencedEntityNotFoundError(_))
        ),
        "expected the missing target to refuse it, got {refusal:?}"
    );

    end_polls_at(&platform, start + DAY_MS, 10, platform_version);
    assert_eq!(
        status(&platform, &poll, platform_version),
        ContestedDocumentVotePollStatus::Awarded(alice.id())
    );
    assert!(end_dates(&platform, platform_version).is_empty());
}

/// A contest moves its end from the join window to the vote window by finding the entry it opened
/// with. The target's windows are frozen, so they are rewritten here to force what a later
/// protocol version reading the windows differently could cause: the join-window entry is not
/// where the windows now say. The second applicant is admitted, the contest keeps the end it has,
/// and nothing fails.
#[tokio::test]
async fn should_keep_the_end_date_when_the_windows_changed_during_an_election() {
    let (mut platform, platform_version, charters, mut rng) = setup();
    let target = elected_target(&platform, 0xA8, ONE_DAY, ONE_WEEK, platform_version);
    let poll = charter_poll(target);
    let mut alice = applicant(&mut platform, &mut rng);
    let mut bob = applicant(&mut platform, &mut rng);

    let (start, _) = apply(
        &platform,
        &charters,
        &mut alice,
        target,
        10_000,
        &mut rng,
        platform_version,
    )
    .await;

    elected_target(&platform, 0xA8, 2 * ONE_DAY, ONE_WEEK, platform_version);

    apply(
        &platform,
        &charters,
        &mut bob,
        target,
        start + TWO_HOURS_MS,
        &mut rng,
        platform_version,
    )
    .await;
    assert_eq!(
        end_dates(&platform, platform_version),
        vec![(
            start + DAY_MS,
            VotePoll::ContestedDocumentResourceVotePoll(poll.clone())
        )],
        "the contest keeps the end it opened with"
    );

    end_polls_at(&platform, start + DAY_MS, 10, platform_version);
    assert_eq!(
        status(&platform, &poll, platform_version),
        ContestedDocumentVotePollStatus::Awarded(alice.id()),
        "with no votes the earliest applicant wins"
    );
    assert!(end_dates(&platform, platform_version).is_empty());
}

/// An application may write the target contract as an array of byte values, which validation
/// accepts for an identifier: the contest still names it as the identifier and runs on the
/// target's own join window.
#[tokio::test]
async fn should_honor_the_target_windows_of_an_application_writing_the_target_as_an_array() {
    let (mut platform, platform_version, charters, mut rng) = setup();
    let target = elected_target(&platform, 0xA9, ONE_DAY, ONE_WEEK, platform_version);
    let poll = charter_poll(target);
    let mut alice = applicant(&mut platform, &mut rng);

    let proposal_id = propose(
        &platform,
        &charters,
        &mut alice,
        target,
        10_000,
        &mut rng,
        platform_version,
    )
    .await;
    let as_array = application_naming_the_target_as(
        &charters,
        &mut alice,
        Value::Array(target.to_buffer().into_iter().map(Value::U8).collect()),
        proposal_id,
        &mut rng,
        platform_version,
    )
    .await;
    let start = 11_000;
    process_valid(&platform, as_array, start, platform_version);

    assert_eq!(
        end_dates(&platform, platform_version),
        vec![(
            start + DAY_MS,
            VotePoll::ContestedDocumentResourceVotePoll(poll.clone())
        )],
        "the contest names the target as an identifier and ends with its one-day join window"
    );
    assert_eq!(
        prefunded_balance(&platform, &poll, platform_version),
        platform_version
            .fee_version
            .vote_resolution_fund_fees
            .moderation_vote_resolution_fund_required_amount
    );

    end_polls_at(&platform, start + DAY_MS, 10, platform_version);
    assert_eq!(
        status(&platform, &poll, platform_version),
        ContestedDocumentVotePollStatus::Awarded(alice.id())
    );
}

/// Two applicants may write the same target in two accepted forms: both name one contest with
/// one poll, so the second one moves the end to the join and vote windows and both prefunds
/// land in the one balance the clean-up releases.
#[tokio::test]
async fn should_move_the_end_when_applicants_write_the_target_in_different_forms() {
    let (mut platform, platform_version, charters, mut rng) = setup();
    let target = elected_target(&platform, 0xAA, ONE_DAY, ONE_WEEK, platform_version);
    let poll = charter_poll(target);
    let mut alice = applicant(&mut platform, &mut rng);
    let mut bob = applicant(&mut platform, &mut rng);

    let (start, _) = apply(
        &platform,
        &charters,
        &mut alice,
        target,
        10_000,
        &mut rng,
        platform_version,
    )
    .await;

    let proposal_id = propose(
        &platform,
        &charters,
        &mut bob,
        target,
        start + 60_000,
        &mut rng,
        platform_version,
    )
    .await;
    let as_bytes = application_naming_the_target_as(
        &charters,
        &mut bob,
        Value::Bytes(target.to_buffer().to_vec()),
        proposal_id,
        &mut rng,
        platform_version,
    )
    .await;
    process_valid(&platform, as_bytes, start + 61_000, platform_version);

    let vote_end = start + DAY_MS + u64::from(ONE_WEEK) * 1000;
    assert_eq!(
        end_dates(&platform, platform_version),
        vec![(
            vote_end,
            VotePoll::ContestedDocumentResourceVotePoll(poll.clone())
        )],
        "the second applicant moves the end to the join and vote windows"
    );
    assert_eq!(
        prefunded_balance(&platform, &poll, platform_version),
        2 * platform_version
            .fee_version
            .vote_resolution_fund_fees
            .moderation_vote_resolution_fund_required_amount,
        "both applications prefund the one contest"
    );

    end_polls_at(&platform, vote_end, 10, platform_version);
    assert_eq!(
        status(&platform, &poll, platform_version),
        ContestedDocumentVotePollStatus::Awarded(alice.id()),
        "with no votes the earliest applicant wins"
    );
    assert_eq!(prefunded_balance(&platform, &poll, platform_version), 0);
}
