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
    /// Cleans up after the polls that ended in this block. Version 1 (protocol version 14)
    /// handles both kinds. It removes every finished poll from the end date index itself,
    /// so that an end date's tree is deleted only once nothing of either kind is left under
    /// it, then cleans up the polls of each kind.
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
        let mut yes_no_polls: Vec<(
            &YesNoVotePoll,
            &BTreeMap<YesNoAbstainVoteChoice, Vec<Identifier>>,
        )> = Vec::new();
        let mut end_date_entries: Vec<(Identifier, TimestampMillis)> = Vec::new();

        for (end_date, vote_polls_for_time) in vote_polls {
            for vote_poll in vote_polls_for_time {
                match vote_poll {
                    ResolvedVotePollWithVotes::ContestedDocumentResourceVotePollWithContractInfoAndVotes(contested_poll, vote_info) => {
                        end_date_entries.push((contested_poll.unique_id()?, *end_date));
                        contested_polls.push((contested_poll, end_date, vote_info));
                    }
                    ResolvedVotePollWithVotes::YesNoVotePollWithVotes(yes_no_poll, voters) => {
                        end_date_entries.push((yes_no_poll.unique_id()?, *end_date));
                        yes_no_polls.push((yes_no_poll, voters));
                    }
                }
            }
        }

        if !end_date_entries.is_empty() {
            let mut operations = vec![];
            self.drive.remove_vote_poll_end_date_query_operations(
                end_date_entries.as_slice(),
                &mut operations,
                transaction,
                platform_version,
            )?;
            self.drive.apply_batch_low_level_drive_operations(
                None,
                transaction,
                operations,
                &mut vec![],
                &platform_version.drive,
            )?;
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
