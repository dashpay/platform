use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;

mod v0;
mod v1;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Checks for ended vote polls and finalizes each one: awards the winner, records the
    /// finished poll and cleans it up, all on the block transaction.
    ///
    /// Generation 0 selects the winner in this crate and inserts it through the generic
    /// document insert. Generation 1 (protocol version 17) delegates selection and insert to
    /// `Drive::award_contested_document_vote_poll`, the native award operation that
    /// re-derives the winner from state and takes no contender.
    pub(in crate::execution) fn check_for_ended_vote_polls(
        &self,
        last_committed_platform_state: &PlatformState,
        block_platform_state: &PlatformState,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive_abci
            .methods
            .voting
            .check_for_ended_vote_polls
        {
            0 => self.check_for_ended_vote_polls_v0(
                last_committed_platform_state,
                block_platform_state,
                block_info,
                transaction,
                platform_version,
            ),
            1 => self.check_for_ended_vote_polls_v1(
                block_platform_state,
                block_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "check_for_ended_vote_polls".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    //! The finalization event through the dispatcher, on both sides of the gate: generation 0
    //! (pinned at protocol version 14, the last version that selects it) and generation 1
    //! (the latest version) award the same winner and leave the same record, and neither
    //! awards before the end date.

    use crate::execution::validation::state_transition::tests::{
        create_dpns_identity_name_contest, get_vote_states, perform_votes_multi,
    };
    use crate::platform_types::platform_state::PlatformStateV0Methods;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dapi_grpc::platform::v0::get_contested_resource_vote_state_request::get_contested_resource_vote_state_request_v0::ResultType;
    use dapi_grpc::platform::v0::get_contested_resource_vote_state_response::get_contested_resource_vote_state_response_v0::{
        finished_vote_info, FinishedVoteInfo,
    };
    use dpp::block::block_info::BlockInfo;
    use dpp::block::extended_block_info::v0::ExtendedBlockInfoV0;
    use dpp::identifier::Identifier;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::version::PlatformVersion;
    use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
    use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice::TowardsIdentity;
    use drive::query::VotePollsByEndDateDriveQuery;
    use std::sync::Arc;

    /// Opens a DPNS contest for "quantum" with votes 5 to 50 in favour of the second
    /// contender, sweeps for ended polls at `sweep_time_ms`, and returns the finished vote
    /// info the vote state query reports afterwards together with the ids of the two
    /// contenders and the number of polls still queued by end date.
    async fn contest_then_sweep(
        protocol_version: u32,
        sweep_time_ms: u64,
    ) -> (
        Option<FinishedVoteInfo>,
        [dpp::prelude::Identifier; 2],
        usize,
    ) {
        let platform_version =
            PlatformVersion::get(protocol_version).expect("expected a known platform version");
        let mut platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(protocol_version)
            .build_with_mock_rpc()
            .set_genesis_state();

        let platform_state = platform.state.load();

        let (contender_1, contender_2, dpns_contract) = create_dpns_identity_name_contest(
            &mut platform,
            &platform_state,
            7,
            "quantum",
            platform_version,
        )
        .await;

        perform_votes_multi(
            &mut platform,
            dpns_contract.as_ref(),
            vec![
                (TowardsIdentity(contender_1.id()), 5),
                (TowardsIdentity(contender_2.id()), 50),
                (ResourceVoteChoice::Abstain, 10),
                (ResourceVoteChoice::Lock, 3),
            ],
            "quantum",
            10,
            None,
            platform_version,
        )
        .await;

        let platform_state = platform.state.load();
        let mut platform_state = (**platform_state).clone();

        let block_info = BlockInfo {
            time_ms: sweep_time_ms,
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

        let (contenders, _, _, finished_vote_info) = get_vote_states(
            &platform,
            &platform_state,
            &dpns_contract,
            "quantum",
            None,
            true,
            None,
            ResultType::DocumentsAndVoteTally,
            platform_version,
        );

        assert_eq!(contenders.len(), 2);

        let queued_polls =
            VotePollsByEndDateDriveQuery::execute_no_proof_for_specialized_end_time_query(
                u64::MAX >> 1,
                100,
                &platform.drive,
                None,
                &mut vec![],
                platform_version,
            )
            .expect("expected to query the end date queue")
            .values()
            .map(|polls| polls.len())
            .sum();

        (
            finished_vote_info,
            [contender_1.id(), contender_2.id()],
            queued_polls,
        )
    }

    /// Well past the 90 minute test network poll duration.
    const AFTER_THE_END_MS: u64 = 1_209_900_000;

    fn expected_award(winner: Identifier) -> FinishedVoteInfo {
        FinishedVoteInfo {
            finished_vote_outcome: finished_vote_info::FinishedVoteOutcome::TowardsIdentity as i32,
            won_by_identity_id: Some(winner.to_vec()),
            finished_at_block_height: 10000,
            finished_at_core_block_height: 42,
            finished_at_block_time_ms: AFTER_THE_END_MS,
            finished_at_epoch: 0,
        }
    }

    #[tokio::test]
    async fn should_award_the_top_contender_through_the_native_award_at_the_latest_version() {
        let (finished_vote_info, [_, contender_2], queued_polls) =
            contest_then_sweep(PlatformVersion::latest().protocol_version, AFTER_THE_END_MS).await;

        assert_eq!(finished_vote_info, Some(expected_award(contender_2)));
        assert_eq!(
            queued_polls, 0,
            "the finalized poll leaves the end date queue"
        );
    }

    /// Generation 0, pinned at protocol version 14 (the last version selecting it), selects
    /// the same winner and leaves the same record: the selection that moved into Drive is
    /// the one it always applied.
    #[tokio::test]
    async fn should_award_the_same_winner_through_generation_0_at_protocol_version_14() {
        let (finished_vote_info, [_, contender_2], queued_polls) =
            contest_then_sweep(14, AFTER_THE_END_MS).await;

        assert_eq!(finished_vote_info, Some(expected_award(contender_2)));
        assert_eq!(
            queued_polls, 0,
            "the finalized poll leaves the end date queue"
        );
    }

    #[tokio::test]
    async fn should_award_nothing_before_the_end_date() {
        let platform_version = PlatformVersion::latest();
        let before_the_end_ms = platform_version
            .dpp
            .voting_versions
            .default_vote_poll_time_duration_test_network_ms
            - 1_000;

        let (finished_vote_info, _, queued_polls) =
            contest_then_sweep(platform_version.protocol_version, before_the_end_ms).await;

        assert_eq!(finished_vote_info, None);
        assert_eq!(queued_polls, 1, "the poll stays queued until its end date");
    }
}
