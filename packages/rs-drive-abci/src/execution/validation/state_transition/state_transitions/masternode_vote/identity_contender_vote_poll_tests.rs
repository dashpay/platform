//! Identity contender vote polls end to end: opening, joining, voting through masternode vote
//! state transitions, the two phase ends, the record and the clean-up (issue #4874).

use crate::execution::validation::state_transition::state_transitions::tests::setup_masternode_voting_identity;
use crate::platform_types::platform_state::PlatformStateV0Methods;
use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult::{
    SuccessfulExecution, UnpaidConsensusError,
};
use crate::rpc::core::MockCoreRPCLike;
use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
use assert_matches::assert_matches;
use dapi_grpc::platform::v0::get_identity_contender_vote_poll_state_request::{
    GetIdentityContenderVotePollStateRequestV0, Version as RequestVersion,
};
use dapi_grpc::platform::v0::get_identity_contender_vote_poll_state_response::get_identity_contender_vote_poll_state_response_v0::{
    Result as ResultV0, Status,
};
use dapi_grpc::platform::v0::get_identity_contender_vote_poll_state_response::Version as ResponseVersion;
use dapi_grpc::platform::v0::GetIdentityContenderVotePollStateRequest;
use dpp::block::block_info::BlockInfo;
use dpp::block::extended_block_info::v0::ExtendedBlockInfoV0;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::ConsensusError;
use dpp::dash_to_credits;
use dpp::identifier::Identifier;
use dpp::identity::{IdentityPublicKey, TimestampMillis};
use dpp::serialization::PlatformSerializable;
use dpp::state_transition::masternode_vote_transition::methods::MasternodeVoteTransitionMethodsV0;
use dpp::state_transition::masternode_vote_transition::MasternodeVoteTransition;
use dpp::voting::contender_structs::IdentityContenderInfo;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use dpp::voting::vote_info_storage::identity_contender_vote_poll_stored_info::{
    IdentityContenderVotePollStatus, IdentityContenderVotePollStoredInfoV0Getters,
};
use dpp::voting::vote_polls::identity_contender_vote_poll::IdentityContenderVotePoll;
use dpp::voting::vote_polls::VotePoll;
use dpp::voting::votes::resource_vote::v0::ResourceVoteV0;
use dpp::voting::votes::resource_vote::ResourceVote;
use dpp::voting::votes::Vote;
use drive::query::identity_contender_vote_poll_state_query::IdentityContenderVotePollStateQuery;
use drive::query::VotePollsByEndDateDriveQuery;
use platform_version::version::PlatformVersion;
use simple_signer::signer::SimpleSigner;
use std::sync::Arc;

const JOIN_END_MS: TimestampMillis = 1_000_000;
const VOTE_END_MS: TimestampMillis = 2_000_000;

fn block(time_ms: TimestampMillis, height: u64) -> BlockInfo {
    BlockInfo {
        time_ms,
        height,
        core_height: 42,
        epoch: Default::default(),
    }
}

fn poll_for(contract_id: u8) -> IdentityContenderVotePoll {
    IdentityContenderVotePoll::new(vec![vec![contract_id; 32], b"election".to_vec()])
}

fn setup() -> (TempPlatform<MockCoreRPCLike>, &'static PlatformVersion) {
    let platform_version = PlatformVersion::latest();
    let platform = TestPlatformBuilder::new()
        .with_latest_protocol_version()
        .build_with_mock_rpc()
        .set_genesis_state();
    (platform, platform_version)
}

/// Opens a poll funded with one Dash.
fn open_poll(
    platform: &TempPlatform<MockCoreRPCLike>,
    vote_poll: &IdentityContenderVotePoll,
    platform_version: &PlatformVersion,
) {
    platform
        .drive
        .open_identity_contender_vote_poll(
            vote_poll,
            JOIN_END_MS,
            VOTE_END_MS,
            &block(1, 1),
            None,
            platform_version,
        )
        .expect("expected to open the poll");
    platform
        .drive
        .add_prefunded_specialized_balance(
            vote_poll
                .specialized_balance_id()
                .expect("expected the balance id"),
            dash_to_credits!(1),
            None,
            platform_version,
        )
        .expect("expected to fund the poll");
}

/// A contender that joined in `joined_at` through the reference `reference`.
fn add_contender(
    platform: &TempPlatform<MockCoreRPCLike>,
    vote_poll: &IdentityContenderVotePoll,
    identity: u8,
    joined_at: BlockInfo,
    reference: u8,
    platform_version: &PlatformVersion,
) -> Identifier {
    let identity_id = Identifier::new([identity; 32]);
    platform
        .drive
        .add_identity_contender(
            vote_poll,
            identity_id,
            IdentityContenderInfo::new(
                joined_at,
                Identifier::new([reference; 32]),
                platform_version,
            )
            .expect("expected the contender info"),
            &joined_at,
            None,
            platform_version,
        )
        .expect("expected to add the contender");
    identity_id
}

/// Runs the end of block work at `time_ms`, ending every poll phase due by then.
fn end_phases_at(
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
        .expect("expected to check for ended vote polls");
    platform
        .drive
        .grove
        .commit_transaction(transaction)
        .unwrap()
        .expect("expected to commit transaction");
}

struct Voter {
    pro_tx_hash: Identifier,
    signer: SimpleSigner,
    voting_key: IdentityPublicKey,
    nonce: u64,
}

fn voter(
    platform: &mut TempPlatform<MockCoreRPCLike>,
    seed: u64,
    platform_version: &PlatformVersion,
) -> Voter {
    let (pro_tx_hash, _, signer, voting_key) =
        setup_masternode_voting_identity(platform, seed, platform_version);
    Voter {
        pro_tx_hash,
        signer,
        voting_key,
        nonce: 0,
    }
}

/// Casts a vote through a masternode vote state transition. A vote expected to be valid also
/// pins, before it is processed, that check_tx accepts it without mutating committed state, as
/// the contested resource fixtures do.
async fn cast_vote(
    platform: &mut TempPlatform<MockCoreRPCLike>,
    vote_poll: &IdentityContenderVotePoll,
    voter: &mut Voter,
    choice: ResourceVoteChoice,
    expect_valid: bool,
    platform_version: &PlatformVersion,
) -> Result<(), ConsensusError> {
    voter.nonce += 1;
    let vote = Vote::ResourceVote(ResourceVote::V0(ResourceVoteV0 {
        vote_poll: VotePoll::IdentityContenderVotePoll(vote_poll.clone()),
        resource_vote_choice: choice,
    }));
    let transition = MasternodeVoteTransition::try_from_vote_with_signer(
        vote,
        &voter.signer,
        voter.pro_tx_hash,
        &voter.voting_key,
        voter.nonce,
        platform_version,
        None,
    )
    .await
    .expect("expected to make the vote transition");
    let serialized = transition
        .serialize_to_bytes()
        .expect("expected to serialize the vote");

    if expect_valid {
        crate::test::helpers::state_mutation_guard::assert_check_tx_valid_at_all_levels(
            platform,
            &serialized,
            "identity contender vote poll vote",
        );
    }

    let platform_state = platform.state.load();
    let transaction = platform.drive.grove.start_transaction();
    let processing_result = platform
        .platform
        .process_raw_state_transitions(
            &[serialized.clone()],
            &platform_state,
            &BlockInfo::default(),
            &transaction,
            platform_version,
            false,
            None,
        )
        .expect("expected to process the vote");
    let execution_result = processing_result.into_execution_results().remove(0);
    match execution_result {
        SuccessfulExecution { .. } => {
            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit the vote");
            Ok(())
        }
        UnpaidConsensusError(error) => {
            platform
                .drive
                .grove
                .rollback_transaction(&transaction)
                .expect("expected to roll back the vote");
            voter.nonce -= 1;
            Err(error)
        }
        other => panic!("unexpected execution result {:?}", other),
    }
}

fn state_of(
    platform: &TempPlatform<MockCoreRPCLike>,
    vote_poll: &IdentityContenderVotePoll,
    platform_version: &PlatformVersion,
) -> drive::query::identity_contender_vote_poll_state_query::IdentityContenderVotePollState {
    IdentityContenderVotePollStateQuery {
        vote_poll_id: vote_poll.unique_id().expect("expected the poll id"),
        limit: None,
        start_at: None,
    }
    .execute_no_proof(&platform.drive, None, &mut vec![], platform_version)
    .expect("expected the poll state")
}

fn tally_of(
    state: &drive::query::identity_contender_vote_poll_state_query::IdentityContenderVotePollState,
    identity_id: Identifier,
) -> Option<u32> {
    state
        .contenders
        .iter()
        .find(|contender| contender.identity_id == identity_id)
        .expect("expected the contender")
        .vote_tally
}

fn end_date_entries(
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

#[tokio::test]
async fn should_resolve_a_three_contender_poll_by_plurality_and_clean_up() {
    let (mut platform, platform_version) = setup();
    let vote_poll = poll_for(1);
    open_poll(&platform, &vote_poll, platform_version);
    let alice = add_contender(
        &platform,
        &vote_poll,
        0xa1,
        block(10, 2),
        1,
        platform_version,
    );
    let bob = add_contender(
        &platform,
        &vote_poll,
        0xb2,
        block(20, 3),
        2,
        platform_version,
    );
    let carol = add_contender(
        &platform,
        &vote_poll,
        0xc3,
        block(30, 4),
        3,
        platform_version,
    );

    // The poll sits at its join end
    let entries = end_date_entries(&platform, platform_version);
    assert_eq!(
        entries,
        vec![(
            JOIN_END_MS,
            VotePoll::IdentityContenderVotePoll(vote_poll.clone())
        )]
    );

    // Nobody votes while contenders still join
    let mut early = voter(&mut platform, 1, platform_version);
    let error = cast_vote(
        &mut platform,
        &vote_poll,
        &mut early,
        ResourceVoteChoice::TowardsIdentity(alice),
        false,
        platform_version,
    )
    .await
    .expect_err("expected the vote to be refused during the join phase");
    assert_matches!(
        error,
        ConsensusError::StateError(StateError::IdentityContenderVotePollNotAvailableForVotingError(
            ref e
        )) if e.status() == IdentityContenderVotePollStatus::Joining
    );

    // The join phase ends: three contenders, so the vote phase starts
    end_phases_at(&platform, JOIN_END_MS + 1, 10, platform_version);
    let stored_info = platform
        .drive
        .fetch_identity_contender_vote_poll_stored_info(&vote_poll, None, platform_version)
        .expect("expected to fetch")
        .expect("expected the stored info");
    assert_eq!(
        stored_info.status(),
        IdentityContenderVotePollStatus::Voting
    );
    assert_eq!(
        end_date_entries(&platform, platform_version),
        vec![(
            VOTE_END_MS,
            VotePoll::IdentityContenderVotePoll(vote_poll.clone())
        )]
    );

    // Votes: Bob 5, Alice 3, Carol 1, abstain 2
    let mut voters = vec![];
    for (seed, choice) in [
        (1, ResourceVoteChoice::TowardsIdentity(bob)),
        (2, ResourceVoteChoice::TowardsIdentity(bob)),
        (3, ResourceVoteChoice::TowardsIdentity(bob)),
        (4, ResourceVoteChoice::TowardsIdentity(bob)),
        (5, ResourceVoteChoice::TowardsIdentity(bob)),
        (6, ResourceVoteChoice::TowardsIdentity(alice)),
        (7, ResourceVoteChoice::TowardsIdentity(alice)),
        (8, ResourceVoteChoice::TowardsIdentity(alice)),
        (9, ResourceVoteChoice::TowardsIdentity(carol)),
        (10, ResourceVoteChoice::Abstain),
        (11, ResourceVoteChoice::Abstain),
    ] {
        let mut voter = voter(&mut platform, 100 + seed, platform_version);
        cast_vote(
            &mut platform,
            &vote_poll,
            &mut voter,
            choice,
            true,
            platform_version,
        )
        .await
        .expect("expected the vote to be accepted");
        voters.push(voter);
    }
    let state = state_of(&platform, &vote_poll, platform_version);
    assert_eq!(tally_of(&state, bob), Some(5));
    assert_eq!(tally_of(&state, alice), Some(3));
    assert_eq!(tally_of(&state, carol), Some(1));
    assert_eq!(state.abstain_vote_tally, Some(2));
    let balance_before_end = platform
        .drive
        .fetch_prefunded_specialized_balance(
            vote_poll.specialized_balance_id().unwrap().to_buffer(),
            None,
            platform_version,
        )
        .expect("expected to fetch the balance")
        .expect("expected the balance");
    assert!(
        balance_before_end < dash_to_credits!(1),
        "the votes were paid from the poll"
    );

    // The vote phase ends: Bob wins by plurality
    end_phases_at(&platform, VOTE_END_MS + 1, 20, platform_version);
    let stored_info = platform
        .drive
        .fetch_identity_contender_vote_poll_stored_info(&vote_poll, None, platform_version)
        .expect("expected to fetch")
        .expect("expected the stored info");
    assert_eq!(
        stored_info.status(),
        IdentityContenderVotePollStatus::Resolved
    );
    assert_eq!(stored_info.winner(), Some(bob));
    let result = stored_info.result().expect("expected the result");
    assert!(result.vote_phase_held);
    assert_eq!(result.finalization_block.height, 20);
    assert_eq!(
        result.tally_of(&ResourceVoteChoice::TowardsIdentity(bob)),
        5
    );
    assert_eq!(
        result.tally_of(&ResourceVoteChoice::TowardsIdentity(alice)),
        3
    );
    assert_eq!(
        result.tally_of(&ResourceVoteChoice::TowardsIdentity(carol)),
        1
    );
    assert_eq!(result.tally_of(&ResourceVoteChoice::Abstain), 2);

    // The clean-up: the contenders, the votes, the references, the end date entry and the
    // prefunded balance are gone; only the record stays
    let state = state_of(&platform, &vote_poll, platform_version);
    assert!(state.contenders.is_empty());
    assert_eq!(state.abstain_vote_tally, None);
    assert!(state.stored_info.is_some());
    assert!(end_date_entries(&platform, platform_version).is_empty());
    for voter in &voters {
        assert_eq!(
            platform
                .drive
                .fetch_identity_contender_vote_poll_identity_vote(
                    voter.pro_tx_hash,
                    vote_poll.unique_id().unwrap(),
                    None,
                    &mut vec![],
                    platform_version,
                )
                .expect("expected to fetch the reference"),
            None
        );
    }
    assert_eq!(
        platform
            .drive
            .fetch_prefunded_specialized_balance(
                vote_poll.specialized_balance_id().unwrap().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch the balance"),
        None
    );

    // Nobody votes on a resolved poll
    let mut late = voter(&mut platform, 2, platform_version);
    let error = cast_vote(
        &mut platform,
        &vote_poll,
        &mut late,
        ResourceVoteChoice::TowardsIdentity(alice),
        false,
        platform_version,
    )
    .await
    .expect_err("expected the vote to be refused after the poll resolved");
    // The prefunded balance went with the poll, and the poll's state says it resolved:
    // whichever check runs first refuses the vote
    assert_matches!(
        error,
        ConsensusError::StateError(StateError::PrefundedSpecializedBalanceNotFoundError(_))
            | ConsensusError::StateError(
                StateError::IdentityContenderVotePollNotAvailableForVotingError(_)
            )
    );
}

#[tokio::test]
async fn should_break_a_tie_by_the_earliest_contender() {
    let (mut platform, platform_version) = setup();

    // Same block, so the lower reference id decides among the tied contenders
    let vote_poll = poll_for(1);
    open_poll(&platform, &vote_poll, platform_version);
    let higher_reference = add_contender(
        &platform,
        &vote_poll,
        0xa1,
        block(10, 2),
        9,
        platform_version,
    );
    let lower_reference = add_contender(
        &platform,
        &vote_poll,
        0xb2,
        block(10, 2),
        4,
        platform_version,
    );
    let later = add_contender(
        &platform,
        &vote_poll,
        0xc3,
        block(20, 3),
        1,
        platform_version,
    );
    end_phases_at(&platform, JOIN_END_MS + 1, 10, platform_version);
    for (seed, choice) in [
        (1, ResourceVoteChoice::TowardsIdentity(higher_reference)),
        (2, ResourceVoteChoice::TowardsIdentity(higher_reference)),
        (3, ResourceVoteChoice::TowardsIdentity(lower_reference)),
        (4, ResourceVoteChoice::TowardsIdentity(lower_reference)),
        (5, ResourceVoteChoice::TowardsIdentity(later)),
        (6, ResourceVoteChoice::TowardsIdentity(later)),
    ] {
        let mut voter = voter(&mut platform, 100 + seed, platform_version);
        cast_vote(
            &mut platform,
            &vote_poll,
            &mut voter,
            choice,
            true,
            platform_version,
        )
        .await
        .expect("expected the vote to be accepted");
    }
    end_phases_at(&platform, VOTE_END_MS + 1, 20, platform_version);
    let stored_info = platform
        .drive
        .fetch_identity_contender_vote_poll_stored_info(&vote_poll, None, platform_version)
        .expect("expected to fetch")
        .expect("expected the stored info");
    assert_eq!(stored_info.winner(), Some(lower_reference));

    // An earlier block beats a lower reference id
    let vote_poll = poll_for(2);
    open_poll(&platform, &vote_poll, platform_version);
    let earlier_block = add_contender(
        &platform,
        &vote_poll,
        0xd4,
        block(5, 1),
        9,
        platform_version,
    );
    let lower_reference = add_contender(
        &platform,
        &vote_poll,
        0xe5,
        block(10, 2),
        1,
        platform_version,
    );
    end_phases_at(&platform, JOIN_END_MS + 1, 30, platform_version);
    for (seed, choice) in [
        (1, ResourceVoteChoice::TowardsIdentity(earlier_block)),
        (2, ResourceVoteChoice::TowardsIdentity(lower_reference)),
    ] {
        let mut voter = voter(&mut platform, 200 + seed, platform_version);
        cast_vote(
            &mut platform,
            &vote_poll,
            &mut voter,
            choice,
            true,
            platform_version,
        )
        .await
        .expect("expected the vote to be accepted");
    }
    end_phases_at(&platform, VOTE_END_MS + 1, 40, platform_version);
    let stored_info = platform
        .drive
        .fetch_identity_contender_vote_poll_stored_info(&vote_poll, None, platform_version)
        .expect("expected to fetch")
        .expect("expected the stored info");
    assert_eq!(stored_info.winner(), Some(earlier_block));
}

#[tokio::test]
async fn should_award_the_first_contender_when_nobody_votes() {
    let (platform, platform_version) = setup();
    let vote_poll = poll_for(1);
    open_poll(&platform, &vote_poll, platform_version);
    // Joined in id order the other way round, so the order of the keys does not decide
    let _last = add_contender(
        &platform,
        &vote_poll,
        0x01,
        block(30, 4),
        1,
        platform_version,
    );
    let first = add_contender(
        &platform,
        &vote_poll,
        0x02,
        block(10, 2),
        2,
        platform_version,
    );
    let _middle = add_contender(
        &platform,
        &vote_poll,
        0x03,
        block(20, 3),
        3,
        platform_version,
    );
    end_phases_at(&platform, JOIN_END_MS + 1, 10, platform_version);
    end_phases_at(&platform, VOTE_END_MS + 1, 20, platform_version);
    let stored_info = platform
        .drive
        .fetch_identity_contender_vote_poll_stored_info(&vote_poll, None, platform_version)
        .expect("expected to fetch")
        .expect("expected the stored info");
    assert_eq!(
        stored_info.status(),
        IdentityContenderVotePollStatus::Resolved
    );
    assert_eq!(stored_info.winner(), Some(first));
    let result = stored_info.result().expect("expected the result");
    assert!(result.vote_phase_held);
    // Every contender is on the record, with nobody behind it
    assert_eq!(result.resource_vote_choices.len(), 4);
    assert!(result
        .resource_vote_choices
        .iter()
        .all(|choice| choice.voters.is_empty()));
    assert!(state_of(&platform, &vote_poll, platform_version)
        .contenders
        .is_empty());
}

#[tokio::test]
async fn should_resolve_a_single_contender_at_the_end_of_the_join_phase() {
    let (mut platform, platform_version) = setup();
    let vote_poll = poll_for(1);
    open_poll(&platform, &vote_poll, platform_version);
    let only = add_contender(
        &platform,
        &vote_poll,
        0xa1,
        block(10, 2),
        1,
        platform_version,
    );
    end_phases_at(&platform, JOIN_END_MS + 1, 10, platform_version);
    let stored_info = platform
        .drive
        .fetch_identity_contender_vote_poll_stored_info(&vote_poll, None, platform_version)
        .expect("expected to fetch")
        .expect("expected the stored info");
    assert_eq!(
        stored_info.status(),
        IdentityContenderVotePollStatus::Resolved
    );
    assert_eq!(stored_info.winner(), Some(only));
    let result = stored_info.result().expect("expected the result");
    assert!(!result.vote_phase_held);
    assert_eq!(result.finalization_block.height, 10);
    // No vote phase: no end date entry, no contender, no balance
    assert!(end_date_entries(&platform, platform_version).is_empty());
    assert!(state_of(&platform, &vote_poll, platform_version)
        .contenders
        .is_empty());
    assert_eq!(
        platform
            .drive
            .fetch_prefunded_specialized_balance(
                vote_poll.specialized_balance_id().unwrap().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch the balance"),
        None
    );
    let mut late = voter(&mut platform, 1, platform_version);
    assert!(cast_vote(
        &mut platform,
        &vote_poll,
        &mut late,
        ResourceVoteChoice::TowardsIdentity(only),
        false,
        platform_version,
    )
    .await
    .is_err());

    // No contender at all: resolved without a winner
    let empty_poll = poll_for(2);
    open_poll(&platform, &empty_poll, platform_version);
    end_phases_at(&platform, JOIN_END_MS + 1, 11, platform_version);
    let stored_info = platform
        .drive
        .fetch_identity_contender_vote_poll_stored_info(&empty_poll, None, platform_version)
        .expect("expected to fetch")
        .expect("expected the stored info");
    assert_eq!(
        stored_info.status(),
        IdentityContenderVotePollStatus::Resolved
    );
    assert_eq!(stored_info.winner(), None);
}

#[tokio::test]
async fn should_refuse_lock_votes_and_votes_towards_non_contenders() {
    let (mut platform, platform_version) = setup();
    let vote_poll = poll_for(1);
    open_poll(&platform, &vote_poll, platform_version);
    let alice = add_contender(
        &platform,
        &vote_poll,
        0xa1,
        block(10, 2),
        1,
        platform_version,
    );
    let _bob = add_contender(
        &platform,
        &vote_poll,
        0xb2,
        block(20, 3),
        2,
        platform_version,
    );
    end_phases_at(&platform, JOIN_END_MS + 1, 10, platform_version);

    let mut voter = voter(&mut platform, 1, platform_version);
    let error = cast_vote(
        &mut platform,
        &vote_poll,
        &mut voter,
        ResourceVoteChoice::Lock,
        false,
        platform_version,
    )
    .await
    .expect_err("expected the lock vote to be refused");
    assert_matches!(
        error,
        ConsensusError::StateError(StateError::VoteChoiceNotAllowedForVotePollError(ref e))
            if e.vote_choice() == ResourceVoteChoice::Lock
    );

    let stranger = Identifier::new([0xee; 32]);
    let error = cast_vote(
        &mut platform,
        &vote_poll,
        &mut voter,
        ResourceVoteChoice::TowardsIdentity(stranger),
        false,
        platform_version,
    )
    .await
    .expect_err("expected the vote towards a non contender to be refused");
    assert_matches!(
        error,
        ConsensusError::StateError(StateError::VoteChoiceNotAllowedForVotePollError(ref e))
            if e.vote_choice() == ResourceVoteChoice::TowardsIdentity(stranger)
    );

    // The same masternode may still vote for a contender, and then change its mind once
    cast_vote(
        &mut platform,
        &vote_poll,
        &mut voter,
        ResourceVoteChoice::TowardsIdentity(alice),
        true,
        platform_version,
    )
    .await
    .expect("expected the vote to be accepted");
    let error = cast_vote(
        &mut platform,
        &vote_poll,
        &mut voter,
        ResourceVoteChoice::TowardsIdentity(alice),
        false,
        platform_version,
    )
    .await
    .expect_err("expected the repeated vote to be refused");
    assert_matches!(
        error,
        ConsensusError::StateError(StateError::MasternodeVoteAlreadyPresentError(_))
    );
    cast_vote(
        &mut platform,
        &vote_poll,
        &mut voter,
        ResourceVoteChoice::Abstain,
        true,
        platform_version,
    )
    .await
    .expect("expected the changed vote to be accepted");
    let state = state_of(&platform, &vote_poll, platform_version);
    assert_eq!(tally_of(&state, alice), Some(0));
    assert_eq!(state.abstain_vote_tally, Some(1));
    assert_eq!(
        platform
            .drive
            .fetch_identity_contender_vote_poll_identity_vote(
                voter.pro_tx_hash,
                vote_poll.unique_id().unwrap(),
                None,
                &mut vec![],
                platform_version,
            )
            .expect("expected to fetch the reference"),
        Some((ResourceVoteChoice::Abstain, 2))
    );
}

#[tokio::test]
async fn should_remove_the_votes_of_masternodes_that_left() {
    let (mut platform, platform_version) = setup();
    let vote_poll = poll_for(1);
    open_poll(&platform, &vote_poll, platform_version);
    let alice = add_contender(
        &platform,
        &vote_poll,
        0xa1,
        block(10, 2),
        1,
        platform_version,
    );
    let _bob = add_contender(
        &platform,
        &vote_poll,
        0xb2,
        block(20, 3),
        2,
        platform_version,
    );
    end_phases_at(&platform, JOIN_END_MS + 1, 10, platform_version);
    let mut leaving = voter(&mut platform, 1, platform_version);
    let mut staying = voter(&mut platform, 2, platform_version);
    for voter in [&mut leaving, &mut staying] {
        cast_vote(
            &mut platform,
            &vote_poll,
            voter,
            ResourceVoteChoice::TowardsIdentity(alice),
            true,
            platform_version,
        )
        .await
        .expect("expected the vote to be accepted");
    }
    assert_eq!(
        tally_of(&state_of(&platform, &vote_poll, platform_version), alice),
        Some(2)
    );

    platform
        .drive
        .remove_all_votes_given_by_identities(
            vec![leaving.pro_tx_hash.to_vec()],
            11,
            platform.config.network,
            platform.config.abci.chain_id.as_str(),
            None,
            platform_version,
        )
        .expect("expected to remove the votes of the masternode that left");

    assert_eq!(
        tally_of(&state_of(&platform, &vote_poll, platform_version), alice),
        Some(1)
    );
    assert_eq!(
        platform
            .drive
            .fetch_identity_contender_vote_poll_identity_vote(
                leaving.pro_tx_hash,
                vote_poll.unique_id().unwrap(),
                None,
                &mut vec![],
                platform_version,
            )
            .expect("expected to fetch the reference"),
        None
    );
    assert_eq!(
        platform
            .drive
            .fetch_identity_contender_vote_poll_identity_vote(
                staying.pro_tx_hash,
                vote_poll.unique_id().unwrap(),
                None,
                &mut vec![],
                platform_version,
            )
            .expect("expected to fetch the reference"),
        Some((ResourceVoteChoice::TowardsIdentity(alice), 1))
    );
}

#[tokio::test]
async fn should_serve_the_poll_state_with_and_without_a_proof() {
    let (mut platform, platform_version) = setup();
    let vote_poll = poll_for(1);
    open_poll(&platform, &vote_poll, platform_version);
    let alice = add_contender(
        &platform,
        &vote_poll,
        0xa1,
        block(10, 2),
        1,
        platform_version,
    );
    let bob = add_contender(
        &platform,
        &vote_poll,
        0xb2,
        block(20, 3),
        2,
        platform_version,
    );
    end_phases_at(&platform, JOIN_END_MS + 1, 10, platform_version);
    let mut voter = voter(&mut platform, 1, platform_version);
    cast_vote(
        &mut platform,
        &vote_poll,
        &mut voter,
        ResourceVoteChoice::TowardsIdentity(bob),
        true,
        platform_version,
    )
    .await
    .expect("expected the vote to be accepted");
    let platform_state = platform.state.load();

    let request = |vote_poll_id: Vec<u8>, prove: bool| GetIdentityContenderVotePollStateRequest {
        version: Some(RequestVersion::V0(
            GetIdentityContenderVotePollStateRequestV0 {
                vote_poll_id,
                start_at_identifier_info: None,
                count: None,
                prove,
            },
        )),
    };
    let vote_poll_id = vote_poll.unique_id().expect("expected the poll id");

    // Without a proof
    let response = platform
        .query_identity_contender_vote_poll_state(
            request(vote_poll_id.to_vec(), false),
            &platform_state,
            platform_version,
        )
        .expect("expected to query")
        .into_data()
        .expect("expected data");
    let ResponseVersion::V0(response) = response.version.expect("expected a version");
    let Some(ResultV0::State(state)) = response.result else {
        panic!("expected the state");
    };
    let info = state.info.expect("expected the poll info");
    assert_eq!(info.status, Status::Voting as i32);
    assert_eq!(info.join_end_time_ms, JOIN_END_MS);
    assert_eq!(info.vote_end_time_ms, VOTE_END_MS);
    assert_eq!(info.finished_vote_info, None);
    assert_eq!(state.abstain_vote_tally, Some(0));
    let contenders: Vec<(Vec<u8>, Option<u32>, u64)> = state
        .contenders
        .iter()
        .map(|contender| {
            (
                contender.identity_id.clone(),
                contender.vote_tally,
                contender.joined_at_block_height,
            )
        })
        .collect();
    assert_eq!(
        contenders,
        vec![(alice.to_vec(), Some(0), 2), (bob.to_vec(), Some(1), 3)]
    );

    // With a proof, verified with the same query the node ran
    let response = platform
        .query_identity_contender_vote_poll_state(
            request(vote_poll_id.to_vec(), true),
            &platform_state,
            platform_version,
        )
        .expect("expected to query")
        .into_data()
        .expect("expected data");
    let ResponseVersion::V0(response) = response.version.expect("expected a version");
    let Some(ResultV0::Proof(proof)) = response.result else {
        panic!("expected a proof");
    };
    let query = IdentityContenderVotePollStateQuery {
        vote_poll_id,
        limit: Some(platform.config.drive.default_query_limit),
        start_at: None,
    };
    let (root_hash, proved_state) = query
        .verify_identity_contender_vote_poll_state_proof(&proof.grovedb_proof, platform_version)
        .expect("expected the proof to verify");
    assert_eq!(
        root_hash,
        platform
            .drive
            .grove
            .root_hash(None, &platform_version.drive.grove_version)
            .unwrap()
            .unwrap()
    );
    assert_eq!(
        proved_state,
        state_of(&platform, &vote_poll, platform_version)
    );
    assert_eq!(proved_state.contenders.len(), 2);
    assert_eq!(proved_state.abstain_vote_tally, Some(0));
    assert_eq!(
        proved_state.stored_info.map(|info| info.status()),
        Some(IdentityContenderVotePollStatus::Voting)
    );

    // A poll that never opened proves absent
    let unknown = poll_for(9).unique_id().expect("expected the poll id");
    let response = platform
        .query_identity_contender_vote_poll_state(
            request(unknown.to_vec(), true),
            &platform_state,
            platform_version,
        )
        .expect("expected to query")
        .into_data()
        .expect("expected data");
    let ResponseVersion::V0(response) = response.version.expect("expected a version");
    let Some(ResultV0::Proof(proof)) = response.result else {
        panic!("expected a proof");
    };
    let query = IdentityContenderVotePollStateQuery {
        vote_poll_id: unknown,
        limit: Some(platform.config.drive.default_query_limit),
        start_at: None,
    };
    let (_, proved_state) = query
        .verify_identity_contender_vote_poll_state_proof(&proof.grovedb_proof, platform_version)
        .expect("expected the absence proof to verify");
    assert_eq!(proved_state.stored_info, None);
    assert!(proved_state.contenders.is_empty());

    // A malformed id is refused
    let result = platform
        .query_identity_contender_vote_poll_state(
            request(vec![1, 2, 3], false),
            &platform_state,
            platform_version,
        )
        .expect("expected to query");
    assert!(!result.is_valid());
}
