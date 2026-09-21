use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::fee::Credits;
use dpp::identifier::Identifier;
use dpp::prelude::TimestampMillis;
use dpp::version::PlatformVersion;
use dpp::voting::vote_polls::identity_contender_vote_poll::IdentityContenderVotePoll;
use drive::drive::votes::resolved::vote_polls::identity_contender_vote_poll::IdentityContenderVotePollEndOutcome;
use drive::grovedb::TransactionArg;
use std::collections::BTreeMap;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    #[inline(always)]
    pub(super) fn clean_up_after_identity_contender_vote_polls_end_v0(
        &self,
        block_info: &BlockInfo,
        vote_polls: Vec<(
            &IdentityContenderVotePoll,
            &TimestampMillis,
            &IdentityContenderVotePollEndOutcome,
        )>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let mut operations = vec![];

        // The end date entries of every poll whose phase ended
        let ended: Vec<(&IdentityContenderVotePoll, TimestampMillis)> = vote_polls
            .iter()
            .map(|(vote_poll, end_date, _)| (*vote_poll, **end_date))
            .collect();
        self.drive
            .remove_identity_contender_vote_poll_end_date_query_operations(
                ended.as_slice(),
                &mut operations,
                transaction,
                platform_version,
            )?;

        let mut vote_poll_ids_by_voter: BTreeMap<Identifier, Vec<Identifier>> = BTreeMap::new();
        let mut total_credits_to_add_to_processing: Credits = 0;
        for (vote_poll, _, outcome) in &vote_polls {
            let IdentityContenderVotePollEndOutcome::Resolved { votes, .. } = outcome else {
                continue;
            };
            self.drive.remove_identity_contender_vote_poll_operations(
                vote_poll,
                votes,
                &mut operations,
                transaction,
                platform_version,
            )?;
            let vote_poll_id = vote_poll.unique_id()?;
            for voters in votes.values() {
                for voter in voters {
                    vote_poll_ids_by_voter
                        .entry(*voter)
                        .or_default()
                        .push(vote_poll_id);
                }
            }
            let (credits, mut empty_specialized_balance_operations) =
                self.drive.empty_prefunded_specialized_balance_operations(
                    vote_poll.specialized_balance_id()?,
                    false,
                    &mut None,
                    transaction,
                    platform_version,
                )?;
            operations.append(&mut empty_specialized_balance_operations);
            total_credits_to_add_to_processing = total_credits_to_add_to_processing
                .checked_add(credits)
                .ok_or(Error::Execution(ExecutionError::Overflow(
                    "Credits from specialized balances are overflowing",
                )))?;
        }

        for (voter, vote_poll_ids) in &vote_poll_ids_by_voter {
            let vote_poll_ids: Vec<&Identifier> = vote_poll_ids.iter().collect();
            self.drive
                .remove_identity_contender_vote_references_given_by_identity_operations(
                    voter,
                    vote_poll_ids.as_slice(),
                    &mut operations,
                    transaction,
                    platform_version,
                )?;
        }

        if total_credits_to_add_to_processing > 0 {
            operations.push(
                self.drive
                    .add_epoch_processing_credits_for_distribution_operation(
                        &block_info.epoch,
                        total_credits_to_add_to_processing,
                        transaction,
                        platform_version,
                    )?,
            );
        }

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
