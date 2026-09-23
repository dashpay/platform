//! Votes on yes/no vote polls: the poll's life from opening to the record of its decision.

use crate::execution::validation::state_transition::state_transitions::tests::{
    setup_masternode_voting_identity, take_down_masternode_identities,
};
use crate::platform_types::platform_state::PlatformStateV0Methods;
use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult;
use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult::{
    InternalError, SuccessfulExecution, UnpaidConsensusError,
};
use crate::rpc::core::MockCoreRPCLike;
use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
use assert_matches::assert_matches;
use dpp::block::block_info::BlockInfo;
use dpp::block::extended_block_info::v0::ExtendedBlockInfoV0;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::ConsensusError;
use dpp::dashcore::hashes::Hash;
use dpp::dashcore::ProTxHash;
use dpp::dashcore_rpc::dashcore_rpc_json::MasternodeType;
use dpp::identifier::Identifier;
use dpp::identity::IdentityPublicKey;
use dpp::platform_value::BinaryData;
use dpp::prelude::IdentityNonce;
use dpp::serialization::PlatformSerializable;
use dpp::state_transition::masternode_vote_transition::methods::MasternodeVoteTransitionMethodsV0;
use dpp::state_transition::masternode_vote_transition::MasternodeVoteTransition;
use dpp::version::PlatformVersion;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use dpp::voting::vote_choices::yes_no_abstain_vote_choice::YesNoAbstainVoteChoice;
use dpp::voting::vote_info_storage::yes_no_vote_poll_stored_info::YesNoVotePollStatus;
use dpp::voting::vote_polls::yes_no_vote_poll::YesNoVotePoll;
use dpp::voting::vote_polls::VotePoll;
use dpp::voting::votes::resource_vote::v0::ResourceVoteV0;
use dpp::voting::votes::resource_vote::ResourceVote;
use dpp::voting::votes::yes_no_vote::v0::YesNoVoteV0;
use dpp::voting::votes::yes_no_vote::YesNoVote;
use dpp::voting::votes::Vote;
use drive::query::yes_no_vote_poll_state_query::YesNoVotePollStateDriveQuery;
use drive::query::VotePollsByEndDateDriveQuery;
use simple_signer::signer::SimpleSigner;
use std::ops::Deref;
use std::sync::Arc;

/// Two weeks after genesis: the polls in these tests end here.
const END_DATE: u64 = 1_209_600_000;
/// Enough for many votes at the single vote cost.
const FUND: u64 = 100_000_000_000;

fn two_thirds_poll(purpose: &[u8], minimum_voting_power: u32) -> YesNoVotePoll {
    YesNoVotePoll {
        resource_path: vec![
            BinaryData::new(vec![0xc1; 32]),
            BinaryData::new(purpose.to_vec()),
        ],
        supermajority_numerator: 2,
        supermajority_denominator: 3,
        minimum_voting_power,
    }
}

fn open_poll(
    platform: &TempPlatform<MockCoreRPCLike>,
    vote_poll: &YesNoVotePoll,
    fund: u64,
    platform_version: &PlatformVersion,
) {
    open_poll_ending_at(platform, vote_poll, END_DATE, fund, platform_version)
}

fn open_poll_ending_at(
    platform: &TempPlatform<MockCoreRPCLike>,
    vote_poll: &YesNoVotePoll,
    end_date: u64,
    fund: u64,
    platform_version: &PlatformVersion,
) {
    platform
        .drive
        .open_yes_no_vote_poll(
            vote_poll,
            end_date,
            &BlockInfo::default(),
            None,
            platform_version,
        )
        .expect("expected to open the yes/no vote poll");
    if fund > 0 {
        platform
            .drive
            .add_prefunded_specialized_balance(
                vote_poll
                    .specialized_balance_id()
                    .expect("expected a specialized balance id"),
                fund,
                None,
                platform_version,
            )
            .expect("expected to fund the poll");
    }
}

/// A voting masternode, promoted to an evonode when asked, so its vote weighs 4.
fn setup_voter(
    platform: &mut TempPlatform<MockCoreRPCLike>,
    seed: u64,
    evonode: bool,
    platform_version: &PlatformVersion,
) -> (Identifier, SimpleSigner, IdentityPublicKey) {
    let (pro_tx_hash, _identity, signer, voting_key) =
        setup_masternode_voting_identity(platform, seed, platform_version);
    if evonode {
        let mut platform_state = platform.state.load().clone().deref().clone();
        platform_state
            .full_masternode_list_mut()
            .get_mut(&ProTxHash::from_byte_array(pro_tx_hash.to_buffer()))
            .expect("expected the masternode")
            .node_type = MasternodeType::Evo;
        platform.state.store(Arc::new(platform_state));
    }
    (pro_tx_hash, signer, voting_key)
}

#[allow(clippy::too_many_arguments)]
async fn perform_yes_no_vote(
    platform: &mut TempPlatform<MockCoreRPCLike>,
    vote_poll: &YesNoVotePoll,
    vote_choice: YesNoAbstainVoteChoice,
    signer: &SimpleSigner,
    pro_tx_hash: Identifier,
    voting_key: &IdentityPublicKey,
    nonce: IdentityNonce,
    platform_version: &PlatformVersion,
) -> Result<(), StateTransitionExecutionResult> {
    let vote = Vote::YesNoVote(YesNoVote::V0(YesNoVoteV0 {
        vote_poll: vote_poll.clone(),
        vote_choice,
    }));
    perform_vote_transition(
        platform,
        vote,
        signer,
        pro_tx_hash,
        voting_key,
        nonce,
        platform_version,
    )
    .await
}

/// Processes one masternode vote transition in a block and reports how it went.
async fn perform_vote_transition(
    platform: &mut TempPlatform<MockCoreRPCLike>,
    vote: Vote,
    signer: &SimpleSigner,
    pro_tx_hash: Identifier,
    voting_key: &IdentityPublicKey,
    nonce: IdentityNonce,
    platform_version: &PlatformVersion,
) -> Result<(), StateTransitionExecutionResult> {
    let masternode_vote_transition = MasternodeVoteTransition::try_from_vote_with_signer(
        vote,
        signer,
        pro_tx_hash,
        voting_key,
        nonce,
        platform_version,
        None,
    )
    .await
    .expect("expected to make transition vote");
    let serialized_transition = masternode_vote_transition
        .serialize_to_bytes()
        .expect("expected to serialize the vote");
    let platform_state = platform.state.load();
    let transaction = platform.drive.grove.start_transaction();
    let processing_result = platform
        .platform
        .process_raw_state_transitions(
            &[serialized_transition],
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
    match processing_result.into_execution_results().remove(0) {
        SuccessfulExecution { .. } => Ok(()),
        other => Err(other),
    }
}

/// Casts one vote per masternode: `count` voters, each with the choice, at consecutive seeds.
async fn cast_votes(
    platform: &mut TempPlatform<MockCoreRPCLike>,
    vote_poll: &YesNoVotePoll,
    vote_choice: YesNoAbstainVoteChoice,
    count: u64,
    start_seed: u64,
    platform_version: &PlatformVersion,
) -> Vec<Identifier> {
    let mut voters = vec![];
    for seed in start_seed..start_seed + count {
        let (pro_tx_hash, signer, voting_key) =
            setup_voter(platform, seed, false, platform_version);
        perform_yes_no_vote(
            platform,
            vote_poll,
            vote_choice,
            &signer,
            pro_tx_hash,
            &voting_key,
            1,
            platform_version,
        )
        .await
        .expect("expected the vote to be accepted");
        voters.push(pro_tx_hash);
    }
    voters
}

/// Moves the chain past the polls' end date and closes the polls that ended.
fn close_ended_polls(
    platform: &mut TempPlatform<MockCoreRPCLike>,
    platform_version: &PlatformVersion,
) {
    close_ended_polls_at(platform, END_DATE + 300_000, platform_version)
}

/// Moves the chain to `time_ms` and closes the polls that ended by then.
fn close_ended_polls_at(
    platform: &mut TempPlatform<MockCoreRPCLike>,
    time_ms: u64,
    platform_version: &PlatformVersion,
) {
    let mut platform_state = platform.state.load().clone().deref().clone();
    let block_info = BlockInfo {
        time_ms,
        height: 10000,
        core_height: 42,
        epoch: Default::default(),
    };
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
        .expect("expected to check for ended vote polls");
    platform
        .drive
        .grove
        .commit_transaction(transaction)
        .unwrap()
        .expect("expected to commit transaction");
}

fn poll_state(
    platform: &TempPlatform<MockCoreRPCLike>,
    vote_poll: &YesNoVotePoll,
    platform_version: &PlatformVersion,
) -> drive::query::yes_no_vote_poll_state_query::YesNoVotePollState {
    YesNoVotePollStateDriveQuery {
        vote_poll: vote_poll.clone(),
    }
    .execute_no_proof(&platform.drive, None, &mut vec![], platform_version)
    .expect("expected the poll state")
}

fn new_platform() -> TempPlatform<MockCoreRPCLike> {
    TestPlatformBuilder::new()
        .with_latest_protocol_version()
        .build_with_mock_rpc()
        .set_genesis_state()
}

#[tokio::test]
async fn should_pass_at_exactly_the_threshold_and_fail_one_power_below_it_with_abstains() {
    let platform_version = PlatformVersion::latest();
    let mut platform = new_platform();
    // Six cast votes: two thirds is four, so five yes passes and four yes does not.
    let passing = two_thirds_poll(b"passing", 6);
    let failing = two_thirds_poll(b"failing", 6);
    open_poll(&platform, &passing, FUND, platform_version);
    open_poll(&platform, &failing, FUND, platform_version);

    cast_votes(
        &mut platform,
        &passing,
        YesNoAbstainVoteChoice::Yes,
        5,
        100,
        platform_version,
    )
    .await;
    cast_votes(
        &mut platform,
        &passing,
        YesNoAbstainVoteChoice::No,
        1,
        200,
        platform_version,
    )
    .await;
    cast_votes(
        &mut platform,
        &passing,
        YesNoAbstainVoteChoice::Abstain,
        2,
        300,
        platform_version,
    )
    .await;

    cast_votes(
        &mut platform,
        &failing,
        YesNoAbstainVoteChoice::Yes,
        4,
        400,
        platform_version,
    )
    .await;
    cast_votes(
        &mut platform,
        &failing,
        YesNoAbstainVoteChoice::No,
        2,
        500,
        platform_version,
    )
    .await;
    cast_votes(
        &mut platform,
        &failing,
        YesNoAbstainVoteChoice::Abstain,
        3,
        600,
        platform_version,
    )
    .await;

    let state = poll_state(&platform, &passing, platform_version);
    assert_eq!(
        (
            state.yes_voting_power,
            state.no_voting_power,
            state.abstain_voting_power
        ),
        (5, 1, 2)
    );
    assert_matches!(
        state.stored_info.expect("stored info").status(),
        YesNoVotePollStatus::Started(_)
    );

    close_ended_polls(&mut platform, platform_version);

    let passing_result = *poll_state(&platform, &passing, platform_version)
        .stored_info
        .expect("stored info")
        .result()
        .expect("expected the poll to be finished");
    assert!(passing_result.passed);
    assert_eq!(passing_result.yes_voting_power, 5);
    assert_eq!(passing_result.no_voting_power, 1);
    assert_eq!(passing_result.abstain_voting_power, 2);
    assert_eq!(
        passing_result.finalization_block.time_ms,
        END_DATE + 300_000
    );

    let failing_result = *poll_state(&platform, &failing, platform_version)
        .stored_info
        .expect("stored info")
        .result()
        .expect("expected the poll to be finished");
    assert!(!failing_result.passed);
    assert_eq!(failing_result.yes_voting_power, 4);
    assert_eq!(failing_result.no_voting_power, 2);
    assert_eq!(failing_result.abstain_voting_power, 3);
}

#[tokio::test]
async fn should_fail_below_the_minimum_voting_power_even_when_every_vote_is_yes() {
    let platform_version = PlatformVersion::latest();
    let mut platform = new_platform();
    let vote_poll = two_thirds_poll(b"unanimous", 6);
    open_poll(&platform, &vote_poll, FUND, platform_version);
    cast_votes(
        &mut platform,
        &vote_poll,
        YesNoAbstainVoteChoice::Yes,
        5,
        100,
        platform_version,
    )
    .await;
    // Abstaining power does not count towards the minimum.
    cast_votes(
        &mut platform,
        &vote_poll,
        YesNoAbstainVoteChoice::Abstain,
        3,
        200,
        platform_version,
    )
    .await;

    close_ended_polls(&mut platform, platform_version);

    let result = *poll_state(&platform, &vote_poll, platform_version)
        .stored_info
        .expect("stored info")
        .result()
        .expect("expected the poll to be finished");
    assert!(!result.passed);
    assert_eq!(result.yes_voting_power, 5);
    assert_eq!(result.no_voting_power, 0);
    assert_eq!(result.abstain_voting_power, 3);
}

#[tokio::test]
async fn should_count_an_evonode_vote_as_four() {
    let platform_version = PlatformVersion::latest();
    let mut platform = new_platform();
    let vote_poll = two_thirds_poll(b"evonode", 5);
    open_poll(&platform, &vote_poll, FUND, platform_version);

    let (evonode, signer, voting_key) = setup_voter(&mut platform, 100, true, platform_version);
    perform_yes_no_vote(
        &mut platform,
        &vote_poll,
        YesNoAbstainVoteChoice::Yes,
        &signer,
        evonode,
        &voting_key,
        1,
        platform_version,
    )
    .await
    .expect("expected the evonode vote to be accepted");
    cast_votes(
        &mut platform,
        &vote_poll,
        YesNoAbstainVoteChoice::No,
        1,
        200,
        platform_version,
    )
    .await;

    let state = poll_state(&platform, &vote_poll, platform_version);
    assert_eq!((state.yes_voting_power, state.no_voting_power), (4, 1));

    close_ended_polls(&mut platform, platform_version);

    // Five cast, minimum five: four is strictly more than two thirds of five.
    let result = *poll_state(&platform, &vote_poll, platform_version)
        .stored_info
        .expect("stored info")
        .result()
        .expect("expected the poll to be finished");
    assert!(result.passed);
    assert_eq!(result.yes_voting_power, 4);
}

#[tokio::test]
async fn should_leave_only_the_record_behind_after_the_poll_ends() {
    let platform_version = PlatformVersion::latest();
    let mut platform = new_platform();
    let vote_poll = two_thirds_poll(b"cleanup", 1);
    open_poll(&platform, &vote_poll, FUND, platform_version);
    let yes_voters = cast_votes(
        &mut platform,
        &vote_poll,
        YesNoAbstainVoteChoice::Yes,
        3,
        100,
        platform_version,
    )
    .await;
    let no_voters = cast_votes(
        &mut platform,
        &vote_poll,
        YesNoAbstainVoteChoice::No,
        1,
        200,
        platform_version,
    )
    .await;
    let vote_poll_id = vote_poll.unique_id().expect("id");
    let vote_cost = platform_version
        .fee_version
        .vote_resolution_fund_fees
        .contested_document_single_vote_cost;

    // Before the end: votes, index entries, end date entry and the fund minus four votes.
    let voters = platform
        .drive
        .fetch_identities_voting_in_yes_no_vote_poll(&vote_poll, None, platform_version)
        .expect("voters");
    assert_eq!(voters[&YesNoAbstainVoteChoice::Yes].len(), 3);
    assert_eq!(voters[&YesNoAbstainVoteChoice::No].len(), 1);
    assert!(voters[&YesNoAbstainVoteChoice::Abstain].is_empty());
    assert_eq!(
        platform
            .drive
            .fetch_identity_yes_no_vote(
                yes_voters[0],
                vote_poll_id,
                None,
                &mut vec![],
                platform_version
            )
            .expect("vote"),
        Some((YesNoAbstainVoteChoice::Yes, 1))
    );
    let balance = platform
        .drive
        .fetch_prefunded_specialized_balance(vote_poll_id.to_buffer(), None, platform_version)
        .expect("balance");
    assert_eq!(balance, Some(FUND - 4 * vote_cost));
    let end_dates = VotePollsByEndDateDriveQuery {
        start_time: None,
        end_time: None,
        limit: None,
        offset: None,
        order_ascending: true,
    }
    .execute_no_proof(&platform.drive, None, &mut vec![], platform_version)
    .expect("end dates");
    assert_eq!(end_dates.len(), 1);

    close_ended_polls(&mut platform, platform_version);

    // After the end: the record with the result, and nothing else.
    let state = poll_state(&platform, &vote_poll, platform_version);
    let result = *state
        .stored_info
        .expect("stored info")
        .result()
        .expect("expected the poll to be finished");
    assert!(result.passed);
    assert_eq!((result.yes_voting_power, result.no_voting_power), (3, 1));
    assert_eq!(
        (
            state.yes_voting_power,
            state.no_voting_power,
            state.abstain_voting_power
        ),
        (0, 0, 0)
    );
    let voters = platform
        .drive
        .fetch_identities_voting_in_yes_no_vote_poll(&vote_poll, None, platform_version)
        .expect("voters");
    assert!(voters.values().all(|voters| voters.is_empty()));
    for voter in yes_voters.iter().chain(no_voters.iter()) {
        assert_eq!(
            platform
                .drive
                .fetch_identity_yes_no_vote(
                    *voter,
                    vote_poll_id,
                    None,
                    &mut vec![],
                    platform_version
                )
                .expect("vote"),
            None
        );
    }
    let balance = platform
        .drive
        .fetch_prefunded_specialized_balance(vote_poll_id.to_buffer(), None, platform_version)
        .expect("balance");
    assert!(balance.is_none() || balance == Some(0));
    let end_dates = VotePollsByEndDateDriveQuery {
        start_time: None,
        end_time: None,
        limit: None,
        offset: None,
        order_ascending: true,
    }
    .execute_no_proof(&platform.drive, None, &mut vec![], platform_version)
    .expect("end dates");
    assert!(end_dates.is_empty());

    // A finished poll takes no more votes, and cannot be opened again under the same id.
    let (pro_tx_hash, signer, voting_key) =
        setup_voter(&mut platform, 300, false, platform_version);
    let error = perform_yes_no_vote(
        &mut platform,
        &vote_poll,
        YesNoAbstainVoteChoice::Yes,
        &signer,
        pro_tx_hash,
        &voting_key,
        1,
        platform_version,
    )
    .await
    .expect_err("expected the vote to be refused");
    // Closing the poll emptied its fund, and the fund pre-check runs before state validation,
    // so the missing fund is what refuses the vote; the finished status refuses it too.
    assert!(
        matches!(
            &error,
            UnpaidConsensusError(ConsensusError::StateError(
                StateError::PrefundedSpecializedBalanceNotFoundError(_)
            )) | UnpaidConsensusError(ConsensusError::StateError(
                StateError::YesNoVotePollNotAvailableForVotingError(_)
            ))
        ),
        "unexpected result {:?}",
        error
    );
    assert!(platform
        .drive
        .open_yes_no_vote_poll(
            &vote_poll,
            END_DATE,
            &BlockInfo::default(),
            None,
            platform_version
        )
        .is_err());
}

#[tokio::test]
async fn should_refuse_a_vote_on_an_unknown_poll_and_the_same_answer_twice() {
    let platform_version = PlatformVersion::latest();
    let mut platform = new_platform();
    let vote_poll = two_thirds_poll(b"rules", 1);
    let unknown_poll = two_thirds_poll(b"unknown", 1);
    open_poll(&platform, &vote_poll, FUND, platform_version);
    // The unknown poll's fund exists, the poll does not.
    platform
        .drive
        .add_prefunded_specialized_balance(
            unknown_poll.specialized_balance_id().expect("id"),
            FUND,
            None,
            platform_version,
        )
        .expect("fund");
    let (pro_tx_hash, signer, voting_key) =
        setup_voter(&mut platform, 100, false, platform_version);

    let error = perform_yes_no_vote(
        &mut platform,
        &unknown_poll,
        YesNoAbstainVoteChoice::Yes,
        &signer,
        pro_tx_hash,
        &voting_key,
        1,
        platform_version,
    )
    .await
    .expect_err("expected the vote on an unknown poll to be refused");
    assert_matches!(
        error,
        UnpaidConsensusError(ConsensusError::StateError(
            StateError::VotePollNotFoundError(_)
        ))
    );

    perform_yes_no_vote(
        &mut platform,
        &vote_poll,
        YesNoAbstainVoteChoice::Yes,
        &signer,
        pro_tx_hash,
        &voting_key,
        1,
        platform_version,
    )
    .await
    .expect("expected the first vote to be accepted");
    let error = perform_yes_no_vote(
        &mut platform,
        &vote_poll,
        YesNoAbstainVoteChoice::Yes,
        &signer,
        pro_tx_hash,
        &voting_key,
        2,
        platform_version,
    )
    .await
    .expect_err("expected the same answer to be refused");
    assert_matches!(
        error,
        UnpaidConsensusError(ConsensusError::StateError(
            StateError::MasternodeVoteAlreadyPresentError(_)
        ))
    );
}

#[tokio::test]
async fn should_refuse_a_resource_vote_that_names_a_yes_no_poll() {
    let platform_version = PlatformVersion::latest();
    let mut platform = new_platform();
    let vote_poll = two_thirds_poll(b"cross kind", 1);
    open_poll(&platform, &vote_poll, FUND, platform_version);
    let (pro_tx_hash, signer, voting_key) =
        setup_voter(&mut platform, 100, false, platform_version);
    // A resource vote choice answers a contested document resource poll, not a yes/no one.
    let vote = Vote::ResourceVote(ResourceVote::V0(ResourceVoteV0 {
        vote_poll: VotePoll::YesNoVotePoll(vote_poll.clone()),
        resource_vote_choice: ResourceVoteChoice::Abstain,
    }));
    let error = perform_vote_transition(
        &mut platform,
        vote,
        &signer,
        pro_tx_hash,
        &voting_key,
        1,
        platform_version,
    )
    .await
    .expect_err("expected the vote to be refused");
    assert_matches!(
        error,
        UnpaidConsensusError(ConsensusError::StateError(
            StateError::VotePollNotFoundError(_)
        ))
    );
    let state = poll_state(&platform, &vote_poll, platform_version);
    assert_eq!(state.abstain_voting_power, 0);
}

#[tokio::test]
async fn should_move_a_changed_vote_and_cap_the_changes() {
    let platform_version = PlatformVersion::latest();
    let mut platform = new_platform();
    let vote_poll = two_thirds_poll(b"changes", 1);
    open_poll(&platform, &vote_poll, FUND, platform_version);
    let (pro_tx_hash, signer, voting_key) = setup_voter(&mut platform, 100, true, platform_version);
    let vote_poll_id = vote_poll.unique_id().expect("id");
    let allowed = platform_version
        .dpp
        .validation
        .voting
        .votes_allowed_per_masternode;

    let choices = [
        YesNoAbstainVoteChoice::Yes,
        YesNoAbstainVoteChoice::No,
        YesNoAbstainVoteChoice::Abstain,
    ];
    let mut nonce = 1;
    for (vote_number, vote_choice) in choices.iter().cycle().take(allowed as usize).enumerate() {
        perform_yes_no_vote(
            &mut platform,
            &vote_poll,
            *vote_choice,
            &signer,
            pro_tx_hash,
            &voting_key,
            nonce,
            platform_version,
        )
        .await
        .expect("expected the vote change to be accepted");
        nonce += 1;
        // Only the latest answer holds the evonode's four.
        let state = poll_state(&platform, &vote_poll, platform_version);
        for choice in choices {
            let expected = if choice == *vote_choice { 4 } else { 0 };
            assert_eq!(state.voting_power(choice), expected, "vote {}", vote_number);
        }
        assert_eq!(
            platform
                .drive
                .fetch_identity_yes_no_vote(
                    pro_tx_hash,
                    vote_poll_id,
                    None,
                    &mut vec![],
                    platform_version
                )
                .expect("vote"),
            Some((*vote_choice, vote_number as u16 + 1))
        );
    }

    let next_choice = choices[allowed as usize % choices.len()];
    let error = perform_yes_no_vote(
        &mut platform,
        &vote_poll,
        next_choice,
        &signer,
        pro_tx_hash,
        &voting_key,
        nonce,
        platform_version,
    )
    .await
    .expect_err("expected one change too many to be refused");
    assert_matches!(
        error,
        UnpaidConsensusError(ConsensusError::StateError(
            StateError::MasternodeVotedTooManyTimesError(_)
        ))
    );
}

#[tokio::test]
async fn should_refuse_a_vote_when_the_poll_has_no_fund() {
    let platform_version = PlatformVersion::latest();
    let mut platform = new_platform();
    let vote_poll = two_thirds_poll(b"unfunded", 1);
    open_poll(&platform, &vote_poll, 0, platform_version);
    let (pro_tx_hash, signer, voting_key) =
        setup_voter(&mut platform, 100, false, platform_version);
    // Block processing does not act on the prefunded balance pre-check today (its result is
    // not read in the processor, for either poll kind), so the vote fails when its cost is
    // deducted; #4904 turns that into an unpaid consensus error. Either way nothing of the
    // vote is recorded.
    let error = perform_yes_no_vote(
        &mut platform,
        &vote_poll,
        YesNoAbstainVoteChoice::Yes,
        &signer,
        pro_tx_hash,
        &voting_key,
        1,
        platform_version,
    )
    .await
    .expect_err("expected the vote to be refused");
    assert!(
        matches!(&error, InternalError(message) if message.contains("prefunded specialized balance"))
            || matches!(
                &error,
                UnpaidConsensusError(ConsensusError::StateError(
                    StateError::PrefundedSpecializedBalanceNotFoundError(_)
                ))
            ),
        "unexpected result {:?}",
        error
    );
    assert_eq!(
        platform
            .drive
            .fetch_identity_yes_no_vote(
                pro_tx_hash,
                vote_poll.unique_id().expect("id"),
                None,
                &mut vec![],
                platform_version
            )
            .expect("vote"),
        None
    );
    let state = poll_state(&platform, &vote_poll, platform_version);
    assert_eq!(state.yes_voting_power, 0);
}

#[tokio::test]
async fn should_remove_the_votes_of_masternodes_that_left_the_list() {
    let platform_version = PlatformVersion::latest();
    let mut platform = new_platform();
    let vote_poll = two_thirds_poll(b"removals", 1);
    open_poll(&platform, &vote_poll, FUND, platform_version);
    let yes_voters = cast_votes(
        &mut platform,
        &vote_poll,
        YesNoAbstainVoteChoice::Yes,
        3,
        100,
        platform_version,
    )
    .await;
    let no_voters = cast_votes(
        &mut platform,
        &vote_poll,
        YesNoAbstainVoteChoice::No,
        2,
        200,
        platform_version,
    )
    .await;
    let vote_poll_id = vote_poll.unique_id().expect("id");

    let platform_state_before_removals = platform.state.load().deref().clone();
    let removed = vec![yes_voters[0], yes_voters[1], no_voters[0]];
    take_down_masternode_identities(&mut platform, &removed);
    let block_platform_state = platform.state.load();
    let transaction = platform.drive.grove.start_transaction();
    platform
        .remove_votes_for_removed_masternodes(
            &BlockInfo::default(),
            &platform_state_before_removals,
            &block_platform_state,
            Some(&transaction),
            platform_version,
        )
        .expect("expected to remove votes for removed masternodes");
    platform
        .drive
        .grove
        .commit_transaction(transaction)
        .unwrap()
        .expect("expected to commit transaction");

    let state = poll_state(&platform, &vote_poll, platform_version);
    assert_eq!((state.yes_voting_power, state.no_voting_power), (1, 1));
    let voters = platform
        .drive
        .fetch_identities_voting_in_yes_no_vote_poll(&vote_poll, None, platform_version)
        .expect("voters");
    assert_eq!(voters[&YesNoAbstainVoteChoice::Yes], vec![yes_voters[2]]);
    assert_eq!(voters[&YesNoAbstainVoteChoice::No], vec![no_voters[1]]);
    for voter in &removed {
        assert_eq!(
            platform
                .drive
                .fetch_identity_yes_no_vote(
                    *voter,
                    vote_poll_id,
                    None,
                    &mut vec![],
                    platform_version
                )
                .expect("vote"),
            None
        );
    }
    assert_eq!(
        platform
            .drive
            .fetch_identity_yes_no_vote(
                yes_voters[2],
                vote_poll_id,
                None,
                &mut vec![],
                platform_version
            )
            .expect("vote"),
        Some((YesNoAbstainVoteChoice::Yes, 1))
    );

    // The poll still closes cleanly with what is left.
    close_ended_polls(&mut platform, platform_version);
    let result = *poll_state(&platform, &vote_poll, platform_version)
        .stored_info
        .expect("stored info")
        .result()
        .expect("expected the poll to be finished");
    assert_eq!((result.yes_voting_power, result.no_voting_power), (1, 1));
    assert!(!result.passed);
}

#[tokio::test]
async fn should_close_a_contested_poll_and_a_yes_no_poll_that_end_at_the_same_time() {
    use crate::execution::validation::state_transition::state_transitions::tests::create_dpns_identity_name_contest;

    let platform_version = PlatformVersion::latest();
    let mut platform = new_platform();
    let platform_state = platform.state.load();
    // The DPNS contest ends two weeks after its creation at the genesis block time.
    let (_contender_1, _contender_2, _dpns_contract) = create_dpns_identity_name_contest(
        &mut platform,
        &platform_state,
        7,
        "quantum",
        platform_version,
    )
    .await;
    // The yes/no poll ends at the very millisecond the contest does.
    let contest_end_date = *VotePollsByEndDateDriveQuery {
        start_time: None,
        end_time: None,
        limit: None,
        offset: None,
        order_ascending: true,
    }
    .execute_no_proof(&platform.drive, None, &mut vec![], platform_version)
    .expect("end dates")
    .keys()
    .next()
    .expect("expected the contest's end date");
    let vote_poll = two_thirds_poll(b"shared end date", 1);
    open_poll_ending_at(
        &platform,
        &vote_poll,
        contest_end_date,
        FUND,
        platform_version,
    );
    cast_votes(
        &mut platform,
        &vote_poll,
        YesNoAbstainVoteChoice::Yes,
        1,
        100,
        platform_version,
    )
    .await;

    let end_dates = VotePollsByEndDateDriveQuery {
        start_time: None,
        end_time: None,
        limit: None,
        offset: None,
        order_ascending: true,
    }
    .execute_no_proof(&platform.drive, None, &mut vec![], platform_version)
    .expect("end dates");
    assert_eq!(end_dates.len(), 1, "both polls end at the same time");
    assert_eq!(end_dates.values().next().expect("polls").len(), 2);

    close_ended_polls_at(&mut platform, contest_end_date + 300_000, platform_version);

    let end_dates = VotePollsByEndDateDriveQuery {
        start_time: None,
        end_time: None,
        limit: None,
        offset: None,
        order_ascending: true,
    }
    .execute_no_proof(&platform.drive, None, &mut vec![], platform_version)
    .expect("end dates");
    assert!(end_dates.is_empty());
    assert!(
        poll_state(&platform, &vote_poll, platform_version)
            .stored_info
            .expect("stored info")
            .result()
            .expect("finished")
            .passed
    );
}
