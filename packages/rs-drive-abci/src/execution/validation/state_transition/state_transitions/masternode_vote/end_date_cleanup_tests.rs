//! The end-date cleanup of ended contested vote polls: a block ends at most
//! `maximum_vote_polls_to_process` vote polls, taken across their end dates in order, and an end
//! date is removed once none of its vote polls remain.

use crate::execution::validation::state_transition::state_transitions::tests::{
    create_dpns_identity_name_contest, dpns_name_vote_poll, get_vote_states,
};
use crate::rpc::core::MockCoreRPCLike;
use crate::test::helpers::fast_forward_to_block::fast_forward_to_block;
use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
use dapi_grpc::platform::v0::get_contested_resource_vote_state_request::get_contested_resource_vote_state_request_v0::ResultType;
use dapi_grpc::platform::v0::get_contested_resource_vote_state_response::get_contested_resource_vote_state_response_v0::FinishedVoteInfo;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::DataContract;
use dpp::prelude::TimestampMillis;
use dpp::version::PlatformVersion;
use dpp::voting::vote_polls::VotePoll;
use drive::drive::votes::paths::vote_end_date_queries_tree_path_vec;
use drive::query::{PathQuery, Query, QueryResultType, VotePollsByEndDateDriveQuery};
use std::collections::BTreeMap;

/// Every vote poll listed under an end date, by end date
fn listed_vote_polls(
    platform: &TempPlatform<MockCoreRPCLike>,
    platform_version: &PlatformVersion,
) -> BTreeMap<TimestampMillis, Vec<VotePoll>> {
    VotePollsByEndDateDriveQuery::execute_no_proof_for_specialized_end_time_query(
        TimestampMillis::MAX >> 1,
        u16::MAX,
        &platform.drive,
        None,
        &mut vec![],
        platform_version,
    )
    .expect("expected to list the vote polls by end date")
}

/// How many end dates exist, including any left with no vote poll under it
fn end_date_count(
    platform: &TempPlatform<MockCoreRPCLike>,
    platform_version: &PlatformVersion,
) -> usize {
    let mut query = Query::new();
    query.insert_all();
    platform
        .drive
        .grove_get_raw_path_query(
            &PathQuery::new_unsized(vote_end_date_queries_tree_path_vec(), query),
            None,
            QueryResultType::QueryKeyElementPairResultType,
            &mut vec![],
            &platform_version.drive,
        )
        .expect("expected to read the end dates")
        .0
        .len()
}

fn finished_vote_info(
    platform: &TempPlatform<MockCoreRPCLike>,
    dpns_contract: &DataContract,
    name: &str,
    platform_version: &PlatformVersion,
) -> Option<FinishedVoteInfo> {
    let platform_state = platform.state.load();
    get_vote_states(
        platform,
        &platform_state,
        dpns_contract,
        name,
        None,
        true,
        None,
        ResultType::DocumentsAndVoteTally,
        platform_version,
    )
    .3
}

/// Runs the ended vote poll check of a block at `time_ms` and commits it
fn end_due_vote_polls(
    platform: &TempPlatform<MockCoreRPCLike>,
    time_ms: TimestampMillis,
    height: u64,
    platform_version: &PlatformVersion,
) {
    fast_forward_to_block(platform, time_ms, height, 42, 0, false);
    let platform_state = platform.state.load();
    let transaction = platform.drive.grove.start_transaction();
    platform
        .check_for_ended_vote_polls(
            &platform_state,
            &platform_state,
            &BlockInfo {
                time_ms,
                height,
                core_height: 42,
                epoch: Default::default(),
            },
            Some(&transaction),
            platform_version,
        )
        .expect("expected the block to end its due vote polls");
    platform
        .drive
        .grove
        .commit_transaction(transaction)
        .unwrap()
        .expect("expected to commit the block");
}

#[tokio::test]
async fn should_end_vote_polls_across_end_dates_and_keep_the_end_date_of_the_one_left() {
    let platform_version = PlatformVersion::latest();
    let mut platform = TestPlatformBuilder::new()
        .with_latest_protocol_version()
        .build_with_mock_rpc()
        .set_genesis_state();

    // A ends at T1, B and C end together at T2
    let platform_state = platform.state.load();
    let (_, _, dpns_contract) = create_dpns_identity_name_contest(
        &mut platform,
        &platform_state,
        7,
        "quantum",
        platform_version,
    )
    .await;
    fast_forward_to_block(&platform, 500_000, 100, 42, 0, false);
    let platform_state = platform.state.load();
    for (seed, name) in [(8, "coolio"), (9, "crazyman")] {
        create_dpns_identity_name_contest(
            &mut platform,
            &platform_state,
            seed,
            name,
            platform_version,
        )
        .await;
    }

    let names = ["quantum", "coolio", "crazyman"];
    let name_of = |vote_poll: &VotePoll| {
        names
            .into_iter()
            .find(|name| {
                *vote_poll
                    == VotePoll::ContestedDocumentResourceVotePoll(dpns_name_vote_poll(
                        &dpns_contract,
                        name,
                    ))
            })
            .expect("expected one of the contests")
    };

    let listed = listed_vote_polls(&platform, platform_version);
    let [(t1, at_t1), (t2, at_t2)]: [(TimestampMillis, Vec<VotePoll>); 2] = listed
        .into_iter()
        .collect::<Vec<_>>()
        .try_into()
        .expect("expected two end dates");
    assert_eq!(at_t1.iter().map(name_of).collect::<Vec<_>>(), ["quantum"]);
    let mut names_at_t2 = at_t2.iter().map(name_of).collect::<Vec<_>>();
    names_at_t2.sort();
    assert_eq!(names_at_t2, ["coolio", "crazyman"]);
    assert!(t1 < t2);

    // The first block past T2 ends two vote polls, the most one block ends: A, then the
    // first listed at T2
    end_due_vote_polls(&platform, t2, 200, platform_version);

    let ended_at_t2 = name_of(&at_t2[0]);
    let left_at_t2 = name_of(&at_t2[1]);
    assert!(finished_vote_info(&platform, &dpns_contract, "quantum", platform_version).is_some());
    assert!(finished_vote_info(&platform, &dpns_contract, ended_at_t2, platform_version).is_some());
    assert_eq!(
        finished_vote_info(&platform, &dpns_contract, left_at_t2, platform_version),
        None
    );
    assert_eq!(
        listed_vote_polls(&platform, platform_version),
        BTreeMap::from([(t2, vec![at_t2[1].clone()])])
    );
    assert_eq!(end_date_count(&platform, platform_version), 1);

    // The next block ends the vote poll left at T2 and removes T2
    end_due_vote_polls(&platform, t2 + 1_000, 201, platform_version);

    assert!(finished_vote_info(&platform, &dpns_contract, left_at_t2, platform_version).is_some());
    assert_eq!(
        listed_vote_polls(&platform, platform_version),
        BTreeMap::new()
    );
    assert_eq!(end_date_count(&platform, platform_version), 0);
}

#[tokio::test]
async fn should_end_every_vote_poll_of_an_end_date_at_the_limit_in_one_block() {
    let platform_version = PlatformVersion::latest();
    let mut platform = TestPlatformBuilder::new()
        .with_latest_protocol_version()
        .build_with_mock_rpc()
        .set_genesis_state();

    let platform_state = platform.state.load();
    let mut dpns_contract = None;
    for (seed, name) in [(7, "quantum"), (8, "coolio")] {
        let (_, _, contract) = create_dpns_identity_name_contest(
            &mut platform,
            &platform_state,
            seed,
            name,
            platform_version,
        )
        .await;
        dpns_contract = Some(contract);
    }
    let dpns_contract = dpns_contract.expect("expected the DPNS contract");

    let listed = listed_vote_polls(&platform, platform_version);
    let [(end_date, at_end_date)]: [(TimestampMillis, Vec<VotePoll>); 1] = listed
        .into_iter()
        .collect::<Vec<_>>()
        .try_into()
        .expect("expected one end date");
    assert_eq!(
        at_end_date.len(),
        platform_version
            .drive_abci
            .validation_and_processing
            .event_constants
            .maximum_vote_polls_to_process as usize
    );

    end_due_vote_polls(&platform, end_date, 200, platform_version);

    for name in ["quantum", "coolio"] {
        assert!(finished_vote_info(&platform, &dpns_contract, name, platform_version).is_some());
    }
    assert_eq!(
        listed_vote_polls(&platform, platform_version),
        BTreeMap::new()
    );
    assert_eq!(end_date_count(&platform, platform_version), 0);
}
