use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    pub(super) fn run_dao_platform_events_v0(
        &self,
        block_info: &BlockInfo,
        last_committed_platform_state: &PlatformState,
        block_platform_state: &PlatformState,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        // Remove any votes that

        self.remove_votes_for_removed_masternodes(
            block_info,
            last_committed_platform_state,
            block_platform_state,
            transaction,
            platform_version,
        )?;

        // Check for any vote polls that might have ended

        self.check_for_ended_vote_polls(
            last_committed_platform_state,
            block_platform_state,
            block_info,
            transaction,
            platform_version,
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::block::block_info::BlockInfo;
    use dpp::version::PlatformVersion;

    /// On a genesis-state platform with no masternode changes and no ended vote
    /// polls, `run_dao_platform_events_v0` must return `Ok(())` — both internal
    /// calls (remove_votes, check_for_ended_vote_polls) must succeed with
    /// no-op behavior.
    #[test]
    fn v0_is_ok_with_no_state_change_on_empty_platform() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let state = platform.state.load();
        let a = state.as_ref().clone();
        let b = state.as_ref().clone();
        drop(state);

        let transaction = platform.drive.grove.start_transaction();
        let block_info = BlockInfo {
            time_ms: 1_234,
            height: 1,
            core_height: 1,
            epoch: Default::default(),
        };

        platform
            .run_dao_platform_events_v0(&block_info, &a, &b, Some(&transaction), platform_version)
            .expect("must succeed with no state changes");
    }

    /// Repeated invocation with no state change must remain Ok — the two
    /// delegated operations are idempotent in the empty case.
    #[test]
    fn v0_idempotent_on_empty_platform() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let state = platform.state.load();
        let transaction = platform.drive.grove.start_transaction();
        let block_info = BlockInfo::default_with_time(0);

        for _ in 0..3 {
            let a = state.as_ref().clone();
            let b = state.as_ref().clone();
            platform
                .run_dao_platform_events_v0(
                    &block_info,
                    &a,
                    &b,
                    Some(&transaction),
                    platform_version,
                )
                .expect("idempotent");
        }
    }
}
