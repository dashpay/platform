use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::identifier::Identifier;
use dpp::prelude::TimestampMillis;
use dpp::version::PlatformVersion;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use dpp::voting::vote_choices::yes_no_abstain_vote_choice::YesNoAbstainVoteChoice;
use dpp::voting::vote_polls::yes_no_vote_poll::YesNoVotePoll;
use drive::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
use drive::drive::votes::resolved::vote_polls::ResolvedVotePollWithVotes;
use drive::grovedb::TransactionArg;
use std::collections::BTreeMap;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Cleans up after the polls that ended in this block. Version 1 (protocol version 15)
    /// handles both kinds: the contested polls go to the contested clean-up and the yes/no
    /// polls to the yes/no clean-up. Each removes its own polls from the shared end date index
    /// and applies its own batch, and each deletes an end date's tree only once its read of
    /// the tree finds nothing of either kind left, so the two can run one after the other.
    #[inline(always)]
    pub(super) fn clean_up_after_vote_polls_end_v1(
        &self,
        block_info: &BlockInfo,
        vote_polls: &BTreeMap<TimestampMillis, Vec<ResolvedVotePollWithVotes>>,
        clean_up_testnet_corrupted_reference_issue: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        // TODO: Use type or struct
        #[allow(clippy::type_complexity)]
        let mut contested_polls: Vec<(
            &ContestedDocumentResourceVotePollWithContractInfo,
            &TimestampMillis,
            &BTreeMap<ResourceVoteChoice, Vec<Identifier>>,
        )> = Vec::new();
        #[allow(clippy::type_complexity)]
        let mut yes_no_polls: Vec<(
            &YesNoVotePoll,
            &TimestampMillis,
            &BTreeMap<YesNoAbstainVoteChoice, Vec<Identifier>>,
        )> = Vec::new();

        for (end_date, vote_polls_for_time) in vote_polls {
            for vote_poll in vote_polls_for_time {
                match vote_poll {
                    ResolvedVotePollWithVotes::ContestedDocumentResourceVotePollWithContractInfoAndVotes(contested_poll, vote_info) => {
                        contested_polls.push((contested_poll, end_date, vote_info));
                    }
                    ResolvedVotePollWithVotes::YesNoVotePollWithVotes(yes_no_poll, voters) => {
                        yes_no_polls.push((yes_no_poll, end_date, voters));
                    }
                }
            }
        }

        if !contested_polls.is_empty() {
            self.clean_up_after_contested_resources_vote_polls_end(
                block_info,
                contested_polls,
                clean_up_testnet_corrupted_reference_issue,
                transaction,
                platform_version,
            )?;
        }

        if !yes_no_polls.is_empty() {
            self.clean_up_after_yes_no_vote_polls_end(
                block_info,
                yes_no_polls,
                transaction,
                platform_version,
            )?;
        }

        Ok(())
    }
}
