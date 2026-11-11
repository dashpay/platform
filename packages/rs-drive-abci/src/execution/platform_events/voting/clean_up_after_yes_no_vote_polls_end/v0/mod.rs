use super::FinishedYesNoVotePoll;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::identifier::Identifier;
use dpp::prelude::TimestampMillis;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    #[inline(always)]
    pub(super) fn clean_up_after_yes_no_vote_polls_end_v0(
        &self,
        block_info: &BlockInfo,
        end_date_entries: &[(Identifier, TimestampMillis)],
        finished_vote_polls: &[FinishedYesNoVotePoll],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let mut operations = vec![];

        // The polls' entries in the end date index, which contested polls share: an end date's
        // tree goes only once nothing of either kind is left under it.
        self.drive.remove_vote_poll_end_date_query_operations(
            end_date_entries,
            &mut operations,
            transaction,
            platform_version,
        )?;

        // The three vote trees of every poll this block closed.
        let vote_poll_ids: Vec<Identifier> = finished_vote_polls
            .iter()
            .map(|vote_poll| vote_poll.poll_id)
            .collect();
        self.drive.remove_yes_no_vote_poll_votes_operations(
            vote_poll_ids.as_slice(),
            &mut operations,
            platform_version,
        )?;

        // Each voter's entry in the identity votes index.
        for vote_poll in finished_vote_polls {
            for voter in &vote_poll.voters {
                self.drive
                    .remove_yes_no_vote_references_given_by_identity_operations(
                        voter,
                        &[&vote_poll.poll_id],
                        &mut operations,
                        transaction,
                        platform_version,
                    )?;
            }
        }

        // What is left of the prefunded balances goes to processing, as for contested polls.
        self.empty_vote_poll_prefunded_balances_operations_v1(
            finished_vote_polls
                .iter()
                .map(|vote_poll| Ok(vote_poll.prefunded_balance_id)),
            block_info,
            &mut operations,
            transaction,
            platform_version,
        )?;

        if !operations.is_empty() {
            self.drive.apply_batch_low_level_drive_operations(
                None,
                transaction,
                operations,
                &mut vec![],
                &platform_version.drive,
            )?;
        }
        Ok(())
    }
}
