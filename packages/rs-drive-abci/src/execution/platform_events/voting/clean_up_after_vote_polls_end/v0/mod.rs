use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::identifier::Identifier;
use dpp::prelude::TimestampMillis;
use dpp::version::PlatformVersion;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use drive::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
use drive::drive::votes::resolved::vote_polls::ResolvedVotePollWithVotes;
use drive::grovedb::TransactionArg;
use std::collections::BTreeMap;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Checks for ended vote polls
    #[inline(always)]
    pub(super) fn clean_up_after_vote_polls_end_v0(
        &self,
        block_info: &BlockInfo,
        vote_polls: &BTreeMap<TimestampMillis, Vec<ResolvedVotePollWithVotes>>,
        clean_up_testnet_corrupted_reference_issue: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        // Create a vector to hold the references to the contested document resource vote polls
        // TODO: Use type or struct
        #[allow(clippy::type_complexity)]
        let mut contested_polls: Vec<(
            &ContestedDocumentResourceVotePollWithContractInfo,
            &TimestampMillis,
            &BTreeMap<ResourceVoteChoice, Vec<Identifier>>,
        )> = Vec::new();

        // Iterate over the vote polls and match on the enum variant
        for (end_date, vote_polls_for_time) in vote_polls {
            for vote_poll in vote_polls_for_time {
                match vote_poll {
                    ResolvedVotePollWithVotes::ContestedDocumentResourceVotePollWithContractInfoAndVotes(contested_poll, vote_info) => {
                        contested_polls.push((contested_poll, end_date, vote_info));
                    } // Add more match arms here for other types of vote polls in the future
                }
            }
        }

        if !contested_polls.is_empty() {
            // Call the function to clean up contested document resource vote polls
            self.clean_up_after_contested_resources_vote_polls_end(
                block_info,
                contested_polls,
                clean_up_testnet_corrupted_reference_issue,
                transaction,
                platform_version,
            )
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::block::block_info::BlockInfo;
    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;

    /// Empty vote_polls map yields zero contested polls, so the function takes
    /// the `else` branch and returns `Ok(())` without touching storage.
    #[test]
    fn v0_empty_vote_polls_returns_ok_without_cleanup() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let transaction = platform.drive.grove.start_transaction();
        let block_info = BlockInfo {
            time_ms: 1_234,
            height: 1,
            core_height: 1,
            epoch: Default::default(),
        };
        let empty: BTreeMap<dpp::prelude::TimestampMillis, Vec<_>> = BTreeMap::new();

        platform
            .clean_up_after_vote_polls_end_v0(
                &block_info,
                &empty,
                false,
                Some(&transaction),
                platform_version,
            )
            .expect("empty map must be a no-op");
    }

    /// A `BTreeMap` where timestamps map to *empty* vectors still has no contested
    /// polls after the drain-style iteration, so we again take the else branch.
    /// This guards against a regression where an empty inner vec is mishandled.
    #[test]
    fn v0_vote_polls_with_empty_timestamps_is_noop() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let transaction = platform.drive.grove.start_transaction();
        let block_info = BlockInfo {
            time_ms: 1_000,
            height: 1,
            core_height: 1,
            epoch: Default::default(),
        };

        let mut vote_polls: BTreeMap<dpp::prelude::TimestampMillis, Vec<_>> = BTreeMap::new();
        vote_polls.insert(100, Vec::new());
        vote_polls.insert(200, Vec::new());

        platform
            .clean_up_after_vote_polls_end_v0(
                &block_info,
                &vote_polls,
                false,
                Some(&transaction),
                platform_version,
            )
            .expect("map of empty buckets must be a no-op");
    }

    /// The testnet cleanup flag does NOT influence the empty-path (otherwise an
    /// empty input could inadvertently do work). Verify both flag values succeed
    /// identically on empty input.
    #[test]
    fn v0_testnet_flag_has_no_effect_on_empty_input() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let transaction = platform.drive.grove.start_transaction();
        let block_info = BlockInfo::default_with_time(0);
        let empty: BTreeMap<dpp::prelude::TimestampMillis, Vec<_>> = BTreeMap::new();

        for flag in [false, true] {
            platform
                .clean_up_after_vote_polls_end_v0(
                    &block_info,
                    &empty,
                    flag,
                    Some(&transaction),
                    platform_version,
                )
                .expect("empty input must succeed regardless of testnet flag");
        }
    }
}
