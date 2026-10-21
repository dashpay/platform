use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::fee::Credits;
use dpp::identifier::Identifier;
use dpp::prelude::TimestampMillis;
use dpp::version::PlatformVersion;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use dpp::ProtocolError;
use drive::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
use drive::fees::op::LowLevelDriveOperation;
use drive::grovedb::TransactionArg;
use std::collections::BTreeMap;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Checks for ended vote polls
    #[inline(always)]
    // TODO: Use type or struct
    #[allow(clippy::type_complexity)]
    pub(super) fn clean_up_after_contested_resources_vote_polls_end_v1(
        &self,
        block_info: &BlockInfo,
        vote_polls: Vec<(
            &ContestedDocumentResourceVotePollWithContractInfo,
            &TimestampMillis,
            &BTreeMap<ResourceVoteChoice, Vec<Identifier>>,
        )>,
        clean_up_testnet_corrupted_reference_issue: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let mut operations = self.clean_up_after_contested_resources_vote_polls_end_operations_v0(
            vote_polls.as_slice(),
            clean_up_testnet_corrupted_reference_issue,
            transaction,
            platform_version,
        )?;

        // We also need to clean out the specialized balances
        self.empty_vote_poll_prefunded_balances_operations_v1(
            vote_polls
                .iter()
                .map(|(vote_poll, _, _)| vote_poll.specialized_balance_id()),
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

    /// Empties the prefunded balances of ended vote polls, in the given order, and credits what
    /// they held to the epoch's processing pool, appending the operations. Moved here unchanged
    /// from the body above, which shipped with protocol version 14, so its operations must not
    /// change; the yes/no clean-up v0 (protocol version 15) drains its polls' balances through it
    /// too.
    pub(in crate::execution::platform_events::voting) fn empty_vote_poll_prefunded_balances_operations_v1(
        &self,
        specialized_balance_ids: impl IntoIterator<Item = Result<Identifier, ProtocolError>>,
        block_info: &BlockInfo,
        operations: &mut Vec<LowLevelDriveOperation>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let mut total_credits_to_add_to_processing: Credits = 0;

        for specialized_balance_id in specialized_balance_ids {
            let (credits, mut empty_specialized_balance_operation) =
                self.drive.empty_prefunded_specialized_balance_operations(
                    specialized_balance_id?,
                    false,
                    &mut None,
                    transaction,
                    platform_version,
                )?;
            operations.append(&mut empty_specialized_balance_operation);
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

        Ok(())
    }
}
