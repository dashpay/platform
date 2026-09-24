use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::fee::Credits;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use dpp::voting::vote_choices::yes_no_abstain_vote_choice::YesNoAbstainVoteChoice;
use dpp::voting::vote_polls::yes_no_vote_poll::YesNoVotePoll;
use drive::grovedb::TransactionArg;
use std::collections::BTreeMap;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    #[inline(always)]
    pub(super) fn clean_up_after_yes_no_vote_polls_end_v0(
        &self,
        block_info: &BlockInfo,
        vote_polls: Vec<(
            &YesNoVotePoll,
            &BTreeMap<YesNoAbstainVoteChoice, Vec<Identifier>>,
        )>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let mut operations = vec![];

        // The votes and the three vote trees of every poll.
        self.drive.remove_yes_no_vote_poll_votes_operations(
            vote_polls.as_slice(),
            &mut operations,
            transaction,
            platform_version,
        )?;

        // Each voter's entries in the identity votes index.
        let mut poll_ids_by_voter: BTreeMap<&Identifier, Vec<Identifier>> = BTreeMap::new();
        for (vote_poll, voters_by_choice) in &vote_polls {
            let vote_poll_id = vote_poll.unique_id()?;
            for voters in voters_by_choice.values() {
                for voter in voters {
                    poll_ids_by_voter
                        .entry(voter)
                        .or_default()
                        .push(vote_poll_id);
                }
            }
        }
        for (voter, vote_poll_ids) in poll_ids_by_voter {
            let vote_poll_ids: Vec<&Identifier> = vote_poll_ids.iter().collect();
            self.drive
                .remove_yes_no_vote_references_given_by_identity_operations(
                    voter,
                    vote_poll_ids.as_slice(),
                    &mut operations,
                    transaction,
                    platform_version,
                )?;
        }

        // What is left of the prefunded balances goes to processing.
        let mut total_credits_to_add_to_processing: Credits = 0;
        for (vote_poll, _) in &vote_polls {
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
